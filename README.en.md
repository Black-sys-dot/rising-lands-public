# Rising Lands — reimplemented engine (macOS)

[Versão em português](README.md)

This repository holds a reimplementation of the **Rising Lands** (1997) game engine in Rust, and a macOS app that runs the game on top of it. The logic was ported routine by routine from the original executable and compared against it.

**The game is not included.** To play, you need the image of the **original Brazilian CD** of Rising Lands. On first run the app reads all the game data from that CD: graphics, sounds, music, texts, stages and movies. No game data is distributed here.

## Current status

- The engine and the app build from this repository (see below).
- **One step is still missing before a build from here can play.** A few tables the renderer uses are still prepared outside the app: the palette, the shadow and lighting tables, the initial terrain cache and the menu screen pictures. They will be generated from the player's CD on first run. Until then, the app built from here opens but reports that its import manifest is missing. Details: [PUBLICATION_AUDIT.md](PUBLICATION_AUDIT.md).

## Requirements

- A Mac with **Apple Silicon** (arm64) and **macOS 13** or later.
- **Xcode Command Line Tools**: `xcode-select --install`.
- Stable **Rust**, installed with rustup (https://rustup.rs).
- The original Brazilian CD image (see "Getting the CD").

## Build

```bash
./build-macos.sh
```

The app is written to `build/RisingLands.app`. To choose another output folder: `./build-macos.sh <folder>`.

Tests:

```bash
cargo test --locked --manifest-path engine/Cargo.toml
```
```bash
./macos/tests/run.sh
```

Some engine tests need the original game files and are ignored when those files are absent.

## Getting the CD

The original Brazilian CD image is available on the **Internet Archive**. Download **RISING_LANDS_BR_.BIN** and **RISING_LANDS_BR_.CUE**. The game music only comes from the **.BIN** image.

## Run and import the CD

1. Open `build/RisingLands.app`. If macOS blocks it as a non-notarized app, right-click the app and choose **Open**.
2. On first run, the **"Rising Lands — importar o CD original"** window asks for the CD. Click **Escolher o CD…** and select **RISING_LANDS_BR_.BIN** (or the .CUE, an .ISO, or the folder with the CD files).
3. The files are checked and copied to the Application Support folder once. After that the game opens directly. To import again: **Jogo → Importar o CD novamente…**.

The app's interface is in Brazilian Portuguese, like the original game.

## Controls

- **Speed:** menu **Jogo → Velocidade** offers 0.5x, 1x, 2x and 4x (15, 30, 60 and 120 frames per second). Normal speed (1x) is 30 frames per second. Shortcuts: **⌘]** faster, **⌘[** slower. **⌘P** pauses and resumes.
- **Fullscreen:** menu **Janela → Tela cheia**. In fullscreen only the game is shown, on black.
- **Moving around the map:** push the mouse against an edge. In fullscreen, just move the mouse to the edge of the screen. In a window, the map scrolls while the mouse is inside the window at its edge. The arrow keys also scroll.
- **Keyboard:** Enter, Esc, Space, arrows, Delete, X, Tab (minimap) and the digits 0–9, as in the original. Keys can be rebound in **Jogo → Configurar teclas…**. The original's F11 key (switch to 640x480) is not used.

## Reporting problems

Open an issue in this repository. It helps a lot to include:
- the macOS version and the Mac model;
- the stage and what you were doing;
- the message shown, if any.

## License

The source code in this repository is released under the MIT License (see `LICENSE`): anyone may read, use, modify and redistribute it. It covers the code only, not the original game.

## Rights

Rising Lands and all its original materials (graphics, sounds, music, texts, stages, movies and executable) remain the property of their rights holders. This repository distributes none of them and grants no license to them. Players use their own copy of the CD.

License of the code in this repository: **to be decided** (TODO).
