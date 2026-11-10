#ifndef RISING_LOAD_TAIL_INPUT_H
#define RISING_LOAD_TAIL_INPUT_H
#include "rising_engine.h"
#ifdef __cplusplus
extern "C" {
#endif
/* Synchronous version-1 extension; ABI 1.6 and save format are unchanged.
 * Production execution mode 1, phase 0 immediately after successful MissionLoad
 * only. Caller attests that boundary; this does not turn raw imports into loaders.
 * Keys are a copied GetAsyncKeyState snapshot (omitted keys explicitly mean 0).
 * Escape's held/repeated polling and any unported menu/window path remain stops.
 * Only flags 2/4 (existing declared text/sound boundaries) are accepted.
 * Cursor flag 1 and messages are rejected: phase1 precedes the WinMain pump and
 * phase2 GetCursorPos. Cursor hold and inherited mouse button bytes are retained.
 * Pending saved input causes RL_SAVE_BUSY. Phase1 remains illegal in the FIFO.
 * Success: original RestoreAll/clipping/counter/menu/zone/CalculGame order,
 * +1 tick, +1 generation, phase1. Caller installs its phase2 registers afterward.
 * Failure: world, clock, generation, cursor hold, caller input, old frame events
 * and renderer unchanged; only the separate stopped-input diagnostics may change.
 * Pointers/spans aligned, valid and disjoint; no pointer retained. */
int32_t rl_load_tail_entrada_v1(RlMundo world, RlFrameInput *input,
                              const RlDeclaredInput *declared);
#ifdef __cplusplus
}
#endif
#endif
