#![no_std]

extern crate alloc;

use {
    alloc::{boxed::Box, format, vec::Vec},
    anyhow::Error,
    crankstart::{
        crankstart_game,
        graphics::{
            rect_make, Bitmap, BitmapData, Graphics, LCDBitmapDrawMode, LCDBitmapFlip, LCDRect,
            PDRect,
        },
        log_to_console,
        sprite::{Sprite, SpriteCollider, SpriteManager},
        system::{PDButtons, System},
        Game, Playdate,
    },
    crankstart_sys::SpriteCollisionResponseType,
    euclid::point2,
    randomize::PCG32,
};

const MAX_MAX_ENEMIES: usize = 119;

struct BackgroundHandler {
    background_image: Bitmap,
    y: i32,
    height: i32,
}

#[derive(Debug)]
struct OverlapCollider;

impl SpriteCollider for OverlapCollider {
    fn response_type(&self, _: Sprite, _: Sprite) -> SpriteCollisionResponseType {
        SpriteCollisionResponseType::kCollisionTypeOverlap
    }
}

fn remove_sprite_from_list(sprites: &mut Vec<Sprite>, target: &Sprite) {
    if let Some(pos) = sprites.iter().position(|s| *s == *target) {
        sprites.remove(pos);
    } else {
        log_to_console!("can't find sprite to remove");
    }
}

fn create_explosion(
    x: i32,
    y: i32,
    explosions: &mut Vec<Sprite>,
    explosion_bitmaps: &Vec<Bitmap>,
) -> Result<(), Error> {
    let sprite_manager = SpriteManager::get_mut();
    let mut explosion = sprite_manager.new_sprite()?;
    explosion.set_image(
        explosion_bitmaps[0].clone(),
        LCDBitmapFlip::kBitmapUnflipped,
    )?;
    explosion.move_to(x as f32, y as f32)?;
    explosion.set_tag(SpriteType::ExplosionBase as u8)?;
    explosion.set_z_index(2000)?;
    sprite_manager.add_sprite(&explosion)?;
    explosions.push(explosion);
    Ok(())
}

fn destroy_enemy_plane(
    enemies: &mut Vec<Sprite>,
    target: &Sprite,
    explosions: &mut Vec<Sprite>,
    explosion_bitmaps: &Vec<Bitmap>,
) -> Result<(), Error> {
    let (x, y) = target.get_position()?;
    create_explosion(x, y, explosions, explosion_bitmaps)?;
    remove_sprite_from_list(enemies, target);
    Ok(())
}

impl BackgroundHandler {
    fn update(&mut self, sprite: &mut Sprite) -> Result<(), Error> {
        self.y += 1;
        if self.y > self.height {
            self.y = 0;
        }
        sprite.set_needs_redraw()?;
        Ok(())
    }

    fn draw(&self) -> Result<(), Error> {
        let r = LCDRect {
            left: 0,
            right: 400,
            top: 0,
            bottom: 240,
        };
        self.background_image.draw(
            None,
            None,
            point2(0, self.y),
            LCDBitmapDrawMode::kDrawModeCopy,
            LCDBitmapFlip::kBitmapUnflipped,
            r,
        )?;
        self.background_image.draw(
            None,
            None,
            point2(0, self.y - self.height),
            LCDBitmapDrawMode::kDrawModeCopy,
            LCDBitmapFlip::kBitmapUnflipped,
            r,
        )?;
        Ok(())
    }
}

struct PlayerHandler;

