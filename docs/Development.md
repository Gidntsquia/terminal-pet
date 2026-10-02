# Development

The app is a Rust binary built on crossterm, ratatui and rand. `src/app.rs` is the state machine (idle, feeding, petting, playing, idle behaviors), `src/art.rs` draws the squid and its surroundings procedurally onto a canvas, and `src/main.rs` sets up the terminal and runs the ~16 fps loop.

## Build

```
cargo build --release     # binary at target/release/squid-pet
```

## Release binary (Linux x86_64)

```
cargo build --release
tar czf squid-pet-x86_64-linux.tar.gz -C target/release squid-pet
```

Pushing a `v*` tag runs `.github/workflows/release.yml`, which builds and attaches the same tarball to a GitHub release.

## Tests

```
cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

## Colors

`NO_COLOR=1` gives monochrome. Truecolor is used when `COLORTERM` contains `truecolor` or `24bit`; otherwise colors are mapped to the 256-color palette. Minimum terminal size is 40x12; smaller windows show a "terminal too small" message.
