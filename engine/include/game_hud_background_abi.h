#ifndef GAME_HUD_BACKGROUND_ABI_H
#define GAME_HUD_BACKGROUND_ABI_H
#include "rising_engine.h"
#ifdef __cplusplus
extern "C" {
#endif
#define RL_GAME_HUD_BACKGROUND_VERSION 1u
#define RL_GAME_HUD_BACKGROUND_PIXELS 480000u
/* Additive extension; engine ABI stays 1.6. No handles or retained pointers.
 * Exact original PLATGAME.PIE (50177 bytes, SHA-256
 * da6a07c93a6a027cd81ce50e4dfc756848223bd951f4a60e70bab1e9bc3c20a7).
 * Readable asset length <=1MiB; writable surface capacity >=480000 bytes.
 * Valid caller allocations required; input/output spans must not overlap.
 * Success copies exactly 800x600 indexed pixels, stride800, leaving capacity
 * tail untouched. Palette remains owned by the existing render assets.
 * Every error leaves all output bytes unchanged; rl_erro_mensagem reports the
 * existing TLS diagnostic. Short capacity (including NULL/0 query) returns6;
 * version mismatch10, invalid pointer/span1, limit9, original asset mismatch5.
 * No world, RNG, cache, generation, callback, I/O or renderer changes occur.
 * Call once when creating the production seed, retain it for entry/restore,
 * then use rl_render_reset to invalidate retained render/tooltip/outcome state.
 * Never call per frame, on scale-only resize, or on frozen Validar inputs. */
uint32_t rl_game_hud_background_version(void);
int32_t rl_game_hud_background_initialize(uint32_t version,
 const uint8_t *asset, uint32_t length, uint8_t *surface, uint32_t capacity);
#ifdef __cplusplus
}
#endif
#endif