impl PlayerHandler {
    fn update(
        &mut self,
        sprite: &mut Sprite,
        enemies: &mut Vec<Sprite>,
        explosions: &mut Vec<Sprite>,
        explosion_bitmaps: &Vec<Bitmap>,
        _playdate: &Playdate,
    ) -> Result<(), Error> {
        let (current, _, _) = System::get().get_button_state()?;

        let mut dx = 0;
        let mut dy = 0;

        if (current & PDButtons::kButtonUp) == PDButtons::kButtonUp {
            dy = -4;
        } else if (current & PDButtons::kButtonDown) == PDButtons::kButtonDown {
            dy = 4;
        }
        if (current & PDButtons::kButtonLeft) == PDButtons::kButtonLeft {
            dx = -4;
        } else if (current & PDButtons::kButtonRight) == PDButtons::kButtonRight {
            dx = 4;
        }

        let (mut x, mut y) = sprite.get_position()?;

        x += dx;
        y += dy;

        let (_, _, collisions) = sprite.move_with_collisions(x as f32, y as f32)?;

        for collision in collisions.iter() {
            let tag = collision.other.get_tag()?;
            if tag == SpriteType::EnemyPlane as u8 {
                destroy_enemy_plane(enemies, &collision.other, explosions, explosion_bitmaps)?;
            }
        }

        Ok(())
    }
}

struct BulletHandler {
    bullet_image_data: BitmapData,
}

impl BulletHandler {
    fn update(
        &mut self,
        bullets: &mut Vec<Sprite>,
        enemies: &mut Vec<Sprite>,
        explosions: &mut Vec<Sprite>,
        explosion_bitmaps: &Vec<Bitmap>,
        sprite: &mut Sprite,
    ) -> Result<(), Error> {
        fn remove_bullet(bullets: &mut Vec<Sprite>, sprite: &mut Sprite) {
            if let Some(pos) = bullets.iter().position(|bullet| *bullet == *sprite) {
                bullets.remove(pos);
            } else {
                log_to_console!("can't find bullet to remove");
            }
        }

        let (x, y) = sprite.get_position()?;
        let new_y = y - 20;
        if new_y < -self.bullet_image_data.height {
            remove_bullet(bullets, sprite);
        } else {
            let (_, _, collisions) = sprite.move_with_collisions(x as f32, new_y as f32)?;
            for collision in collisions.iter() {
                let tag = collision.other.get_tag()?;
                if tag == SpriteType::EnemyPlane as u8 {
                    remove_bullet(bullets, sprite);
                    destroy_enemy_plane(enemies, &collision.other, explosions, explosion_bitmaps)?;
                }
            }
        }
        Ok(())
    }
}

struct EnemyPlaneHandler {
    enemy_image_data: BitmapData,
}

impl EnemyPlaneHandler {
    fn update(&mut self, enemies: &mut Vec<Sprite>, sprite: &mut Sprite) -> Result<(), Error> {
        let (x, y) = sprite.get_position()?;
        let new_y = y + 4;
        if new_y > 400 + self.enemy_image_data.height {
            if let Some(pos) = enemies.iter().position(|enemy| *enemy == *sprite) {
                enemies.remove(pos);
            } else {
                log_to_console!("can't find enemy to remove");
            }
        } else {
            sprite.move_to(x as f32, new_y as f32)?;
        }
        Ok(())
    }
}

struct BackgroundPlaneHandler {
    background_plane_image_data: BitmapData,
}

impl BackgroundPlaneHandler {
    fn update(
        &mut self,
        background_planes: &mut Vec<Sprite>,
        sprite: &mut Sprite,
    ) -> Result<(), Error> {
        let (x, y) = sprite.get_position()?;
        let new_y = y + 2;
        if new_y > 400 + self.background_plane_image_data.height {
            if let Some(pos) = background_planes.iter().position(|p| *p == *sprite) {
                background_planes.remove(pos);
            } else {
                log_to_console!("can't find enemy to remove");
            }
        } else {
            sprite.move_to(x as f32, new_y as f32)?;
        }
        Ok(())
    }
}

struct ExplosionHandler {}

impl ExplosionHandler {
    fn update(
        &mut self,
        explosion_bitmaps: &Vec<Bitmap>,
        explosions: &mut Vec<Sprite>,
        sprite: &mut Sprite,
    ) -> Result<(), Error> {
        let frame_number = (sprite.get_tag()? - SpriteType::ExplosionBase as u8 + 1) as usize;
        if frame_number >= explosion_bitmaps.len() {
            remove_sprite_from_list(explosions, sprite);
        } else {
            sprite.set_image(
                explosion_bitmaps[frame_number].clone(),
                LCDBitmapFlip::kBitmapUnflipped,
            )?;
            sprite.set_tag(SpriteType::ExplosionBase as u8 + frame_number as u8)?;
        }
        Ok(())
    }
}

