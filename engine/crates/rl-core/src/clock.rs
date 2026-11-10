use crate::{EngineError, Result};
use std::num::NonZeroU32;

/// Integer virtual time; no wall clock, host delta or simulation speed factor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickClock {
    tick: u64,
    rate: NonZeroU32,
    epoch_ms: u32,
}
impl TickClock {
    /// 20Hz is pilot pacing policy, not an oracle-proven historical fixed FPS.
    pub const DEFAULT_RATE: u32 = 20;
    pub fn new(rate: u32, epoch_ms: u32) -> Result<Self> {
        Ok(Self {
            tick: 0,
            rate: NonZeroU32::new(rate).ok_or(EngineError::InvalidTickRate)?,
            epoch_ms,
        })
    }
    pub fn advance(&mut self, count: u64) {
        self.tick = self.tick.wrapping_add(count);
    }
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn milliseconds(&self) -> u32 {
        // u64*1000 fits u128. Intentional low32 truncation models a Win32 clock.
        let elapsed = (u128::from(self.tick) * 1000 / u128::from(self.rate.get())) as u32;
        self.epoch_ms.wrapping_add(elapsed)
    }
}
