# Prepared shell C entry (extension v1)

Include `rising_shell.h`; existing engine ABI1.6, render contracts and layouts
are unchanged. Swift may import it through its lane's bridging header (the
existing `RisingEngine` module map is deliberately unchanged). The dylib exports
`rl_shell_*` directly. No app file is changed in this lane.

The dependency is **640a04d48ec9b8e4bcd1c9714cb1f3d39b3f1bd0**;
[MENU_SHELL](../research/MENU_SHELL.md) defines the prepared Rust semantics.
This extension is host policy and copying/lifetime code, not a new original
routine port. Its native differential matrix **13,278/13,278** is a pinned
dependency; the ABI checks do not rerun or extend that claim.

## Ownership and preparation

A session owns a snapshot of an existing real world and copied renderer inputs.
It survives `rl_liberar_mundo(source)`. It never changes that source. Export
publishes a fresh independent world; it survives `rl_shell_free(session)`.
Neither handle retains host pointers. Shell and world handles use the common
monotonic token allocator and cannot be confused with each other. Serialize
calls per session, including free. Session generation starts0; changed entry,
input and drawing increment it. Copy/plan/export and same-mode entry do not.
Stale generation returns11. All error returns preserve outputs and state;
there is no partially rendered success. Read `rl_erro_mensagem` immediately
for original addressed stops. Fixed output is800×600 indices, stride800, and
768 palette bytes (256 RGB24 entries, existing renderer convention).

Before creating a session, complete **actual** preparation on the source world.
`rl_shell_plan` exposes `Screen::required_game_calls()` verbatim, in order:
main2 requires mpClose0x46ce60, RechercheReset0x470b46,
Engin_ResetExperience0x441470, InitPlayer0x46dcb4; briefing17 requires
RechercheCalculNew0x47150f. Other supported modes3/16/19/23 have no listed calls.
The corresponding mask bits attest these completed calls; they do not execute
or validate them. Missing attestation returns an explicit unsupported error.
If preparation requires changing the world after a plan query, create a NEW
session from the prepared world. Never enter using the old snapshot.
Same-mode entry retains the existing surface and original no-op behavior.
The constructor does not reconstruct the picture of an already-entered screen.

Use actual startup/mission/save state already produced by the engine. Existing
`rl_missao_iniciar`, `rl_missao_continuar`, `rl_carregar_textos` and modern save
APIs remain the supported services; do not duplicate their gameplay work.
This lane adds no public automatic main-menu preparer, MissionReset or full
MenuInit/CalculGame. If the host lacks one of the listed preparers, report that
boundary and continue through coordinator-owned preparation work. An S0 UI
fixture is validation input, not a production campaign bootstrap. Empty worlds,
zero research/gameplay state and empty callbacks are not substitutes.

## App wiring after the campaign chooser

1. Add `#include "rising_shell.h"` to the app lane's C/Swift bridge and check
   `rl_shell_version()==1`. Retain the active world's actual carried
   `RlFrameInput`, virtual clock and input queue ownership separately.
2. Prepare the source as above, including actual PT-BR `TEXTES.TXT` through
   `rl_carregar_textos` if its text table is not loaded. Call `rl_shell_create`
   with the existing nine renderer blobs, original MENU/INVENTE CMPs and the
   explicit initial800×600 surface. Banks are hash-validated by the dependency.
   Private terrain-cache scratch starts empty and is never read by shell drawing.
3. Query `rl_shell_plan(target_mode)`. Resolve its original background path in
   the bundle: TITRE.PIE for2/3/16; PAGEMIS2.PIE before mission7, otherwise
   PAGEMIS.PIE for17/23; ECRANFIN.LZS for19. Supply complete decoded original
   pixels. Reuse `shell_picture::pie/lzss` through the host's asset preparation;
   this v1 extension accepts decoded bytes, not file envelopes or OS paths.
