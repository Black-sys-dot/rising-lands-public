// Included in clock's module so the on-disk clock has no inferred timing values.
impl TickClock {
    pub(crate) fn save_parts(&self) -> (u64, u32, u32) {
        (self.tick, self.rate.get(), self.epoch_ms)
    }
    pub(crate) fn from_save_parts(tick: u64, rate: u32, epoch: u32) -> Result<Self> {
        let mut clock = Self::new(rate, epoch)?;
        clock.tick = tick;
        Ok(clock)
    }
}
