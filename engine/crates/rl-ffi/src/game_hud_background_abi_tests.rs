use super::*;

#[test]
fn errors_are_atomic_and_addressed() {
    let asset = [0u8; 16];
    let mut output = vec![0x97; PIXELS + 16];
    unsafe {
        assert_eq!(rl_game_hud_background_version(), 1);
        for (version, pointer, length, capacity, expected) in [
            (2, asset.as_ptr(), 16, PIXELS as u32, 10),
            (1, asset.as_ptr(), 16, PIXELS as u32 - 1, 6),
            (1, ptr::null(), 16, PIXELS as u32, 1),
            (1, asset.as_ptr(), 0, PIXELS as u32, 1),
            (1, asset.as_ptr(), ASSET_LIMIT as u32 + 1, PIXELS as u32, 9),
            (1, asset.as_ptr(), 16, PIXELS as u32, 5),
        ] {
            assert_eq!(
                rl_game_hud_background_initialize(
                    version,
                    pointer,
                    length,
                    output.as_mut_ptr(),
                    capacity
                ),
                expected
            );
            assert!(output.iter().all(|&b| b == 0x97));
        }
        LAST_ERROR.with(|e| assert!(e.borrow().contains("4799c4")));
        assert_eq!(
            rl_game_hud_background_initialize(
                1,
                output.as_ptr(),
                16,
                output.as_mut_ptr(),
                PIXELS as u32
            ),
            1
        );
        assert!(output.iter().all(|&b| b == 0x97));
        assert_eq!(
            rl_game_hud_background_initialize(
                1,
                asset.as_ptr(),
                16,
                ptr::null_mut(),
                PIXELS as u32
            ),
            1
        );
    }
}
