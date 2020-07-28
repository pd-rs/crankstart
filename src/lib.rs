#![no_std]
#![feature(alloc_error_handler, core_intrinsics)]
#![allow(unused_variables, dead_code, unused_imports)]

extern crate alloc;

pub mod display;
pub mod file;
pub mod geometry;
pub mod graphics;
pub mod sprite;
pub mod system;

use {
    crate::{
        display::Display,
        file::FileSystem,
        graphics::{Graphics, PDRect},
        sprite::{
            Sprite, SpriteCollideFunction, SpriteDrawFunction, SpriteManager, SpriteUpdateFunction,
        },
        system::System,
    },
    alloc::boxed::Box,
    anyhow::Error,
    core::{fmt, panic::PanicInfo},
    crankstart_sys::{playdate_sprite, LCDRect, LCDSprite, SpriteCollisionResponseType},
};

pub struct Playdate {
    playdate: *mut crankstart_sys::PlaydateAPI,
}

impl Playdate {
    pub fn new(
        playdate: *mut crankstart_sys::PlaydateAPI,
        sprite_update: SpriteUpdateFunction,
        sprite_draw: SpriteDrawFunction,
    ) -> Self {
        let system = unsafe { (*playdate).system };
        System::new(system);
        let playdate_sprite = unsafe { (*playdate).sprite };
        SpriteManager::new(playdate_sprite, sprite_update, sprite_draw);
        let file = unsafe { (*playdate).file };
        FileSystem::new(file);
        let graphics = unsafe { (*playdate).graphics };
        Graphics::new(graphics);
        let display = unsafe { (*playdate).display };
        Display::new(display);
        Self { playdate }
    }
}

#[macro_export]
macro_rules! log_to_console {
    ($($arg:tt)*) => ($crate::system::System::log_to_console(&alloc::format!($($arg)*)));
}

#[macro_export]
macro_rules! pd_func_caller {
    ($raw_fn_opt:expr, $($arg:tt)*) => {
        unsafe {
            use alloc::format;
            let raw_fn = $raw_fn_opt
                .ok_or_else(|| anyhow::anyhow!("{} did not contain a function pointer", stringify!($raw_fn_opt)))?;
            Ok(raw_fn($($arg)*))
        }
    };
    ($raw_fn_opt:expr) => {
        unsafe {
            use alloc::format;
            let raw_fn = $raw_fn_opt
                .ok_or_else(|| anyhow::anyhow!("{} did not contain a function pointer", stringify!($raw_fn_opt)))?;
            Ok(raw_fn())
        }
    };
}

#[macro_export]
macro_rules! pd_func_caller_log {
    ($raw_fn_opt:expr, $($arg:tt)*) => {
        unsafe {
            if let Some(raw_fn) = $raw_fn_opt {
                raw_fn($($arg)*);
            } else {
                crate::log_to_console!("{} did not contain a function pointer", stringify!($raw_fn_opt));
            }
        }
    };
}

pub trait Game {
    fn update_sprite(&mut self, sprite: &mut Sprite, playdate: &mut Playdate) -> Result<(), Error> {
        Err(anyhow::anyhow!("Error: sprite {:?} needs update but this game hasn't implemented the update_sprite trait method"))
    }

    fn draw_sprite(
        &self,
        sprite: &Sprite,
        bounds: &PDRect,
        draw_rect: &LCDRect,
        playdate: &Playdate,
    ) -> Result<(), Error> {
        Err(anyhow::anyhow!("Error: sprite {:?} needs to draw but this game hasn't implemented the draw_sprite trait method"))
    }

    fn update(&mut self, playdate: &mut Playdate) -> Result<(), Error>;

    fn draw_fps(&self) -> bool {
        false
    }
}

pub type GamePtr<T> = Box<T>;

pub struct GameRunner<T: Game> {
    game: Option<GamePtr<T>>,
    init_failed: bool,
    playdate: Playdate,
}

