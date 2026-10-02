# squid-pet
Rust (crossterm + ratatui + rand 0.9). `src/app.rs` state machine, `src/art.rs` procedural drawing to a Canvas, `src/main.rs` terminal/loop.
Rust is installed via rustup in ~/.cargo (`. ~/.cargo/env`). Check: `cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`.
Try it headlessly: `tmux -L x new-session -d -x 120 -y 40 ./target/release/squid-pet`, then `capture-pane -p`.
