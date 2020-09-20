extern crate alloc;

use {
    crate::{
        graphics::{Bitmap, LCDBitmapFlip, PDRect},
        log_to_console, pd_func_caller, pd_func_caller_log,
        system::System,
        Playdate,
    },
    alloc::{
        boxed::Box,
        collections::BTreeMap,
        rc::{Rc, Weak},
    },
    anyhow::{anyhow, Error, Result},
    core::{
        cell::RefCell,
        fmt::Debug,
        hash::{Hash, Hasher},
        slice,
    },
    crankstart_sys::{
        playdate_sprite, LCDRect, LCDSprite, LCDSpriteCollisionFilterProc, SpriteCollisionInfo,
    },
    hashbrown::HashMap,
};

pub use crankstart_sys::SpriteCollisionResponseType;

pub type SpriteUpdateFunction = unsafe extern "C" fn(sprite: *mut crankstart_sys::LCDSprite);
pub type SpriteDrawFunction = unsafe extern "C" fn(
    sprite: *mut crankstart_sys::LCDSprite,
    bounds: PDRect,
    frame: *mut u8,
    drawrect: LCDRect,
);
pub type SpriteCollideFunction = unsafe extern "C" fn(
    sprite: *mut crankstart_sys::LCDSprite,
    other: *mut crankstart_sys::LCDSprite,
) -> SpriteCollisionResponseType;

static mut SPRITE_UPDATE: Option<SpriteUpdateFunction> = None;
static mut SPRITE_DRAW: Option<SpriteDrawFunction> = None;

pub trait SpriteCollider: Debug + 'static {
    fn response_type(&self, sprite: Sprite, other: Sprite) -> SpriteCollisionResponseType;
}

pub type SpriteCollisionResponses =
    HashMap<*const crankstart_sys::LCDSprite, Box<dyn SpriteCollider>>;

static mut SPRITE_COLLISION_RESPONSES: Option<SpriteCollisionResponses> = None;
static mut SPRITE_MANAGER: Option<SpriteManager> = None;

pub struct Collisions(*mut SpriteCollisionInfo, crankstart_sys::ctypes::c_int);

impl Collisions {
    pub fn iter(&self) -> CollisionInfoIter<'_> {
        CollisionInfoIter {
            collisions: self,
            index: 0,
        }
    }
}

#[derive(Debug)]
pub struct CollisionInfo<'a> {
    pub sprite: Sprite,
    pub other: Sprite,
    pub info: &'a SpriteCollisionInfo,
}

pub struct CollisionInfoIter<'a> {
    collisions: &'a Collisions,
    index: usize,
}

impl<'a> Iterator for CollisionInfoIter<'a> {
    type Item = CollisionInfo<'a>;

    fn next(&mut self) -> Option<CollisionInfo<'a>> {
        if self.index >= self.collisions.1 as usize {
            None
        } else {
            let index = self.index;
            self.index += 1;
            let collision_slice =
                unsafe { slice::from_raw_parts(self.collisions.0, self.collisions.1 as usize) };

            let sprite_manager = SpriteManager::get_mut();
            let sprite = sprite_manager.get_sprite(collision_slice[index].sprite);
            let other = sprite_manager.get_sprite(collision_slice[index].other);
            if sprite.is_none() || other.is_none() {
                return None;
            }
            let sprite = sprite.unwrap();
            let other = other.unwrap();
            let collision_info = CollisionInfo {
                sprite: sprite,
                other: other,
                info: &collision_slice[index],
            };
            Some(collision_info)
        }
    }
}

impl Drop for Collisions {
    fn drop(&mut self) {
        System::get().realloc(self.0 as *mut core::ffi::c_void, 0);
    }
}

pub struct SpriteInner {
    pub raw_sprite: *mut crankstart_sys::LCDSprite,
    playdate_sprite: *mut playdate_sprite,
    image: Option<Bitmap>,
}

pub type SpritePtr = Rc<RefCell<SpriteInner>>;
pub type SpriteWeakPtr = Weak<RefCell<SpriteInner>>;

