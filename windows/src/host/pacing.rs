//! Host frame scheduling, ported from `NativeGameSpeed.swift`.
//!
//! One engine frame is one original frame at every speed: speed only changes
//! how many complete frames the host runs per wall-clock second. The engine's
//! virtual TickClock stays at 20 Hz; that is virtual time, not the play pace.

/// Player-facing speed over the engine's virtual clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GameSpeed {
    Half,
    Normal,
    Double,
    Quadruple,
}

impl GameSpeed {
    pub const ALL: [GameSpeed; 4] = [
        GameSpeed::Half,
        GameSpeed::Normal,
        GameSpeed::Double,
        GameSpeed::Quadruple,
    ];

    /// The original's nominal `ticks_per_second`; never used as the play rate.
    pub const TICK_CLOCK_RATE: f64 = 20.0;
    /// Complete game frames per wall-clock second at 1x.
    pub const NORMAL_FRAME_RATE: f64 = 30.0;

    pub fn multiplier(self) -> f64 {
        match self {
            GameSpeed::Half => 0.5,
            GameSpeed::Normal => 1.0,
            GameSpeed::Double => 2.0,
            GameSpeed::Quadruple => 4.0,
        }
    }

    /// Complete game frames per wall-clock second at this speed.
    pub fn frame_rate(self) -> f64 {
        Self::NORMAL_FRAME_RATE * self.multiplier()
    }

    /// PT-BR label shown by the player menu.
    pub fn label(self) -> &'static str {
        match self {
            GameSpeed::Half => "0,5x",
            GameSpeed::Normal => "1x",
            GameSpeed::Double => "2x",
            GameSpeed::Quadruple => "4x",
        }
    }

    pub fn menu_title(self) -> String {
        format!("{} · {} quadros/s", self.label(), self.frame_rate() as u32)
    }

    pub fn from_frame_rate(rate: f64) -> Option<Self> {
        Self::ALL.into_iter().find(|speed| speed.frame_rate() == rate)
    }

    pub fn faster(self) -> Self {
        match self {
            GameSpeed::Half => GameSpeed::Normal,
            GameSpeed::Normal => GameSpeed::Double,
            GameSpeed::Double | GameSpeed::Quadruple => GameSpeed::Quadruple,
        }
    }

    pub fn slower(self) -> Self {
        match self {
            GameSpeed::Quadruple => GameSpeed::Double,
            GameSpeed::Double => GameSpeed::Normal,
            GameSpeed::Normal | GameSpeed::Half => GameSpeed::Half,
        }
    }

    pub fn stored_value(self) -> u32 {
        self as u32
    }

    /// An invalid or absent stored value falls back to 1x, as on macOS.
    pub fn from_stored(value: Option<u32>) -> Self {
        match value {
            Some(0) => GameSpeed::Half,
            Some(1) => GameSpeed::Normal,
            Some(2) => GameSpeed::Double,
            Some(3) => GameSpeed::Quadruple,
            _ => GameSpeed::Normal,
        }
    }
}

/// Decides when the worker runs the next complete frame.
///
/// A rate of 0 runs frames back to back (diagnostic "sem limite"). Late frames
/// still run one by one; wall-clock debt beyond `MAXIMUM_BACKLOG` (a stall, a
/// hidden window) is forgiven instead of replayed as a burst. Pause/resume and
/// speed changes re-anchor the deadline.
#[derive(Clone, Copy, Debug)]
pub struct FramePacer {
    rate: f64,
    deadline: f64,
}

impl FramePacer {
    pub const MAXIMUM_BACKLOG: f64 = 0.25;

    pub fn new(rate: f64, now: f64) -> Self {
        Self {
            rate: rate.max(0.0),
            deadline: now,
        }
    }

    pub fn rate(&self) -> f64 {
        self.rate
    }

    pub fn deadline(&self) -> f64 {
        self.deadline
    }

    pub fn restart(&mut self, now: f64) {
        self.deadline = now;
    }

    pub fn set_rate(&mut self, rate: f64, now: f64) {
        self.rate = rate.max(0.0);
        self.deadline = now;
    }

    /// Returns 0 and books one frame when it is due; otherwise the seconds to wait.
    pub fn delay(&mut self, now: f64) -> f64 {
        if self.rate <= 0.0 {
            return 0.0;
        }
        if now < self.deadline {
            return self.deadline - now;
        }
        if now - self.deadline > Self::MAXIMUM_BACKLOG {
            self.deadline = now;
        }
        self.deadline += 1.0 / self.rate;
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speeds_and_labels_are_the_macos_table() {
        assert_eq!(
            GameSpeed::ALL.map(|s| s.frame_rate()),
            [15.0, 30.0, 60.0, 120.0]
        );
        assert_eq!(
            GameSpeed::ALL.map(|s| s.label()),
            ["0,5x", "1x", "2x", "4x"]
        );
        assert_eq!(GameSpeed::from_frame_rate(60.0), Some(GameSpeed::Double));
        assert_eq!(GameSpeed::from_frame_rate(40.0), None);
        assert_eq!(GameSpeed::from_stored(None), GameSpeed::Normal);
        assert_eq!(GameSpeed::from_stored(Some(9)), GameSpeed::Normal);
        assert_eq!(GameSpeed::from_stored(Some(3)), GameSpeed::Quadruple);
    }

    #[test]
    fn unlimited_rate_never_waits() {
        let mut pacer = FramePacer::new(0.0, 0.0);
        assert!((0..100).all(|_| pacer.delay(0.0) == 0.0));
    }
}
