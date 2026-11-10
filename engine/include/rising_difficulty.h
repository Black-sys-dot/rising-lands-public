#ifndef RISING_DIFFICULTY_H
#define RISING_DIFFICULTY_H
#include "rising_engine.h"
#ifdef __cplusplus
extern "C" {
#endif
/* Additive to ABI 1.6; existing world handles/layouts are unchanged.
 * Modern presets over original raid periods/masks, MONSTRE.INI targeting
 * weights and the local player's starting Pecas/Comida/Pedra. See DIFFICULTY.md.
 */
enum {
 RL_DIFICULDADE_FACIL=0,
 RL_DIFICULDADE_NORMAL=1,
 RL_DIFICULDADE_DIFICIL=2
};
/* Call exactly once per freshly loaded stage, AFTER rl_carregar_blocos and
 * optional rl_carregar_textos, BEFORE RL_LOAD_TAIL/frame0. Requires tick=0 and
 * phase=0 and an unused fresh-import capability; create a fresh world handle
 * before importing another raw stage. rl_missao_iniciar also grants this
 * capability. World::load and save restoration do not grant it. This applies to the loaded
 * stage, not a persistent application preference. To change/reapply a preset,
 * create a new handle and load fresh original stage inputs first.
 * Normal is byte-identical to no setting, including custom original INI values,
 * and does not change generation. Easy/Hard publish atomically, incrementing
 * generation once; clock, RNG and callbacks are untouched by setup.
 * Missing original bytes: RL_MEMORY_ERROR. Invalid preset: RL_INVALID_ARGUMENT.
 * Invalid handle: RL_INVALID_HANDLE. Already-started or invalid initial stage:
 * RL_UNSUPPORTED_PATH, with a named/addressed rl_erro_mensagem diagnostic.
 * Every failure preserves all world state. Unknown gameplay paths still stop.
 */
int32_t rl_aplicar_dificuldade(RlMundo world, uint32_t preset);
/* Same inputs and transactional publication as rl_missao_continuar, plus preset.
 * Run original MissionNext/carry-over/MissionLoad, apply the preset exactly once
 * at that successful fresh-load boundary, then publish a new handle. Campaign
 * tick/rate/epoch and RNG are preserved. Normal equals rl_missao_continuar byte
 * for byte, including generation. The returned handle's setup capability is
 * closed even at tick zero/wrap; do not call rl_aplicar_dificuldade on it.
 * Then install events if missing and prepare renderer/load-tail/autosave as in
 * mission entry. On save restoration never apply a preset; use saved bytes.
 * Invalid preset: RL_INVALID_ARGUMENT. Missing/empty ETA, wrong path, invalid
 * victory, memory/policy/load failure: no new handle or input is published and
 * the winner survives. A missing MONDE26 must be reported by the host; never
 * substitute another ETA. No menus or load-tail are executed by this call.
 */
int32_t rl_missao_continuar_dificuldade(
 RlMundo world, const uint8_t *name, uint32_t name_len,
 const uint8_t *eta, uint32_t eta_len,
 const uint8_t *monsters, uint32_t monsters_len,
 const RlFrameInput *input, uint32_t preset,
 RlMundo *output, RlFrameInput *input_out);
#ifdef __cplusplus
}
#endif
#endif
