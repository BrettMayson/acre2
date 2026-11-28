# Building the Extension

YARD has one extension, `yard_x64.dll` (`yard_x64.so` for Linux dedicated servers), written in Rust in the `rust/` folder. It carries voice (client, and the voice server the Arma server hosts) and computes radio signal strength over the terrain. See `rust/README.md` for how the crates fit together.

## Requirements

- [Rust](https://rustup.rs) (stable; the toolchain and targets are pinned in `rust/rust-toolchain.toml`)
- [CMake](https://cmake.org/), to build the bundled Opus codec for release builds
- Windows: the Visual Studio C++ build tools (MSVC)
- Linux: ALSA headers (`libasound2-dev`) for the client build

## Build

_The [release workflow](https://github.com/BrettMayson/yard/blob/main/.github/workflows/release.yml) builds everything for releases; `build-extension.sh` builds both binaries on Linux._

From the `rust/` folder:

```sh
# Windows: the extension, and the wav2b64 tool shipped in extras/
cargo build --release -p acre2_ext -p acre2_tools --features acre2_voice_codec/bundled
# Linux dedicated server (no audio devices)
cargo build --release -p acre2_ext --no-default-features --features acre2_voice_codec/bundled
```

Copy `target/release/yard.dll` to the repository root as `yard_x64.dll` (or `libyard.so` as `yard_x64.so`), and `wav2b64.exe` to `extras/`.

## Tests

```sh
cargo test --workspace
```

The signal propagation tests compare the Rust models with the original C++ ones on a real terrain (`rust/acre/signal/tests/golden`).

## Tools

- `wav2b64 input.wav [output.b64]`: converts a mono 16-bit 48 kHz WAV into a sound file for `CfgAcreSounds`.
- `fakewrp`: makes a `.fakewrp` terrain file for terrains whose WRP the extension can't read. See [Terrain Files](../frameworks/terrain-files.md).
