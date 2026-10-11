//! The `Pet` trait: everything the shared engine (`app`, `social`, `art`, `lib`) needs to
//! know about a non-squid pet. Each pet implements it in `src/<pet>.rs` (state, timers,
//! reactions) and `src/<pet>_art.rs` (drawing). The squid keeps its own code path and is
//! not on the trait. See the wiki page `Adding-a-pet.md` for the checklist.
use crate::app::{App, Behavior, Kind};
use crate::art::{Body, Canvas, Geo};
use crate::link::Msg;
use crate::social::Guest;

/// What a pet sends to a neighbour on `space` (the squid sends ink).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Throw {
    /// A toy mouse that skitters across the window (the cat).
    Mouse,
    /// A bark: no object, the other pet just reacts (the dog).
    Bark,
}

pub trait Pet: Sync {
    fn kind(&self) -> Kind;
    /// Name used by `--pet`.
    fn name(&self) -> &'static str;
    /// Breed names in `c` cycle order; the index is the wire value in `Msg::Arrive*`/`Color`.
    fn breeds(&self) -> Vec<&'static str>;

    // ---- state and timers ----
    /// Set up pet-specific state right after `App::new`.
    fn init(&self, app: &mut App);
    /// Layout for a terminal size and scale.
    fn geo(&self, w: u16, h: u16, scale: f32) -> Geo;
    /// Choose the next idle behavior.
    fn pick_idle(&self, app: &mut App) -> Behavior;
    /// An idle behavior just began.
    fn start_idle(&self, app: &mut App, b: Behavior);
    /// Per-frame pet timers (after the shared ones).
    fn update(&self, app: &mut App, dt: f32);

    // ---- messages shown on screen ----
    fn msg_for(&self, k: crate::app::Key) -> &'static str;
    fn boop_msg(&self) -> &'static str;
    /// Glyphs for dust, bubbles and the like that drift in this pet's scene.
    fn mote_chars(&self) -> [char; 3];
    /// Glyph of the bubble stream another pet sends.
    fn stream_char(&self) -> char;
    /// Bottom key hint line.
    fn hints(&self, social: bool) -> &'static str;

    // ---- reactions ----
    /// A counted feed / pet / play key press (the shared code has already set the message).
    fn on_press(&self, app: &mut App, k: crate::app::Key);
    /// Mouse scroll (ruffle) on the pet at a cell.
    fn on_ruffle(&self, app: &mut App, col: u16, row: u16);
    /// Positions of treats in flight (cell coordinates).
    fn foods(&self, app: &App) -> Vec<(f32, f32)> {
        app.default_foods()
    }

    // ---- visits ----
    /// Wire message announcing a guest of this pet with a breed index.
    fn arrive_msg(&self, breed: u8) -> Msg;
    /// What `space` sends to neighbours.
    fn throws(&self) -> Throw;
    /// A toy mouse ran past the host; `dir` is its direction.
    fn on_mouse(&self, app: &mut App, dir: f32);
    /// Ink hit the host at `ox` cells from its centre.
    fn on_ink(&self, app: &mut App, ox: f32);
    /// A bark arrived at (or was aimed at) the host.
    fn on_bark(&self, app: &mut App);

    // ---- drawing data and drawing ----
    fn offset(&self, app: &App) -> (f32, f32);
    fn host_body(&self, app: &App) -> Body;
    fn guest_body(&self, app: &App, g: &Guest) -> Body;
    /// Size factor and row shift for a guest of kind `guest` visiting this pet's scene,
    /// so e.g. a visiting cat is smaller than the dog and stands on the lawn.
    fn guest_fit(&self, _app: &App, _guest: Kind) -> (f32, f32) {
        (1.0, 0.0)
    }
    /// Column a guest of kind `guest` settles at, given its evenly spaced slot `x`, so e.g.
    /// a visiting squid floats on the open lawn instead of over the doghouse.
    fn guest_x(&self, _app: &App, _guest: Kind, x: f32) -> f32 {
        x
    }
    /// Draw the background scene.
    fn draw_scene(&self, cv: &mut Canvas, app: &App);
    /// Draw one body (host or guest) with owner id `id` for hit testing.
    fn draw_body(&self, cv: &mut Canvas, body: &Body, app: &App, id: u8);
    /// Draw this pet's props in flight (treats, ball, laser dot...).
    fn draw_fx(&self, cv: &mut Canvas, app: &App);
    /// Glyphs of a treat tossed to a neighbour.
    fn treat_glyph(&self) -> &'static str;
}

impl Kind {
    /// The pet behind this kind; `None` for the squid, which has its own code path.
    pub fn pet(self) -> Option<&'static dyn Pet> {
        match self {
            Kind::Squid => None,
            Kind::Cat => Some(&crate::cat::CAT),
            Kind::Dog => Some(&crate::dog::DOG),
        }
    }

    /// Every kind, squid first.
    pub const ALL: [Kind; 3] = [Kind::Squid, Kind::Cat, Kind::Dog];

    /// Color / breed names of this kind.
    pub fn breed_names(self) -> Vec<&'static str> {
        match self.pet() {
            Some(p) => p.breeds(),
            None => crate::art::PRESETS.iter().map(|p| p.0).collect(),
        }
    }
}

/// `"a, b, random"` for `--breed` messages.
pub fn breed_list(p: &dyn Pet) -> String {
    let mut v = p.breeds();
    v.push("random");
    v.join(", ")
}

/// Breed index for `--breed NAME`; `Err(())` means random.
pub fn parse_breed(p: &dyn Pet, s: &str) -> Option<Result<usize, ()>> {
    let s = s.to_ascii_lowercase().replace(['-', '_'], " ");
    if s == "random" {
        return Some(Err(()));
    }
    p.breeds().iter().position(|b| *b == s).map(Ok)
}

/// Breed picked by `--breed random` for a seed.
pub fn random_breed(p: &dyn Pet, seed: u64) -> usize {
    use rand::{Rng, SeedableRng};
    rand::rngs::StdRng::seed_from_u64(seed).random_range(0..p.breeds().len())
}
