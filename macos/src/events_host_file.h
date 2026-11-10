#ifndef RL_EVENTS_HOST_FILE_H
#define RL_EVENTS_HOST_FILE_H
#include "rising_engine.h"
/* The caller authorizes this directory capability. No path from original memory
 * is ever opened. Exactly the direct child EVENTS.INI is read without symlinks.
 * Caller owns output after success; failure leaves it untouched. */
typedef struct { RlEventsHostV1 input; uint8_t *directory,*events; } RlOwnedEventsV1;
int32_t rl_events_file_read_v1(int owned_directory_fd,const uint8_t *logical,
 uint32_t logical_length,RlOwnedEventsV1 *output);
void rl_events_file_free_v1(RlOwnedEventsV1 *owned);
#endif