4. For briefing/objectives supply an exact binary file map, preserving CP1252,
   CR/LF and DOS EOF bytes. The original path is the byte string at0x498430
   with byte12/13 set to the two mission digits, prefixed first by the CD-root
   string at0x509594, then (if absent) installed-root at0x509494. These strings
   are available through existing world projection calls. Do not lowercase,
   translate, basename-match or silently treat host I/O errors as absence.
   Read the file(s) first; a missing map entry deliberately represents absence.
   The native loader retains old count/dirty state on failed second open. Empty
   files are valid. `Files` open/read/close order belongs to the Rust dependency;
   the host map uses exact paths and bounded reads without OS callbacks.
5. Call `rl_shell_enter` with plan generation and the preparation mask. Copy
   with `rl_shell_copy` and upload indices/palette through the app's existing
   display path. Capacities are fixed constants; a size query returns6 and
   writes nothing. For animation, call `rl_shell_draw` with the actual virtual
   counter0x508f84, then copy. There is no implicit tick or wall-clock read.
6. Convert host input to completed logical key actions, suppress repeats, and
   scale mouse coordinates to original800×600 space before supplying raw u16
   words/u8 flags. Keys retain native Enter/Up/Down/Escape priority. Supply
   counters0x508f7c/84 explicitly. `rl_shell_input` returns the exact Request;
   finish it before another shell input/draw. The prepared API does not execute
   CalculGame's common-tail0x5099b0 clear; the host supplies its subsequent raw
   flag value explicitly and must preserve that tail when returning to gameplay.
7. Export current bytes with `rl_shell_export_world` when gameplay/save/chain
   continuation needs them. Use the new handle with the real existing services,
   paired with the original carried input. Export keeps source clock, execution
   mode and frame phase; it does not synthesize start registers, advance time,
   copy pending save-input queues, run load-tail, or mark the mission started.
   The source cursor hold is preserved as well; provide the actual current host
   cursor explicitly on the next gameplay input. Counter inputs do not advance
   the retained world clock; host clock continuation remains explicit.
   Free old world/renderer/session only after the new continuation succeeds.

## Requests and host gaps

| Event | Host continuation |
|---|---|
| SHOW(mode) | Finish that screen's preparers, then enter its original picture |
| RESET_CAMPAIGN(next_mode17/13) | Original MissionReset/player setup;17 also requires RechercheCalculNew |
| START_MISSION | `rl_shell_campaign_load` after actual briefing input sets mode0; preserve its research selection and then complete the existing load tail |
| MISSION_NEXT | `rl_shell_campaign_prepare` pauses at the real briefing; the older `rl_missao_continuar` remains available for explicitly host-confirmed briefing. End of campaign mode24 stays unsupported |
| RESUME | Original mode18 transition via existing `frame_input::menu`, including zones, viewport, bank lifecycle and music selection; there is no new resume ABI in this extension |
| SAVE_LOAD(mode12/13/14) | Existing modern save/load UI/service; preserve selected mode |
| INTRODUCTION | Host plays `event.path`, then `rl_shell_defeat_finish` (absent/finished) runs MenuInit(1), CalculGame mode1 and MenuInit(2) |
| CREDITS | `rl_shell_menu_init` with `<CD root>\Events\CREDITS.FLC` in the VFS enters mode10; `rl_shell_credits_frame` per host frame (rising_shell.h) |
| MULTIPLAYER/QUIT | Platform or separately owned screens/services |
| DEFEAT_MOVIE(path) | Raw CD-root failure path; check/play it, then prepare main2 when absent/finished |

**Never resume by writing mode18 alone.** Its original0x4684ae music selection
consumes the shared permutation cursor; inserting/reordering it changes later
RNG behavior. Shell entry/input/drawing/export do not consume RNG/permutation.
The existing frame load-tail reaches `frame_input::menu` in its supported domain;
it is not a general resume shortcut for unprepared shell/platform resources.
Return a clear unsupported continuation if that domain is unavailable.
The same rule applies to START_MISSION: do not replace the exported briefing
state with a fresh `rl_missao_iniciar` world and silently lose its selected
research. The fresh loader remains useful for the existing campaign chooser;
a briefing-start adapter must preserve the original preparation order/state.

Result19 keeps the actual latch at0x4e1d84. Both latches show the original
“Missão Completa”; latch0 requests the failure movie immediately, nonzero
requires activation before MissionNext. No invented defeat panel, automatic
progression, confirmation, movie completion or ignored request is introduced.
Gameplay result preparation is now owned by `campaign_flow::result`; use the
additive repaint call below to display that already-prepared mode19.