#[repr(u8)]
enum SpriteType {
    Player = 0,
    PlayerBullet = 1,
    EnemyPlane = 2,
    Background = 3,
    BackgroundPlane = 4,
    ExplosionBase = 5,
}

impl From<u8> for SpriteType {
    fn from(tag: u8) -> Self {
        let sprite_type = match tag {
            0 => SpriteType::Player,
            1 => SpriteType::PlayerBullet,
            2 => SpriteType::EnemyPlane,
            3 => SpriteType::Background,
            4 => SpriteType::BackgroundPlane,
            _ => SpriteType::ExplosionBase,
        };
        sprite_type
    }
}

struct SpriteGame {
    rng: PCG32,
    #[allow(unused)]
    background: Sprite,
    background_handler: BackgroundHandler,
    player: Sprite,
    player_handler: PlayerHandler,
    bullet_image: Bitmap,
    bullet_handler: BulletHandler,
    bullets: Vec<Sprite>,
    enemy_plane_image: Bitmap,
    enemy_plane_handler: EnemyPlaneHandler,
    enemies: Vec<Sprite>,
    background_plane_image: Bitmap,
    background_plane_handler: BackgroundPlaneHandler,
    background_planes: Vec<Sprite>,
    explosion_handler: ExplosionHandler,
    explosions: Vec<Sprite>,
    explosion_bitmaps: Vec<Bitmap>,
    max_enemies: usize,
    max_background_planes: usize,
}

impl SpriteGame {
    fn new(_playdate: &mut Playdate) -> Result<Box<Self>, Error> {
        let graphics = Graphics::get();
        crankstart::display::Display::get().set_refresh_rate(20.0)?;
        // setup background
        let sprite_manager = SpriteManager::get_mut();
        let mut background = sprite_manager.new_sprite()?;
        let background_image = graphics.load_bitmap("sprite_game_images/background")?;
        let background_image_data = background_image.get_data()?;
        let bounds = rect_make(0.0, 0.0, 400.0, 240.0);
        background.set_bounds(&bounds)?;
        background.set_z_index(0)?;
        background.set_tag(SpriteType::Background as u8)?;
        background.set_use_custom_draw()?;
        sprite_manager.add_sprite(&background)?;
        let background_handler = BackgroundHandler {
            background_image,
            height: background_image_data.height,
            y: 0,
        };

        // setup player
        let mut player = sprite_manager.new_sprite()?;
        let player_image = graphics.load_bitmap("sprite_game_images/player")?;
        let player_image_data = player_image.get_data()?;
        player.set_image(player_image, LCDBitmapFlip::kBitmapUnflipped)?;
        let center_x: i32 = 200 - player_image_data.width / 2;
        let center_y: i32 = 180 - player_image_data.height / 2;
        let cr = rect_make(
            5.0,
            5.0,
            player_image_data.width as f32 - 10.0,
            player_image_data.height as f32 - 10.0,
        );
        player.set_collide_rect(&cr)?;
        player.set_collision_response_type(Some(Box::new(OverlapCollider {})))?;
        player.set_tag(SpriteType::Player as u8)?;

        player.move_to(center_x as f32, center_y as f32)?;

        let bullet_image = graphics.load_bitmap("sprite_game_images/doubleBullet")?;
        let bullet_image_data = bullet_image.get_data()?;

        let enemy_plane_image = graphics.load_bitmap("sprite_game_images/plane1")?;
        let enemy_image_data = enemy_plane_image.get_data()?;
        let enemy_plane_handler = EnemyPlaneHandler { enemy_image_data };

        let background_plane_image = graphics.load_bitmap("sprite_game_images/plane2")?;
        let background_plane_image_data = background_plane_image.get_data()?;
        let background_plane_handler = BackgroundPlaneHandler {
            background_plane_image_data,
        };

        let explosion_handler = ExplosionHandler {};
        let explosion_bitmaps: Vec<Bitmap> = (1..12)
            .flat_map(|index| {
                graphics.load_bitmap(&format!("sprite_game_images/explosion/{}", index))
            })
            .collect();

        let rng = PCG32::seed(1, 1);
        let mut sprite_game = Self {
            rng,
            background,
            background_handler,
            player,
            player_handler: PlayerHandler {},
            bullet_handler: BulletHandler { bullet_image_data },
            bullet_image,
            bullets: Vec::with_capacity(32),
            enemy_plane_image,
            enemy_plane_handler,
            enemies: Vec::with_capacity(32),
            background_plane_image,
            background_plane_handler,
            background_planes: Vec::with_capacity(32),
            explosion_handler,
            explosions: Vec::with_capacity(32),
            explosion_bitmaps,
            max_enemies: 10,
            max_background_planes: 10,
        };
        sprite_game.setup()?;
        Ok(Box::new(sprite_game))
    }

