# tetris-bench

A basic Tetris that runs in your terminal, written in Rust.
Colored pieces, ghost piece, next-piece preview, score / lines / level,
and a board that scales up and centers itself to fit your window.

## Windows

You need Rust and Git. The whole setup takes a few minutes.

1. **Install Rust.** Download and run `rustup-init.exe` from
   [rustup.rs](https://rustup.rs), or use winget:

   ```powershell
   winget install Rustlang.Rustup
   rustup default stable
   ```

   If the installer asks for the Visual Studio C++ Build Tools, accept —
   Rust needs the Microsoft linker to build on Windows.

2. **Install Git** (skip if you already have it):

   ```powershell
   winget install Git.Git
   ```

3. **Get the code and run it.** Use Windows Terminal (built into
   Windows 11, or free from the Microsoft Store) rather than the old
   console — colors and borders look right there.

   ```powershell
   git clone https://github.com/ljgrohn/tetris-bench.git
   cd tetris-bench
   cargo run --release
   ```

   No Git? On the GitHub page, click **Code → Download ZIP**, unzip it,
   open the folder in Windows Terminal, and run `cargo run --release`.

## Mac

You need Rust and Git (Git comes with macOS).

1. **Install Rust:**

   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

   Follow the prompts, then restart your terminal (or run
   `source "$HOME/.cargo/env"`). If asked, install the Xcode Command
   Line Tools with `xcode-select --install` — Rust needs the linker.

   Prefer Homebrew? `brew install rust` works too.

2. **Get the code and run it:**

   ```bash
   git clone https://github.com/ljgrohn/tetris-bench.git
   cd tetris-bench
   cargo run --release
   ```

## How to play

| Key | Action |
| --- | --- |
| ← / → or A / D | Move |
| ↓ or S | Soft drop |
| ↑ / X | Rotate (Z rotates the other way) |
| Space | Hard drop |
| P | Pause |
| R | Restart after game over |
| Q / Esc / Ctrl-C | Quit |

Clear a full row to score. Every 10 lines the level goes up and pieces
fall faster.

## Notes

- **Make the window bigger to make the game bigger.** Blocks scale up
  (up to chunky 4×2 blocks) and the play field centers itself.
- To build a standalone binary instead of running through Cargo:
  `cargo build --release`, then run `target/release/tetris`
  (`tetris.exe` on Windows).
- First build downloads two small crates (`crossterm`, `rand`), so the
  first run needs an internet connection and takes a few seconds.