## Reproduction and handoff

Run `sh motor/testes/shell_abi_validate.sh motor/target/shell-abi-final`.
It takes one local mini heavy lease with four slots and empty work-hours,
uses the existing fixture exporter, runs strict Rust checks, original-backed
prepared-API equivalence, strict C11/C++ header/client checks (including C
ASan/UBSan), then the unchanged `sh motor/validate.sh`. No native menu routine
matrix is rerun. Logs and private media-derived binaries stay under target;
committed receipts are in `validation/shell-abi/` (private research repository).

API inputs, bounds, layouts, callbacks (none), errors and ownership are in the
header. Mutation order is supplied raw mouse/counters -> original shell input
-> returned request; entry is actual preparation -> original picture/research/
caption/zones/draw; draw sets only the explicit animation counter before the
original prepared draw. No other owner file or research evidence is modified.
Original oracle identity remains RISING.EXE SHA256
`6c556520c084b9300d9ac01900d1505126085c5f64b1ac5e5b615ea41c8543fb`.
No stop-free play time, app integration, human-play or frame-rate result is
measured by the original prepared-ABI tests. The additive campaign coverage
below closes its named preparation gaps; platform requests remain explicit.

## Campaign suspension extension (overnight lane B)

The additive functions below preserve the prepared-shell v1 ABI. They close the
result repaint and between-stage preparation gaps described above; they do not
wire the macOS UI or play an existing defeat movie.

1. Create the shell from the actual result world. Decode its real ECRANFIN.LZS
   background and call `rl_shell_paint_prepared`, even when `plan.same_mode=1`.
   This redraws the prepared caption/screen without re-running MenuInit, changing
   world bytes, advancing clock/RNG or consuming a rendering effect twice.
2. Let `rl_shell_input` return the actual MISSION_NEXT request. Resolve
   `rl_missao_nome_proxima` against the CD, with I/O errors handled explicitly.
   Supply that exact ETA path and the actual briefing-file VFS to
   `rl_shell_campaign_prepare`. No host preparation mask is used: this call runs
   MissionNext, its carry-over stores, briefing load, research preparation and
   MenuInit17 in original order. The session retains mode17 before loading.
3. Paint the prepared briefing with `rl_shell_paint_prepared` and the original
   PAGEMIS2.PIE/PAGEMIS.PIE selected by its plan. Send the actual research clicks
   and confirmation through `rl_shell_input`. No detached display copy is used.
   The source world remains immutable; research changes belong to the session.
4. Only after START_MISSION sets mode0, call `rl_shell_campaign_load` with that
   session generation, the same exact ETA, MONSTRE.INI and the carried input.
   Supply the last shell cursor in that input; mismatched cursor words are an
   error. The new world preserves the selected research, inherited EVENTS,
   source clock and execution mode. This call neither repeats MissionNext nor
   recalculates research. Failed calls publish no world or output mutation.
5. The returned world is at MissionLoad's return. Complete the existing declared
   phase1 load-tail/carried-register protocol, first render and stage autosave
   before replacing the app's old world. Load-tail advances the clock once.
   Do not invoke the legacy `rl_missao_continuar` on this already advanced world.

Preparation requires a real preceding MISSION_NEXT event; load requires a real
START_MISSION event. Stale generations, wrong names, missing ETA descriptors,
invalid input and failing original loads remain errors. Repaint and preparation
increment shell generation. Load creates a new independent world and leaves the
session unchanged, so a failed load can be retried without replaying stores.
Prepared-shell raw mouse/counter/common-tail ownership remains as documented
above; this extension does not synthesize host frame timing or input events.

## Defeat movie host completion

`RL_SHELL_DEFEAT_MOVIE` carries the original raw path. Retain the result session
while the host resolves playback. Do not fabricate a file lookup or complete the
request merely because another game frame arrived. When the host cannot play
it (missing file or unavailable decoder), report `RL_SHELL_MOVIE_UNAVAILABLE`;
after actual playback ends, report `RL_SHELL_MOVIE_FINISHED`.