impl<T: 'static + Game> GameRunner<T> {
    pub fn new(game: Option<GamePtr<T>>, playdate: Playdate) -> Self {
        Self {
            init_failed: false,
            game,
            playdate,
        }
    }

    pub fn update(&mut self) {
        if self.init_failed {
            return;
        }

        if let Some(game) = self.game.as_mut() {
            match game.update(&mut self.playdate) {
                Err(err) => log_to_console!("Error in update: {}", err),
                _ => (),
            }
            match SpriteManager::get_mut().update_and_draw_sprites() {
                Err(err) => {
                    log_to_console!("Error from sprite_manager.update_and_draw_sprites: {}", err)
                }
                _ => (),
            }
            if game.draw_fps() {
                match System::get().draw_fps(0, 0) {
                    Err(err) => log_to_console!("Error from system().draw_fps: {}", err),
                    _ => (),
                }
            }
        } else {
            log_to_console!("can't get game to update");
            self.init_failed = true;
        }
    }

    pub fn update_sprite(&mut self, sprite: *mut LCDSprite) {
        if let Some(game) = self.game.as_mut() {
            if let Some(mut sprite) = SpriteManager::get_mut().get_sprite(sprite) {
                match game.update_sprite(&mut sprite, &mut self.playdate) {
                    Err(err) => log_to_console!("Error in update_sprite: {}", err),
                    _ => (),
                }
            } else {
                log_to_console!("Can't find sprite {:?} to update", sprite);
            }
        } else {
            log_to_console!("can't get game to update_sprite");
        }
    }

    pub fn draw_sprite(
        &mut self,
        sprite: *mut LCDSprite,
        bounds: PDRect,
        frame: *mut u8,
        draw_rect: LCDRect,
    ) {
        if let Some(game) = self.game.as_ref() {
            if let Some(sprite) = SpriteManager::get_mut().get_sprite(sprite) {
                match game.draw_sprite(&sprite, &bounds, &draw_rect, &self.playdate) {
                    Err(err) => log_to_console!("Error in draw_sprite: {}", err),
                    _ => (),
                }
            } else {
                log_to_console!("Can't find sprite {:?} to draw", sprite);
            }
        } else {
            log_to_console!("can't get game to draw_sprite");
        }
    }

    pub fn playdate_sprite(&self) -> *mut playdate_sprite {
        SpriteManager::get_mut().playdate_sprite
    }
}

#[macro_export]
macro_rules! crankstart_game {
    ($game_struct:tt) => {
        pub mod game_setup {
            extern crate alloc;
            use super::*;
            use {
                alloc::{boxed::Box, format},
                crankstart::{
                    graphics::PDRect, log_to_console, sprite::SpriteManager, system::System,
                    GameRunner, Playdate,
                },
                crankstart_sys::{
                    LCDRect, LCDSprite, PDSystemEvent, PlaydateAPI, SpriteCollisionResponseType,
                },
            };

            static mut GAME_RUNNER: Option<GameRunner<$game_struct>> = None;

            extern "C" fn sprite_update(sprite: *mut LCDSprite) {
                let game_runner = unsafe { GAME_RUNNER.as_mut().expect("GAME_RUNNER") };
                game_runner.update_sprite(sprite);
            }

            extern "C" fn sprite_draw(
                sprite: *mut LCDSprite,
                bounds: PDRect,
                frame: *mut u8,
                drawrect: LCDRect,
            ) {
                let game_runner = unsafe { GAME_RUNNER.as_mut().expect("GAME_RUNNER") };
                game_runner.draw_sprite(sprite, bounds, frame, drawrect);
            }

            extern "C" fn update(_user_data: *mut core::ffi::c_void) -> i32 {
                let game_runner = unsafe { GAME_RUNNER.as_mut().expect("GAME_RUNNER") };

                game_runner.update();

                1
            }

            #[no_mangle]
            extern "C" fn eventHandler(
                playdate: *mut PlaydateAPI,
                event: PDSystemEvent,
                _arg: u32,
            ) -> crankstart_sys::ctypes::c_int {
                if event == PDSystemEvent::kEventInit {
                    let mut playdate = Playdate::new(playdate, sprite_update, sprite_draw);
                    System::get()
                        .set_update_callback(Some(update))
                        .unwrap_or_else(|err| {
                            log_to_console!("Got error while setting update callback: {}", err);
                        });
                    let game = match $game_struct::new(&mut playdate) {
                        Ok(game) => Some(game),
                        Err(err) => {
                            log_to_console!("Got error while creating game: {}", err);
                            None
                        }
                    };

                    unsafe {
                        GAME_RUNNER = Some(GameRunner::new(game, playdate));
                    }
                }
                0
            }
        }
    };
}

