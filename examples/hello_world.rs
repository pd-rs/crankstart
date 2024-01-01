#![no_std]

extern crate alloc;

use {
    alloc::boxed::Box,
    anyhow::Error,
    crankstart::{
        crankstart_game,
        geometry::{ScreenPoint, ScreenVector},
        graphics::{Graphics, LCDColor, LCDSolidColor},
        log_to_console,
        system::System,
        Game, Playdate,
    },
    crankstart_sys::{LCD_COLUMNS, LCD_ROWS},
    euclid::{point2, vec2},
};

struct State {
    location: ScreenPoint,
    velocity: ScreenVector,
}

impl State {
    pub fn new(_playdate: &Playdate) -> Result<Box<Self>, Error> {
        crankstart::display::Display::get().set_refresh_rate(20.0)?;
        System::get().add_menu_item(
            "Say Hello",
            Box::new(|| {
                log_to_console!("Hello");
            }),
        )?;
        System::get().add_menu_item(
            "Say Goodbye",
            Box::new(|| {
                log_to_console!("Goodbye");
            }),
        )?;
        System::get().add_menu_item(
            "Sausage",
            Box::new(|| {
                log_to_console!("Sausage");
            }),
        )?;
        Ok(Box::new(Self {
            location: point2(INITIAL_X, INITIAL_Y),
            velocity: vec2(1, 2),
        }))
    }
}

impl Game for State {
    fn update(&mut self, _playdate: &mut Playdate) -> Result<(), Error> {
        let graphics = Graphics::get();
        graphics.clear(LCDColor::Solid(LCDSolidColor::kColorWhite))?;
        graphics.draw_text("Hello World Rust", self.location)?;

        self.location += self.velocity;

        if self.location.x < 0 || self.location.x > LCD_COLUMNS as i32 - TEXT_WIDTH {
            self.velocity.x = -self.velocity.x;
        }

        if self.location.y < 0 || self.location.y > LCD_ROWS as i32 - TEXT_HEIGHT {
            self.velocity.y = -self.velocity.y;
        }

        System::get().draw_fps(0, 0)?;

        Ok(())
    }
}

const INITIAL_X: i32 = (400 - TEXT_WIDTH) / 2;
const INITIAL_Y: i32 = (240 - TEXT_HEIGHT) / 2;

const TEXT_WIDTH: i32 = 86;
const TEXT_HEIGHT: i32 = 16;

crankstart_game!(State);
