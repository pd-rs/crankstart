#![no_std]
#![allow(internal_features)]
#![feature(lang_items, alloc_error_handler, core_intrinsics)]
#![allow(unused_variables, dead_code, unused_imports)]

extern crate alloc;

pub mod display;
pub mod file;
pub mod geometry;
pub mod graphics;
pub mod lua;
pub mod sound;
pub mod sprite;
pub mod system;

use alloc::format;
use {
    crate::{
        display::Display,
        file::FileSystem,
        graphics::{Graphics, PDRect},
        lua::Lua,
        sound::Sound,
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
    playdate: *const crankstart_sys::PlaydateAPI,
}

impl Playdate {
    pub fn new(
        playdate: *const crankstart_sys::PlaydateAPI,
        sprite_update: SpriteUpdateFunction,
        sprite_draw: SpriteDrawFunction,
    ) -> Result<Self, Error> {
        let playdate_api = unsafe { *playdate };
        let system = playdate_api.system;
        System::new(system);
        let playdate_sprite = playdate_api.sprite;
        SpriteManager::new(playdate_sprite, sprite_update, sprite_draw);
        let file = playdate_api.file;
        FileSystem::new(file);
        let graphics = playdate_api.graphics;
        Graphics::new(graphics);
        let lua = playdate_api.lua;
        Lua::new(lua);
        let sound = playdate_api.sound;
        Sound::new(sound)?;
        let display = playdate_api.display;
        Display::new(display);
        Ok(Self { playdate })
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
                $crate::log_to_console!("{} did not contain a function pointer", stringify!($raw_fn_opt));
            }
        }
    };
}

pub trait Game {
    fn update_sprite(&mut self, sprite: &mut Sprite, playdate: &mut Playdate) -> Result<(), Error> {
        use alloc::format;
        Err(anyhow::anyhow!("Error: sprite {:?} needs update but this game hasn't implemented the update_sprite trait method", sprite))
    }

    fn draw_sprite(
        &self,
        sprite: &Sprite,
        bounds: &PDRect,
        draw_rect: &PDRect,
        playdate: &Playdate,
    ) -> Result<(), Error> {
        use alloc::format;
        Err(anyhow::anyhow!("Error: sprite {:?} needs to draw but this game hasn't implemented the draw_sprite trait method", sprite))
    }

    fn update(&mut self, playdate: &mut Playdate) -> Result<(), Error>;

    fn draw_fps(&self) -> bool {
        false
    }

    fn draw_and_update_sprites(&self) -> bool {
        true
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
            if let Err(err) = game.update(&mut self.playdate) {
                log_to_console!("Error in update: {err:#}")
            }
            if game.draw_and_update_sprites() {
                if let Err(err) = SpriteManager::get_mut().update_and_draw_sprites() {
                    log_to_console!("Error from sprite_manager.update_and_draw_sprites: {err:#}")
                }
            }
            if game.draw_fps() {
                if let Err(err) = System::get().draw_fps(0, 0) {
                    log_to_console!("Error from system().draw_fps: {err:#}")
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
                if let Err(err) = game.update_sprite(&mut sprite, &mut self.playdate) {
                    log_to_console!("Error in update_sprite: {err:#}")
                }
            } else {
                log_to_console!("Can't find sprite {sprite:?} to update");
            }
        } else {
            log_to_console!("can't get game to update_sprite");
        }
    }

    pub fn draw_sprite(&mut self, sprite: *mut LCDSprite, bounds: PDRect, draw_rect: PDRect) {
        if let Some(game) = self.game.as_ref() {
            if let Some(sprite) = SpriteManager::get_mut().get_sprite(sprite) {
                if let Err(err) = game.draw_sprite(&sprite, &bounds, &draw_rect, &self.playdate) {
                    log_to_console!("Error in draw_sprite: {err:#}")
                }
            } else {
                log_to_console!("Can't find sprite {sprite:?} to draw");
            }
        } else {
            log_to_console!("can't get game to draw_sprite");
        }
    }

    pub fn playdate_sprite(&self) -> *const playdate_sprite {
        SpriteManager::get_mut().playdate_sprite
    }
}

