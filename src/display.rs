use sdl2::{pixels::Color, render::Canvas, video::Window, Sdl};

use crate::constants;

pub struct Display {
    canvas: Canvas<Window>,
    scale: u32,
    background_color: Color,
    foreground_color: Color,
}

impl Display {
    pub fn build(
        sdl: &Sdl,
        scale: u32,
        background_color: (u8, u8, u8),
        foreground_color: (u8, u8, u8),
    ) -> Self {
        let video_subsystem = sdl.video().unwrap();
        let window = video_subsystem
            .window(
                constants::WINDOW_TITLE,
                constants::DISPLAY_WIDTH as u32 * scale,
                constants::DISPLAY_HEIGHT as u32 * scale,
            )
            .position_centered()
            .build()
            .unwrap();

        let canvas = window.into_canvas().build().unwrap();

        Display {
            canvas,
            scale,
            background_color: Color::RGB(
                background_color.0,
                background_color.1,
                background_color.2,
            ),
            foreground_color: Color::RGB(
                foreground_color.0,
                foreground_color.1,
                foreground_color.2,
            ),
        }
    }

    pub fn render_buffer(&mut self, buffer: [u8; constants::DISPLAY_LEN]) {
        todo!("Implement render_buffer")
    }
}
