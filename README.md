# terminal-pet 🦑🐈

<p align="center">
  <img alt="A red-orange ASCII giant squid filling a terminal window, with hearts floating above it after being petted" src="docs/squid-pet.png">
  <img alt="A calico ASCII cat sitting on a rug in a cozy room, eyes closed with hearts floating above it after being petted" src="docs/cat-pet.png">
</p>

Two full-screen, colored ASCII pets for your terminal: a giant squid and a cat (12 breeds, in a cozy room). The squid is the default (`squid-pet` still works as its own command; `terminal-pet --pet cat` is the cat). The squid fills the window It sways, blinks, and blows bubbles, and you can feed it, pet it, and play with it. It's written in Rust with [ratatui](https://ratatui.rs), and the default red is based on photos of real giant squid. Nothing is saved between runs.

## Quickstart 🚀

Requires Rust (install it with [rustup](https://rustup.rs)).

```
git clone https://github.com/Gidntsquia/squid-pet
cd squid-pet
cargo install --path .   # Installs both binaries: terminal-pet and squid-pet
terminal-pet --pet cat   # the cat
squid-pet                # the squid (same as terminal-pet --pet squid)
```

Press `f` to feed, `p` to pet, `space` to play, `c` to fade to the next color, `v` to send your squid to visit another squid-pet, and `q`, `Esc` or Ctrl-C to quit.

The mouse works too: click the squid to boop it, drag across it to stroke it (hearts, a lean toward your hand, curling tentacles), and scroll over it to ruffle its tentacles. Visiting guests react in their own color. Mouse capture is on by default; use `--no-mouse` to keep your terminal's own text selection.

Cat keys: `f` drops a treat (the cat watches it, bats it, eats it), `space` shines a laser dot (crouch, pounce, miss, then zoomies), `p` purrs, `c` cycles breeds ("I'm a black today!"). Mouse: click = boop ("Mrrp?"), drag = stroke (purring, hearts, a lean toward your hand), scroll = ruffle ("Mrow~").

On Linux x86_64 you can skip Rust and download a prebuilt binary from the [releases page](https://github.com/Gidntsquia/squid-pet/releases).

Other commands:

```
terminal-pet --pet cat --breed siamese   # tabby, black, calico, siamese, orange, tuxedo, gray, white, maine-coon, sphynx, scottish-fold, persian, random
terminal-pet --pet cat --breed random   # Picks a breed at launch (respects --seed)
squid-pet --ascii      # Draws without Unicode glyphs
squid-pet --solo       # Don't look for other squid-pets
squid-pet --no-mouse   # Don't capture the mouse
squid-pet --seed 42    # Same random behavior every run
squid-pet --color purple   # red, purple, blue, pink, green, gold, ghost, blue-ringed
squid-pet --color random   # Picks a color at launch (respects --seed)
NO_COLOR=1 squid-pet   # Monochrome
```

## Features 🔬

- The squid is drawn procedurally to fit the window and is redrawn when you resize it.
- Tentacles sway, the body bobs, the eyes blink, and bubbles drift up past seaweed and waves.
- The squid rests, then swims sideways in short bursts to a new spot.
- Feeding drops a small fish from the top of the screen; the tentacles grab it and the squid puffs up.
- Petting closes its eyes, makes it blush, and sends hearts floating up.
- Playing makes it squirt an ink cloud and dart around the screen.
- Every so often it yawns, looks around, or lets out a small ink puff on its own.
- Comes in eight colors. Pick one with `--color`, or press `c` to fade to the next one while it runs.
- Several squid-pets on the same computer find each other automatically (no network, no setup; `--solo` turns it off).
- Visits: press `v`, or wait 20–30 minutes, and your squid swims out the side of its terminal and shows up in another one in its own colors. It plays there for about 20 seconds (chase, tentacle high-five, ink tag, color flash chat), then swims home. Your terminal shows empty water meanwhile. Press `v` again to call it back. After a visit, squids don't start another on their own for 10 minutes.
- Messages: a fed squid sometimes tosses a fish to a neighbor, squids blow bubble streams across to each other, and playing sends a little ink puff.
- Feed, pet and play stack: pressing them together or repeatedly layers the reactions, adding more and lasting longer.
- Uses truecolor when the terminal supports it and falls back to 256 colors.

## The cat 🐈

- A cozy room: rug, a window with a sunbeam, dust motes drifting in it (instead of bubbles), a plant on one side and a curtain on the other.
- It rests, then walks sideways in short trips. Now and then it blinks, slow-blinks, flicks an ear, twitches its tail, stretches, grooms, looks around, or sneezes (the cat's ink puff).
- Breeds differ by coat and markings, drawn with glyphs so they survive `--ascii` and `NO_COLOR`. Maine coon is bigger and shaggy with a bushy tail, sphynx is hairless with big ears, scottish fold has folded ears, persian has a flat face.
- Cats and squid visit each other, each drawn as itself. Play moves adapt: chase, paw/tentacle high-five, tag (ink or a pounce), and a color/breed flash chat. Messages: a treat or fish toss, mote or bubble streams, ink puffs and pounce flashes.

## Release notes

Both binaries ship together: `terminal-pet` (`--pet squid|cat`) and `squid-pet` (always the squid, unchanged). Link sockets now live under `terminal-pet`, so older squid-pet builds won't see new ones.

## Documentation 📚

- [Development](docs/Development.md) — code layout, building, releases, tests

## License 📄

[MIT](LICENSE).
