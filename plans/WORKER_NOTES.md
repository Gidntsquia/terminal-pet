Launch: `. ~/.cargo/env; cargo install --path . && squid` (or `cargo run --release`).
Rust was not installed; I installed rustup (minimal profile) into ~/.cargo.

Observed (tmux pty, 100x30 and 120x40, release build):
- Launch full-screen, `q` exits and tmux session ended cleanly: met (cursor/tput not inspected beyond that).
- Span ~80% rows + centered: unit test passes at 40x12, 80x24, 120x40, 200x60, 300x80. Live resize NOT tested.
- Colors: truecolor escapes 181;64;31 (#B5401F) mantle seen in capture; visual match to photo not judged by eye.
- Idle animation: bubbles/seaweed/waves seen; sway/blink/bob are coded, not watched live; idle behavior within 30 s covered by unit test.
- f/p/space: pet showed hearts + "Blub blub" (and open-mouth yawn behavior seen); feed/play not captured visually; mashing keys in tmux ended with clean quit.
- CPU idle: 0.8% (top, 5 s, release).
- NO_COLOR / --ascii: ascii covered by unit test; NO_COLOR not run live.
- Release build + README: met. test/clippy/fmt: pass.
- Startup under 100 ms: not measured.
Release CI workflow written, not run. Not a git repo (no commits made).

## Round 2 (user: fill space more; diagonal; move around)
- Squid widened (mantle ~35 cols at 120x40, arms spread ~90), sheared into a diagonal lean, and swims side to side across the terminal (wander + lean in app.rs). Seen in tmux 120x40 at t=1s and t=7s: it moved ~15 cols and tilt changed. Not watched at other sizes live; tests pass at 5 sizes.
- Spec deviation (user wins): "horizontally centered" no longer holds while it wanders; the center test tolerance is now w/3.
- test/clippy/fmt pass. Launch: `. ~/.cargo/env; cargo run --release`.

## Round 3: --color presets
Launch: `. ~/.cargo/env; cargo run --release -- --color blue-ringed` (`c` cycles colors).
Observed (tmux 120x40, release): `--color teal`/bare `--color`/`NO_COLOR=1 --color teal` print list to stderr, exit 2: met. `--help` + README updated: met.
`--color random --seed 42` showed "I'm purple today!"; seed determinism covered by unit test: met. `c` showed "blue!", mid-fade colors seen at 0.25 s; 8-cycle wrap in unit test: met.
blue-ringed: 48-70 rings (unit test, 120x40; ~60 live), rings flash to 16;144;255 after `p`: met. `--ascii` pet capture has 0 non-ASCII chars: met. NO_COLOR purple: 0 color escapes: met.
red == old constants (unit test): met. Not checked by me: user's eye on each hue / 256-color look, `c` mid-reaction live, return-to-muted live (covered by flash() formula only), feed/play flash.
