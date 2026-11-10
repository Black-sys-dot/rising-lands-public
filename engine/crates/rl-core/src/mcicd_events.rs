//! Sparse production transport of ALREADY executed original CD call boundaries.
//! No selector, device state, clock answer or RNG call is introduced here.
use crate::frame::FrameEvent;

pub(crate) fn retained(event: &FrameEvent) -> bool {
    event.kind == 2
        && matches!(
            (event.id, event.site),
            (102, 0x42d32d | 0x4684bd) | (104, 0x460225)
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::EventLog;

    #[test]
    fn production_keeps_exact_cd_registers_in_order_without_other_audits() {
        let mut log = EventLog::new(false);
        for (id, site) in [
            (103, 0x460220),
            (104, 0x460225),
            (102, 0x42d32d),
            (102, 0x4684bd),
            (102, 0),
        ] {
            log.push(FrameEvent {
                kind: 2,
                id,
                site,
                args: [0, 0, 0, 0, 0],
                registers: [3, 0, 255, 2],
                ..Default::default()
            });
        }
        let events = log.into_vec();
        assert_eq!(events.len(), 3);
        assert_eq!(
            events.iter().map(|e| e.site).collect::<Vec<_>>(),
            [0x460225, 0x42d32d, 0x4684bd]
        );
        assert!(events
            .iter()
            .all(|e| e.registers == [3, 0, 255, 2] && e.args == [0; 5]));
    }
}
