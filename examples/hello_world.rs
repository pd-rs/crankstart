#![no_std]

extern crate alloc;

use {
    alloc::boxed::Box,
    anyhow::Error,
    crankstart::{
        crankstart_game,
        graphics::{BitmapDrawMode, Font, Graphics, SolidColor},
        system::System,
        Game, Playdate,
    },
    crankstart_sys::{LCDRect, LCD_COLUMNS, LCD_ROWS},
};

struct State {
    x: i32,
    y: i32,
    dx: i32,
    dy: i32,
    graphics: Graphics,
    system: System,
    font: Font,
}

impl State {
    pub fn new(playdate: &Playdate) -> Result<Box<Self>, Error> {
        let system = playdate.system();
        let graphics = playdate.graphics();
        playdate.display().set_refresh_rate(20.0)?;
        let font = graphics.load_font("/System/Fonts/Asheville-Sans-14-Bold.pft")?;
        Ok(Box::new(Self {
            graphics: graphics,
            system: system,
            font: font,
            x: INITIAL_X,
            y: INITIAL_Y,
            dx: 1,
            dy: 2,
        }))
    }
}

impl Game for State {
    fn update(&mut self, _playdate: &mut Playdate) -> Result<(), Error> {
        self.graphics.clear(SolidColor::White)?;
        self.graphics.draw_text(
            &self.font,
            None,
            None,
            "Hello World Rust",
            self.x,
            self.y,
            BitmapDrawMode::Copy,
            0,
            LCDRect {
                left: 0,
                right: 0,
                top: 0,
                bottom: 0,
            },
        )?;

        self.x += self.dx;
        self.y += self.dy;

        if self.x < 0 || self.x > LCD_COLUMNS as i32 - TEXT_WIDTH {
            self.dx = -self.dx;
        }

        if self.y < 0 || self.y > LCD_ROWS as i32 - TEXT_HEIGHT {
            self.dy = -self.dy;
        }

        self.system.draw_fps(0, 0)?;

        Ok(())
    }
}

const INITIAL_X: i32 = (400 - TEXT_WIDTH) / 2;
const INITIAL_Y: i32 = (240 - TEXT_HEIGHT) / 2;

const TEXT_WIDTH: i32 = 86;
const TEXT_HEIGHT: i32 = 16;

crankstart_game!(State);
