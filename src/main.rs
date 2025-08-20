mod http_client;
mod url;

#[path = "utils/winit_app.rs"]
mod winit_app;

use http_client::get;
use log::info;
use rusttype::{Font, Scale};
use softbuffer::{Context, Surface};
use std::{env, num::NonZeroU32};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::Window,
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const BOX_SIZE: i16 = 64;

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let softbuffer_context = Context::new(event_loop.owned_display_handle()).unwrap();

    let font_data = include_bytes!("./B612-Regular.ttf");
    let font = Font::try_from_bytes(font_data).unwrap();
    let scale = Scale::uniform(50.0);
    let v_metrics = font.v_metrics(scale);

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

                let glyphs = font.layout(
                    "HeJsansvejsan",
                    scale,
                    rusttype::point(0.0, v_metrics.ascent),
                );

                for glyph in glyphs {
                    let bounds = glyph.pixel_bounding_box().unwrap();
                    glyph.draw(|x, y, v| {
                        println!("{}", v);
                        let x = x + bounds.min.x as u32;
                        let y = y + bounds.min.y as u32;
                        let index = y * size.width + x;
                        let (red, blue, green) = (
                            255 * v.round() as u32,
                            255 * v.round() as u32,
                            255 * v.round() as u32,
                        );
                        buffer[index as usize] = blue | (green << 8) | (red << 16);
                    });
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

    let args: Vec<String> = env::args().collect();
    println!("args: {:?}", args);
    let url_from_commandline = args.get(1);

    if let None = url_from_commandline {
        println!("no url passed to args: {:?}", args);
    }

    let response = get(url_from_commandline.unwrap().clone(), None).unwrap();
    println!("response: {:?}", response);

    if let Some(body) = response.body {
        let mut in_tag = false;

        for char in body.chars() {
            match (char, in_tag) {
                ('<', _) => in_tag = true,
                ('>', _) => in_tag = false,
                (c, false) => {
                    print!("{c}")
                }
                _ => {}
            }
        }
    }
}
