# In-game FLC events (ABI addition, lane B, 2026-11-03)

Original behaviour (intact `RISING.EXE`, SHA-256 `6c556520…`): a notification queued
by MessageBoxSet (kind 10) shows its text and a blinking icon at the bottom left of
the game view. Clicking it (InputGame zone 0x1b, 0x42bc98) runs MessageBoxRun
0x469baf, which opens the notification's EVENEMENT name as
`<CD root 0x509594>\Events\<NAME>.FLC` (EvenementInit 0x4580d4, FlicOpen 0x45a60c).
When the file opens, the game logic pauses (CalculGame skips its body while
0x4e1b0e is set), that frame's AffGame greys the view above the 42-pixel HUD band
(FilledBoxGris 0x45cd91) and starts the event sound, and the game enters the modal
mode 22. In mode 22 every AffGame draws the frame border bob 0x2b1c and the current
FLC frame centred in the view, with the FLC's own palette (DDSetPalette); the frame
advances with the wall clock (timeGetTime / FLC speed). After the last frame the
view is greyed again, the event ends (FlicClose, next notification, sounds), the
game palette 0x4e24b0 is restored and the game returns to mode 18. A click or
Escape in mode 22 (CalculGame 0x42d197), or Escape in game (InputGame 0x42933f),
ends it at once (EvenementStop 0x458088). If the file does not open nothing happens.

## Engine / host split

The engine owns every state change. The host owns file access, FLC decoding,
palette and timing, as the original's DirectDraw/CRT layer did.

1. Declare the table once per world, before its first frame (stage entry, campaign
   load, every new world the host creates or loads):

   ```c
   typedef struct { uint8_t name[16]; uint32_t available; uint8_t header[128]; } RlEventFlcV1;
   int32_t rl_eventos_flc_v1(RlMundo world, const RlEventFlcV1 *entries, uint32_t count);
   ```

   One entry per distinct EVENEMENT value of `INSTALHD\EVENTS.INI` (case-insensitive;
   these are the only names a notification can open), checked against the
   `Events` folder of the CD root in use (in the player build, the imported copy of
   the Internet Archive CD). Extra entries for other `Events\*.FLC` files are
   allowed. Note that the original data name some events that are not on the CD
   (EVENTS.INI says `HELICOP`, the CD has `HELICO.FLC`): declare those as
   unavailable, exactly as the original's failed open. `name` is the stem (1..15 ASCII
   letters/digits/`_`, NUL-terminated, unique ignoring case, e.g. `LABO3`).
   `available = 1` and `header` = the file's first 128 bytes when the file opens and
   is at least 128 bytes long; `available = 0` (header ignored) when it does not open
   — the original FlicOpen then returns 0 and the click does nothing. 1..256 entries;
   a second declaration on the same world is refused. The table is part of the world
   and its saves. A clicked notification whose name is not in the table is an
   addressed engine stop at 0x45a640, never treated as absent.

2. After each frame (phase 3), poll:

   ```c
   typedef struct { uint32_t running, modal, index, frames, width, height, speed; } RlEventFlcStateV1;
   int32_t rl_evento_flc_estado_v1(RlMundo world, RlEventFlcStateV1 *output);
   ```

   `running` 1 once an event has opened (the click frame); `index` is the entry's
   position in the declared table; `frames`, `width`, `height`, `speed` are the
   header's +6, +8, +10 (u16) and +0x10 (u32, the FLC frame delay). `modal` is 1 in
   mode 22 (from the AffGame after the click frame on).

3. While `running`: keep stepping frames as usual (phases 2/3; the engine draws the
   grey band and the border bob through the ordinary renderer). Overlay the decoded
   FLC frame at x = (800 − width) / 2, y = (600 − height) / 2 (truncating division,
   game coordinates, 800×600 view) and apply the FLC palette chunks to the whole
   indexed screen, advancing frames by your clock at the FLC's speed.

4. When you have shown the last frame, call once:

   ```c
   int32_t rl_evento_flc_terminar_v1(RlMundo world);
   ```

   It requires a running event in mode 22 (else `RL_UNSUPPORTED…`/stop error, world
   unchanged). It records the state the original player leaves after its last frame
   (frame counter = `frames`, end flag 0x4e1b14); the next AffGame greys the view
   and runs the original end; the following frames are mode 18 again with the game
   palette (restore your palette to the engine's game palette).

5. A click or Escape delivered as ordinary declared input in mode 22, or Escape in
   game, ends the event inside the engine: `running` becomes 0; stop playback and
   restore the game palette.

Sounds requested by the event (PostSound 0x41 at start and end, StopAllSound) go
through the engine's existing sound requests. The FLC's own audio, if any, is host
presentation.
