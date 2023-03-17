use crate::{log_to_console, pd_func_caller, pd_func_caller_log};
use crankstart_sys::ctypes;

use anyhow::{anyhow, ensure, Error, Result};

/// Note: Make sure you hold on to a SamplePlayer until the sample has played as much as you want,
/// because dropping it will stop playback.
#[derive(Debug)]
pub struct SamplePlayer {
    raw_subsystem: *const crankstart_sys::playdate_sound_sampleplayer,
    raw_player: *mut crankstart_sys::SamplePlayer,
}

impl Drop for SamplePlayer {
    fn drop(&mut self) {
        // Use _log to leak rather than fail
        pd_func_caller_log!((*self.raw_subsystem).freePlayer, self.raw_player);
    }
}

// Not implemented: newPlayer (use Sound::get_sample_player), and setFinishCallback and setLoopCallback
// (waiting on crankstart callback strategy).
impl SamplePlayer {
    pub(crate) fn new(
        raw_subsystem: *const crankstart_sys::playdate_sound_sampleplayer,
        raw_player: *mut crankstart_sys::SamplePlayer,
    ) -> Result<Self> {
        ensure!(
            !raw_subsystem.is_null(),
            "Null pointer given as subsystem to SamplePlayer::new"
        );
        ensure!(
            !raw_player.is_null(),
            "Null pointer given as player to SamplePlayer::new"
        );
        Ok(Self {
            raw_subsystem,
            raw_player,
        })
    }

    /// Sets the sound effect to be played by this player.
    pub fn set_sample(&self, audio_sample: &mut AudioSample) -> Result<()> {
        pd_func_caller!(
            (*self.raw_subsystem).setSample,
            self.raw_player,
            audio_sample.raw_audio_sample
        )
    }

    /// Play the sample 'repeat_count' times; if 0, play until `stop` is called; if -1, play
    /// forward, backward, forward, etc.  (See set_play_range to change which part is looped.)
    /// 'playback_speed' is how fast the sample plays; 1.0 is normal speed, 0.5 is down an octave,
    /// 2.0 is up one, etc.  A negative rate plays the sample in reverse.
    pub fn play(&self, repeat_count: ctypes::c_int, playback_speed: f32) -> Result<()> {
        let result = pd_func_caller!(
            (*self.raw_subsystem).play,
            self.raw_player,
            repeat_count,
            playback_speed
        )?;
        if result == 1 {
            Ok(())
        } else {
            Err(anyhow!(
                "sampleplayer.play should return 1; returned {}",
                result
            ))
        }
    }

    /// Can be used to stop a sample early, or stop one that's repeating endlessly because 'repeat'
    /// was set to 0.
    pub fn stop(&self) -> Result<()> {
        pd_func_caller!((*self.raw_subsystem).stop, self.raw_player)
    }

    /// Pause or resume playback.
    pub fn set_paused(&self, paused: bool) -> Result<()> {
        pd_func_caller!(
            (*self.raw_subsystem).setPaused,
            self.raw_player,
            paused as ctypes::c_int
        )
    }

    /// Returns whether the player is currently playing the sample.
    pub fn is_playing(&self) -> Result<bool> {
        let result = pd_func_caller!((*self.raw_subsystem).isPlaying, self.raw_player)?;
        Ok(result == 1)
    }

    /// Sets the start and end position, in frames, when looping a sample with repeat_count -1.
    pub fn set_play_range(&self, start: ctypes::c_int, end: ctypes::c_int) -> Result<()> {
        pd_func_caller!(
            (*self.raw_subsystem).setPlayRange,
            self.raw_player,
            start,
            end
        )
    }

    /// Returns the current offset into the sample, in seconds, increasing as it plays.  This is not
    /// adjusted for rate.
    pub fn get_offset(&self) -> Result<f32> {
        pd_func_caller!((*self.raw_subsystem).getOffset, self.raw_player)
    }

    /// Set how far into the sample to start playing, in seconds.  This is not adjusted for rate.
    pub fn set_offset(&self, offset: f32) -> Result<()> {
        pd_func_caller!((*self.raw_subsystem).setOffset, self.raw_player, offset)
    }

    /// Gets the current volume of the left and right audio channels, out of 1.
    pub fn get_volume(&self) -> Result<(f32, f32)> {
        let mut left = 0.0;
        let mut right = 0.0;
        pd_func_caller!(
            (*self.raw_subsystem).getVolume,
            self.raw_player,
            &mut left,
            &mut right,
        )?;
        Ok((left, right))
    }

    /// Sets the volume of the left and right audio channels, out of 1.
    pub fn set_volume(&self, left: f32, right: f32) -> Result<()> {
        pd_func_caller!(
            (*self.raw_subsystem).setVolume,
            self.raw_player,
            left,
            right
        )
    }

    /// Gets the current playback speed.  Returns 1 unless the value was changed by `set_rate` - it
    /// still returns 1 if the rate is changed with the argument to `play`.
    pub fn get_rate(&self) -> Result<f32> {
        pd_func_caller!((*self.raw_subsystem).getRate, self.raw_player)
    }

    /// Sets the playback speed of the player after a sample has started playing.  1.0 is normal,
    /// 0.5 is down an octave, 2.0 is up one, etc.  A negative rate plays the sample in reverse.
    pub fn set_rate(&self, playback_speed: f32) -> Result<()> {
        pd_func_caller!(
            (*self.raw_subsystem).setRate,
            self.raw_player,
            playback_speed
        )
    }

    /// Returns the length of the assigned sample, in seconds.
    pub fn get_length(&self) -> Result<f32> {
        pd_func_caller!((*self.raw_subsystem).getLength, self.raw_player)
    }
}

/// A loaded sound effect.
#[derive(Debug)]
pub struct AudioSample {
    raw_subsystem: *const crankstart_sys::playdate_sound_sample,
    raw_audio_sample: *mut crankstart_sys::AudioSample,
}

impl Drop for AudioSample {
    fn drop(&mut self) {
        // Use _log to leak rather than fail
        pd_func_caller_log!((*self.raw_subsystem).freeSample, self.raw_audio_sample);
    }
}

// Not implemented: getData, newSampleBuffer, loadIntoSample, newSampleFromData -
// only Sound::load_audio_sample for now.
impl AudioSample {
    pub(crate) fn new(
        raw_subsystem: *const crankstart_sys::playdate_sound_sample,
        raw_audio_sample: *mut crankstart_sys::AudioSample,
    ) -> Result<Self, Error> {
        ensure!(
            !raw_subsystem.is_null(),
            "Null pointer given as subsystem to AudioSample::new"
        );
        ensure!(
            !raw_audio_sample.is_null(),
            "Null pointer given as sample to AudioSample::new"
        );
        Ok(Self {
            raw_subsystem,
            raw_audio_sample,
        })
    }

    /// Returns the length of the sample, in seconds.
    pub fn get_length(&self) -> Result<f32> {
        pd_func_caller!((*self.raw_subsystem).getLength, self.raw_audio_sample)
    }
}
