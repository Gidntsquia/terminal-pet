# squid-pet

A full-screen, colored ASCII giant squid pet for your terminal. It fills the window, sways, blinks, and blows bubbles. You can feed it, pet it, and play with it. Nothing is saved.

## Quickstart

```
cargo install --path .
squid-pet
```

Keys: `f` feed, `p` pet, `space` play, `q` / `Esc` / Ctrl-C quit.

Flags: `--ascii` (no Unicode glyphs), `--seed N` (repeatable run). `NO_COLOR=1` gives monochrome; without `COLORTERM=truecolor` it uses 256 colors. Minimum size 40x12.

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
