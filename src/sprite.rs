extern crate alloc;

use {
    crate::{
        graphics::{Bitmap, Graphics, LCDBitmapFlip, LCDColor, PDRect},
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
        cell::{Ref, RefCell},
        fmt::Debug,
        hash::{Hash, Hasher},
        slice,
    },
    crankstart_sys::{
        playdate_sprite, LCDRect, LCDSprite, LCDSpriteCollisionFilterProc, SpriteCollisionInfo,
    },
    euclid::default::Vector2D,
    euclid::{point2, size2, vec2},
    hashbrown::HashMap,
};

pub use crankstart_sys::SpriteCollisionResponseType;

// Currently no font:getHeight in C API.
const SYSTEM_FONT_HEIGHT: i32 = 18;

pub type SpriteUpdateFunction = unsafe extern "C" fn(sprite: *mut crankstart_sys::LCDSprite);
pub type SpriteDrawFunction =
    unsafe extern "C" fn(sprite: *mut crankstart_sys::LCDSprite, bounds: PDRect, drawrect: PDRect);
pub type SpriteCollideFunction = unsafe extern "C" fn(
    sprite: *const crankstart_sys::LCDSprite,
    other: *const crankstart_sys::LCDSprite,
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
    playdate_sprite: *const playdate_sprite,
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
    pub fn get_userdata<T>(&self) -> Result<Rc<Box<T>>, Error>
    where
        T: Userdata,
    {
        let ptr =
            pd_func_caller!((*self.playdate_sprite).getUserdata, self.raw_sprite)? as *const Box<T>;

        let rc = unsafe { Rc::from_raw(ptr) };

        unsafe { Rc::increment_strong_count(Rc::as_ptr(&rc)) }
        Ok(rc)
    }

    pub fn set_userdata<T>(&mut self, userdata: Rc<Box<T>>) -> Result<(), Error>
    where
        T: Userdata,
    {
        let ptr = Rc::into_raw(userdata);

        pd_func_caller!(
            (*self.playdate_sprite).setUserdata,
            self.raw_sprite,
            ptr as *mut core::ffi::c_void
        )
    }

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

    /// Returns a reference to the bitmap assigned to the sprite, if any.
    pub fn get_image(&self) -> Option<&Bitmap> {
        self.image.as_ref()
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

    pub fn mark_dirty(&mut self) -> Result<(), Error> {
        pd_func_caller!((*self.playdate_sprite).markDirty, self.raw_sprite,)
    }
}

pub trait Userdata {}

impl Drop for SpriteInner {
    fn drop(&mut self) {
        fn free_userdata(sprite: &SpriteInner) -> Result<(), Error> {
            let ptr = pd_func_caller!((*sprite.playdate_sprite).getUserdata, sprite.raw_sprite)?;

            if ptr as *const _ == core::ptr::null() {
                // No userdata on this sprite, nothing to do
                return Ok(());
            }

            let rc = unsafe { Rc::from_raw(ptr as *const Box<dyn Userdata>) };

            // Just for clarity, we're dropping the rc which will decrease the strong count
            drop(rc);

            Ok(())
        }

        if let Err(err) = free_userdata(self) {
            log_to_console!("error dropping userdata: {}", err);
        }

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
    pub fn get_userdata<T>(&self) -> Result<Rc<Box<T>>, Error>
    where
        T: Userdata,
    {
        self.inner.try_borrow().map_err(Error::msg)?.get_userdata()
    }

    pub fn set_userdata<T>(&mut self, userdata: Rc<Box<T>>) -> Result<(), Error>
    where
        T: Userdata,
    {
        self.inner
            .try_borrow_mut()
            .map_err(Error::msg)?
            .set_userdata(userdata)
    }

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

    /// Returns a reference to the bitmap assigned to the sprite, if any.  Specifically,
    /// returns Err if the inner data is already mutably borrowed; Ok(None) if no sprite has
    /// been assigned; Ok(Some(Ref<Bitmap>)) if a sprite has been assigned.
    pub fn get_image(&self) -> Result<Option<Ref<Bitmap>>> {
        let borrowed: Ref<SpriteInner> = self.inner.try_borrow().map_err(Error::msg)?;
        let filtered: Result<Ref<Bitmap>, _> =
            Ref::filter_map(borrowed, |b: &SpriteInner| b.get_image());
        // filter_map gives back the original if the closure returns None, which we don't need
        Ok(filtered.ok())
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

    pub fn mark_dirty(&mut self) -> Result<(), Error> {
        self.inner
            .try_borrow_mut()
            .map_err(Error::msg)?
            .mark_dirty()
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
    pub playdate_sprite: *const playdate_sprite,
    sprites: HashMap<*const crankstart_sys::LCDSprite, SpriteWeakPtr>,
}

impl SpriteManager {
    pub(crate) fn new(
        playdate_sprite: *const playdate_sprite,
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

    pub fn get_sprite_count(&self) -> Result<i32, Error> {
        pd_func_caller!((*self.playdate_sprite).getSpriteCount)
    }

    pub fn remove_sprite(&self, sprite: &Sprite) -> Result<(), Error> {
        pd_func_caller!(
            (*self.playdate_sprite).removeSprite,
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

/// This is a helper type for drawing text into a sprite.  Drawing text into a sprite is the
/// recommended way to display text when using sprites in your game; it removes timing issues and
/// gives you the flexibility of the sprite system rather than draw_text alone.
///
/// After creation with `new`, you can `update_text` as desired, and use `get_sprite` or
/// `get_sprite_mut` to access the `Sprite` for other operations like `move_to` and `get_bounds`
/// (which can tell you the height and width of the generated bitmap).
///
/// Note: it's assumed that you're using the system font and haven't changed its tracking; we have
/// no way to retrieve the current font or tracking with C APIs.
#[derive(Clone, Debug)]
pub struct TextSprite {
    sprite: Sprite,
    background: LCDColor,
}

impl TextSprite {
    /// Creates a `TextSprite`, draws the given text into it over the given background color,
    /// and adds the underlying sprite to the `SpriteManager`.
    pub fn new<S>(text: S, background: LCDColor) -> Result<Self, Error>
    where
        S: AsRef<str>,
    {
        let text = text.as_ref();
        let graphics = Graphics::get();
        let sprite_manager = SpriteManager::get_mut();

        // Currently no getTextTracking C API; assume none has been set.
        let tracking = 0;

        let width = graphics.get_system_text_width(text, tracking)?;

        let mut text_bitmap =
            graphics.new_bitmap(size2(width, SYSTEM_FONT_HEIGHT), background.clone())?;
        graphics.with_context(&mut text_bitmap, || {
            graphics.draw_text(text, point2(0, 0))?;
            Ok(())
        })?;

        let mut sprite = sprite_manager.new_sprite()?;
        sprite.set_image(text_bitmap, LCDBitmapFlip::kBitmapUnflipped)?;
        sprite_manager.add_sprite(&sprite)?;

        Ok(Self { sprite, background })
    }

    pub fn get_sprite(&self) -> &Sprite {
        &self.sprite
    }

    pub fn get_sprite_mut(&mut self) -> &mut Sprite {
        &mut self.sprite
    }

    /// Recreates the underlying bitmap with the given text; use `get_sprite().get_bounds()`
    /// to see the new size.
    pub fn update_text<S>(&mut self, text: S) -> Result<(), Error>
    where
        S: AsRef<str>,
    {
        let text = text.as_ref();
        let graphics = Graphics::get();

        // Currently no getTextTracking C API; assume none has been set.
        let tracking = 0;

        let width = graphics.get_system_text_width(text, tracking)?;

        let mut text_bitmap =
            graphics.new_bitmap(size2(width, SYSTEM_FONT_HEIGHT), self.background.clone())?;
        graphics.with_context(&mut text_bitmap, || {
            graphics.draw_text(text, point2(0, 0))?;
            Ok(())
        })?;

        self.sprite
            .set_image(text_bitmap, LCDBitmapFlip::kBitmapUnflipped)?;

        Ok(())
    }
}

/// This is a helper type for rotating and scaling an image in a sprite.
///
/// After creation with `new`, you can `set_rotation` to update the parameters, and use
/// `get_sprite` or `get_sprite_mut` to access the `Sprite` for other operations like `move_to`
/// and `get_bounds` (which can tell you the height and width of the generated bitmap).
///
/// Note: the image is rotated around its center point.  If you want to rotate around another
/// point, there are a few options:
/// 1. Extend the image with transparent pixels in one direction so it appears to be rotating
///    about another point.
/// 2. Rotate about the center, then move the sprite to an equivalent position.
/// 3. Manage the image and sprite manually: do the math to find the size after rotation, create
///    a fresh Bitmap of that size, and use Graphics.draw_rotated() to draw into it, since
///    draw_rotated allows specifying the center point.
#[derive(Clone, Debug)]
pub struct RotatedSprite {
    /// The managed sprite.
    sprite: Sprite,
    /// The original, unrotated/unscaled bitmap; use this rather than reading back a
    /// rotated/scaled image because of compounding error introduced in that process.
    bitmap: Bitmap,
}

impl RotatedSprite {
    /// Creates a `RotatedSprite`, draws the rotated and scaled image into it, and adds the
    /// underlying sprite to the `SpriteManager`.
    pub fn new(bitmap: Bitmap, angle: f32, scaling: Vector2D<f32>) -> Result<Self, Error> {
        let rotated_bitmap = bitmap.rotated(angle, scaling)?;

        let sprite_manager = SpriteManager::get_mut();
        let mut sprite = sprite_manager.new_sprite()?;
        sprite.set_image(rotated_bitmap, LCDBitmapFlip::kBitmapUnflipped)?;
        sprite_manager.add_sprite(&sprite)?;

        Ok(Self { sprite, bitmap })
    }

    pub fn get_sprite(&self) -> &Sprite {
        &self.sprite
    }

    pub fn get_sprite_mut(&mut self) -> &mut Sprite {
        &mut self.sprite
    }

    /// Recreates the underlying bitmap with the given rotation angle and scaling; use
    /// `get_sprite().get_bounds()` to see the new size.
    pub fn set_rotation(&mut self, angle: f32, scaling: Vector2D<f32>) -> Result<(), Error> {
        let rotated_bitmap = self.bitmap.rotated(angle, scaling)?;
        self.sprite
            .set_image(rotated_bitmap, LCDBitmapFlip::kBitmapUnflipped)?;
        Ok(())
    }
}
