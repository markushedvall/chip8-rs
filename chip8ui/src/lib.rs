extern crate sdl2_window;
extern crate opengl_graphics;
extern crate piston;
extern crate graphics;
extern crate chip8core;
mod pixel;

use sdl2_window::Sdl2Window;
use piston::event_loop::*;
use piston::input::*;
use piston::window::WindowSettings;
use graphics::Rectangle;
use opengl_graphics::{
    GlGraphics,
    OpenGL,
};
use pixel::Pixel;
use chip8core::Key as Chip8Key;

pub struct Runner {}

impl Runner {
    pub fn run<T: chip8core::Vm>(vm: &mut T) -> Result<(), String> {
        let mut pixels: [Pixel; 64 * 32] = [Default::default(); 64 * 32];

        vm.load_rom(&mut std::io::stdin()).unwrap();

        let (width, height) = (800, 400);
        let opengl = OpenGL::V3_2;

        let mut window: Sdl2Window = WindowSettings::new("Chip8", (width, height))
            .fullscreen(true)
            .exit_on_esc(true)
            .opengl(opengl)
            .build()
            .unwrap();

        let ref mut gl = GlGraphics::new(opengl);

        let mut events = window.events();
        while let Some(e) = events.next(&mut window) {
            if let Some(args) = e.update_args() {
                vm.step(args.dt).unwrap();
                for p in pixels.iter_mut() {
                    p.update(args.dt);
                }
            }

            if let Some(args) = e.render_args() {
                gl.draw(args.viewport(), |c, gl| {
                        graphics::clear([1.0, 1.0, 1.0, 1.0], gl);
                        let r = Rectangle::new([1.0, 1.0, 1.0, 1.0]);
                        let w = args.width as f64 / 64.0;
                        let h = args.height as f64 / 32.0;

                        for (y_row, row) in vm.pixels().enumerate() {
                            for (x_col, on) in row.iter().enumerate() {
                                let x = x_col as f64 * w;
                                let y = y_row as f64 * h;
                                if *on {
                                    pixels[y_row * 64 + x_col].turn_on();
                                }
                                let color = pixels[y_row * 64 + x_col].color_arr();
                                r.color(color).draw([x, y, w, h], &c.draw_state, c.transform, gl);
                            }
                        }
                    }
                );
            }

            if let Some(button) = e.press_args() {
                if let Some(key) = chip8_key_from_button(button) {
                    vm.press_key(key);
                }
            }

            if let Some(button) = e.release_args() {
                if let Some(key) = chip8_key_from_button(button) {
                    vm.release_key(key);
                }
            }
        }

        Ok(())
    }
}

fn chip8_key_from_button(button: Button) -> Option<Chip8Key> {
    if let Button::Keyboard(key) = button {
        return match key {
            Key::D1 => Some(Chip8Key::D1),
            Key::D2 => Some(Chip8Key::D2),
            Key::D3 => Some(Chip8Key::D3),
            Key::Q  => Some(Chip8Key::D4),
            Key::W  => Some(Chip8Key::D5),
            Key::E  => Some(Chip8Key::D6),
            Key::A  => Some(Chip8Key::D7),
            Key::S  => Some(Chip8Key::D8),
            Key::D  => Some(Chip8Key::D9),
            Key::Z  => Some(Chip8Key::A),
            Key::X  => Some(Chip8Key::D0),
            Key::C  => Some(Chip8Key::B),
            Key::D4 => Some(Chip8Key::C),
            Key::R  => Some(Chip8Key::D),
            Key::F  => Some(Chip8Key::E),
            Key::V  => Some(Chip8Key::F),
            _ => None,
        }
    }
    None
}
