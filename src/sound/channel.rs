use crate::sound::effect::Effect;
use crate::sound::SoundSource;
use crate::{pd_func_caller, pd_func_caller_log};
use alloc::boxed::Box;
use alloc::vec::Vec;
use anyhow::{Error, Result};
use core::marker::PhantomData;

pub struct SoundChannel {
    raw_subsystem: *const crankstart_sys::playdate_sound_channel,
    raw_channel: *mut crankstart_sys::SoundChannel,
    effects: Vec<Box<dyn Effect>>,
    sources: Vec<Box<dyn SoundSource>>,
}

impl SoundChannel {
    pub fn new(
        raw_subsystem: *const crankstart_sys::playdate_sound_channel,
    ) -> Result<Self, Error> {
        Ok(Self {
            raw_subsystem,
            raw_channel: pd_func_caller!((*raw_subsystem).newChannel)?,
            effects: Vec::new(),
            sources: Vec::new(),
        })
    }

    pub fn set_volume(&mut self, volume: f32) -> Result<()> {
        pd_func_caller!((*self.raw_subsystem).setVolume, self.raw_channel, volume)
    }

    pub fn add_effect<E: Effect>(&mut self, effect: E) -> Result<()> {
        let result = pd_func_caller!(
            (*self.raw_subsystem).addEffect,
            self.raw_channel,
            effect.get_sound_effect()
        );
        self.effects.push(Box::new(effect));
        match result {
            Ok(0) => Err(anyhow::anyhow!("effect already in use")),
            Ok(1) => Ok(()),
            Ok(num) => panic!("invalid result from addEffect: {}", num),
            Err(e) => Err(e),
        }
    }

    pub fn remove_effect<E: Effect>(&mut self, effect: E) -> Result<()> {
        let result = pd_func_caller!(
            (*self.raw_subsystem).removeEffect,
            self.raw_channel,
            effect.get_sound_effect()
        );
        self.effects
            .retain(|e| e.get_sound_effect() != effect.get_sound_effect());
        match result {
            Ok(0) => Err(anyhow::anyhow!("effect not found in channel")),
            Ok(1) => Ok(()),
            Ok(num) => panic!("invalid result from removeEffect: {}", num),
            Err(e) => Err(e),
        }
    }

    pub fn add_source<S: SoundSource>(&mut self, source: S) -> Result<i32> {
        let result = pd_func_caller!(
            (*self.raw_subsystem).addSource,
            self.raw_channel,
            source.get_sound_source()
        );
        self.sources.push(Box::new(source));
        result
    }

    pub fn remove_source<S: SoundSource>(&mut self, source: S) -> Result<bool> {
        let result = pd_func_caller!(
            (*self.raw_subsystem).removeSource,
            self.raw_channel,
            source.get_sound_source()
        );
        self.sources
            .retain(|s| s.get_sound_source() != source.get_sound_source());
        result.map(|r| r != 0)
    }
}

impl Drop for SoundChannel {
    fn drop(&mut self) {
        // Sources and effects must be removed before they are freed, otherwise you get
        // crashes in the audio thread. We could alternatively keep the sound channel alive
        // until all sources/effects are dropped but this also keeps the audio playing which is probably
        // undesirable if someone has dropped the channel.
        for source in &self.sources {
            pd_func_caller_log!(
                (*self.raw_subsystem).removeSource,
                self.raw_channel,
                source.get_sound_source()
            );
        }

        for effect in &self.effects {
            pd_func_caller_log!(
                (*self.raw_subsystem).removeEffect,
                self.raw_channel,
                effect.get_sound_effect()
            );
        }

        pd_func_caller_log!((*self.raw_subsystem).freeChannel, self.raw_channel);
    }
}