Call `rl_shell_defeat_finish` with the event's current generation. Both outcomes
join the original branch at `0x42c925` and execute `MenuInit(2)`, including its
research, experience and player reset effects. Failure is atomic; a request
cannot complete twice. The call leaves the source world and its clock unchanged.
Then paint the current main screen with the original `titre.pie` using
`rl_shell_paint_prepared`; do not repeat preparation. The prepared API's existing
common-tail `mouse_b0` ownership remains with the host. Export a new world
explicitly if the host needs to retain the prepared gameplay memory.

The headless `Presenter::step` similarly returns a `DefeatMovie` request instead
of decoding video or failing at `PlayVideo`. Repeated polling preserves the last
completed world/input/save boundary. `finish_defeat_movie` explicitly resolves
it, publishes the original menu transition, and completes exactly that frame.
The diagnostic replay defaults to a pending-host artifact and does not claim
completion. `--movie-unavailable` declares its actual decoder-less host; its
report still records the real file's presence and SHA-256. It never claims to
have played that movie.

## Campaign preparation extension (Claude lane B, 2026-11-02)

These additive calls make the original menu fully preparable by the engine, so
the host no longer attests `plan.required_calls` by itself. They are the
original MenuInit 0x4682bb gameplay effects and the MenuNewSelect single-player
blocks 0x42c474/0x42c4d3; presentation (cursor, surface lock, pictures,
palette, bank handles, FilledBox/AffString pixels) stays with the host and the
prepared painter. The DDFlipPage gameplay prefix runs inside the call.

1. Start in the original main menu with `rl_missao_iniciar_menu` (or the
   `_eventos_v1` variant) and the first nine stage descriptors (no ETA). The
   world is ProgInit's gameplay state plus MenuInit(local player, 2): it has
   no frame state, must not be stepped, and keeps mode2. Create the prepared
   shell on it and paint `titre.pie` with `rl_shell_paint_prepared`.
2. Every `RL_SHELL_SHOW` event is completed with `rl_shell_menu_init`, then
   painted with the plan background. Supported targets: main2 (mpClose,
   RechercheReset, Engin_ResetExperience, InitPlayer), single3, pause16,
   briefing17/objectives23 (ObjectifLoadText, zones, RechercheCalculNew for
   17) and the save/load lists12/13/14. Unsupported targets keep their
   original addressed stop; a local-player mismatch is the original no-op.
3. `RL_SHELL_RESET_CAMPAIGN` (single-player item0 "new game" -> mode17, item1
   "load game" -> mode13) is completed with `rl_shell_campaign_reset`:
   MissionReset (stage1), local player0, SetPlayerType human/computer x3,
   MenuInit(17|13), HUD refresh latch. For17 supply the original
   `WORLD\MONDE01.TXT` paths exactly as for rl_shell_enter; the briefing
   research clicks then go through `rl_shell_input`, START_MISSION sets mode0
   and `rl_shell_campaign_load` loads MONDE01 with the chosen research. Set the
   stage world's execution mode with `rl_execucao_definir` before the load tail
   when the source was the menu world. Difficulty presets apply to that fresh
   world with `rl_aplicar_dificuldade` (the capability is granted at the
   MissionLoad boundary, with the clock still running).
4. Save/load lists (12/13/14) run GameSaveScanNames over the ten original slot
   paths `save1.sav`..`save9.sav`,`save0.sav` (exact bytes at0x497128). The
   VFS entry for a slot is the header the original reads: `SAV` plus a version
   digit; `'3'` or later is followed by the32-byte caption, anything else gets
   the original default caption `Sauvegarde N`, and an absent entry gets text
   0x72. The host maps its modern saves onto these headers; the engine does not
   read modern save files here. Selecting a slot (`RL_SHELL_SAVE_LOAD`) remains
   the host's modern save/load implementation.

All four calls are atomic on failure, generation-checked and leave the source
world unchanged. The native evidence is
menu-native.json (private research repository): whole
intact MenuInit for targets2/3/12/13/14/16/17/23 and both MenuNewSelect
blocks from the actual played D1 main menu, a real in-game S1 state and the
Rust ProgInit prelude, with byte-exact FileOpen/FileRead/FileSeek/FileClose
traces. ProgInit's colour-table routines (0x474eff..0x474f2e) are not ported
yet; `mission_boot::prog_init` is the single place where their port lands.