#[macro_export]
macro_rules! crankstart_game {
    ($game_struct:ty) => {
        crankstart_game!($game_struct, PDSystemEvent::kEventInit);
    };
    ($game_struct:ty, $pd_system_event:expr) => {
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

            extern "C" fn sprite_draw(sprite: *mut LCDSprite, bounds: PDRect, drawrect: PDRect) {
                let game_runner = unsafe { GAME_RUNNER.as_mut().expect("GAME_RUNNER") };
                game_runner.draw_sprite(sprite, bounds, drawrect);
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
                if event == $pd_system_event {
                    // This would only fail if PlaydateAPI has null pointers, which shouldn't happen.
                    let mut playdate = match Playdate::new(playdate, sprite_update, sprite_draw) {
                        Ok(playdate) => playdate,
                        Err(e) => {
                            log_to_console!("Failed to construct Playdate system: {e:#}");
                            return 1;
                        }
                    };
                    System::get()
                        .set_update_callback(Some(update))
                        .unwrap_or_else(|err| {
                            log_to_console!("Got error while setting update callback: {err:#}");
                        });
                    let game = match <$game_struct>::new(&mut playdate) {
                        Ok(game) => Some(game),
                        Err(err) => {
                            log_to_console!("Got error while creating game: {err:#}");
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
    use arrayvec::ArrayString;
    use core::fmt::Write;
    if let Some(location) = panic_info.location() {
        let mut output = ArrayString::<1024>::new();
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
        #[cfg(debug_assertions)]
        {
            // Try and save panic to file if built in debug mode
            let fs = FileSystem::get();
            let panic_str = format!("{:?}", panic_info);
            let file = fs.open("panic.txt", FileOptions::kFileWrite).unwrap();
            let _num_bytes_written = file.write(panic_str.as_bytes()).unwrap();
            file.flush().unwrap();
        }
        System::log_to_console(output.as_str());
    } else {
        System::log_to_console("panic\0");
    }
    #[cfg(target_os = "macos")]
    {
        unsafe {
            core::intrinsics::breakpoint();
        }
        abort_with_addr(0xdeadbeef);
    }
    #[cfg(not(target_os = "macos"))]
    {
        abort_with_addr(0x04);
    }
}

use core::alloc::{GlobalAlloc, Layout};
use crankstart_sys::FileOptions;

pub(crate) struct PlaydateAllocator;

unsafe impl Sync for PlaydateAllocator {}

unsafe impl GlobalAlloc for PlaydateAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let system = System::get();
        system.realloc(core::ptr::null_mut(), layout.size()) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let system = System::get();
        system.realloc(ptr as *mut core::ffi::c_void, 0);
    }

    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        System::get().realloc(ptr as *mut core::ffi::c_void, new_size) as *mut u8
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

#[cfg(target_os = "macos")]
#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    dest
}

#[cfg(target_os = "macos")]
#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if src < dest as *const u8 {
        // copy from end
        let mut i = n;
        while i != 0 {
            i -= 1;
            *dest.add(i) = *src.add(i);
        }
    } else {
        // copy from beginning
        let mut i = 0;
        while i < n {
            *dest.add(i) = *src.add(i);
            i += 1;
        }
    }
    dest
}

#[cfg(target_os = "macos")]
#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return a as i32 - b as i32;
        }
        i += 1;
    }
    0
}

#[cfg(target_os = "macos")]
#[no_mangle]
pub unsafe extern "C" fn bcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    memcmp(s1, s2, n)
}

#[cfg(target_os = "macos")]
pub unsafe fn memset_internal(s: *mut u8, c: crankstart_sys::ctypes::c_int, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *s.add(i) = c as u8;
        i += 1;
    }
    s
}

#[cfg(target_os = "macos")]
#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut u8, c: crankstart_sys::ctypes::c_int, n: usize) -> *mut u8 {
    memset_internal(s, c, n)
}

#[cfg(target_os = "macos")]
#[no_mangle]
pub unsafe extern "C" fn __bzero(s: *mut u8, n: usize) {
    memset_internal(s, 0, n);
}

#[no_mangle]
pub extern "C" fn _sbrk() {}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub extern "C" fn _write() {}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub extern "C" fn _close() {}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub extern "C" fn _lseek() {}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub extern "C" fn _read() {}

#[no_mangle]
pub extern "C" fn _fstat() {}

#[no_mangle]
pub extern "C" fn _isatty() {}

#[cfg(not(target_os = "windows"))]
#[no_mangle]
pub extern "C" fn _exit() {}

#[no_mangle]
pub extern "C" fn _open() {}

#[no_mangle]
pub extern "C" fn _kill() {}

#[no_mangle]
pub extern "C" fn _getpid() {}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {
    unimplemented!();
}

#[cfg(target_os = "macos")]
#[no_mangle]
extern "C" fn _Unwind_Resume() {
    unimplemented!();
}

#[no_mangle]
extern "C" fn __exidx_start() {
    unimplemented!();
}

#[no_mangle]
extern "C" fn __exidx_end() {
    unimplemented!();
}

#[cfg(target_os = "macos")]
#[link(name = "System")]
extern "C" {}
