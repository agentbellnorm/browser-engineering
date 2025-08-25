mod http_client;
mod url;

#[path = "utils/winit_app.rs"]
mod winit_app;

#[path = "utils/fonts.rs"]
mod fonts;

use fonts::{BrowserFont, FontAndMetadata, FontStyle, FontWeight};
use http_client::get;
use rusttype::{PositionedGlyph, Scale, point};
use softbuffer::{Context, Surface};
use std::{env, num::NonZeroU32};
use winit::{
    dpi::PhysicalSize,
    event::{Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
};

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
    let browser_font = BrowserFont::load(scale).expect("failed to load fonts");

    let app = winit_app::WinitAppBuilder::with_init(
        |elwt| winit_app::make_window(elwt, |w| w),
        move |_elwt, window| Surface::new(&softbuffer_context, window.clone()).unwrap(),
    )
    .with_event_handler(move |window, surface, event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::RedrawRequested,
            } if window_id == window.id() => {
                let Some(surface) = surface else {
                    eprintln!("RedrawRequested fired before Resumed or after Suspended");
                    return;
                };
                let size = window.inner_size();
                let display_list = layout(&tokens, size, &browser_font, scale);

                println!("{}, {}", size.width, size.height);
                let mut buffer = surface.buffer_mut().unwrap();

                for display_item in display_list {
                    let DisplayItem {
                        x: item_x,
                        y: item_y,
                        glyphs,
                    } = display_item;

                    for glyph in glyphs {
                        glyph.draw(|x, y, v| {
                            if let Some(bounds) = glyph.pixel_bounding_box() {
                                let x = item_x + x as i32 + bounds.min.x;
                                let y = item_y + y as i32 + bounds.min.y;
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

                                buffer[index as usize] = blue | (green << 8) | (red << 16);
                            } else {
                                println!("no bb");
                            }
                        });
                    }
                }

                buffer.present().unwrap();
            }
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

struct DisplayItem<'a> {
    x: i32,
    y: i32,
    glyphs: Vec<PositionedGlyph<'a>>,
}

type DisplayList<'a> = Vec<DisplayItem<'a>>;

fn layout<'a>(
    tokens: &Vec<Node>,
    size: PhysicalSize<u32>,
    browser_font: &'a BrowserFont,
    scale: Scale,
) -> DisplayList<'a> {
    let line_height = 1.5;

    let mut display_list = DisplayList::new();

    let mut cursor_x: i32 = 0;
    let mut cursor_y: i32 = browser_font.roman.v_metrics.ascent.floor() as i32;

    let mut font_style = FontStyle::Roman;
    let mut font_weight = FontWeight::Normal;

    for token in tokens {
        match token {
            Node::Text(text) => {
                let FontAndMetadata {
                    font,
                    v_metrics,
                    space_width,
                } = match (&font_style, &font_weight) {
                    (FontStyle::Roman, FontWeight::Normal) => &browser_font.roman,
                    (FontStyle::Roman, FontWeight::Bold) => &browser_font.bold,
                    (FontStyle::Italic, FontWeight::Normal) => &browser_font.italic,
                    (FontStyle::Italic, FontWeight::Bold) => &browser_font.bold_italic,
                };

                for word in text.split_whitespace() {
                    let glyphs: Vec<_> = font.layout(word, scale, point(0.0, 0.0)).collect();

                    let word_width = glyphs
                        .iter()
                        .rev()
                        .map(|g| g.position().x as f32 + g.unpositioned().h_metrics().advance_width)
                        .next()
                        .unwrap_or(0.0)
                        .floor() as i32;

                    if cursor_x + word_width >= (size.width as i32) {
                        cursor_x = 0;
                        cursor_y = cursor_y + (v_metrics.ascent.ceil() * line_height) as i32;
                    }

                    display_list.push(DisplayItem {
                        x: cursor_x,
                        y: cursor_y,
                        glyphs,
                    });

                    cursor_x = cursor_x + word_width + space_width;
                }
            }
            Node::Tag(val) if val == "i" => font_style = FontStyle::Italic,

            Node::Tag(val) if val == "/i" => font_style = FontStyle::Roman,

            Node::Tag(val) if val == "b" => font_weight = FontWeight::Bold,
            Node::Tag(val) if val == "/b" => font_weight = FontWeight::Normal,

            Node::Tag(_) => println!("unknown tag"),
        }
    }

    display_list
}
