# terminal-pet 🦑🐈

<p align="center">
  <img alt="A red-orange ASCII giant squid filling a terminal window, with hearts floating above it after being petted" src="docs/squid-pet.png">
  <img alt="A calico ASCII cat sitting on a rug in a cozy room, eyes closed with hearts floating above it after being petted" src="docs/cat-pet.png">
</p>

Two full-screen, colored ASCII pets for your terminal: a giant squid and a cat with 12 breeds in a cozy room. You can feed them, pet them, play with them, and boop them with the mouse. It's written in Rust with [ratatui](https://ratatui.rs), and the default squid color is based on photos of real giant squid. Nothing is saved between runs.

Pets in different terminals on the same computer find each other and visit. A cat that plays sends a squid a toy mouse, and a squid that plays can ink the cat.

<p align="center">
  <img alt="A calico cat hosting a purple squid while a toy mouse runs along the floor toward the squid" src="docs/cat-toy-mouse.png">
</p>

## Quickstart 🚀

Requires Rust (install it with [rustup](https://rustup.rs)).

```
git clone https://github.com/Gidntsquia/squid-pet
cd squid-pet
cargo install --path .   # Installs both binaries: terminal-pet and squid-pet
terminal-pet --pet cat   # The cat
squid-pet                # The squid (same as terminal-pet --pet squid)
```

Press `f` to feed, `p` to pet, `space` to play, `c` to change color or breed, `v` to visit another pet, and `q` to quit. Click a pet to boop it, drag across it to stroke it, and scroll over it to ruffle it.

On Linux x86_64 you can skip Rust and download a prebuilt binary from the [releases page](https://github.com/Gidntsquia/squid-pet/releases).

Other commands:

```
terminal-pet --pet cat --breed siamese   # Also tabby, black, calico, orange, tuxedo, gray, white and more
squid-pet --color purple                 # Also red, blue, pink, green, gold, ghost, blue-ringed
squid-pet --ascii                        # Draws without Unicode glyphs
squid-pet --solo                         # Don't look for other pets
squid-pet --no-mouse                     # Don't capture the mouse
squid-pet --seed 42                      # Same random behavior every run
NO_COLOR=1 squid-pet                     # Monochrome
```

## Features 🔬

- The squid is drawn to fit the window and is redrawn when you resize it.
- Tentacles sway, the body bobs, the eyes blink, and bubbles drift up past seaweed and waves.
- Feeding drops a fish for the squid to grab, and petting sends hearts floating up.
- The cat bats treats, chases a laser dot, purrs when stroked, and swings its tail as your mouse crosses it.
- Cats come in 12 breeds and squid in eight colors. Press `c` to change while it runs.
- Pets visit each other every 20-30 minutes, or when you press `v`, and play chase, high-five and tag.
- A squid that gets a toy mouse from a cat jumps and puffs up. A cat that gets inked sneezes it off.
- Feed, pet and play stack: pressing them together or repeatedly layers the reactions.
- Uses truecolor when the terminal supports it and falls back to 256 colors.

## Documentation 📚

More details in the
[wiki](https://github.com/Gidntsquia/squid-pet/wiki):

- [Home](https://github.com/Gidntsquia/squid-pet/wiki/Home) — index of all pages
- [Squid](https://github.com/Gidntsquia/squid-pet/wiki/Squid) — keys, mouse, colors
- [Cat](https://github.com/Gidntsquia/squid-pet/wiki/Cat) — keys, mouse, breeds
- [Visits](https://github.com/Gidntsquia/squid-pet/wiki/Visits) — finding neighbors, games, messages
- [Controls](https://github.com/Gidntsquia/squid-pet/wiki/Controls) — every key and gesture
- [Development](https://github.com/Gidntsquia/squid-pet/wiki/Development) — build, tests, code layout
- [Breed Gallery](https://github.com/Gidntsquia/squid-pet/wiki/Breed-Gallery) — every cat and squid color

## License 📄

[MIT](LICENSE).