extern "C" fn get_sprite_collision_response(
    sprite: *mut crankstart_sys::LCDSprite,
    other: *mut crankstart_sys::LCDSprite,
) -> SpriteCollisionResponseType {
    if let Some(collision_responses) = unsafe { SPRITE_COLLISION_RESPONSES.as_ref() } {
        let collider = collision_responses.get(&(sprite as *const crankstart_sys::LCDSprite));
        if let Some(collider) = collider {
            if let Some(sprite) = SpriteManager::get_sprite_static(sprite) {
                if let Some(other) = SpriteManager::get_sprite_static(other) {
                    return collider.response_type(sprite, other);
                }
            }
        }
    }

    SpriteCollisionResponseType::kCollisionTypeOverlap
}

impl SpriteInner {
    pub fn set_use_custom_draw(&mut self) -> Result<(), Error> {
        self.set_draw_function(unsafe { SPRITE_DRAW.expect("SPRITE_DRAW") })
    }

    pub fn set_collision_response_type(
        &mut self,
        response_type: Option<Box<dyn SpriteCollider>>,
    ) -> Result<(), Error> {
        if let Some(response_type) = response_type {
            unsafe {
                if let Some(collision_responses) = SPRITE_COLLISION_RESPONSES.as_mut() {
                    collision_responses.insert(self.raw_sprite, response_type);
                } else {
                    log_to_console!("Can't access SPRITE_COLLISION_RESPONSES");
                }
            }
            self.set_collision_response_function(Some(get_sprite_collision_response))?;
        } else {
            self.set_collision_response_function(None)?;
            unsafe {
                if let Some(collision_responses) = SPRITE_COLLISION_RESPONSES.as_mut() {
                    collision_responses
                        .remove(&(self.raw_sprite as *const crankstart_sys::LCDSprite));
                } else {
                    log_to_console!("Can't access SPRITE_COLLISION_RESPONSES");
                }
            }
        }
        Ok(())
    }

    fn set_update_function(&self, f: SpriteUpdateFunction) -> Result<(), Error> {
        pd_func_caller!(
            (*self.playdate_sprite).setUpdateFunction,
            self.raw_sprite,
            Some(f)
        )
    }

    fn set_draw_function(&self, f: SpriteDrawFunction) -> Result<(), Error> {
        pd_func_caller!(
            (*self.playdate_sprite).setDrawFunction,
            self.raw_sprite,
            Some(f)
        )
    }

    fn set_collision_response_function(
        &self,
        f: LCDSpriteCollisionFilterProc,
    ) -> Result<(), Error> {
        pd_func_caller!(
            (*self.playdate_sprite).setCollisionResponseFunction,
            self.raw_sprite,
            f
        )
    }

    pub fn get_bounds(&self) -> Result<PDRect, Error> {
        pd_func_caller!((*self.playdate_sprite).getBounds, self.raw_sprite)
    }

    pub fn set_bounds(&self, bounds: &PDRect) -> Result<(), Error> {
        pd_func_caller!((*self.playdate_sprite).setBounds, self.raw_sprite, *bounds)
    }

    pub fn get_z_index(&self) -> Result<i16, Error> {
        pd_func_caller!((*self.playdate_sprite).getZIndex, self.raw_sprite)
    }

    pub fn set_z_index(&self, z_index: i16) -> Result<(), Error> {
        pd_func_caller!((*self.playdate_sprite).setZIndex, self.raw_sprite, z_index)
    }

    pub fn set_image(&mut self, bitmap: Bitmap, flip: LCDBitmapFlip) -> Result<(), Error> {
        pd_func_caller!(
            (*self.playdate_sprite).setImage,
            self.raw_sprite,
            bitmap.inner.borrow().raw_bitmap,
            flip.into()
        )?;
        self.image = Some(bitmap);
        Ok(())
    }

    pub fn set_tag(&mut self, tag: u8) -> Result<(), Error> {
        pd_func_caller!((*self.playdate_sprite).setTag, self.raw_sprite, tag)
    }

    pub fn get_tag(&self) -> Result<u8, Error> {
        pd_func_caller!((*self.playdate_sprite).getTag, self.raw_sprite)
    }

    pub fn set_needs_redraw(&self) -> Result<(), Error> {
        pd_func_caller!((*self.playdate_sprite).setNeedsRedraw, self.raw_sprite)
    }

