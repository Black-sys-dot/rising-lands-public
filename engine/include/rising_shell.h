#ifndef RISING_SHELL_H
#define RISING_SHELL_H
#include "rising_engine.h"
#ifdef __cplusplus
extern "C" {
#endif
/* Independent prepared-screen extension v1. Existing engine ABI stays1.6.
 * 64-bit hosts, native C alignment; integer handles, never host pointers.
 * Session owns a COPY of world/render/text inputs, survives source world free.
 * It never updates that source world. Export explicitly publishes a NEW world.
 * Caller serializes session use/free. All input spans are copied during calls;
 * all pointers must be valid/aligned, outputs disjoint from input/output spans.
 * Errors leave session, source world and output bytes unchanged (including
 * BUFFER_TOO_SMALL). rl_erro_mensagem gives the addressed TLS diagnostic.
 * Gameplay preparation is explicit in campaign_prepare/campaign_load and
 * defeat_finish below.
 * Other calls do not prepare gameplay, tick the clock or consume game RNG.
 */
#define RL_SHELL_VERSION 1u
#define RL_SHELL_PIXELS 480000u
#define RL_SHELL_PALETTE_BYTES 768u
#define RL_SHELL_WIDTH 800u
#define RL_SHELL_HEIGHT 600u
#define RL_SHELL_STRIDE 800u
typedef uint64_t RlShell;
typedef struct { const uint8_t *data; uint32_t length; } RlShellBytes;
typedef struct { RlShellBytes path, bytes; } RlShellFile;
enum { RL_SHELL_MAIN=2, RL_SHELL_SINGLE=3, RL_SHELL_LOAD_LIST=12,
 RL_SHELL_LOAD_MENU=13, RL_SHELL_SAVE_LIST=14, RL_SHELL_PAUSE=16,
 RL_SHELL_BRIEFING=17, RL_SHELL_RESULT=19, RL_SHELL_OBJECTIVES=23,
 RL_SHELL_CREDITS_SCREEN=10 };
enum { RL_SHELL_ENTER=1, RL_SHELL_UP=2, RL_SHELL_DOWN=4, RL_SHELL_ESCAPE=8 };
typedef struct {
 uint32_t version, keys, mouse_x, mouse_y, mouse_ae, mouse_af, mouse_b0,
 clock_7c, animation_84, reserved;
} RlShellInput;
enum { RL_SHELL_NONE=0, RL_SHELL_SHOW=1, RL_SHELL_RESET_CAMPAIGN=2,
 RL_SHELL_START_MISSION=3, RL_SHELL_MISSION_NEXT=4, RL_SHELL_RESUME=5,
 RL_SHELL_SAVE_LOAD=6, RL_SHELL_INTRODUCTION=7, RL_SHELL_CREDITS=8,
 RL_SHELL_MULTIPLAYER=9, RL_SHELL_QUIT=10, RL_SHELL_DEFEAT_MOVIE=11,
 RL_SHELL_LOAD_SLOT=12, RL_SHELL_SAVE_SLOT=13, RL_SHELL_CAMPAIGN_MOVIE=14 };
/* CAMPAIGN_MOVIE: rl_shell_campaign_prepare found no next stage (the VFS has no
 * entry for the engine's own next ETA path), so the session entered the
 * original campaign-end screen (mode24, RL_SHELL_FINAL, painted with titre.pie).
 * The next rl_shell_input returns this request with the original CD-root path
 * (\events\extrofr.avi); complete it with rl_shell_defeat_finish (unavailable
 * or finished), which runs MenuInit(2) like the defeat branch. */
#define RL_SHELL_FINAL 24u
/* INTRODUCTION (kind7, mode1): the main-menu item. event.path is the exact
 * original CD-root movie path (\events\introfr.avi). The host probes and plays
 * it, then completes the request with rl_shell_defeat_finish: UNAVAILABLE = the
 * file is absent (original DDSetPalette branch), FINISHED = the file exists
 * (played, or the host has no decoder: original FileClose + PlayVideo, whose
 * declared-memory effects the engine runs). Both run MenuInit(1), CalculGame
 * mode1 and MenuInit(2); then paint main with titre.pie, as after a defeat.
 * CREDITS (kind8): complete with rl_shell_menu_init. Its VFS answers the exact
 * path <CD root>\Events\CREDITS.FLC (first 128 bytes are read; a missing entry
 * means the file does not open). The session enters mode10: paint the plan
 * background (\credits.pie) with rl_shell_paint_prepared, then call
 * rl_shell_credits_frame once per host frame instead of rl_shell_draw. */
/* LOAD_SLOT/SAVE_SLOT: a slot was activated in an original list prepared by
 * rl_shell_menu_init (mode12 in-game load, 13 main-menu load, 14 save):
 * event.mode is that list, event.reserved the slot index0..9 and event.path
 * the exact original slot filename ("save1.sav".."save9.sav","save0.sav").
 * The original then runs GameLoad/GameSave on that file and MenuInit(18); the
 * host performs its own modern load/save and resumes or replaces the world.
 * Escape in a list returns MenuBefore: SHOW single3 from13, pause16 from12/14. */
typedef struct {
 uint64_t generation;
 uint32_t kind, mode, path_length, reserved;
 uint8_t path[512]; /* Raw length-delimited CP1252/DOS path, no NUL promised. */
} RlShellEvent;
typedef struct {
 uint64_t generation;
 uint32_t mode, same_mode, required_count, required_mask;
 uint32_t required_calls[4]; /* Original addresses in exact call order. */
 uint8_t background[32]; /* NUL-terminated original relative asset path. */
} RlShellPlan;
uint32_t rl_shell_version(void);
/* count=11, total<=64MiB. Nine blobs in rl_render_criar order, then original
 * MENU.CMP and INVENTE.CMP. Source must contain real prepared gameplay and
 * original texts (rl_carregar_textos). surface.length=480000: caller's initial
 * surface, retained on same-mode entry. Private unused terrain cache starts
 * empty; it is not game state. Session generation starts0. */
int32_t rl_shell_create(uint32_t version, RlMundo world,
 const RlShellBytes *assets, uint32_t count, RlShellBytes surface, RlShell *output);
/* Query only. Main: mpClose, RechercheReset, Engin_ResetExperience, InitPlayer;
 * briefing: RechercheCalculNew. Others: none. A same-mode entry is a no-op.
 * The plan is a dependency list, not evidence those calls ran. */
int32_t rl_shell_plan(RlShell shell, uint32_t mode, RlShellPlan *output);
/* prepared_mask MUST attest actual required calls completed in plan order on
 * source world BEFORE creating this session. Missing attestation -> UNSUPPORTED.
 * Do not use zero-filled worlds or empty gameplay callbacks as preparation.
 * background: complete original decoded800x600 indexed pixels, per plan path.
 * files: <=64 exact raw path->binary mappings, total<=64MiB, unique nonempty
 * paths<=512 bytes without NUL. Empty file is valid. Missing entry explicitly
 * means absent file; host I/O errors must be handled BEFORE entry. Includes
 * original CD-root or installed-root WORLD/MONDExx.TXT paths. Original loader
 * probes CD, falls back to installed, then opens/reads/closes in native order.
 * See shell-entry.md for prefix/filename projection and continuation gaps.
 * Same mode retains surface/memory/generation, needs no preparation mask.
 * Changed-mode success increments generation. No files/pointers retained. */
int32_t rl_shell_enter(RlShell shell, uint64_t generation, uint32_t mode,
 uint32_t prepared_mask, RlShellBytes background, const RlShellFile *files,
 uint32_t count, uint64_t *output_generation);
/* keys bitmask0..15, native Enter/Up/Down/Escape priority. Completed logical
 * press/release, host suppresses repeats. mouse_x/y raw u16 words (no scaling);
 * ae/af/b0 raw u8 flags. Counters7c/84 are explicit original virtual counters.
 * reserved=0. Successful input increments generation and returns ONE Request;
 * host must complete it before further input/draw. NONE permits another frame.
 * Common-tail0x5099b0 clear stays host-owned, as in the prepared Rust API.
 * Latch0 -> DEFEAT_MOVIE without Enter; nonzero+activation -> MISSION_NEXT.
 * Both native result latches display the original “Missão Completa” caption. */
int32_t rl_shell_input(RlShell shell, uint64_t generation,
 const RlShellInput *input, RlShellEvent *output);
/* Sets only explicit animation counter84, draws the persistent indexed surface.
 * Success increments generation; result19 preserves its initial picture. */
int32_t rl_shell_draw(RlShell shell, uint64_t generation,
 uint32_t animation_84, uint64_t *output_generation);
/* A real DEFEAT_MOVIE request suspends at the host. Preserve the original path.
 * Host reports unavailable (missing file or no decoder) or finished playback;
 * neither status claims decoding in the engine. Both execute original MenuInit2.
 * Requires pending event11 (result mode19/end1/victory0), event14 (mode24) or
 * event7 INTRODUCTION (main mode2; see above). Atomic on failure,
 * generation-checked, single-use. Does not tick source clock or draw. Afterward
 * paint the current main mode with the original titre.pie; do not prepare again.
 * Prepared-API common-tail mouse_b0 clear remains host-owned. */
enum { RL_SHELL_MOVIE_UNAVAILABLE=0, RL_SHELL_MOVIE_FINISHED=1 };
int32_t rl_shell_defeat_finish(RlShell shell, uint64_t generation,
 uint32_t completion, uint64_t *generation_out);

/* One original AffCredits frame in mode10. elapsed_ms = host milliseconds since
 * its last FLC frame read (since the open before the first): the original reads
 * timeGetTime minus the time ReadFli stored after its last frame; the engine adds
 * elapsed_ms / FLC speed (header +0x10) to its frame counter, as the original
 * does. state ENDED (no animation: the FLC did not open) leaves a
 * pending SHOW main request: complete it with rl_shell_menu_init. DRAW: decode
 * the FLC sequentially up to frame `frame`, copy width x height to (x,y)=(464,155)
 * of the persistent surface and apply the FLC palette; the engine already ran
 * DDFlipPage's gameplay prefix. HOLD: the counter passed the last frame; draw
 * nothing, the last picture stays. rl_shell_input in mode10 returns SHOW main on
 * Escape or a click (mouse_ae and mouse_af); rl_shell_menu_init then runs
 * MenuInit(2), which closes the FLC (FreeCredits). Requires no pending request.
 * Success increments generation; failure leaves the session unchanged. */
enum { RL_SHELL_CREDITS_ENDED=0, RL_SHELL_CREDITS_DRAW=1, RL_SHELL_CREDITS_HOLD=2 };
typedef struct {
 uint64_t generation;
 uint32_t state, frame, frames, width, height, x, y, reserved;
} RlShellCredits;
int32_t rl_shell_credits_frame(RlShell shell, uint64_t generation,
 uint32_t elapsed_ms, RlShellCredits *output);

/* Complete the pending SHOW request (kind1) or SAVE_LOAD request (kind6, the
 * pause menu's save/load items) with the original MenuInit(local
 * player, mode) gameplay effects: main2 (mpClose, items, RechercheReset,
 * Engin_ResetExperience, InitPlayer), single3/pause16 items, briefing17/
 * objectives23 (ObjectifLoadText, zones, RechercheCalculNew for17) and the
 * save/load menus12/13/14 (GameSaveScanNames over save1.sav..save0.sav).
 * files: exact raw path->bytes VFS. Briefing targets need the original
 * WORLD\MONDExx.TXT paths as in rl_shell_enter; save menus need the ten slot
 * paths exactly as stored at0x497128 ("save1.sav".."save0.sav"): a "SAV"
 * header at version digit>='3' is followed by the32-byte slot name, any other
 * header gets the original default caption, an absent entry gets text0x72.
 * No preparation mask: this call IS the preparation. Then paint with
 * rl_shell_paint_prepared and the plan background. Failure is atomic. */
int32_t rl_shell_menu_init(RlShell shell, uint64_t generation,
 const RlShellFile *files, uint32_t count, uint64_t *output_generation);
/* Complete the pending RESET_CAMPAIGN request (kind2, single-player mode3):
 * original MissionReset (stage1), local player0, SetPlayerType human/computer
 * x3, then MenuInit(17) new-game briefing (mode17 in event.mode) or MenuInit(13)
 * load menu (mode13), and the HUD refresh latch. files as in rl_shell_menu_init
 * (MONDE01.TXT for17, slot files for13). Afterwards paint the briefing/list with
 * rl_shell_paint_prepared; briefing input then yields START_MISSION and
 * rl_shell_campaign_load loads MONDE01 with the chosen research. */
int32_t rl_shell_campaign_reset(RlShell shell, uint64_t generation,
 const RlShellFile *files, uint32_t count, uint64_t *output_generation);

/* Complete a pending RESUME request (kind5: Escape or the "continue" item of the
 * pause menu16). Runs the original in-game MenuInit(18): cursor/picture/music
 * boundaries become frame events (binding kind1 id1, boundaries kind2 ids101/102) on the returned world, read with
 * rl_eventos_quadro before the next frame; the DDFlipPage gameplay prefix runs.
 * The returned world continues the paused game (same clock, execution mode and
 * carried input: pass the host's current RlFrameInput; it is returned unchanged).
 * The session is then stale (mode18): free it. Failure publishes nothing.
 * Also completes a pending SAVE_SLOT (kind13) from the in-game save list14,
 * AFTER the host has written its own save (the engine writes no save): the
 * original MenuNewSelect 0x42c570 tail, MenuInit(18) then the HUD refresh latch
 * 0x4e05a2=1, with the same world/event/input contract as RESUME. Any other
 * pending request or mode is RL_INVALID_ARGUMENT. */
int32_t rl_shell_resume(RlShell shell, uint64_t generation,
 const RlFrameInput *input, RlMundo *output, RlFrameInput *input_out);

/* Repaint the CURRENT prepared mode from its real decoded background, including
 * same-mode result19. No MenuInit, research, RNG, clock or memory writes.
 * Success increments session generation; source world remains unchanged. */
int32_t rl_shell_paint_prepared(RlShell shell, uint64_t generation,
 RlShellBytes background, uint64_t *output_generation);
/* Complete the last actual MISSION_NEXT input request on a victorious result19.
 * name is rl_missao_nome_proxima's exact path. VFS holds that ETA and the
 * original briefing files; missing entries mean absent files, never I/O errors.
 * An absent ETA entry is the end of the campaign: the session enters mode24
 * (see RL_SHELL_CAMPAIGN_MOVIE) instead of the briefing.
 * Runs MissionNext -> AI research adjustment -> experience/research/player stores
 * -> briefing text -> zones -> RechercheCalculNew -> MenuInit17 flip/reset.
 * Stops BEFORE mode17 input and MissionLoad. Does not paint: then call
 * rl_shell_paint_prepared with the briefing background from rl_shell_plan.
 * No preparation attestation or artificial mode transition is needed.
 * Failure leaves memory, surface, request, generation and output unchanged. */
int32_t rl_shell_campaign_prepare(RlShell shell, uint64_t generation,
 RlShellBytes name, const RlShellFile *files, uint32_t count,
 uint64_t *output_generation);
/* After real briefing input returns START_MISSION (mode0), load its exact ETA.
 * Research choice comes from that session: no second MissionNext/recalculation.
 * Input carries the host registers and LAST shell cursor (must match session).
 * Returns a new world at MissionLoad's return, before load-tail phase1. Caller
 * completes the existing declared load tail and renderer before publication.
 * Preserves source clock/execution mode and inherited EVENTS; load tail advances
 * the clock once. Session/source remain unchanged, including on failure.
 * Do not call rl_missao_continuar on the returned world; that is the older
 * single-call path with host-confirmed briefing, and would advance again. */
int32_t rl_shell_campaign_load(RlShell shell, uint64_t generation,
 RlShellBytes name, RlShellBytes eta, RlShellBytes monsters,
 const RlFrameInput *input, RlMundo *output, RlFrameInput *input_out);
/* Same load, then the stage-start difficulty preset (RL_DIFICULDADE_*) inside
 * the same transaction, as rl_missao_continuar_dificuldade does for the older
 * path. Serves campaign continuation, new game and restart; Normal returns the
 * loaded bytes unchanged. Do not call rl_aplicar_dificuldade afterwards. */
int32_t rl_shell_campaign_load_dificuldade(RlShell shell, uint64_t generation,
 RlShellBytes name, RlShellBytes eta, RlShellBytes monsters,
 const RlFrameInput *input, uint32_t preset, RlMundo *output,
 RlFrameInput *input_out);
/* Pure copy. Fixed sizes above, RGB palette bytes in existing render convention.
 * Insufficient capacity -> BUFFER_TOO_SMALL, including NULL/zero size queries.
 * Overlapping output spans -> INVALID_ARGUMENT. No draw, clock or generation change. */
int32_t rl_shell_copy(RlShell shell, uint64_t generation,
 uint8_t *pixels, uint32_t pixel_capacity, uint8_t *palette, uint32_t palette_capacity);
/* New independent world from current session bytes, retains source virtual
 * clock/execution mode/frame phase. It is NOT a MissionLoad/start/resume.
 * Pair with source's carried RlFrameInput; host must execute real continuation.
 * No pending save input queue is copied. Free with rl_liberar_mundo. */
int32_t rl_shell_export_world(RlShell shell, uint64_t generation, RlMundo *output);
int32_t rl_shell_free(RlShell shell);
#ifdef __cplusplus
}
#endif
#endif