### Save-slot lists (block2)

The original lists12 (load in game), 13 (load from the main menu) and14 (save)
are prepared screens. `rl_shell_menu_init` completes `RL_SHELL_SHOW`/`RL_SHELL_SAVE_LOAD`
into them (GameSaveScanNames over the slot VFS, see above) and
`rl_shell_campaign_reset` with mode13 enters the main-menu load list; paint with
`titre.pie`. Input is the shared CalculGame list block: Up/Down/mouse select,
Escape returns MenuBefore (SHOW single3 from13, pause16 from12/14) and Enter or
a click on an item yields `RL_SHELL_LOAD_SLOT` (kind12, mode12/13) or
`RL_SHELL_SAVE_SLOT` (kind13, mode14) with `reserved` = slot0..9 and `path` =
the exact original slot filename. The original continues with FileExists,
GameLoad0x46b88d (13 additionally re-initialises the cache/viewport) or
GameSave0x46bd45, then MenuInit(18); the engine stops at that host boundary:
the host performs its modern load/save with its own save files, replaces the
world (load) or resumes the game (save), and frees or keeps the session. To
resume after a save, complete the SAVE_SLOT request with `rl_shell_resume`
(below): it runs the original MenuInit(18) and the HUD refresh latch, so the
game continues directly as in the original instead of reopening the pause menu. No
legacy `.sav` is read or written by the engine. `rl_shell_enter` refuses these
lists (0x469292): their captions come from the host files, so entry is always
through preparation.

### Campaign end (block3)

When `rl_shell_campaign_prepare` is given the engine's own next ETA path with
no VFS entry for it, the original CalculGame 0x42ca9d continuation runs:
MissionNext (stage incremented) and MenuInit(local player,24). The session is
then mode24 (`RL_SHELL_FINAL`, paint `titre.pie`; AffGame draws nothing new);
the next `rl_shell_input` returns `RL_SHELL_CAMPAIGN_MOVIE` (kind14) with the
original CD-root path `\events\extrofr.avi`, exactly as the defeat branch
returns its movie. Complete it with `rl_shell_defeat_finish` (unavailable or
finished): both join 0x42c925 and run MenuInit(2); then paint the main menu.
A name that is not the engine's next path is still an error (0x46c08c).

### In-game pause and continue (block4)

Escape held in game follows InputGame 0x429305: the pause order13 is queued for
the local player and ProcessInputGame runs the original MenuInit(16) in the same
frame (the key-release wait is the host's; the engine silences the sound device
as the original does). The world is then mode16: create the prepared shell on it
like the result screen, paint `titre.pie` and drive the pause menu. Escape or the
"continue" item yields `RL_SHELL_RESUME` (kind5): call `rl_shell_resume` with the
host's current `RlFrameInput`; it runs the original in-game MenuInit(18), returns
the resumed world (same clock/execution mode) whose first `rl_eventos_quadro`
carries the ShowCursor binding (kind1 id1) and the LoadPie/MCICD_Play boundaries (kind2 ids101/102) exactly as after the
load tail, and the carried input unchanged. Free the session.

After a save in list14 (`RL_SHELL_SAVE_SLOT`, kind13, mode14) the host first
writes its own modern save, then calls the same `rl_shell_resume`: the original
MenuNewSelect 0x42c570 tail runs MenuInit(local player,18) and sets the HUD
refresh latch 0x4e05a2=1; the returned world, events 101/102 and input follow
the RESUME contract above. GameSave0x46bd45 itself stays the host's: its only
declared-memory effect is the slot-name rescan into 0x496fe8, which reproduces
the MenuInit(14) scan of the same files (native rows `game-gamesave-*`). The
engine does not write or verify the host's save. The other pause
items are completed with `rl_shell_menu_init` (quit to main2, objectives23,
research17, lists12/14). Escape during a running in-game event keeps an explicit
stop (EvenementStop 0x458088 is not ported).
