mod http_client;
mod url;

#[path = "utils/winit_app.rs"]
mod winit_app;

#[path = "utils/fonts.rs"]
mod fonts;

use fonts::{BrowserFont, FontAndMetadata, FontStyle, FontWeight};
use http_client::get;
use rusttype::{Scale, point};
use softbuffer::{Context, Surface};
use std::{env, num::NonZeroU32};
use winit::{
    event::{Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BOX_SIZE: i16 = 64;

fn main() {
    env_logger::init();

    // parse args

    let args: Vec<String> = env::args().collect();
    println!("args: {:?}", args);
    let url_from_commandline = args.get(1);

    if let None = url_from_commandline {
        println!("no url passed to args: {:?}", args);
    }

    // fetch page
    let response = get(url_from_commandline.unwrap().clone(), None).unwrap();
    println!("response: {:?}", response);

    // render page
    let tokens = lex(response.body);
    println!("tokens: {:?}", tokens);

    let event_loop = EventLoop::new().unwrap();
    let softbuffer_context = Context::new(event_loop.owned_display_handle()).unwrap();
    let scale = Scale::uniform(50.0);

    let mut current_font = BrowserFont::load(scale).expect("failed to load fonts");

    let line_height: f32 = 1.5;

    let app = winit_app::WinitAppBuilder::with_init(
        |elwt| winit_app::make_window(elwt, |w| w),
        move |_elwt, window| Surface::new(&softbuffer_context, window.clone()).unwrap(),
    )
    .with_event_handler(move |window, surface, event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::Resized(size),
            } if window_id == window.id() => {
                let Some(surface) = surface else {
                    eprintln!("Resized fired before Resumed or after Suspended");
                    return;
                };

                if let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                {
                    surface.resize(width, height).unwrap();
                }
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::RedrawRequested,
            } if window_id == window.id() => {
                let Some(surface) = surface else {
                    eprintln!("RedrawRequested fired before Resumed or after Suspended");
                    return;
                };
                let size = window.inner_size();

                println!("{}, {}", size.width, size.height);
                let mut buffer = surface.buffer_mut().unwrap();

                let mut cursor_x: i32 = 0;
                let mut cursor_y: i32 = current_font.current_height();

                for token in &tokens {
                    println!("token: {:?}", token);
                    let FontAndMetadata {
                        font,
                        v_metrics,
                        space_width,
                    } = current_font.get_font_and_metadata();

                    match token {
                        Node::Text(text) => {
                            for word in text.split_whitespace() {
                                let glyphs: Vec<_> =
                                    font.layout(&word, scale, point(0.0, 0.0)).collect();

                                let word_width = glyphs
                                    .iter()
                                    .rev()
                                    .map(|g| {
                                        g.position().x as f32
                                            + g.unpositioned().h_metrics().advance_width
                                    })
                                    .next()
                                    .unwrap_or(0.0)
                                    .floor()
                                    as i32;

                                println!("cursor_x {}, cursor_y {}", cursor_x, cursor_y);
                                for glyph in glyphs {
                                    if let Some(bounds) = glyph.pixel_bounding_box() {
                                        // let w = bounds.width();
                                        if cursor_x + word_width >= (size.width as i32) {
                                            cursor_x = 0;
                                            cursor_y = cursor_y
                                                + (v_metrics.ascent.ceil() * line_height) as i32;
                                        }

                                        glyph.draw(|x, y, v| {
                                            let x = cursor_x + x as i32 + bounds.min.x;
                                            let y = cursor_y + y as i32 + bounds.min.y;
                                            let index = y as i32 * size.width as i32 + x;
                                            let (red, blue, green) = (
                                                (255.0 * v).round() as u32,
                                                (255.0 * v).round() as u32,
                                                (255.0 * v).round() as u32,
                                            );

                                            if index >= buffer.len().try_into().unwrap() {
                                                // not drawing outside of buffer if content doesnt fit
                                                return;
                                            }

                                            buffer[index as usize] =
                                                blue | (green << 8) | (red << 16);
                                        });
                                    } else {
                                        println!("no bb");
                                    }
                                }
                                cursor_x = cursor_x + word_width + space_width;
                            }
                        }
                        Node::Tag(val) if val == "i" => current_font.set_style(FontStyle::Italic),
                        Node::Tag(val) if val == "/i" => current_font.set_style(FontStyle::Roman),
                        Node::Tag(val) if val == "b" => current_font.set_weight(FontWeight::Bold),
                        Node::Tag(val) if val == "/b" => {
                            current_font.set_weight(FontWeight::Normal)
                        }
                        Node::Tag(_) => println!("unknown tag"),
                    }
                }

                buffer.present().unwrap();
            }
            Event::WindowEvent {
                event:
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                logical_key: Key::Named(NamedKey::Escape),
                                ..
                            },
                        ..
                    },
                window_id,
            } if window_id == window.id() => {
                elwt.exit();
            }
            _ => {}
        }
    });

    winit_app::run_app(event_loop, app);
}

#[derive(Debug, PartialEq)]
enum Node {
    Text(String),
    Tag(String),
}

#[test]
fn test_lex() {
    assert_eq!(
        lex(Some("<p>hej</p>".into())),
        vec![
            Node::Tag("p".to_string()),
            Node::Text("hej".to_string()),
            Node::Tag("/p".to_string())
        ]
    )
}
fn lex(body: Option<String>) -> Vec<Node> {
    let mut out: Vec<Node> = Vec::new();
    if let Some(body) = body {
        let mut buffer = String::new();
        let mut in_tag = false;

        for char in body.chars() {
            match (char, in_tag) {
                ('<', _) => {
                    in_tag = true;
                    if !buffer.is_empty() {
                        out.push(Node::Text(buffer));
                    }
                    buffer = String::new();
                }
                ('>', _) => {
                    in_tag = false;
                    out.push(Node::Tag(buffer));
                    buffer = String::new();
                }
                (c, _) => {
                    buffer.push(c);
                }
            }
        }

        if in_tag == false && !buffer.is_empty() {
            out.push(Node::Text(buffer));
        }
    }

    return out;
}