    pub fn move_to(&mut self, x: f32, y: f32) -> Result<(), Error> {
        pd_func_caller!((*self.playdate_sprite).moveTo, self.raw_sprite, x, y)
    }

    pub fn get_position(&self) -> Result<(f32, f32), Error> {
        let mut x = 0.0;
        let mut y = 0.0;
        pd_func_caller!(
            (*self.playdate_sprite).getPosition,
            self.raw_sprite,
            &mut x,
            &mut y
        )?;
        Ok((x, y))
    }

    pub fn set_collide_rect(&mut self, collide_rect: &PDRect) -> Result<(), Error> {
        pd_func_caller!(
            (*self.playdate_sprite).setCollideRect,
            self.raw_sprite,
            *collide_rect,
        )
    }

    pub fn move_with_collisions(
        &mut self,
        goal_x: f32,
        goal_y: f32,
    ) -> Result<(f32, f32, Collisions), Error> {
        let mut actual_x = 0.0;
        let mut actual_y = 0.0;
        let mut count = 0;
        let raw_collision_info = pd_func_caller!(
            (*self.playdate_sprite).moveWithCollisions,
            self.raw_sprite,
            goal_x,
            goal_y,
            &mut actual_x,
            &mut actual_y,
            &mut count,
        )?;
        Ok((actual_x, actual_y, Collisions(raw_collision_info, count)))
    }
}

impl Drop for SpriteInner {
    fn drop(&mut self) {
        pd_func_caller_log!((*self.playdate_sprite).freeSprite, self.raw_sprite);
        unsafe {
            if let Some(collision_responses) = SPRITE_COLLISION_RESPONSES.as_mut() {
                collision_responses.remove(&(self.raw_sprite as *const crankstart_sys::LCDSprite));
            }
        }
    }
}

impl Debug for SpriteInner {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::result::Result<(), core::fmt::Error> {
        f.debug_struct("Sprite")
            .field("raw_sprite", &self.raw_sprite)
            .finish()
    }
}

impl PartialEq for SpriteInner {
    fn eq(&self, other: &Self) -> bool {
        self.raw_sprite == other.raw_sprite
    }
}

#[derive(Clone, Debug)]
pub struct Sprite {
    inner: SpritePtr,
}

impl Sprite {
    pub fn set_use_custom_draw(&mut self) -> Result<(), Error> {
        self.inner
            .try_borrow_mut()
            .map_err(Error::msg)?
            .set_use_custom_draw()
    }

    pub fn set_collision_response_type(
        &mut self,
        response_type: Option<Box<dyn SpriteCollider>>,
    ) -> Result<(), Error> {
        self.inner
            .try_borrow_mut()
            .map_err(Error::msg)?
            .set_collision_response_type(response_type)
    }

    pub fn get_bounds(&self) -> Result<PDRect, Error> {
        self.inner.try_borrow().map_err(Error::msg)?.get_bounds()
    }

    pub fn set_bounds(&self, bounds: &PDRect) -> Result<(), Error> {
        self.inner
            .try_borrow_mut()
            .map_err(Error::msg)?
            .set_bounds(bounds)
    }

    pub fn get_z_index(&self) -> Result<i16, Error> {
        self.inner
            .try_borrow_mut()
            .map_err(Error::msg)?
            .get_z_index()
    }

    pub fn set_z_index(&self, z_index: i16) -> Result<(), Error> {
        self.inner
            .try_borrow_mut()
            .map_err(Error::msg)?
            .set_z_index(z_index)
    }

    pub fn set_image(&mut self, bitmap: Bitmap, flip: LCDBitmapFlip) -> Result<(), Error> {
        self.inner
            .try_borrow_mut()
            .map_err(Error::msg)?
            .set_image(bitmap, flip)
    }

    pub fn set_tag(&mut self, tag: u8) -> Result<(), Error> {
        self.inner
            .try_borrow_mut()
            .map_err(Error::msg)?
            .set_tag(tag)
    }

    pub fn get_tag(&self) -> Result<u8, Error> {
        self.inner.try_borrow().map_err(Error::msg)?.get_tag()
    }

