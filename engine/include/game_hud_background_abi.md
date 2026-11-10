# Gameplay background C extension v1 (RL-VIS-001)

This engine-only checkpoint adds `game_hud_background_abi.h` and two exported
symbols. It requires no change to engine ABI 1.6, world/player/mission/render
contracts, or the existing Rust provider. The production host must opt in.

`rl_game_hud_background_initialize(1, pie, length, surface, capacity)` validates
the complete original asset, decodes with the existing proven provider into
private storage and copies exactly 480000 indexed pixels to the caller. Larger
capacity is accepted; its tail remains unchanged. Version, capacity, null,
overflow, overlap and resource-limit checks precede reading the asset. Asset
failure and every ABI error leave the caller's entire output unchanged. The
shared `rl_erro_mensagem` TLS diagnostic preserves the original addressed stop.
Caller allocations must be valid for their declared lengths. No pointer survives
the call, and no world/renderer handle can be supplied.

The original input is 50177-byte `PLATGAME.PIE`, SHA-256
`da6a07c93a6a027cd81ce50e4dfc756848223bd951f4a60e70bab1e9bc3c20a7`.
The full decoded output SHA-256 is
`85c2f076250f0f741d0a5d976e81ee2d3ae5468a136f8feed16553373beec1f6`.
This includes stone/icons at y544..599 and black above them. Palette ownership
stays with the existing render assets. The call performs no I/O, world writes,
RNG draws, cache writes, clock advances, callbacks or game logic.

Original provenance remains the provider checkpoint
`4e2c6ae47461f0dd716cdfa656a35069862c308a`,
`validation/hud-background/HANDOFF.md` (private research repository): intact EXE SHA-256
`6c556520c084b9300d9ac01900d1505126085c5f64b1ac5e5b615ea41c8543fb`,
LoadPie `0x4799c4`, MenuInit block ending `0x468476`, RestoreAll
`0x474b6a`/`0x474bb1`. That unchanged native proof contains 11/11 executed
comparisons equal and one explicit S0 skip: five initialization/palette cases
and six AffGame pixel-body prefixes, not six whole renderer frames.

Host policy: decode once when selecting production gameplay; retain the seed
and reuse it at real entry/reconstruction boundaries before the first publish.
For a retained renderer call existing `rl_render_reset` with that seed and the
proper cache, invalidating saved tooltip/outcome state. Do not initialize per
frame or for scale-only resizing. Frozen Validar `surface.bin`, expected hashes
and proof inputs remain unchanged. Save/load presentation version policy belongs
to the host transaction, not this stateless API.

The C fixture takes mandatory original PIE and a fresh decoded output filename:

```sh
cargo test --manifest-path motor/Cargo.toml -p rl-ffi game_hud_background_abi
cargo build --manifest-path motor/Cargo.toml -p rl-ffi --release
cc -std=c11 -Wall -Wextra -Werror -I motor/include \
  motor/testes/game_hud_background_abi.c -L motor/target/release -lrising_engine \
  -Wl,-rpath,"$PWD/motor/target/release" -o game_hud_background_abi_test
./game_hud_background_abi_test /read-only/PLATGAME.PIE decoded.bin
shasum -a 256 decoded.bin
```

Run all native/build commands through the mini's `heavy_lease.py`. This tests
the actual new export; the native provider comparison is reused under identical
provider/input/output hashes. No app/UI, full motor gate, play-time or FPS claim
belongs to this engine-only checkpoint. Root reviews/integrates it through
maintenance; the task branch alone is pushed.

## Engine-only validation receipt

Kept in the private research repository.
All three invocations used that machine's heavy lease (six slots configured,
empty work-hours setting). Free space before work was22GiB. No other worktree,
build directory or native input was written.

`validation/game_hud_background_abi/` (private research repository) retains the actual logs. The first
format invocation from outside `motor/` reported no targets; the repeat from
inside the workspace passed `cargo fmt --all -- --check`. Strict Clippy passed.
FFI library tests:8 passed,0 failed,1 pre-existing prepared-shell fixture test
ignored for its explicit external dependencies. The new ABI test passed1/1;
the actual C client checks passed7/7. Final decoded output has the native hash
above; dylib SHA-256 is
`49c4728cab594130d03b106de018a5bf47a6be80e61ddc6fe14878d61f30937f`.
Provider SHA-256 remains
`1af7dbd6523fac40f24a5d42b126dc665cee5ffcb7eaa8fd2899ca8d601bee7e`.
Native11/11 proof is reused, not rerun or relabeled as new ABI/full-app evidence.
