//! `Sound` is the parent structure for the Playdate audio API, and you can get access specific
//! subsystems through its 'get' methods.
//!
//! For example, to play an audio sample (sound effect):
//!
//! ```rust
//! let sound = Sound::get();
//! let player = sound.get_sample_player()?;
//! let mut sample = sound.load_audio_sample("test.wav")?;
//! player.set_sample(&mut sample)?;
//! player.play(1, 1.0)?;
//! ```
//!
//! To play a music file:
//! ```rust
//! let music = Sound::get().get_file_player()?;
//! music.load_into_player("music.pda")?;
//! music.play(0)?;
//! ```

use crate::{pd_func_caller, pd_func_caller_log};
use crankstart_sys::ctypes;

use anyhow::{anyhow, ensure, Error, Result};
use core::ptr;
use cstr_core::CString;

pub mod sampleplayer;
pub use sampleplayer::{AudioSample, SamplePlayer};
pub mod fileplayer;
pub use fileplayer::FilePlayer;

// When the Playdate system struct is created, it passes the given playdate_sound to Sound::new,
// which then replaces this.
static mut SOUND: Sound = Sound::null();

/// `Sound` is the main interface to the Playdate audio subsystems.
#[derive(Clone, Debug)]
pub struct Sound {
    raw_sound: *const crankstart_sys::playdate_sound,

    // Each audio API subsystem has a struct with all of the relevant functions for that subsystem.
    // These functions are used repeatedly, so pointers to them are stored here for convenience.
    raw_file_player: *const crankstart_sys::playdate_sound_fileplayer,
    raw_sample: *const crankstart_sys::playdate_sound_sample,
    raw_sample_player: *const crankstart_sys::playdate_sound_sampleplayer,
}

// Not implemented: addSource, removeSource, setMicCallback, and getHeadphoneState (waiting on
// crankstart callback strategy), getDefaultChannel, addChannel, removeChannel.
impl Sound {
    const fn null() -> Self {
        Self {
            raw_sound: ptr::null(),
            raw_file_player: ptr::null(),
            raw_sample: ptr::null(),
            raw_sample_player: ptr::null(),
        }
    }

    /// Internal: builds the `Sound` struct from the pointers given in the Playdate SDK after it's started.
    #[allow(clippy::new_ret_no_self)]
    pub(crate) fn new(raw_sound: *const crankstart_sys::playdate_sound) -> Result<()> {
        ensure!(!raw_sound.is_null(), "Null pointer passed to Sound::new");

        // Get supported subsystem pointers.
        let raw_file_player = unsafe { (*raw_sound).fileplayer };
        ensure!(!raw_file_player.is_null(), "Null sound.fileplayer");
        let raw_sample = unsafe { (*raw_sound).sample };
        ensure!(!raw_sample.is_null(), "Null sound.sample");
        let raw_sample_player = unsafe { (*raw_sound).sampleplayer };
        ensure!(!raw_sample_player.is_null(), "Null sound.sampleplayer");

        let sound = Self {
            raw_sound,
            raw_file_player,
            raw_sample,
            raw_sample_player,
        };
        unsafe { SOUND = sound };
        Ok(())
    }

    /// Gets a handle to the Sound system.  This is the primary entry point for users.
    pub fn get() -> Self {
        unsafe { SOUND.clone() }
    }

    /// Get a `FilePlayer` that can be used to stream audio from disk, e.g. for music.
    pub fn get_file_player(&self) -> Result<FilePlayer> {
        let raw_player = pd_func_caller!((*self.raw_file_player).newPlayer)?;
        ensure!(
            !raw_player.is_null(),
            "Null returned from fileplayer.newPlayer"
        );
        FilePlayer::new(self.raw_file_player, raw_player)
    }

    /// Get a `SamplePlayer` that can be used to play sound effects.
    pub fn get_sample_player(&self) -> Result<SamplePlayer> {
        let raw_player = pd_func_caller!((*self.raw_sample_player).newPlayer)?;
        ensure!(
            !raw_player.is_null(),
            "Null returned from sampleplayer.newPlayer"
        );
        SamplePlayer::new(self.raw_sample_player, raw_player)
    }

    /// Loads an `AudioSample` sound effect.  Assign it to a `SamplePlayer` with
    /// `SamplePlayer.set_sample`.
    pub fn load_audio_sample(&self, sample_path: &str) -> Result<AudioSample> {
        let sample_path_c = CString::new(sample_path).map_err(Error::msg)?;
        let arg_ptr = sample_path_c.as_ptr() as *const ctypes::c_char;
        let raw_audio_sample = pd_func_caller!((*self.raw_sample).load, arg_ptr)?;
        ensure!(
            !raw_audio_sample.is_null(),
            "Null returned from sample.load"
        );
        AudioSample::new(self.raw_sample, raw_audio_sample)
    }

    /// Returns the sound engine's current time, in frames, 44.1k per second.
    pub fn get_current_time(&self) -> Result<ctypes::c_uint> {
        pd_func_caller!((*self.raw_sound).getCurrentTime)
    }

    /// Sets which audio outputs should be active.  Note: if you disable headphones and enable
    /// speaker, sound will be played through the speaker even if headphones are plugged in.
    pub fn set_outputs_active(&self, headphone: bool, speaker: bool) -> Result<()> {
        pd_func_caller!(
            (*self.raw_sound).setOutputsActive,
            headphone as ctypes::c_int,
            speaker as ctypes::c_int
        )
    }
}
