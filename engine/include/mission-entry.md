# Mission entry extension v1

`rising_engine.h` adds `rl_missao_versao`, `rl_missao_iniciar`,
`rl_missao_nome_proxima`, and `rl_missao_continuar` without changing ABI 1.6.
The header documents every span, option, error, and lifecycle rule.

Read the ten original asset files listed in the header into ordered `RlBloco`
descriptors. Supply the actual virtual clock, allocator fill, AI startup clock,
thread identity, cursor, prior presentation counters and WinMain input context
in `RlMissaoInicio`. The MONDE01/S0 comparison uses fill `0xa7`, AI clock
`0x123456`, thread 1, 20 Hz, cursor (400,300), and counters (5,2,1). These are
explicit comparison inputs, not inferred gameplay defaults. The virtual x86
stack/offscreen addresses are opaque software bindings, not host pointers.

`rl_missao_iniciar` returns a fresh world at the successful MissionLoad return,
phase 0, tick 0. No captured world is an input. Run `rl_passo_quadro` with
`RL_LOAD_TAIL`; on success install the carried WinMain registers for phase 2,
then use the existing input/render API. The load tail ticks once. Capture the
stage-start autosave with `rl_autosalvar_inicio_fase` after that success.
Create/bind the renderer using the host's explicit assets/surface/cache. A
named stop in a later phase leaves the loaded world available to inspect.

On victory, preview the next original path with `rl_missao_nome_proxima`.
Resolve that path to the bundled WORLD directory and check for the actual file;
after MONDE25 no next campaign ETA is shipped. After confirming the intervening
screens, pass that exact path and the original ETA/MONSTRE bytes to
`rl_missao_continuar`. It returns a fresh loaded world after original
MissionNext, AI research adjustment, carry-over, research recalculation and
MissionLoad, preserving the clock and execution policy. Run its load tail as
above. Release the previous renderer/world after success. Handle any queued
old-stage inputs explicitly before transitioning; the new handle has no old
save-session FIFO. No menu, missing file, stop, victory, or research choice is
silently synthesized. P4 owns the actual screen UI.

Direct selection accepts all 38 shipped filenames, including base files for
stages whose campaign path normally chooses a suffix. It starts fresh campaign
carry-over. The caller supplies the selected file bytes; campaign continuation
always uses the live original filename and sympathy/variant selection.

The software host supplies an installed palette token and active session flag.
DDInit's dimensions and InitInputPlayer's five allocations run the native
initialization semantics. DD COM objects, audio devices and the renderer remain
host resources. MONDE06/07/23 currently load but reach the pre-existing sampled
origin display stop in phase 1; this P2 change does not bypass or port it.
