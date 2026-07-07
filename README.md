# 🕹️ Atari 2600 Emulator in Rust

A cycle-accurate (work in progress) Atari 2600 emulator written in **Rust**, focused on performance and clean architecture.

<p align="center">
  <img src="print.png" alt="Enduro running on the emulator" width="500">
</p>

<p align="center">
  <em>Enduro running on the emulator.</em>
</p>

---

## ✨ Features

- ✅ MOS 6507 CPU emulator
- ✅ TIA video rendering
- ✅ RIOT (6532) support
- ✅ Keyboard joystick emulation
- ✅ Adjustable screen alignment
- ✅ Configurable viewport
- ✅ Optional audio backend
- ✅ Debug & trace mode
- 🚧 Save States (planned)
- 🚧 CRT shaders (planned)

---

# Running

Clone the project:

```bash
git clone https://github.com/sl4ureano/atari2600-rust.git
cd atari2600-rust
```

Run any ROM:

```bash
cargo run -- roms/Enduro.bin
```

Example with horizontal adjustment:

```bash
cargo run -- roms/Enduro.bin --x-adjust -6
```

Without audio:

```bash
cargo run -- roms/Enduro.bin --no-audio
```

CPU/TIA trace:

```bash
RUST_LOG=trace cargo run -- roms/Enduro.bin --trace
```

---

# Controls

## Joystick (Player 1)

| Key | Action |
|------|--------|
| ← ↑ ↓ → | Move |
| Space / Z / X | Fire |

## Atari Console

| Key | Action |
|------|--------|
| Enter | Reset |
| F2 / 2 | Reset |
| F1 / 1 | Select |
| C | Color / B&W |
| A | Left Difficulty |
| S | Right Difficulty |
| R | Emulator Reset |

---

# Command Line Options

| Option | Description |
|---------|-------------|
| `--scale <N>` | Window scale |
| `--speed <N>` | Turbo mode |
| `--visible-start <N>` | First visible color clock |
| `--x-adjust <N>` | Horizontal adjustment |
| `--y-crop <N>` | Crop top scanlines |
| `--fps <N>` | Target FPS (default 60) |
| `--no-audio` | Disable audio |
| `--trace` | Enable CPU/TIA trace |

Example:

```bash
cargo run -- roms/Enduro.bin \
    --visible-start 68 \
    --x-adjust -6 \
    --fps 60
```

---

# Audio (Linux)

By default the project builds **without audio support**, avoiding `alsa-sys` issues on systems without native ALSA development headers.

Run without audio:

```bash
cargo run -- roms/Enduro.bin --no-audio
```

Enable audio:

Ubuntu / Debian

```bash
sudo apt install pkg-config libasound2-dev
cargo run --features audio -- roms/Enduro.bin
```

If you don't want to install ALSA, simply omit the `audio` feature and the emulator will still run normally.

---

# Project Structure

```
src/
 ├── cpu.rs
 ├── tia.rs
 ├── riot.rs
 ├── cartridge.rs
 ├── audio.rs
 ├── bus.rs
 ├── main.rs
```

---

# Compatibility

Current testing:

| Game | Status |
|-------|--------|
| Enduro | ✅ Playable |
| River Raid | ✅ Playable |
| Pitfall! | 🚧 |
| Space Invaders | 🚧 |

---

# Roadmap

- Accurate TIA timing
- Better collision emulation
- More cartridge mappers
- Save states
- NTSC/PAL auto detection
- CRT filters
- Gamepad support
- Debugger UI

---

# License

MIT