    fn setup(&mut self) -> Result<(), Error> {
        SpriteManager::get_mut().add_sprite(&self.player)?;
        self.player.set_z_index(1000)?;
        Ok(())
    }

    fn player_fire(&mut self) -> Result<(), Error> {
        let sprite_manager = SpriteManager::get_mut();
        let player_bounds = self.player.get_bounds()?;
        let bullet_image_data = self.bullet_image.get_data()?;
        let x: i32 =
            player_bounds.x as i32 + player_bounds.width as i32 / 2 - bullet_image_data.width / 2;
        let y: i32 = player_bounds.y as i32;

        let mut bullet = sprite_manager.new_sprite()?;
        bullet.set_image(self.bullet_image.clone(), LCDBitmapFlip::kBitmapUnflipped)?;
        let cr = rect_make(
            0.0,
            0.0,
            bullet_image_data.width as f32,
            bullet_image_data.height as f32,
        );
        bullet.set_collide_rect(&cr)?;
        bullet.set_collision_response_type(Some(Box::new(OverlapCollider {})))?;
        bullet.move_to(x as f32, y as f32)?;
        bullet.set_z_index(999)?;
        sprite_manager.add_sprite(&bullet)?;
        bullet.set_tag(SpriteType::PlayerBullet as u8)?;
        self.bullets.push(bullet);
        Ok(())
    }

    fn check_buttons(&mut self, _playdate: &mut Playdate) -> Result<(), Error> {
        let (_, pushed, _) = System::get().get_button_state()?;
        if (pushed & PDButtons::kButtonA) == PDButtons::kButtonA
            || (pushed & PDButtons::kButtonB) == PDButtons::kButtonB
        {
            self.player_fire()?;
        }
        Ok(())
    }

    fn check_crank(&mut self, _playdate: &mut Playdate) -> Result<(), Error> {
        let change = System::get().get_crank_change()? as i32;

        if change > 1 {
            self.max_enemies += 1;
            if self.max_enemies > MAX_MAX_ENEMIES {
                self.max_enemies = MAX_MAX_ENEMIES;
            }
            log_to_console!("Maximum number of enemy planes: {}", self.max_enemies);
        } else if change < -1 {
            self.max_enemies = self.max_enemies.saturating_sub(1);
            log_to_console!("Maximum number of enemy planes: {}", self.max_enemies);
        }
        Ok(())
    }

