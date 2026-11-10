//! Synchronous production-only declared load-tail transaction, version 1.
//! This is not a saved FIFO phase and does not sample the host cursor or pump messages.
use crate::{
    frame,
    player::{DeclaredInput, PlayerEvent, Runtime},
    EngineError, FrameEvent, FrameInput, MemoriaJogo, Result, World,
};

pub type TailOutput = (FrameInput, Vec<FrameEvent>, MemoriaJogo);

/// Execute the existing original load-tail body with a strictly local Runtime.
/// Raw fixture callers receive prefix mutations on error; World callers isolate them.
pub fn attempt(
    memory: &mut MemoriaJogo,
    input: FrameInput,
    answers: DeclaredInput,
) -> (Result<TailOutput>, Vec<PlayerEvent>) {
    if answers.cursor.is_some() || !answers.messages.is_empty() || memory.player.is_some() {
        return (Err(EngineError::InvalidFramePhase(1)), Vec::new());
    }
    memory.player = Some(Runtime {
        answers,
        events: Vec::new(),
        phase: 1,
        // 42d2be resets all three counters before the first player iteration.
        frame: 0,
    });
    let result = frame::phase(memory, 1, input);
    let runtime = memory.player.take().expect("local load-tail Runtime");
    (result, runtime.events)
}

impl World {
    /// Only a genuine post-MissionLoad production boundary (phase 0) is accepted.
    /// No host cursor answer is consumed or held. Inherited mouse bytes are untouched
    /// until an original routine changes them. Success publishes one generation/tick;
    /// failure publishes only stopped-input diagnostics, as in the declared-input ABI.
    pub fn load_tail_input_v1(
        &mut self,
        input: FrameInput,
        answers: DeclaredInput,
    ) -> Result<FrameInput> {
        if self.execution_mode() != 1
            || self.frame_phase != 0
            || answers.cursor.is_some()
            || !answers.messages.is_empty()
        {
            return Err(EngineError::InvalidFramePhase(1));
        }
        let mut memory = self.project()?;
        memory.audit = false;
        let (result, consumed) = attempt(&mut memory, input, answers.clone());
        match result {
            Ok((output, events, freed)) => {
                // replace_memory can inspect/rebind/fail. Do all of it on the candidate.
                let mut next = self.clone();
                next.replace_memory(memory)?;
                next.difficulty_setup = None;
                next.frame_phase = 1;
                next.frame_events = events;
                next.frame_freed = freed;
                next.player_events = consumed;
                next.clock.advance(1);
                *self = next;
                Ok(output)
            }
            Err(error) => {
                // Same explicit stopped-input diagnostics contract as play_phase.
                let mut diagnostic = self.project()?;
                diagnostic.audit = true;
                let (replay, events) = attempt(&mut diagnostic, input, answers);
                if replay.err().as_ref() != Some(&error) {
                    return Err(EngineError::ExecutionReplayMismatch);
                }
                self.player_events = events;
                Err(error)
            }
        }
    }
}

/// MenuInit's final DDFlipPage call 468ca2, after its margin/state stores and
/// DDUnlockSurface. RestoreAll's two earlier flips have already executed, and
/// CalculGame has reset the counters to zero. Called only by version-1 Runtime.
/// Literal gameplay prefix 43f15c..43f1a6: cursor store precedes shake decrement.
pub fn menu_flip_effects_v1(memory: &mut MemoriaJogo) -> Result<()> {
    let shake = memory.u8(0x4dfb44)?;
    if shake != 0 {
        let cursor = memory.u8(0x4e1d18)?;
        // Original permutation read feeds only the backend presentation offset.
        memory.u8(0x4e1c18 + u32::from(cursor))?;
        memory.set_u8(0x4e1d18, cursor.wrapping_add(1))?;
        if memory.u8(0x508f7c)? & 1 == 0 {
            memory.set_u8(0x4dfb44, shake.wrapping_sub(1))?;
        }
    }
    Ok(())
}
