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
use alloc::string::String;
use core::marker::PhantomData;
use crankstart_sys::ctypes;
use crankstart_sys::LFOType;

use anyhow::{anyhow, ensure, Error, Result};
use core::ptr;
use cstr_core::{CStr, CString};

pub mod sampleplayer;
pub use sampleplayer::{AudioSample, SamplePlayer};
pub mod fileplayer;
pub use fileplayer::FilePlayer;
pub mod synth;
pub use synth::Synth;
pub use synth::LFO;
pub mod effect;
pub use effect::Overdrive;
pub mod channel;
pub use channel::SoundChannel;

// When the Playdate system struct is created, it passes the given playdate_sound to Sound::new,
// which then replaces this.
static mut SOUND: Sound = Sound::null();

// From https://sdk.play.date/3.1.2/Inside%20Playdate%20with%20C.html#f-sound.getCurrentTime
static SAMPLES_PER_SECOND: u32 = 44100;

/// `Sound` is the main interface to the Playdate audio subsystems.
#[derive(Clone, Debug)]
pub struct Sound {
    raw_sound: *const crankstart_sys::playdate_sound,

    // Each audio API subsystem has a struct with all of the relevant functions for that subsystem.
    // These functions are used repeatedly, so pointers to them are stored here for convenience.
    raw_file_player: *const crankstart_sys::playdate_sound_fileplayer,
    raw_sample: *const crankstart_sys::playdate_sound_sample,
    raw_sample_player: *const crankstart_sys::playdate_sound_sampleplayer,
    raw_synth: *const crankstart_sys::playdate_sound_synth,
    raw_lfo: *const crankstart_sys::playdate_sound_lfo,
    raw_sound_effect: *const crankstart_sys::playdate_sound_effect,
    raw_overdrive: *const crankstart_sys::playdate_sound_effect_overdrive,
    raw_one_pole_filter: *const crankstart_sys::playdate_sound_effect_onepolefilter,
    raw_delay_line: *const crankstart_sys::playdate_sound_effect_delayline,
    raw_channel: *const crankstart_sys::playdate_sound_channel,
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
            raw_synth: ptr::null(),
            raw_lfo: ptr::null(),
            raw_sound_effect: ptr::null(),
            raw_overdrive: ptr::null(),
            raw_one_pole_filter: ptr::null(),
            raw_delay_line: ptr::null(),
            raw_channel: ptr::null(),
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
        let raw_synth = unsafe { (*raw_sound).synth };
        ensure!(!raw_synth.is_null(), "Null sound.synth");
        let raw_lfo = unsafe { (*raw_sound).lfo };
        ensure!(!raw_lfo.is_null(), "Null sound.lfo");
        let raw_sound_effect = unsafe { (*raw_sound).effect };
        ensure!(!raw_sound_effect.is_null(), "Null sound.effect");
        let raw_overdrive = unsafe { (*(*raw_sound).effect).overdrive };
        ensure!(!raw_overdrive.is_null(), "Null sound.effect_overdrive");
        let raw_one_pole_filter = unsafe { (*(*raw_sound).effect).onepolefilter };
        ensure!(
            !raw_one_pole_filter.is_null(),
            "Null sound.effect_onepolefilter"
        );
        let raw_delay_line = unsafe { (*(*raw_sound).effect).delayline };
        ensure!(!raw_delay_line.is_null(), "Null sound.effect_delayline");
        let raw_channel = unsafe { (*raw_sound).channel };
        ensure!(!raw_channel.is_null(), "Null sound.channel");

        let sound = Self {
            raw_sound,
            raw_file_player,
            raw_sample,
            raw_sample_player,
            raw_synth,
            raw_lfo,
            raw_sound_effect,
            raw_overdrive,
            raw_one_pole_filter,
            raw_delay_line,
            raw_channel,
        };
        unsafe { SOUND = sound };
        Ok(())
    }

    /// Gets a handle to the Sound system.  This is the primary entry point for users.
    pub fn get() -> Self {
        unsafe { (&raw const SOUND).read().clone() }
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

    /// Returns the sound library's most recent error string, if one is available.
    pub fn get_error(&self) -> Result<Option<String>> {
        let raw_error = pd_func_caller!((*self.raw_sound).getError)?;
        if raw_error.is_null() {
            return Ok(None);
        }

        Ok(Some(
            unsafe { CStr::from_ptr(raw_error) }
                .to_string_lossy()
                .into_owned(),
        ))
    }

    pub fn new_synth(&self) -> Result<Synth> {
        crate::sound::Synth::new(self.raw_synth)
    }

    pub fn new_lfo(&self, lfo_type: LFOType) -> Result<LFO> {
        crate::sound::LFO::new(self.raw_lfo, lfo_type)
    }

    pub fn new_overdrive(&self) -> Result<Overdrive> {
        crate::sound::Overdrive::new(self.raw_sound_effect, self.raw_overdrive)
    }

    pub fn new_one_pole_filter(&self) -> Result<effect::OnePoleFilter> {
        crate::sound::effect::OnePoleFilter::new(self.raw_sound_effect, self.raw_one_pole_filter)
    }

    pub fn new_delay_line(&self, length_seconds: f32, stereo: bool) -> Result<effect::DelayLine> {
        crate::sound::effect::DelayLine::new(
            self.raw_sound_effect,
            self.raw_delay_line,
            length_seconds,
            stereo,
        )
    }

    pub fn new_channel(&self) -> Result<SoundChannel> {
        crate::sound::SoundChannel::new(self.raw_channel)
    }
}

/// # Safety
/// This trait must guarantee that the returned pointer is valid for the `self` lifetime.
pub unsafe trait SoundSource: 'static {
    fn get_sound_source(&self) -> *mut crankstart_sys::SoundSource;
}