    fn create_enemy_plane(&mut self) -> Result<(), Error> {
        let sprite_manager = SpriteManager::get_mut();
        let mut plane = sprite_manager.new_sprite()?;
        plane.set_collision_response_type(Some(Box::new(OverlapCollider {})))?;
        let plane_image_data = self.enemy_plane_image.get_data()?;
        plane.set_image(
            self.enemy_plane_image.clone(),
            LCDBitmapFlip::kBitmapUnflipped,
        )?;
        let cr = rect_make(
            0.0,
            0.0,
            plane_image_data.width as f32,
            plane_image_data.height as f32,
        );
        plane.set_collide_rect(&cr)?;
        let x = (self.rng.next_u32() % 400) as i32 - plane_image_data.width / 2;
        let y = -((self.rng.next_u32() % 30) as i32) - plane_image_data.height;

        plane.move_to(x as f32, y as f32)?;
        plane.set_z_index(500)?;
        plane.set_tag(SpriteType::EnemyPlane as u8)?;
        sprite_manager.add_sprite(&plane)?;
        self.enemies.push(plane);
        Ok(())
    }

    fn spawn_enemy_if_needed(&mut self) -> Result<(), Error> {
        if self.enemies.len() < self.max_enemies {
            let rand_v = self.rng.next_u32() as usize;
            if rand_v % (120 / self.max_enemies) == 0 {
                self.create_enemy_plane()?;
            }
        }
        Ok(())
    }

    fn create_background_plane(&mut self) -> Result<(), Error> {
        let sprite_manager = SpriteManager::get_mut();
        let mut plane = sprite_manager.new_sprite()?;
        let plane_image_data = self.background_plane_image.get_data()?;
        plane.set_image(
            self.background_plane_image.clone(),
            LCDBitmapFlip::kBitmapUnflipped,
        )?;
        let x = (self.rng.next_u32() % 400) as i32 - plane_image_data.width / 2;
        let y = -plane_image_data.height;
        plane.move_to(x as f32, y as f32)?;
        plane.set_tag(SpriteType::BackgroundPlane as u8)?;
        plane.set_z_index(100)?;
        sprite_manager.add_sprite(&plane)?;
        self.background_planes.push(plane);
        Ok(())
    }

    fn spawn_background_plane_if_needed(&mut self) -> Result<(), Error> {
        if self.background_planes.len() < self.max_background_planes {
            let rand_v = self.rng.next_u32() as usize;
            if rand_v % (120 / self.max_background_planes) == 0 {
                self.create_background_plane()?;
            }
        }
        Ok(())
    }
}

impl Game for SpriteGame {
    fn update_sprite(&mut self, sprite: &mut Sprite, playdate: &mut Playdate) -> Result<(), Error> {
        let tag = sprite.get_tag()?.into();
        match tag {
            SpriteType::Background => self.background_handler.update(sprite)?,
            SpriteType::Player => self.player_handler.update(
                sprite,
                &mut self.enemies,
                &mut self.explosions,
                &self.explosion_bitmaps,
                playdate,
            )?,
            SpriteType::PlayerBullet => self.bullet_handler.update(
                &mut self.bullets,
                &mut self.enemies,
                &mut self.explosions,
                &self.explosion_bitmaps,
                sprite,
            )?,
            SpriteType::EnemyPlane => self.enemy_plane_handler.update(&mut self.enemies, sprite)?,
            SpriteType::BackgroundPlane => self
                .background_plane_handler
                .update(&mut self.background_planes, sprite)?,
            _ => {
                self.explosion_handler.update(
                    &self.explosion_bitmaps,
                    &mut self.explosions,
                    sprite,
                )?;
            }
        }
        Ok(())
    }

    fn draw_sprite(
        &self,
        sprite: &Sprite,
        _bounds: &PDRect,
        _draw_rect: &LCDRect,
        _playdate: &Playdate,
    ) -> Result<(), Error> {
        let tag = sprite.get_tag()?.into();
        match tag {
            SpriteType::Background => self.background_handler.draw()?,
            _ => (),
        }
        Ok(())
    }

    fn update(&mut self, playdate: &mut Playdate) -> Result<(), Error> {
        self.check_buttons(playdate)?;
        self.check_crank(playdate)?;
        self.spawn_enemy_if_needed()?;
        self.spawn_background_plane_if_needed()?;
        Ok(())
    }

    fn draw_fps(&self) -> bool {
        true
    }
}

crankstart_game!(SpriteGame);
