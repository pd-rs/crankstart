use crate::{pd_func_caller, pd_func_caller_log};
use crankstart_sys::ctypes;

use anyhow::{anyhow, ensure, Error, Result};
use cstr_core::CString;

/// Note: Make sure you hold on to a FilePlayer until the file has played as much as you want,
/// because dropping it will stop playback.
#[derive(Debug)]
pub struct FilePlayer {
    raw_subsystem: *const crankstart_sys::playdate_sound_fileplayer,
    raw_player: *mut crankstart_sys::FilePlayer,
}

impl Drop for FilePlayer {
    fn drop(&mut self) {
        // Use _log to leak rather than fail
        pd_func_caller_log!((*self.raw_subsystem).freePlayer, self.raw_player);
    }
}

// Not implemented: newPlayer (use Sound::get_file_player), setFinishCallback and fadeVolume
// (waiting on crankstart callback strategy), and setLoopRange (does not seem to do anything).
impl FilePlayer {
    pub(crate) fn new(
        raw_subsystem: *const crankstart_sys::playdate_sound_fileplayer,
        raw_player: *mut crankstart_sys::FilePlayer,
    ) -> Result<Self> {
        ensure!(
            !raw_subsystem.is_null(),
            "Null pointer given as subsystem to FilePlayer::new"
        );
        ensure!(
            !raw_player.is_null(),
            "Null pointer given as player to FilePlayer::new"
        );
        Ok(Self {
            raw_subsystem,
            raw_player,
        })
    }

    /// Loads the given file into the player.  Unlike with SamplePlayer, you must give the
    /// compiled audio filename here, e.g. "file.pda" instead of "file.wav".  MP3 files are
    /// not compiled, so they keep their original .mp3 extension.
    pub fn load_into_player(&self, file_path: &str) -> Result<()> {
        let file_path_c = CString::new(file_path).map_err(Error::msg)?;
        let arg_ptr = file_path_c.as_ptr() as *const ctypes::c_char;
        let result = pd_func_caller!(
            (*self.raw_subsystem).loadIntoPlayer,
            self.raw_player,
            arg_ptr
        )?;
        if result == 1 {
            Ok(())
        } else {
            Err(anyhow!(
                "load_into_player given nonexistent file '{}'",
                file_path
            ))
        }
    }

    /// Play the file 'repeat_count' times; if 0, play until `stop` is called.  See set_loop_range
    /// for the portion of the file that will repeat.
    pub fn play(&self, repeat_count: ctypes::c_int) -> Result<()> {
        let result = pd_func_caller!((*self.raw_subsystem).play, self.raw_player, repeat_count,)?;
        if result == 1 {
            Ok(())
        } else {
            Err(anyhow!(
                "fileplayer.play should return 1; returned {}",
                result
            ))
        }
    }

    /// Can be used to stop a played file early, or stop one that's repeating endlessly because
    /// 'repeat' was set to 0.
    pub fn stop(&self) -> Result<()> {
        pd_func_caller!((*self.raw_subsystem).stop, self.raw_player)
    }

    /// Pause playback.  To resume playback at the same point, use play().
    pub fn pause(&self) -> Result<()> {
        pd_func_caller!((*self.raw_subsystem).pause, self.raw_player)
    }

    /// Returns whether the player is currently playing the file.
    pub fn is_playing(&self) -> Result<bool> {
        let result = pd_func_caller!((*self.raw_subsystem).isPlaying, self.raw_player)?;
        Ok(result == 1)
    }

    /// How much audio to buffer, in seconds.  Larger buffers use more memory but help avoid
    /// underruns, which can cause stuttering (see set_stop_on_underrun).
    pub fn set_buffer_length(&self, length: f32) -> Result<()> {
        pd_func_caller!(
            (*self.raw_subsystem).setBufferLength,
            self.raw_player,
            length
        )
    }

    /// If set to true, and the buffer runs out of data (known as an underrun), the player
    /// will stop playing.  If false (the default), the player will continue playback as soon
    /// as more data is available; this will come across as audio stuttering, particularly
    /// with small buffer sizes.  (Note that Inside Playdate with C says the reverse, but
    /// seems wrong.)
    pub fn set_stop_on_underrun(&self, stop: bool) -> Result<()> {
        pd_func_caller!(
            (*self.raw_subsystem).setStopOnUnderrun,
            self.raw_player,
            stop as ctypes::c_int
        )
    }

    /// Returns whether the buffer has underrun.
    pub fn did_underrun(&self) -> Result<bool> {
        let result = pd_func_caller!((*self.raw_subsystem).didUnderrun, self.raw_player)?;
        Ok(result == 1)
    }

    /// Returns the current offset into the file, in seconds, increasing as it plays.
    pub fn get_offset(&self) -> Result<f32> {
        pd_func_caller!((*self.raw_subsystem).getOffset, self.raw_player)
    }

    /// Set how far into the file to start playing, in seconds.
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

    /// Gets the current playback speed.
    pub fn get_rate(&self) -> Result<f32> {
        pd_func_caller!((*self.raw_subsystem).getRate, self.raw_player)
    }

    /// Sets the playback speed of the player; 1.0 is normal speed, 0.5 is down an octave,
    /// 2.0 is up one, etc.
    pub fn set_rate(&self, playback_speed: f32) -> Result<()> {
        ensure!(
            playback_speed >= 0.0,
            "FilePlayer cannot play in reverse (playback_speed < 0)"
        );
        pd_func_caller!(
            (*self.raw_subsystem).setRate,
            self.raw_player,
            playback_speed
        )
    }

    /// Returns the length of the loaded file, in seconds.
    pub fn get_length(&self) -> Result<f32> {
        pd_func_caller!((*self.raw_subsystem).getLength, self.raw_player)
    }
}