    pub fn set_needs_redraw(&mut self) -> Result<(), Error> {
        self.inner
            .try_borrow_mut()
            .map_err(Error::msg)?
            .set_needs_redraw()
    }

    pub fn move_to(&mut self, x: f32, y: f32) -> Result<(), Error> {
        self.inner
            .try_borrow_mut()
            .map_err(Error::msg)?
            .move_to(x, y)
    }

    pub fn get_position(&self) -> Result<(f32, f32), Error> {
        self.inner.try_borrow().map_err(Error::msg)?.get_position()
    }

    pub fn set_collide_rect(&mut self, collide_rect: &PDRect) -> Result<(), Error> {
        self.inner
            .try_borrow_mut()
            .map_err(Error::msg)?
            .set_collide_rect(collide_rect)
    }

    pub fn move_with_collisions(
        &mut self,
        goal_x: f32,
        goal_y: f32,
    ) -> Result<(f32, f32, Collisions), Error> {
        self.inner
            .try_borrow_mut()
            .map_err(Error::msg)?
            .move_with_collisions(goal_x, goal_y)
    }
}

impl Hash for Sprite {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.borrow().raw_sprite.hash(state);
    }
}

impl PartialEq for Sprite {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for Sprite {}

pub struct SpriteManager {
    pub playdate_sprite: *mut playdate_sprite,
    sprites: HashMap<*const crankstart_sys::LCDSprite, SpriteWeakPtr>,
}

impl SpriteManager {
    pub(crate) fn new(
        playdate_sprite: *mut playdate_sprite,
        update: SpriteUpdateFunction,
        draw: SpriteDrawFunction,
    ) {
        unsafe {
            SPRITE_UPDATE = Some(update);
            SPRITE_DRAW = Some(draw);
            SPRITE_COLLISION_RESPONSES = Some(HashMap::with_capacity(32))
        }
        let sm = Self {
            playdate_sprite,
            sprites: HashMap::with_capacity(32),
        };

        unsafe {
            SPRITE_MANAGER = Some(sm);
        }
    }

    pub fn get_mut() -> &'static mut SpriteManager {
        unsafe { SPRITE_MANAGER.as_mut().expect("SpriteManager") }
    }

    pub fn new_sprite(&mut self) -> Result<Sprite, Error> {
        let raw_sprite = pd_func_caller!((*self.playdate_sprite).newSprite)?;
        if raw_sprite == core::ptr::null_mut() {
            Err(anyhow!("new sprite failed"))
        } else {
            let sprite = SpriteInner {
                raw_sprite,
                playdate_sprite: self.playdate_sprite,
                image: None,
            };
            sprite.set_update_function(unsafe { SPRITE_UPDATE.expect("SPRITE_UPDATE") })?;
            let sprite_ptr = Rc::new(RefCell::new(sprite));
            let weak_ptr = Rc::downgrade(&sprite_ptr);
            self.sprites.insert(raw_sprite, weak_ptr);
            Ok(Sprite { inner: sprite_ptr })
        }
    }

    pub fn add_sprite(&self, sprite: &Sprite) -> Result<(), Error> {
        pd_func_caller!(
            (*self.playdate_sprite).addSprite,
            sprite.inner.borrow().raw_sprite
        )
    }

    pub fn add_dirty_rect(dirty_rect: LCDRect) -> Result<(), Error> {
        pd_func_caller!((*Self::get_mut().playdate_sprite).addDirtyRect, dirty_rect)
    }

    pub fn get_sprite_static(raw_sprite: *const LCDSprite) -> Option<Sprite> {
        Self::get_mut().get_sprite(raw_sprite)
    }

    pub fn get_sprite(&self, raw_sprite: *const LCDSprite) -> Option<Sprite> {
        let weak_sprite = self.sprites.get(&raw_sprite);
        weak_sprite
            .and_then(|weak_sprite| weak_sprite.upgrade())
            .and_then(|inner_ptr| {
                Some(Sprite {
                    inner: inner_ptr.clone(),
                })
            })
    }

    pub fn update_and_draw_sprites(&mut self) -> Result<(), Error> {
        pd_func_caller!((*self.playdate_sprite).updateAndDrawSprites)?;
        self.sprites.retain(|k, v| v.weak_count() != 0);
        Ok(())
    }
}
