use sdl2::{pixels::Color, render::Canvas, video::Window, Sdl};

use crate::constants;

pub struct Display {
    pub canvas: Canvas<Window>,
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

        let mut canvas = window.into_canvas().build().unwrap();
        canvas.set_draw_color(Color::RGB(
            background_color.0,
            background_color.1,
            background_color.2,
        ));
        canvas.clear();
        canvas.present();

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

    pub fn render_buffer(&mut self, buffer: [bool; constants::DISPLAY_LEN]) {
        for x in 0..constants::DISPLAY_WIDTH {
            for y in 0..constants::DISPLAY_HEIGHT {
                if buffer[x + y * constants::DISPLAY_WIDTH] {
                    self.canvas.set_draw_color(self.foreground_color);
                } else {
                    self.canvas.set_draw_color(self.background_color);
                }

                let x = x as u32 * self.scale;
                let y = y as u32 * self.scale;

                self.canvas
                    .fill_rect(sdl2::rect::Rect::new(
                        x as i32, y as i32, self.scale, self.scale,
                    ))
                    .unwrap();
            }
        }
        self.canvas.present();
    }
}
