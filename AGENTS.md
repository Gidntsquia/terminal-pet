# terminal-pet
Rust (crossterm + ratatui + rand 0.9). Shared engine: `src/app.rs` state machine, `src/art.rs` Canvas + squid drawing, `src/social.rs`/`src/link.rs` visits. Per-pet: `src/cat.rs` (child module of app; breeds, cat poses/timers), `src/cat_art.rs` (cat + room drawing). `src/lib.rs` has `cli()`; bins `terminal-pet` (`--pet`) and `squid-pet` (squid only). The squid must stay byte-identical (golden test): never touch the squid rng stream from cat code.
Rust is installed via rustup in ~/.cargo (`. ~/.cargo/env`). Check: `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`.
Try it headlessly: `tmux -L x new-session -d -x 120 -y 40 ./target/release/terminal-pet --pet cat`, then `capture-pane -p`.
