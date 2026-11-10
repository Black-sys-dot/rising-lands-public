import Foundation

/// The player edition (compiled with -D RISING_PLAYER_EDITION) has no research
/// chrome (Jogar/Validar, frame stepping, input recording, inspection, the
/// bundled Python tools) and opens straight into the original main menu, from
/// data imported from the player's own CD. The development build is unchanged.
#if RISING_PLAYER_EDITION
let risingPlayerEdition = true
#else
let risingPlayerEdition = false
#endif