fn abort_with_addr(addr: usize) -> ! {
    let p = addr as *mut i32;
    unsafe {
        *p = 0;
    }
    core::intrinsics::abort()
}

#[panic_handler]
fn panic(#[allow(unused)] panic_info: &PanicInfo) -> ! {
    use {
        core::fmt::Write,
        heapless::{consts::*, String},
    };
    if let Some(location) = panic_info.location() {
        let mut output: String<U1024> = String::new();
        let payload = if let Some(payload) = panic_info.payload().downcast_ref::<&str>() {
            payload
        } else {
            "no payload"
        };
        write!(
            output,
            "panic: {} @ {}:{}\0",
            payload,
            location.file(),
            location.line()
        )
        .expect("write");
        System::log_to_console(output.as_str());
    } else {
        System::log_to_console("panic\0");
    }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        unsafe {
            core::intrinsics::breakpoint();
        }
        abort_with_addr(0xdeadbeef);
    }
    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    {
        abort_with_addr(0xdeadbeef);
    }
}

use core::alloc::{GlobalAlloc, Layout};

pub(crate) struct PlaydateAllocator;

unsafe impl Sync for PlaydateAllocator {}

unsafe impl GlobalAlloc for PlaydateAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let system = System::get();
        system.realloc(
            core::ptr::null_mut(),
            layout.size() as crankstart_sys::ctypes::realloc_size,
        ) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let system = System::get();
        system.realloc(ptr as *mut core::ffi::c_void, 0);
    }

    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        System::get().realloc(
            ptr as *mut core::ffi::c_void,
            new_size as crankstart_sys::ctypes::realloc_size,
        ) as *mut u8
    }
}

#[global_allocator]
pub(crate) static mut A: PlaydateAllocator = PlaydateAllocator;

// define what happens in an Out Of Memory (OOM) condition
#[alloc_error_handler]
fn alloc_error(_layout: Layout) -> ! {
    System::log_to_console("Out of Memory\0");
    abort_with_addr(0xDEADFA11);
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.offset(i as isize) = *src.offset(i as isize);
        i += 1;
    }
    dest
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if src < dest as *const u8 {
        // copy from end
        let mut i = n;
        while i != 0 {
            i -= 1;
            *dest.offset(i as isize) = *src.offset(i as isize);
        }
    } else {
        // copy from beginning
        let mut i = 0;
        while i < n {
            *dest.offset(i as isize) = *src.offset(i as isize);
            i += 1;
        }
    }
    dest
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = *s1.offset(i as isize);
        let b = *s2.offset(i as isize);
        if a != b {
            return a as i32 - b as i32;
        }
        i += 1;
    }
    0
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[no_mangle]
pub unsafe extern "C" fn bcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    memcmp(s1, s2, n)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub unsafe fn memset_internal(s: *mut u8, c: crankstart_sys::ctypes::c_int, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *s.offset(i as isize) = c as u8;
        i += 1;
    }
    s
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut u8, c: crankstart_sys::ctypes::c_int, n: usize) -> *mut u8 {
    memset_internal(s, c, n)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[no_mangle]
pub unsafe extern "C" fn __bzero(s: *mut u8, n: usize) {
    memset_internal(s, 0, n);
}
