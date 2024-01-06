#![no_std]

extern crate alloc;

use crankstart::log_to_console;
use crankstart::sprite::{Sprite, SpriteManager};
use crankstart_sys::{LCDBitmapFlip, PDButtons};
use {
    alloc::boxed::Box,
    anyhow::Error,
    crankstart::{
        crankstart_game,
        geometry::{ScreenPoint, ScreenVector},
        graphics::{Graphics, LCDColor, LCDSolidColor},
        system::System,
        Game, Playdate,
    },
    crankstart_sys::{LCD_COLUMNS, LCD_ROWS},
    euclid::{point2, vec2},
};

struct State {
    location: ScreenPoint,
    velocity: ScreenVector,
    sprite: Sprite,
}

fn load_sprite() -> Result<Sprite, Error> {
    let sprite_manager = SpriteManager::get_mut();
    let mut sprite = sprite_manager.new_sprite()?;
    let image = Graphics::get().load_bitmap("examples/assets/heart")?;
    sprite.set_image(image, LCDBitmapFlip::kBitmapUnflipped)?;
    sprite.move_to(200.0, 120.0)?;
    sprite.set_z_index(10)?;
    sprite.set_opaque(false)?;
    sprite_manager.add_sprite(&sprite)?;
    Ok(sprite)
}

impl State {
    pub fn new(_playdate: &Playdate) -> Result<Box<Self>, Error> {
        crankstart::display::Display::get().set_refresh_rate(20.0)?;
        let sprite = load_sprite()?;
        Ok(Box::new(Self {
            location: point2(INITIAL_X, INITIAL_Y),
            velocity: vec2(1, 2),
            sprite,
        }))
    }
}

impl Game for State {
    fn update(&mut self, _playdate: &mut Playdate) -> Result<(), Error> {
        let graphics = Graphics::get();
        graphics.clear_context()?;
        graphics.clear(LCDColor::Solid(LCDSolidColor::kColorWhite))?;
        graphics.draw_text("Hello World Rust", self.location)?;

        self.location += self.velocity;

        if self.location.x < 0 || self.location.x > LCD_COLUMNS as i32 - TEXT_WIDTH {
            self.velocity.x = -self.velocity.x;
        }

        if self.location.y < 0 || self.location.y > LCD_ROWS as i32 - TEXT_HEIGHT {
            self.velocity.y = -self.velocity.y;
        }

        let (_, pushed, _) = System::get().get_button_state()?;
        if (pushed & PDButtons::kButtonA).0 != 0 {
            log_to_console!("Button A pushed");
            self.sprite
                .set_visible(!self.sprite.is_visible().unwrap_or(false))
                .unwrap();
        }

        System::get().draw_fps(0, 0)?;

        Ok(())
    }

    fn update_sprite(
        &mut self,
        sprite: &mut Sprite,
        _playdate: &mut Playdate,
    ) -> Result<(), Error> {
        sprite.mark_dirty()?;
        Ok(())
    }
}

const INITIAL_X: i32 = (400 - TEXT_WIDTH) / 2;
const INITIAL_Y: i32 = (240 - TEXT_HEIGHT) / 2;

const TEXT_WIDTH: i32 = 86;
const TEXT_HEIGHT: i32 = 16;

crankstart_game!(State);
