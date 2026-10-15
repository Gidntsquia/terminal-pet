//! The dog: breeds, per-dog animation state and the `App` methods that drive it.
//! Drawing lives in `dog_art`. Everything here only runs when `App::kind` is `Dog`.
//! `DogPet` implements the shared `Pet` trait and routes the engine to the methods below.
//! (Step 2 of the wiki's Adding-a-pet.md: this is the `src/<pet>.rs` file.)
use super::{App, BOOP, BURST, Behavior, Heart, Key, Kind, State, ease};
use crate::art::{Body, Canvas, Geo, Rgb};
use crate::link::Msg;
use crate::pet::{Pet, Throw};
use crate::social::{GPhase, Guest};
use rand::Rng;
use rand::seq::SliceRandom;

/// Coat markings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pat {
    Plain,
    /// White face mask, chest and belly.
    Husky,
    Dalmatian,
    /// Dark saddle, white chest and tail tip.
    Beagle,
    Shepherd,
    /// White chest and socks.
    Corgi,
    Poodle,
    /// Mottled cattle dog: speckled back, cream chest and legs.
    Heeler,
    /// Long floppy ears and a long muzzle.
    Dachshund,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tail {
    Plume,
    Straight,
    Curl,
    Stub,
    Pom,
    Bushy,
}

#[derive(Clone, Copy, Debug)]
pub struct Breed {
    pub name: &'static str,
    pub coat: Rgb,
    /// Saddle, spots, ears or mask.
    pub dark: Rgb,
    /// Muzzle, chest and socks.
    pub light: Rgb,
    pub nose: Rgb,
    pub eye: Rgb,
    pub pat: Pat,
    /// Leg length factor (the corgi is 0.5).
    pub leg: f32,
    /// Overall size factor.
    pub size: f32,
    pub ears_up: bool,
    pub tail: Tail,
}

const NOSE: Rgb = (0x1A, 0x14, 0x14);
const BROWN: Rgb = (0x3A, 0x24, 0x14);

const fn b(name: &'static str, coat: Rgb, dark: Rgb, light: Rgb, pat: Pat) -> Breed {
    Breed {
        name,
        coat,
        dark,
        light,
        nose: NOSE,
        eye: (0x24, 0x14, 0x0C),
        pat,
        leg: 1.0,
        size: 1.0,
        ears_up: false,
        tail: Tail::Straight,
    }
}

/// Cycle order for `c`.
pub const BREEDS: [Breed; 11] = [
    Breed {
        tail: Tail::Plume,
        ..b(
            "golden",
            (0xD8, 0xA0, 0x48),
            (0xB8, 0x78, 0x28),
            (0xF0, 0xD0, 0x90),
            Pat::Plain,
        )
    },
    Breed {
        eye: (0xD8, 0xB0, 0x50),
        ..b(
            "lab",
            (0x2A, 0x2A, 0x32),
            (0x1E, 0x1E, 0x24),
            (0x44, 0x44, 0x50),
            Pat::Plain,
        )
    },
    Breed {
        ears_up: true,
        tail: Tail::Curl,
        eye: (0x50, 0xA8, 0xE8),
        size: 1.05,
        ..b(
            "husky",
            (0x78, 0x80, 0x90),
            (0x3C, 0x40, 0x4C),
            (0xF4, 0xF4, 0xFA),
            Pat::Husky,
        )
    },
    Breed {
        ears_up: true,
        tail: Tail::Stub,
        leg: 0.5,
        size: 0.95,
        ..b(
            "corgi",
            (0xD8, 0x84, 0x3C),
            (0xB0, 0x60, 0x24),
            (0xF8, 0xEC, 0xDC),
            Pat::Corgi,
        )
    },
    Breed {
        ..b(
            "dalmatian",
            (0xF2, 0xF2, 0xF4),
            (0x20, 0x20, 0x26),
            (0xFF, 0xFF, 0xFF),
            Pat::Dalmatian,
        )
    },
    Breed {
        size: 0.9,
        ..b(
            "beagle",
            (0xB8, 0x78, 0x40),
            BROWN,
            (0xFA, 0xF4, 0xE8),
            Pat::Beagle,
        )
    },
    Breed {
        tail: Tail::Pom,
        size: 1.0,
        ..b(
            "poodle",
            (0xF6, 0xF6, 0xFA),
            (0xDC, 0xDC, 0xE8),
            (0xFF, 0xFF, 0xFF),
            Pat::Poodle,
        )
    },
    Breed {
        ears_up: true,
        tail: Tail::Bushy,
        size: 1.1,
        ..b(
            "german shepherd",
            (0xC4, 0x8C, 0x4C),
            (0x24, 0x20, 0x20),
            (0xE0, 0xB8, 0x80),
            Pat::Shepherd,
        )
    },
    // the hot dog
    Breed {
        leg: 0.4,
        size: 1.0,
        tail: Tail::Straight,
        ..b(
            "dachshund",
            (0x9A, 0x4E, 0x2A),
            (0x5A, 0x2A, 0x16),
            (0xC8, 0x82, 0x50),
            Pat::Dachshund,
        )
    },
    // Bluey and Bingo, the Heeler sisters
    Breed {
        ears_up: true,
        tail: Tail::Bushy,
        size: 1.0,
        eye: (0x24, 0x14, 0x0C),
        ..b(
            "bluey",
            (0x5C, 0x7C, 0xB0),
            (0x32, 0x4A, 0x78),
            (0xF0, 0xD4, 0xA0),
            Pat::Heeler,
        )
    },
    Breed {
        ears_up: true,
        tail: Tail::Bushy,
        size: 0.92,
        ..b(
            "bingo",
            (0xD0, 0x6A, 0x38),
            (0xA0, 0x48, 0x20),
            (0xF4, 0xDC, 0xB0),
            Pat::Heeler,
        )
    },
];

/// Marker in `Fish::from.1`: the biscuit is tossed to be caught in the air...
pub const TOSS_CATCH: f32 = -1.0;
/// ... or tossed to land on the grass.
pub const TOSS_LAND: f32 = -2.0;
/// Seconds the dog fetches the ball for.
pub const PLAY_LEN: f32 = 7.0;
/// Seconds a dug hole stays open before it fills back in, and the filling time.
pub const HOLE_STAY: f32 = 6.0;
pub const HOLE_FILL: f32 = 1.5;
/// Seconds the dog chases a toy mouse.
pub const CHASE_T: f32 = 2.4;

/// Seconds the head takes to turn from the side view to the front face after stopping.
pub const TURN_T: f32 = 0.25;

/// Per-dog animation state that the squid does not have.
#[derive(Default)]
pub struct DogState {
    pub bag: Vec<Behavior>,
    /// Facing: 1 right, -1 left.
    pub face: f32,
    prev_x: f32,
    /// A dug hole: (x in cells, seconds since digging began, seconds the dig lasted or 0 while digging).
    pub hole: Option<(f32, f32, f32)>,
    /// Seconds of panting left after play.
    pub warm: f32,
    /// Chasing a toy mouse: (direction, seconds left).
    pub chase: (f32, f32),
    /// Seconds left of a bark pose.
    pub bark_t: f32,
    /// How far (0..1) the dog has gone into the doghouse.
    pub nap_k: f32,
    /// `--nap`: stay asleep in the doghouse until something wakes the dog.
    pub nap_hold: bool,
    /// Play: ball target in cells from the start spot, and the side the ball was thrown to.
    pub ball_dx: f32,
    pub was_play: bool,
    /// Biscuits alternate between catch-in-the-air and landing on the grass.
    pub toss_flip: bool,
    pub next_wag: f32,
    /// Columns the standing dog is pushed right so it never overlaps the doghouse.
    pub clear_dx: f32,
    /// Widest reach (columns left and right of its centre, per unit of the visit scale
    /// `App::scale`) each kind of guest was seen drawn at, and the pane size.
    pub guest_half: Vec<(Kind, u16, u16, f32, f32)>,
    /// 1 right after a side-view frame, easing to 0 as the head turns to face the viewer.
    pub sideways: f32,
    /// The side view came from walking or running (stay on its feet while turning), not
    /// from a roll-over (which sits down by itself).
    pub turn_up: bool,
}

/// Everything about the dog's pose that is not already in `Body`.
#[derive(Clone, Copy, Debug)]
pub struct DogPose {
    pub walking: bool,
    pub running: bool,
    pub step: f32,
    /// Tail wag amplitude 0..1 and phase.
    pub wag: f32,
    pub wag_ph: f32,
    /// 0..1 nose to the ground.
    pub head_down: f32,
    /// -1..1 head up (+) or down a little (-): tracking a flying object.
    pub look_up: f32,
    /// -1..1 head tilt (boop).
    pub tilt: f32,
    pub open: f32,
    pub tongue: f32,
    pub bark: f32,
    /// Front paws digging, hind leg scratching the ear: 0..1 and a fast phase.
    pub dig: f32,
    pub scratch: f32,
    pub fast_ph: f32,
    /// Horizontal squash for a turn (1 facing right, -1 facing left).
    pub spin: f32,
    /// Roll-over flipbook: 0 when not rolling, else `pose + progress` with pose 1..=5
    /// (see [`RollPose`]) and progress 0..1 through that pose.
    pub roll: f32,
    /// Whole-body shake amplitude.
    pub shake: f32,
    /// Inside the doghouse: only the head and tail show.
    pub inside: bool,
    /// 0 nothing, 1 biscuit, 2 ball in the mouth.
    pub carry: u8,
    pub crouch: f32,
    pub sniff: f32,
    /// 0..1 how far into the doghouse the dog has gone.
    pub nap: f32,
    /// Held on its feet (roll-over, pet) even when nothing else says so.
    pub up: bool,
    /// 0..1 how far the head has turned from the side view to the front face after the
    /// dog stops (1 = facing the viewer).
    pub turn: f32,
}

impl DogPose {
    /// The default idle pose is sitting, facing the viewer; the dog gets up (side view)
    /// to walk, sniff, dig, fetch, spin or roll.
    /// A happy dog on the move: panting smile, tongue out, tail wagging.
    pub fn happy_on_the_move(&mut self) {
        if self.walking || self.running {
            self.wag = self.wag.max(0.8);
            self.open = self.open.max(0.45);
            self.tongue = 1.0;
        }
    }

    pub fn seated(&self) -> bool {
        !(self.walking
            || self.running
            || self.up
            || self.inside
            || self.head_down > 0.05
            || self.dig > 0.0
            || self.crouch > 0.0
            || self.roll > 0.0
            || self.nap > 0.0
            || self.carry == 2
            || self.spin.abs() < 0.999)
    }
}

/// The hand-drawn poses of the `p` roll-over, played in this order after the seated pose.
/// It follows how a real dog rolls: down on its belly, flop onto one side, over the back
/// with the legs bent and tucked, onto the other side, up, and sit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollPose {
    /// Lying down on its belly (sphinx): front legs forward, hind legs tucked, head up.
    Down,
    /// Flat on its side, belly toward the viewer, legs out to that side.
    SideA,
    /// Half way over: back partly down, legs bent and tucked toward the belly.
    TwistA,
    /// On its back, belly up, head tilted back, paws curled, hind legs bent and splayed.
    Back,
    /// Half way over the other way.
    TwistB,
    /// Flat on the other side, back toward the viewer, paws peeking over the belly.
    SideB,
    /// Up on its feet, seen from the side.
    Stand,
    /// Sitting back down (the seated pose).
    Sit,
}

/// Seconds in each roll pose, in order. Down and up take their time; the roll itself
/// (side, over the back, other side) is one quick, continuous flop with only a short
/// wiggle belly-up, so nothing stalls in the middle.
pub const ROLL_TIMES: [f32; 8] = [0.45, 0.3, 0.2, 0.35, 0.2, 0.3, 0.35, 0.4];
/// Whole roll-over (about 2.55 s).
pub const ROLL_LEN: f32 = 0.45 + 0.3 + 0.2 + 0.35 + 0.2 + 0.3 + 0.35 + 0.4;
const ROLL_POSES: [RollPose; 8] = [
    RollPose::Down,
    RollPose::SideA,
    RollPose::TwistA,
    RollPose::Back,
    RollPose::TwistB,
    RollPose::SideB,
    RollPose::Stand,
    RollPose::Sit,
];

impl DogPose {
    /// The roll pose being shown and how far through it (0..1), or None when not rolling.
    pub fn roll_pose(&self) -> Option<(RollPose, f32)> {
        if self.roll < 1.0 {
            return None;
        }
        let i = (self.roll as usize - 1).min(ROLL_POSES.len() - 1);
        Some((ROLL_POSES[i], self.roll.fract()))
    }
}

/// Flipbook position (`DogPose::roll`) at `st` seconds into a pet reaction: pose index + 1
/// plus the progress through that pose; 0 when not rolling. More petting while it rolls
/// makes the reaction longer but never holds a roll pose longer: the dog finishes the roll
/// and wags seated for the rest.
pub fn roll_at(st: f32) -> f32 {
    let t = ROLL_TIMES;
    let mut start = 0.0;
    if st < 0.0 {
        return 0.0;
    }
    for (i, d) in t.iter().enumerate() {
        if st < start + d {
            return (i + 1) as f32 + ((st - start) / d).min(0.999);
        }
        start += d;
    }
    0.0
}

impl Default for DogPose {
    fn default() -> Self {
        DogPose {
            walking: false,
            running: false,
            step: 0.0,
            wag: 0.0,
            wag_ph: 0.0,
            head_down: 0.0,
            look_up: 0.0,
            tilt: 0.0,
            open: 0.0,
            tongue: 0.0,
            bark: 0.0,
            dig: 0.0,
            scratch: 0.0,
            fast_ph: 0.0,
            spin: 1.0,
            roll: 0.0,
            shake: 0.0,
            inside: false,
            carry: 0,
            crouch: 0.0,
            sniff: 0.0,
            nap: 0.0,
            up: false,
            turn: 1.0,
        }
    }
}

fn tri(s: f32, a: f32, b: f32) -> f32 {
    if (a..b).contains(&s) {
        (std::f32::consts::PI * (s - a) / (b - a)).sin()
    } else {
        0.0
    }
}

/// Smooth in at `a`, out at `b` (each over `r` seconds).
fn window(s: f32, a: f32, b: f32, r: f32) -> f32 {
    ease((s - a) / r) * (1.0 - ease((s - (b - r)) / r))
}

impl App {
    /// Switch this process to the dog (call right after `App::new`).
    pub fn init_dog(&mut self) {
        self.kind = Kind::Dog;
        self.dog.face = 1.0;
        self.dog.next_wag = self.t + self.rng.random_range(2.0..6.0);
    }

    fn dog_pick_idle(&mut self) -> Behavior {
        if self.dog.bag.is_empty() {
            let mut v = vec![
                Behavior::Sniff,
                Behavior::Dig,
                Behavior::Nap,
                Behavior::EarScratch,
                Behavior::TailSpin,
                Behavior::Bark,
            ];
            v.shuffle(&mut self.rng);
            self.dog.bag = v;
        }
        self.dog.bag.pop().unwrap_or(Behavior::Sniff)
    }

    /// Start asleep in the doghouse (`--nap`).
    pub fn dog_start_asleep(&mut self) {
        self.state = State::IdleBehavior(Behavior::Nap);
        self.st = 0.0;
        self.dog_start_idle(Behavior::Nap);
        self.dog.nap_k = 1.0;
        self.dog.nap_hold = true;
    }

    fn dog_start_idle(&mut self, b: Behavior) {
        let ox = self.offset().0;
        let x = self.geo().cx as f32 + ox;
        match b {
            Behavior::Dig => {
                // the hole opens just ahead of the front paws
                self.dog.hole = Some((x + self.dog.face * self.geo().maxw * 1.25, 0.0, 0.0));
            }
            Behavior::Bark => {
                self.dog.bark_t = 1.0;
                self.set_msg("Woof!");
            }
            Behavior::Nap => self.set_msg("Zzz..."),
            _ => {}
        }
    }

    /// Dust motes and seeds drifting through the yard.
    fn dog_motes(&mut self, dt: f32) {
        if self.bubbles.len() < 24 && self.rng.random_range(0.0..1.0) < 1.2 * dt {
            let y = self.rng.random_range(4.0..(self.h as f32 - 12.0).max(5.0));
            self.bubbles.push(super::Bubble {
                x: self.rng.random_range(0.0..self.w as f32),
                y,
                vy: self.rng.random_range(0.6..1.6),
                ch: ['.', '*', '\u{b0}'][self.rng.random_range(0..3)],
                phase: self.rng.random_range(0.0..std::f32::consts::TAU),
            });
        }
        for b in &mut self.bubbles {
            b.y -= b.vy * dt * 0.3;
            b.x += 0.8 * b.vy * dt;
        }
        let w = self.w as f32;
        self.bubbles.retain(|b| b.y > 2.0 && b.x < w + 2.0);
    }

    fn dog_update(&mut self, dt: f32) {
        let t = self.t;
        self.dog.bark_t = (self.dog.bark_t - dt).max(0.0);
        self.dog.warm = (self.dog.warm - dt).max(0.0);
        self.dog.chase.1 = (self.dog.chase.1 - dt).max(0.0);
        if self.dog.nap_hold {
            if matches!(self.state, State::IdleBehavior(Behavior::Nap)) {
                self.st = 0.0;
            } else {
                self.dog.nap_hold = false;
            }
        }
        // nap: walk to the door, stay in, walk out
        let want = match self.state {
            State::IdleBehavior(Behavior::Nap) => {
                f32::from(self.st < App::idle_duration(Behavior::Nap) - 1.4)
            }
            _ => 0.0,
        };
        let step = dt / 1.4;
        self.dog.nap_k += (want - self.dog.nap_k).clamp(-step, step);
        // the hole fills back in after a while
        if let Some((x, age, len)) = self.dog.hole {
            let digging = matches!(self.state, State::IdleBehavior(Behavior::Dig));
            let age = age + dt;
            let len = if digging {
                0.0
            } else if len == 0.0 {
                age
            } else {
                len
            };
            self.dog.hole = if age - len > HOLE_STAY + HOLE_FILL {
                None
            } else {
                Some((x, age, len))
            };
        }
        // the ball is over: pant for a while
        if self.play.on {
            self.dog.was_play = true;
        } else if self.dog.was_play {
            self.dog.was_play = false;
            self.dog.warm = 9.0;
        }
        // facing: follow the direction of travel, or the biscuit
        let x = self.geo().cx as f32 + self.offset().0 - self.dog.clear_dx;
        if (x - self.dog.prev_x).abs() > 0.03 {
            self.dog.face = (x - self.dog.prev_x).signum();
        }
        self.dog.prev_x = x;
        if self.feed.on
            && let Some(f) = self.fish.last()
            && self.fish_age(f) < 1.2
        {
            self.dog.face = (f.from.0 - x).signum();
        }
        if self.dog.chase.1 > 0.0 {
            self.dog.face = self.dog.chase.0;
        }
        let _ = t;
    }

    /// Where the dog's play offset is (cells): runs to the ball, picks it up, brings it back.
    pub fn dog_play_dx(&self) -> f32 {
        if !self.play.on {
            return 0.0;
        }
        let s = self.dart_s;
        let from = self.dart_from.0;
        let t = self.dog.ball_dx;
        if s < 0.9 {
            from * (1.0 - ease(s / 0.6))
        } else if s < 2.8 {
            let k = ease((s - 0.9) / 1.9);
            from * (1.0 - ease(s / 0.6)) + (t - from * (1.0 - ease(s / 0.6))) * k
        } else if s < 3.4 {
            t
        } else if s < 5.3 {
            t * (1.0 - ease((s - 3.4) / 1.9))
        } else {
            0.0
        }
    }

    fn dog_nap_dx(&self) -> f32 {
        if self.dog.nap_k <= 0.0 {
            return 0.0;
        }
        let door = crate::dog_art::door_x(self.w, self.h);
        let base = self.geo().cx as f32 + self.dog_seat_dx() + self.dog_play_dx();
        (door - base) * ease(self.dog.nap_k)
    }

    /// Offset of the dog's resting spot: the idle drift, the guest layout shift and the
    /// walk off screen, kept so the seated dog (paws and haunches) is clear of the doghouse.
    fn dog_seat_dx(&self) -> f32 {
        let free = self.wander() + self.host_dx + self.fx.dx;
        let min = (crate::dog_art::house_right(self.w, self.h) + 2) as f32
            + crate::dog_art::seat_half(self.w, self.h)
            - self.geo().cx as f32;
        free.max(min) + self.home_ox()
    }

    fn dog_chase_dx(&self) -> f32 {
        let (dir, left) = self.dog.chase;
        if left <= 0.0 {
            return 0.0;
        }
        let u = 1.0 - left / CHASE_T;
        let reach = self.w as f32 * 0.28;
        let k = if u < 0.45 {
            ease(u / 0.45)
        } else if u < 0.6 {
            1.0
        } else {
            1.0 - ease((u - 0.6) / 0.4)
        };
        dir * reach * k
    }

    pub fn dog_dx(&self) -> f32 {
        self.dog_seat_dx()
            + self.dog_play_dx()
            + self.dog_chase_dx()
            + self.dog_nap_dx()
            + self.dog.clear_dx
    }

    /// First column clear of the doghouse.
    fn house_left(&self) -> f32 {
        (crate::dog_art::house_right(self.w, self.h) + 2) as f32
    }

    /// First column the standing dog may reach on the left past a guest (a floating squid,
    /// a cat on the lawn) between the house and the dog, or past the house without one.
    fn guest_left(&self) -> f32 {
        // the walking dog passes in front of the doghouse: only the pane edge limits it
        let mut left: f32 = 1.0;
        let dog_x = self.geo().cx as f32 + self.host_dx;
        for g in &self.guests {
            let x = guest_drawn_x(g);
            if x < dog_x {
                left = left.max(x + guest_reach(self, g.kind).1 + 2.0);
            }
        }
        left
    }

    /// Leftmost and rightmost columns of the dog drawn as `body` (without `clear_dx`),
    /// measured past the pane edges by probing it shifted back into view.
    fn dog_extent(&self, body: &Body) -> (f32, f32) {
        let w = self.w as f32;
        let probe = |shift: f32| {
            let mut b = body.clone();
            b.dx += shift - self.dog.clear_dx;
            let mut cv = crate::art::Canvas::new(self.w, self.h, false);
            crate::dog_art::draw_dog(&mut cv, &b);
            let col = |x: usize| (0..cv.h).any(|y| cv.cells[y * cv.w + x].fg.is_some());
            let l = (0..cv.w).find(|&x| col(x));
            let r = (0..cv.w).rev().find(|&x| col(x));
            (l, r)
        };
        let (mut left, mut right) = (w, 0.0);
        for k in 0..4 {
            let shift = k as f32 * w * 0.5;
            match probe(shift).0 {
                Some(0) => continue,
                Some(x) => left = x as f32 - shift,
                None => {}
            }
            break;
        }
        for k in 0..4 {
            let shift = -(k as f32) * w * 0.5;
            match probe(shift).1 {
                Some(x) if x + 1 == self.w as usize => continue,
                Some(x) => right = x as f32 - shift,
                None => {}
            }
            break;
        }
        (left, right)
    }

    /// Columns a ball at play offset `ball_dx` must move (right +, left -) so the dog
    /// fetching it (running there, then nose down at it) stays clear of the doghouse and
    /// inside the pane.
    fn ball_shift(&self, ball_dx: f32, side: f32) -> f32 {
        let mut b = self.dog_host_body();
        b.dx = self.dog_dx() - self.dog_play_dx() + ball_dx;
        b.dpose.roll = 0.0;
        b.dpose.inside = false;
        b.dpose.nap = 0.0;
        b.dpose.spin = side;
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for (run, down) in [(true, 0.0), (false, 1.0)] {
            b.dpose.walking = run;
            b.dpose.running = run;
            b.dpose.head_down = down;
            let (l, r) = self.dog_extent(&b);
            lo = lo.min(l);
            hi = hi.max(r);
        }
        // the house always wins; a guest only as far as the dog still fits the pane (a
        // squid playing with the dog drifts toward it for a moment)
        let edge = self.w as f32 - 1.0;
        let clear = self.guest_left().min(edge - (hi - lo));
        if lo < clear {
            clear - lo
        } else if hi > edge {
            edge - hi
        } else {
            0.0
        }
    }

    /// Keep the standing dog clear of the doghouse: step right at once when its pose would
    /// reach over the house, drift back while it walks once it no longer needs to.
    /// Measure how wide each guest is drawn: the columns left of the dog that change when
    /// the guest is taken out of the scene. Keeps the widest seen per kind and pane size.
    fn measure_guests(&mut self) {
        for i in 0..self.guests.len() {
            self.measure_guest(i);
        }
    }

    fn measure_guest(&mut self, i: usize) {
        let g = &self.guests[i];
        if g.phase != GPhase::Present || g.startle > 0.0 {
            return;
        }
        let kind = g.kind;
        let mut dv = crate::art::Canvas::new(self.w, self.h, false);
        crate::dog_art::draw_dog(&mut dv, &self.dog_host_body());
        let dog_left = (0..dv.w)
            .find(|&x| (0..dv.h).any(|y| dv.cells[y * dv.w + x].fg.is_some()))
            .unwrap_or(dv.w);
        let x0 = self.guests[i].x;
        let gx = guest_drawn_x(&self.guests[i]);
        if gx + 1.0 >= dog_left as f32 {
            return;
        }
        // the dog may stand somewhere else once the guest is gone (it keeps clear of
        // guests): only columns left of the dog in both scenes count
        let drawn_left = |app: &App| {
            let mut cv = crate::art::Canvas::new(app.w, app.h, false);
            crate::dog_art::draw_dog(&mut cv, &app.host_body());
            (0..cv.w)
                .find(|&x| (0..cv.h).any(|y| cv.cells[y * cv.w + x].fg.is_some()))
                .unwrap_or(cv.w)
        };
        let with = crate::art::render(self, false);
        let left_with = drawn_left(self);
        self.guests[i].x = -10_000.0;
        let without = crate::art::render(self, false);
        let left_without = drawn_left(self);
        self.guests[i].x = x0;
        let n = dog_left
            .min(left_with)
            .min(left_without)
            .saturating_sub(1)
            .min(with.w);
        let changed = |x: usize| {
            (0..with.h).any(|y| {
                let k = y * with.w + x;
                with.cells[k].ch != without.cells[k].ch || with.cells[k].fg != without.cells[k].fg
            })
        };
        // the run of changed columns around the guest (gaps of up to 2, e.g. between tentacles)
        let c = (gx.round().max(0.0) as usize).min(n.saturating_sub(1));
        let (mut lo, mut hi) = (c, c);
        let mut x = c;
        while x > 0 && (1..=3).any(|d| x >= d && changed(x - d)) {
            x = (1..=3)
                .find(|&d| x >= d && changed(x - d))
                .map_or(x, |d| x - d);
            lo = x;
        }
        let mut x = c;
        while x + 1 < n && (1..=3).any(|d| x + d < n && changed(x + d)) {
            x = (1..=3)
                .find(|&d| x + d < n && changed(x + d))
                .map_or(x, |d| x + d);
            hi = x;
        }
        let k = self.scale.max(0.05);
        let (l, r) = ((gx - lo as f32 + 1.0) / k, (hi as f32 - gx + 1.0) / k);
        let (w, h) = (self.w, self.h);
        let list = &mut self.dog.guest_half;
        list.retain(|e| (e.1, e.2) == (w, h));
        match list.iter_mut().find(|e| e.0 == kind) {
            Some(e) => (e.3, e.4) = (e.3.max(l), e.4.max(r)),
            None => list.push((kind, w, h, l, r)),
        }
    }

    fn dog_keep_clear(&mut self, dt: f32) {
        self.measure_guests();
        let body = self.dog_host_body();
        let p = &body.dpose;
        let free = p.seated() || p.inside || p.roll > 0.0 || self.dog.nap_k > 0.0;
        if free {
            if p.inside || self.dog.nap_k > 0.0 {
                self.dog.clear_dx = 0.0;
            } else {
                // seated, the resting spot already fits the lawn: scoot back from a push
                // left over from standing up (e.g. past a guest), never onto the house
                let c = self.dog.clear_dx;
                let lo = self.house_left() - self.dog_extent(&body).0;
                self.dog.clear_dx = (c - c.signum() * (6.0 * dt).min(c.abs())).max(lo.min(c));
            }
            return;
        }
        let (lo, hi) = self.yard_range(&body);
        let c = self.dog.clear_dx;
        // drift back toward no push while walking, never out of the allowed range
        let relaxed = if c > 0.0 {
            (c - 6.0 * dt).max(0.0)
        } else {
            (c + 6.0 * dt).min(0.0)
        };
        self.dog.clear_dx = relaxed.min(hi).max(lo);
    }

    /// The range of pushes (columns, right +) that keep the dog drawn as `body` (without
    /// `clear_dx`) clear of the doghouse, a guest, the tree and the right edge, in that order.
    fn yard_range(&self, body: &Body) -> (f32, f32) {
        let (left, right) = self.dog_extent(body);
        // inside the right edge, and off the tree unless a guest leaves no room
        let tree = crate::dog_art::tree_left(self.w, self.h) as f32 + 1.0;
        let hi_free = self.w as f32 - 2.0 - right;
        let lo = (self.guest_left() - left).min(hi_free);
        let hi = (tree - right).min(hi_free).max(lo);
        (lo, hi)
    }

    /// The thrown ball in cell coordinates while it is on screen and not in the mouth.
    pub fn ball_pos(&self) -> Option<(f32, f32)> {
        if !self.play.on || self.kind != Kind::Dog {
            return None;
        }
        let s = self.dart_s;
        let g = self.geo();
        let feet = (g.top + g.mh + g.hh + g.al) as f32;
        let base = g.cx as f32 + self.dog_seat_dx();
        let start = (
            base + self.dart_from.0 + self.dog.face * g.maxw * 0.5,
            feet - g.maxw * 0.8,
        );
        let land = (base + self.dog.ball_dx, feet - 1.0);
        if s < 1.0 {
            let u = s / 1.0;
            let x = start.0 + (land.0 - start.0) * u;
            let y = start.1 + (land.1 - start.1) * u - 9.0 * 4.0 * u * (1.0 - u);
            Some((x, y))
        } else if s < 1.7 {
            // bounce and roll a little further
            let u = (s - 1.0) / 0.7;
            let dir = (land.0 - start.0).signum();
            Some((
                land.0 + dir * 4.0 * ease(u),
                land.1 - 2.5 * 4.0 * u * (1.0 - u),
            ))
        } else if s < 2.9 {
            let dir = (land.0 - start.0).signum();
            Some((land.0 + dir * 4.0, land.1))
        } else if s < 5.3 {
            None
        } else if s < 5.7 {
            // dropped at the feet
            let u = (s - 5.3) / 0.4;
            let m = self.dog_mouth_now();
            let drop = (base + self.dog.face * g.maxw * 1.1, feet - 1.0);
            Some((
                m.0 + (drop.0 - m.0) * u,
                m.1 + (drop.1 - m.1) * u - 2.0 * 4.0 * u * (1.0 - u),
            ))
        } else {
            Some((base + self.dog.face * g.maxw * 1.1, feet - 1.0))
        }
    }

    /// Where the dog's mouth is right now (cell coordinates).
    pub fn dog_mouth_now(&self) -> (f32, f32) {
        crate::dog_art::mouth_pos(self, &self.dog_host_body())
    }

    pub fn dog_pose(&self) -> DogPose {
        let t = self.t;
        let mut p = DogPose {
            // walks its bursts, and walks to the doghouse door and back out for a nap
            walking: self.burst.2 < BURST && self.home_ox() == 0.0 && self.dog.nap_k <= 0.0
                || self.dog.nap_k > 0.0 && self.dog.nap_k < 1.0,
            step: t * 9.0,
            wag_ph: t * 14.0,
            fast_ph: t * 30.0,
            ..Default::default()
        };
        p.wag = if t < self.dog.next_wag + 0.8 && t >= self.dog.next_wag {
            0.5
        } else {
            0.0
        };
        if self.t >= self.dog.next_wag + 0.8 {
            // (scheduling is done in update; this keeps pose a pure function of state)
        }
        if self.dog.warm > 0.0 {
            p.open = 0.4 + 0.15 * (t * 9.0).sin();
            p.tongue = 1.0;
        }
        if let State::IdleBehavior(b) = self.state {
            let st = self.st;
            let d = App::idle_duration(b);
            match b {
                Behavior::Sniff => {
                    p.head_down = window(st, 0.0, d, 0.6);
                    p.sniff = p.head_down;
                }
                Behavior::Dig => {
                    p.head_down = 0.7 * window(st, 0.0, d, 0.5);
                    p.dig = window(st, 0.4, d - 0.4, 0.4);
                }
                Behavior::Nap => {
                    p.inside = self.dog.nap_k >= 1.0;
                }
                Behavior::EarScratch => {
                    p.scratch = window(st, 0.0, d, 0.4);
                    p.tilt = 0.5 * p.scratch;
                }
                Behavior::TailSpin => {
                    let k = window(st, 0.0, d, 0.5);
                    let ph = st * std::f32::consts::TAU * 1.7;
                    p.spin = if k > 0.0 { ph.cos() } else { 1.0 };
                    p.wag = 1.0;
                    p.running = false;
                }
                Behavior::Bark => {
                    p.bark = tri(st, 0.3, 0.9);
                    p.open = p.bark;
                    p.look_up = 0.5 * p.bark;
                }
                _ => {}
            }
        }
        if self.dog.bark_t > 0.0 && p.bark == 0.0 {
            p.bark = tri(0.6 - self.dog.bark_t.min(0.6), 0.0, 0.6).max(0.0);
            p.open = p.bark;
            p.look_up = 0.4 * p.bark;
        }
        // boop: head tilt
        if self.touch.boop > 0.0 {
            p.tilt += (self.touch.boop / BOOP).clamp(0.0, 1.0) * 0.8;
        }
        // stroke: wag and lean in
        if self.touch.touch > 0.0 {
            p.wag = p.wag.max(ease(self.touch.touch));
        }
        // shake-off after ink, and the scroll ruffle
        if let Some((left, _)) = self.splat
            && left < 1.4
        {
            p.shake = (left / 0.5).clamp(0.3, 1.0);
        }
        p.shake = p.shake.max(self.touch.ruffle_k());
        // pet: wag hard, roll over
        if self.pet.on {
            let r = &self.pet;
            p.wag = 1.0;
            p.roll = roll_at(r.st);
            if p.roll > 0.0 {
                p.tongue = 1.0;
                p.open = p.open.max(0.3);
            }
        }
        // biscuit: watch it, catch it, or pick it off the grass
        if self.feed.on
            && let Some(f) = self.fish.last()
        {
            let a = self.fish_age(f);
            let land = f.from.1 == TOSS_LAND;
            if land {
                if a < 0.9 {
                    p.look_up = 0.5;
                } else if a < 1.9 {
                    p.head_down = ease((a - 0.9) / 0.5) * (1.0 - ease((a - 1.4) / 0.5));
                }
                if a >= 1.4 {
                    p.carry = 1;
                }
            } else {
                if a < 1.1 {
                    p.look_up = 0.8;
                    p.open = ease((a - 0.5) / 0.4);
                } else {
                    p.carry = 1;
                }
            }
            if p.carry == 1 {
                p.open = p.open.max(0.35 * (1.0 + (a * 22.0).sin()).abs() * 0.5);
            }
            p.wag = p.wag.max(0.6);
        }
        // ball: watch, run, pick up, bring back, drop, wag
        if self.play.on {
            let s = self.dart_s;
            p.crouch = tri(s, 0.1, 0.9);
            p.look_up = p.look_up.max(0.6 * tri(s, 0.0, 1.4));
            let moving = (0.9..2.8).contains(&s) || (3.4..5.3).contains(&s);
            if moving {
                p.running = true;
                p.walking = false;
                p.step = t * 20.0;
                p.open = 0.5;
                p.tongue = 1.0;
            }
            p.head_down = p.head_down.max(window(s, 2.6, 3.4, 0.25));
            if (2.9..5.4).contains(&s) {
                p.carry = 2;
            }
            if s >= 5.3 {
                p.wag = 1.0;
                p.open = 0.5;
                p.tongue = 1.0;
            }
        }
        if self.dog.chase.1 > 0.0 {
            p.running = true;
            p.walking = false;
            p.step = t * 20.0;
            p.open = 0.5;
            p.tongue = 1.0;
            p.wag = 0.7;
        }
        p.walking &= !p.running;
        p.happy_on_the_move();
        p.nap = self.dog.nap_k;
        // held up by the roll-over only while it plays (it ends sitting)
        p.up = self.pet.on && p.roll > 0.0;
        // after moving, the dog stays on its feet while its head turns to the viewer
        p.turn = 1.0 - self.dog.sideways;
        if p.roll == 0.0 && self.dog.sideways > 0.0 && self.dog.turn_up {
            p.up = true;
        }
        p
    }

    /// Draw data for the host dog.
    pub fn dog_host_body(&self) -> Body {
        let g = self.geo();
        let (dx, mut dy) = self.offset();
        dy += (crate::dog_art::horizon(self.w, self.h) - crate::dog_art::ground(self.w, self.h).0)
            as f32
            * ease(self.dog.nap_k.clamp(0.0, 1.0));
        let pose = self.dog_pose();
        Body {
            kind: Kind::Dog,
            breed: self.color_idx(),
            breed_from: self.breed_from,
            bfade: ease(self.fade),
            boop: (self.touch.boop / BOOP).clamp(0.0, 1.0),
            dpose: DogPose {
                // face is carried in `dir` of the pose via spin sign
                spin: pose.spin * self.dog.face,
                ..pose
            },
            cx: g.cx,
            dx,
            dy,
            scale: self.scale
                * (1.0 - (1.0 - crate::dog_art::NAP_SCALE) * ease(self.dog.nap_k.clamp(0.0, 1.0))),
            blush: self.blush(),
            flash: self.flash(),
            bright: self.fx.bright,
            dark: self.fx.dark,
            // eyes stay open (the seated eye with its shine) through the whole roll-over
            closed: (self.eyes_closed() && pose.roll == 0.0) || pose.inside,
            blink: self.blinking(),
            lean: 0.0,
            food_x: g.cx as f32 + dx,
            t: self.t,
            touch: ease(self.touch.touch),
            touch_at: self.touch.at,
            ruffle: self.touch.ruffle_k(),
            ruffle_at: self.touch.ruffle_at,
            head_dy: self.touch.dy(),
            dir: self.touch.dir,
            splat: self.splat,
            ..Body::default()
        }
    }

    pub(crate) fn dog_msg(k: Key) -> &'static str {
        match k {
            Key::Feed => "Crunch!",
            Key::Pet => "Good dog \u{2665}",
            Key::Play => "Fetch!",
        }
    }
}

pub struct DogPet;
pub static DOG: DogPet = DogPet;

/// Column a guest is drawn centred on (a squid also jerks aside when startled).
fn guest_drawn_x(g: &Guest) -> f32 {
    match g.kind {
        Kind::Squid => g.x + g.fx.dx - g.startle_dir * g.jerk(),
        _ => g.x + g.fx.dx,
    }
}

/// How far (columns) a guest of `kind` reaches left and right of its centre (a cat's tail
/// makes it lopsided): as measured on screen, or a generous estimate before that.
fn guest_reach(app: &App, kind: Kind) -> (f32, f32) {
    if let Some(e) = app
        .dog
        .guest_half
        .iter()
        .find(|e| e.0 == kind && (e.1, e.2) == (app.w, app.h))
    {
        let k = app.scale.max(0.05);
        return (e.3 * k, e.4 * k);
    }
    let half = guest_half_estimate(app, kind);
    (half, half)
}

/// A guest's drawn width (columns, at the current scale), from the half that faces away
/// from the dog: the measured half towards the dog can run into the dog's own cells.
fn guest_span(app: &App, g: &Guest) -> f32 {
    let (l, r) = guest_reach(app, g.kind);
    2.0 * if g.side < 0.0 { l } else { r }
}

/// How far (in seated widths, `seat_dims().0`) the seated dog reaches from its centre to
/// the left (floppy ears and paws); the tail reaches a little further right.
const SEAT_REACH: f32 = 0.65;

/// Half the seated dog's whole drawn width (ears, paws and tail), in seated widths.
const DOG_REACH: f32 = 0.72;

fn guest_half_estimate(app: &App, kind: Kind) -> f32 {
    if kind == Kind::Squid {
        let (s, _) = squid_guest_size(app);
        let g = crate::art::geo_s(app.w, app.h, s);
        (g.hwh * 0.9 * 3.4 + g.maxw * 0.5).max(g.maxw + 2.0)
    } else {
        app.w as f32 * 0.12
    }
}

/// A squid guest's size factor (absolute, not relative to the host's) and height in rows:
/// about 0.8 of the seated dog's height.
fn squid_guest_size(app: &App) -> (f32, i32) {
    let (_, hh) = crate::dog_art::seat_dims(app.w, app.h, app.scale);
    let s = 0.7 * (hh + 1.0) * 0.75 / 0.8 / (app.h as f32 * 0.8);
    let total = ((app.h as f32 * 0.8 * s).round() as i32).max(8);
    (s, total)
}

impl Pet for DogPet {
    fn kind(&self) -> Kind {
        Kind::Dog
    }
    fn name(&self) -> &'static str {
        "dog"
    }
    fn breeds(&self) -> Vec<&'static str> {
        BREEDS.iter().map(|b| b.name).collect()
    }
    fn init(&self, app: &mut App) {
        app.init_dog();
    }
    fn geo(&self, w: u16, h: u16, scale: f32) -> Geo {
        crate::dog_art::geo_dog(w, h, scale)
    }
    fn pick_idle(&self, app: &mut App) -> Behavior {
        app.dog_pick_idle()
    }
    fn start_idle(&self, app: &mut App, b: Behavior) {
        app.dog_start_idle(b);
    }
    fn update(&self, app: &mut App, dt: f32) {
        app.dog_motes(dt);
        // no wander across the yard during an action or right after it: the dog sits a
        // moment first, so e.g. a roll-over does not run on into a walk
        if app.any_react() || app.state != State::Idle {
            app.next_burst = app.next_burst.max(app.t + 2.5);
        }
        app.dog_update(dt);
        // side view while moving, spinning or rolling; then the head turns to the front
        let p = app.dog_pose();
        let moving = p.walking || p.running || p.spin.abs() < 0.999;
        let side = moving || p.roll_pose().is_some_and(|(r, _)| r != RollPose::Sit);
        if side {
            app.dog.turn_up = moving;
        }
        app.dog.sideways = if side {
            1.0
        } else {
            (app.dog.sideways - dt / TURN_T).max(0.0)
        };
        if app.t >= app.dog.next_wag + 0.8 {
            app.dog.next_wag = app.t + app.rng.random_range(3.0..8.0);
        }
        // last, once facing and pose for this frame are settled
        app.dog_keep_clear(dt);
    }
    fn msg_for(&self, k: Key) -> &'static str {
        App::dog_msg(k)
    }
    fn boop_msg(&self) -> &'static str {
        "Arf?"
    }
    fn mote_chars(&self) -> [char; 3] {
        ['.', '*', '\u{b0}']
    }
    fn stream_char(&self) -> char {
        '*'
    }
    fn drag_label(&self) -> &'static str {
        "Scritched"
    }
    fn drag_verb(&self) -> &'static str {
        "scritch"
    }
    fn hints(&self, social: bool) -> &'static str {
        if social {
            "[f] biscuit   [p] pet   [space] ball   [v] visit   [c] breed   [q] quit"
        } else {
            "[f] biscuit   [p] pet   [space] ball   [c] breed   [q] quit"
        }
    }
    fn on_press(&self, app: &mut App, k: Key) {
        let g = app.geo();
        match k {
            Key::Feed => {
                // toss from a random side; alternate catching it and letting it land
                let x = g.cx as f32 + app.dog_dx();
                let side = if app.rng.random_bool(0.5) { 1.0 } else { -1.0 };
                let sx = (x + side * app.w as f32 * 0.42).clamp(2.0, app.w as f32 - 3.0);
                app.dog.toss_flip = !app.dog.toss_flip;
                let mode = if app.dog.toss_flip {
                    TOSS_CATCH
                } else {
                    TOSS_LAND
                };
                app.add_fish((sx, mode));
            }
            Key::Pet => {
                let fresh = !app.pet.on;
                app.bump(k, 3.0);
                if fresh {
                    app.pet.dur = app.pet.dur.max(ROLL_LEN);
                }
                let n = app.rng.random_range(3..=5);
                for _ in 0..n {
                    let x = app.spawn_x() + app.rng.random_range(-g.hwh..=g.hwh);
                    let age = -app.rng.random_range(0.0..0.6);
                    app.hearts.push(Heart {
                        x,
                        y: (g.top + g.mh / 2) as f32,
                        age,
                        life: 2.0,
                    });
                }
            }
            Key::Play => {
                app.bump(k, PLAY_LEN);
                app.dart_from = (app.dog_play_dx(), 0.0);
                app.dart_s = 0.0;
                let lim = (app.w as f32 / 2.0 - g.maxw * 1.3).max(0.0);
                let first = if app.rng.random_bool(0.5) { 1.0 } else { -1.0 };
                let r = app.rng.random_range(0.6..=1.0);
                // the ball lands where the dog can fetch it without covering the doghouse or
                // running out of the pane; the other side when that leaves too short a run
                let mut best = (f32::MIN, first, 0.0);
                for side in [first, -first] {
                    let d = side * lim * r;
                    let d = d + app.ball_shift(d, side);
                    if side * d > best.0 {
                        best = (side * d, side, d);
                    }
                    if side * d >= 0.3 * lim {
                        break;
                    }
                }
                let (_, side, d) = best;
                app.dog.ball_dx = d;
                app.dog.face = side;
                app.dog.bark_t = 0.6;
                app.play.dur = app.play.dur.max(app.play.st + PLAY_LEN);
            }
        }
    }
    fn on_ruffle(&self, app: &mut App, _col: u16, _row: u16) {
        app.set_msg("Brrrr!");
    }
    fn foods(&self, app: &App) -> Vec<(f32, f32)> {
        if !app.feed.on {
            return vec![];
        }
        let body = app.dog_host_body();
        let mouth = crate::dog_art::mouth_pos(app, &body);
        let g = app.geo();
        let feet = (g.top + g.mh + g.hh + g.al) as f32;
        let x = g.cx as f32 + app.offset().0;
        app.fish
            .iter()
            .filter_map(|f| {
                let a = app.fish_age(f);
                if !(0.0..=2.4).contains(&a) {
                    return None;
                }
                let start = (f.from.0, feet - g.maxw * 0.9);
                let land = f.from.1 == TOSS_LAND;
                let (end_t, carry_t) = if land { (0.9, 1.4) } else { (1.1, 1.1) };
                if land {
                    let side = (f.from.0 - x).signum();
                    let mut hb = body.clone();
                    hb.dpose.head_down = 1.0;
                    hb.dpose.look_up = 0.0;
                    hb.dpose.spin = side;
                    let gp = crate::dog_art::mouth_pos(app, &hb);
                    let ground = (gp.0, feet);
                    if a < end_t {
                        let u = a / end_t;
                        Some((
                            start.0 + (ground.0 - start.0) * u,
                            start.1 + (ground.1 - start.1) * u - 8.0 * 4.0 * u * (1.0 - u),
                        ))
                    } else if a < carry_t {
                        Some(ground)
                    } else {
                        Some(mouth)
                    }
                } else if a < end_t {
                    let u = a / end_t;
                    Some((
                        start.0 + (mouth.0 - start.0) * u,
                        start.1 + (mouth.1 - start.1) * u - 8.0 * 4.0 * u * (1.0 - u),
                    ))
                } else {
                    Some(mouth)
                }
            })
            .collect()
    }
    fn arrive_msg(&self, breed: u8) -> Msg {
        Msg::ArriveDog(breed)
    }
    fn throws(&self) -> Throw {
        Throw::Bark
    }
    fn on_mouse(&self, app: &mut App, dir: f32) {
        // chase the toy mouse across the yard
        app.dog.chase = (dir, CHASE_T);
        app.set_msg("Woof woof!");
    }
    fn on_ink(&self, app: &mut App, ox: f32) {
        // inked: a splat on the coat, shaken off
        app.splat = Some((2.0, ox));
        app.note = Some(("Pfft!".to_string(), 2.0));
    }
    fn on_bark(&self, app: &mut App) {
        // another dog barks: tilt the head and bark back
        app.touch.boop = BOOP;
        app.dog.bark_t = 0.6;
    }
    fn offset(&self, app: &App) -> (f32, f32) {
        let jolt = if app.jolt > 0.0 {
            -(app.jolt / 0.4).min(1.0)
        } else {
            0.0
        };
        (app.dog_dx(), jolt + app.fx.dy)
    }
    fn host_body(&self, app: &App) -> Body {
        let mut body = app.dog_host_body();
        // the pose may have moved on since the last update measured it: top up the step
        // away from the doghouse for this frame
        let p = &body.dpose;
        if !(p.seated() || p.inside || p.roll > 0.0 || app.dog.nap_k > 0.0) {
            let (lo, hi) = app.yard_range(&body);
            let c = app.dog.clear_dx;
            body.dx += c.min(hi).max(lo) - c;
        }
        body
    }
    fn guest_fit(&self, app: &App, guest: Kind) -> (f32, f32) {
        let ground = crate::dog_art::ground(app.w, app.h).0;
        match guest {
            // a visiting cat is smaller than the host and stands on the lawn
            Kind::Cat => {
                let (_, hh) = crate::dog_art::seat_dims(app.w, app.h, app.scale);
                let total = ((hh * 0.75).round() as i32).max(6);
                let fit = total as f32 / (app.h as f32 * 0.68 * app.scale).max(1.0);
                (fit, (ground - (3 + total) + 1) as f32)
            }
            // a visiting squid floats over the lawn, about 0.8 of the seated dog's height
            // (as in the cat room) with all its tentacles above the grass
            Kind::Squid => {
                let (s, total) = squid_guest_size(app);
                let fit = s / app.scale.max(0.05);
                (fit, (ground - 4 - total) as f32 - 1.0)
            }
            Kind::Dog => (1.0, 0.0),
        }
    }
    fn guest_drift(&self, app: &App, guest: Kind, x: f32, dx: f32) -> f32 {
        let dog_x = app.geo().cx as f32 + app.host_dx;
        if x >= dog_x {
            return dx;
        }
        // a guest between the doghouse and the dog keeps to its stretch of lawn: never
        // over the house, never into the room the dog runs and fetches in
        let (l, r) = guest_reach(app, guest);
        let (ww, _) = crate::dog_art::seat_dims(app.w, app.h, app.scale);
        let gap = if guest == Kind::Squid { 5.0 } else { 3.0 };
        let lo = (1.0 + l + gap).min(x);
        let hi = (dog_x - SEAT_REACH * ww - r - 1.0).max(x);
        (x + dx).clamp(lo, hi) - x
    }
    fn visit_scale(&self, app: &App, target: f32) -> f32 {
        // the dog and its guests keep their solo size while they fit side by side across
        // the pane; only a crowd shrinks, and never below the engine's `target`
        let k = app.scale.max(0.05);
        let (ww, _) = crate::dog_art::seat_dims(app.w, app.h, 1.0);
        let mut need = 2.0 * DOG_REACH * ww;
        for g in &app.guests {
            need += guest_span(app, g) / k + 2.0;
        }
        let room = app.w as f32 - 2.0;
        (room / need).min(1.0).max(target)
    }
    fn host_x(&self, app: &App, x: f32) -> f32 {
        // keep the seated dog on the pane with room for the guests on each side of it
        let (ww, _) = crate::dog_art::seat_dims(app.w, app.h, app.scale);
        let half = DOG_REACH * ww;
        let (mut left, mut right) = (0.0, 0.0);
        for g in &app.guests {
            let span = guest_span(app, g) + 1.0;
            if g.side < 0.0 {
                left += span;
            } else {
                right += span;
            }
        }
        let lo = 1.0 + left + half;
        let hi = app.w as f32 - 2.0 - right - half;
        if lo <= hi {
            x.clamp(lo, hi)
        } else {
            (lo + hi) / 2.0
        }
    }

    fn stage(&self, app: &App) -> (f32, f32) {
        (1.0, app.w as f32 - 1.0)
    }
    fn guest_x(&self, app: &App, guest: Kind, x: f32) -> f32 {
        // keep the guest (a squid's tentacle tips, a cat's tail included) on the open
        // lawn, clear of the doghouse and of the seated dog
        let (l, r) = guest_reach(app, guest);
        let (ww, _) = crate::dog_art::seat_dims(app.w, app.h, app.scale);
        let dog_x = app.geo().cx as f32 + app.host_dx;
        let dog_half = SEAT_REACH * ww;
        if x < dog_x {
            let (lo, hi) = (1.0 + l, dog_x - dog_half - r - 1.0);
            if lo <= hi {
                // next to the house, leaving the standing dog the most room to play
                lo
            } else {
                (lo + hi) / 2.0
            }
        } else {
            x.max(dog_x + dog_half + l + 1.0)
        }
    }
    fn guest_body(&self, app: &App, g: &Guest) -> Body {
        let cx = app.geo().cx;
        let tt = app.t + g.t0;
        let running = g.startle > 0.0;
        let dir = if g.side < 0.0 { 1.0 } else { -1.0 };
        let face = if running { -g.startle_dir } else { dir };
        let face = if face == 0.0 { dir } else { face };
        let shake: f32 = g
            .splat
            .map_or(0.0, |(l, _)| if l < 1.4 { 0.8 } else { 0.0 });
        let barked = g.bark > 0.0;
        Body {
            kind: Kind::Dog,
            breed: g.color,
            breed_from: g.color,
            bfade: 1.0,
            boop: (g.touch.boop / BOOP).clamp(0.0, 1.0),
            head_dy: g.touch.dy(),
            dir: g.touch.dir,
            dpose: DogPose {
                walking: false,
                running,
                step: tt * 20.0,
                wag: if g.touch.stroke > 0.0 || (tt % 7.0) < 1.5 {
                    0.8
                } else {
                    0.0
                },
                wag_ph: tt * 14.0,
                fast_ph: tt * 30.0,
                shake: shake.max(g.touch.ruffle_k()),
                tilt: if barked {
                    0.8
                } else {
                    (g.touch.boop / BOOP).clamp(0.0, 1.0) * 0.8
                },
                spin: face,
                tongue: running as u8 as f32,
                open: if running { 0.5 } else { 0.0 },
                ..Default::default()
            },
            splat: g.splat,
            cx,
            dx: g.x - cx as f32 + g.fx.dx,
            dy: g.fx.dy,
            scale: app.scale,
            blush: (g.touch.stroke / 0.5).min(1.0),
            bright: g.fx.bright,
            dark: g.fx.dark,
            closed: g.touch.stroke > 0.0,
            blink: tt % 5.0 < 0.15,
            food_x: g.x,
            t: tt,
            touch: ease(g.touch.touch),
            touch_at: g.touch.at,
            ruffle: g.touch.ruffle_k(),
            ruffle_at: g.touch.ruffle_at,
            tag: (g.tag > 0.0).then_some("guest"),
            ..Body::default()
        }
    }
    fn draw_scene(&self, cv: &mut Canvas, app: &App) {
        crate::dog_art::draw_yard(cv, app);
    }
    fn draw_body(&self, cv: &mut Canvas, body: &Body, app: &App, id: u8) {
        crate::dog_art::draw_dog_into(cv, body, app.mono, id, app.w, app.h);
    }
    fn draw_fx(&self, cv: &mut Canvas, app: &App) {
        crate::dog_art::draw_props(cv, app);
    }
    fn treat_glyph(&self) -> &'static str {
        "(o)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pet;

    pub fn dog(w: u16, h: u16, seed: u64) -> App {
        let mut a = App::new(w, h, seed);
        a.init_dog();
        a.init_color(0, false);
        a
    }

    #[test]
    fn eleven_breeds_in_order_and_parse() {
        let names: Vec<_> = BREEDS.iter().map(|b| b.name).collect();
        assert_eq!(
            names,
            [
                "golden",
                "lab",
                "husky",
                "corgi",
                "dalmatian",
                "beagle",
                "poodle",
                "german shepherd",
                "dachshund",
                "bluey",
                "bingo"
            ]
        );
        assert_eq!(pet::parse_breed(&DOG, "Corgi"), Some(Ok(3)));
        assert_eq!(pet::parse_breed(&DOG, "random"), Some(Err(())));
        assert_eq!(pet::parse_breed(&DOG, "sphinx"), None);
        assert!(pet::breed_list(&DOG).starts_with("golden, lab, husky"));
        assert_eq!(pet::random_breed(&DOG, 7), pet::random_breed(&DOG, 7));
    }

    #[test]
    fn c_cycles_through_all_eleven_with_a_message() {
        let mut a = dog(120, 40, 1);
        a.next_color();
        assert_eq!(a.cmsg.as_ref().unwrap().0, "I'm a lab today!");
        a.next_color();
        a.next_color();
        assert_eq!(a.cmsg.as_ref().unwrap().0, "I'm a corgi today!");
        for _ in 0..8 {
            a.next_color();
        }
        assert_eq!(a.color_idx(), 0);
    }

    #[test]
    fn corgi_has_short_legs() {
        assert!(BREEDS[3].leg < 0.6 * BREEDS[0].leg);
        let pals: std::collections::HashSet<_> = BREEDS.iter().map(|b| (b.coat, b.dark)).collect();
        assert_eq!(pals.len(), 11);
    }

    #[test]
    fn keys_produce_their_sequences_and_layer() {
        let mut a = dog(120, 40, 1);
        assert!(a.press(Key::Feed));
        assert_eq!(a.msg, Some("Crunch!"));
        assert_eq!(a.fish.len(), 1);
        assert_eq!(a.foods().len(), 1);
        assert!(a.press(Key::Pet));
        assert!(a.pet.on && !a.hearts.is_empty());
        assert!(a.press(Key::Play));
        assert!(a.play.on && a.feed.on && a.pet.on);
        // repeated presses add biscuits
        a.press(Key::Feed);
        assert_eq!(a.fish.len(), 2);
        for _ in 0..(14 * 16) {
            a.update(1.0 / 16.0);
        }
        assert!(!a.any_react());
        assert!(a.ball_pos().is_none());
    }

    #[test]
    fn biscuit_is_caught_or_picked_off_the_grass() {
        for flip in 0..2 {
            let mut a = dog(120, 40, 1);
            a.dog.toss_flip = flip == 1;
            a.press(Key::Feed);
            let mut carried = false;
            for _ in 0..(2 * 16) {
                a.update(1.0 / 16.0);
                carried |= a.dog_pose().carry == 1;
            }
            assert!(carried, "flip {flip}");
        }
    }

    /// Leftmost drawn column of the dog as it is on screen (clearance push included).
    fn drawn_left(a: &App) -> i32 {
        let body = a.host_body();
        let mut cv = crate::art::Canvas::new(a.w, a.h, false);
        crate::dog_art::draw_dog(&mut cv, &body);
        (0..cv.w)
            .find(|&x| (0..cv.h).any(|y| cv.cells[y * cv.w + x].fg.is_some()))
            .map_or(i32::MAX, |x| x as i32)
    }

    #[test]
    fn standing_dog_may_pass_in_front_of_the_doghouse_but_stays_in_the_pane() {
        let mut left_fetches = 0;
        let mut crossed = 0;
        for (w, h) in [(120u16, 40u16), (200, 50), (90, 30)] {
            for seed in 1..=8u64 {
                let mut a = dog(w, h, seed);
                a.init_color((seed as usize) % 11, false);
                a.pos = -1.0;
                let house = crate::dog_art::house_right(w, h);
                for i in 0..(70 * 16) {
                    if i % (7 * 16) == 3 * 16 {
                        a.dog.chase = (-1.0, CHASE_T);
                    }
                    if i % (7 * 16) == 0 {
                        a.press(if i % (21 * 16) == 0 {
                            Key::Pet
                        } else {
                            Key::Play
                        });
                        if a.dog.ball_dx < 0.0 && a.play.on {
                            left_fetches += 1;
                        }
                    }
                    a.update(1.0 / 16.0);
                    let p = a.dog_host_body().dpose;
                    if p.seated() || p.inside || p.roll > 0.0 || a.dog.nap_k > 0.0 {
                        continue;
                    }
                    let left = drawn_left(&a);
                    crossed += i32::from(left <= house);
                    assert!(
                        left >= 0,
                        "{w}x{h} seed {seed} t {:.2}: dog at col {left}, house ends {house}; clear {} state {:?} pose {:?}",
                        a.t,
                        a.dog.clear_dx,
                        a.state,
                        p
                    );
                }
            }
        }
        assert!(left_fetches > 5, "only {left_fetches} fetches to the left");
        assert!(crossed > 0, "the dog never passed in front of the house");
    }

    #[test]
    fn nap_goes_into_the_doghouse() {
        let mut a = dog(120, 40, 1);
        a.state = State::IdleBehavior(Behavior::Nap);
        a.st = 0.0;
        let door = crate::dog_art::door_x(120, 40);
        for i in 0..60 {
            a.update(1.0 / 16.0);
            let _ = i;
        }
        assert!(a.dog.nap_k >= 1.0);
        assert!((a.geo().cx as f32 + a.offset().0 - door).abs() < 1.0);
        assert!(a.dog_pose().inside);
    }

    #[test]
    fn nap_walk_shrinks_and_feeding_wakes_the_dog_before_it_eats() {
        let mut a = dog(120, 40, 1);
        a.state = State::IdleBehavior(Behavior::Nap);
        a.st = 0.0;
        a.dog_start_idle(Behavior::Nap);
        let mut min_mid = 1.0f32;
        for _ in 0..30 {
            a.update(1.0 / 16.0);
            let sc = a.dog_host_body().scale;
            if a.dog.nap_k > 0.2 && a.dog.nap_k < 0.9 {
                min_mid = min_mid.min(sc);
            }
        }
        assert!(min_mid < 0.9, "no shrink on the way in: {min_mid}");
        for _ in 0..40 {
            a.update(1.0 / 16.0);
        }
        assert!(a.dog_pose().inside);
        assert!((a.dog_host_body().scale - crate::dog_art::NAP_SCALE).abs() < 0.01);
        a.press(Key::Feed);
        let mut grew = false;
        for _ in 0..80 {
            a.update(1.0 / 16.0);
            let sc = a.dog_host_body().scale;
            grew |= a.dog.nap_k > 0.0 && a.dog.nap_k < 1.0 && sc > crate::dog_art::NAP_SCALE;
        }
        assert!(grew, "did not grow on the way out");
        assert_eq!(a.dog.nap_k, 0.0);
        assert!((a.dog_host_body().scale - 1.0).abs() < 0.01);
        assert!(a.stats.fed >= 1, "never ate");
    }

    /// Cells the dog occupies when drawn alone (owner id 1) on an empty pane.
    fn dog_cells(a: &App) -> Vec<(i32, i32)> {
        let mut cv = crate::art::Canvas::new(a.w, a.h, false);
        crate::dog_art::draw_dog_into(&mut cv, &a.host_body(), false, 1, a.w, a.h);
        let mut out = vec![];
        for y in 0..cv.h {
            for x in 0..cv.w {
                if cv.owner[y * cv.w + x] == 1 {
                    out.push((x as i32, y as i32));
                }
            }
        }
        out
    }

    /// (rows, columns, top row, bottom row) of the cells drawn with owner id 1.
    fn extent(cells: &[(i32, i32)]) -> (i32, i32, i32, i32) {
        let top = cells.iter().map(|c| c.1).min().unwrap();
        let bottom = cells.iter().map(|c| c.1).max().unwrap();
        let l = cells.iter().map(|c| c.0).min().unwrap();
        let r = cells.iter().map(|c| c.0).max().unwrap();
        (bottom - top + 1, r - l + 1, top, bottom)
    }

    #[test]
    fn seated_dog_is_at_least_the_cat_and_level_with_it() {
        for (w, h) in [
            (65u16, 23u16),
            (86, 22),
            (60, 28),
            (80, 24),
            (120, 40),
            (122, 56),
        ] {
            let a = seated_dog(w, h, 0);
            let d = extent(&dog_cells(&a));
            let mut c = App::new(w, h, 1);
            c.init_cat();
            c.init_color(0, false);
            let mut cv = crate::art::Canvas::new(w, h, false);
            crate::cat_art::draw_cat_into(&mut cv, &c.host_body(), false, 1, w, h);
            let mut cells = vec![];
            for y in 0..cv.h {
                for x in 0..cv.w {
                    if cv.owner[y * cv.w + x] == 1 {
                        cells.push((x as i32, y as i32));
                    }
                }
            }
            let k = extent(&cells);
            eprintln!(
                "{w}x{h}: dog {d:?} cat {k:?} seat {:?}",
                crate::dog_art::seat_dims(w, h, 1.0)
            );
            // heights match the cat's; widths are the seated body (`seat_dims`), compared with
            // the cat body widths the spec measured (whiskers and tail left out)
            assert!(d.0 + 1 >= k.0, "{w}x{h}: dog {d:?} cat {k:?}");
            let (ww, _) = crate::dog_art::seat_dims(w, h, 1.0);
            let min_w = match (w, h) {
                (86, 22) => 24.0,
                (60, 28) => 27.0,
                (122, 56) => 44.0,
                _ => 0.0,
            };
            assert!(ww >= min_w - 0.5, "{w}x{h}: seated width {ww} < {min_w}");
            assert!(
                (d.2 - k.2).abs() <= 1 && (d.3 - k.3).abs() <= 1,
                "{w}x{h}: not level dog {d:?} cat {k:?}"
            );
        }
    }

    fn seated_dog(w: u16, h: u16, breed: usize) -> App {
        let mut a = dog(w, h, 1);
        a.init_color(breed, false);
        for _ in 0..40 {
            a.update(0.05);
        }
        assert!(a.dog_pose().seated());
        a
    }

    const YARD_SIZES: [(u16, u16); 4] = [(60, 20), (80, 24), (120, 40), (200, 60)];

    #[test]
    fn yard_props_scale_with_the_seated_dog_and_never_touch_it() {
        use crate::dog_art as da;
        for (w, h) in YARD_SIZES {
            let (ww, hh) = da::seat_dims(w, h, 1.0);
            let (feet, _) = da::ground(w, h);
            let (_, dw, dh) = da::door(w, h);
            let (tx, tw, ccy, rv, ru) = da::tree(w, h);
            let (ft, fb) = da::fence(w, h);
            for b in 0..BREEDS.len() {
                let a = seated_dog(w, h, b);
                let cells = dog_cells(&a);
                let top = cells.iter().map(|c| c.1).min().unwrap();
                let left = cells.iter().map(|c| c.0).min().unwrap();
                let right = cells.iter().map(|c| c.0).max().unwrap();
                // R1: the house never overlaps the seated dog
                assert!(left > da::house_right(w, h), "{w}x{h} b{b}: house {left}");
                // R3: crown top above the head, crown clear of the dog and the house
                assert!(
                    ccy - rv <= top as f32 + 1.0,
                    "{w}x{h} b{b}: crown {} vs head {top}",
                    ccy - rv
                );
                assert!(
                    (tx as f32 - ru) > right as f32,
                    "{w}x{h} b{b}: tree {right}"
                );
                assert!((tx as f32 - ru) as i32 > da::house_right(w, h));
            }
            // house: a small prop, about 0.6 of the seated height, shorter than the dog
            let (_, hwid, wall, roof) = da::house(w, h);
            assert!((wall + roof) as f32 <= hh, "{w}x{h}: house {}", wall + roof);
            assert!(hwid <= ww as i32 + 3, "{w}x{h}: house width {hwid}");
            assert!(
                dw as f32 <= ww * 0.85 && dh < wall,
                "{w}x{h}: door {dw}x{dh}"
            );
            // crown about 0.8x the seated height, trunk in proportion
            assert!((2.0 * rv / hh - 0.82).abs() < 0.05);
            assert!(tw >= 3);
            // fence about 0.3 of the seated dog, on the horizon behind the feet
            let fh = (fb - ft + 1) as f32;
            assert!((fh - 0.3 * hh).abs() <= 1.0, "{w}x{h}: fence {fh}");
            assert!(fb < feet);
        }
        let (ft, fb) = da::fence(120, 40);
        assert!((7..=9).contains(&(fb - ft + 1)));
    }

    #[test]
    fn yard_sun_is_four_rows_and_props_stay_off_the_footer() {
        for (w, h) in YARD_SIZES {
            let a = seated_dog(w, h, 0);
            let mut cv = crate::art::Canvas::new(w, h, false);
            crate::dog_art::draw_yard(&mut cv, &a);
            let sun = |c: &crate::art::Cell| c.fg == Some(crate::dog_art::SUN);
            let rows = (0..cv.h)
                .filter(|&y| (0..cv.w).any(|x| sun(&cv.cells[y * cv.w + x])))
                .count();
            if (w, h) == (120, 40) {
                let disc = (0..cv.h)
                    .filter(|&y| {
                        (0..cv.w).any(|x| {
                            let c = cv.cells[y * cv.w + x];
                            sun(&c) && (c.ch == '@' || c.ch == '\u{2588}')
                        })
                    })
                    .count();
                assert!((3..=5).contains(&disc), "sun disc {disc} rows");
            }
            assert!(rows >= 3, "{w}x{h}: sun {rows} rows");
            // the footer row stays empty
            let y = cv.h - 1;
            assert!((0..cv.w).all(|x| cv.cells[y * cv.w + x].fg.is_none()));
        }
    }

    #[test]
    fn asleep_in_the_doghouse_the_door_frames_the_dog() {
        use crate::dog_art as da;
        for (w, h) in YARD_SIZES {
            let mut a = dog(w, h, 1);
            a.state = State::IdleBehavior(Behavior::Nap);
            a.st = 0.0;
            a.dog_start_idle(Behavior::Nap);
            for _ in 0..(4.0 * 16.0) as usize {
                a.update(1.0 / 16.0);
            }
            assert!(a.dog_pose().inside, "{w}x{h}");
            // only the head shows, inside the door, no lower than its floor row
            let base = da::horizon(w, h) - 1;
            let cells = dog_cells(&a);
            assert!(cells.len() > 10, "{w}x{h}: head missing");
            let (_, hd) = da::seat_dims(w, h, da::NAP_SCALE);
            for c in &cells {
                assert!(
                    da::in_door(w, h, c.0, c.1),
                    "{w}x{h}: ({},{}) outside the door",
                    c.0,
                    c.1
                );
                assert!(c.1 <= base);
            }
            let top = cells.iter().map(|c| c.1).min().unwrap();
            let bot = cells.iter().map(|c| c.1).max().unwrap();
            assert!(
                bot >= base - 1,
                "{w}x{h}: chin not on the floor ({bot} vs {base})"
            );
            assert!(
                ((bot - top + 1) as f32) < 0.8 * hd,
                "{w}x{h}: more than a head"
            );
        }
    }

    /// Dev helper: `SNAP_DIR=/tmp/x cargo test snap_poses` writes ANSI frames of each idle pose.
    #[test]
    fn snap_poses() {
        let Ok(dir) = std::env::var("SNAP_DIR") else {
            return;
        };
        let sz = |k: &str, d: u16| {
            std::env::var(k)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(d)
        };
        let (sw, sh) = (sz("SNAP_W", 120), sz("SNAP_H", 40));
        let ansi = |a: &App, name: &str| {
            let cv = crate::art::render(a, false);
            let mut out = String::new();
            for y in 0..cv.h {
                for x in 0..cv.w {
                    let c = cv.cells[y * cv.w + x];
                    match c.fg {
                        Some((r, g, b)) => out.push_str(&format!("\x1b[38;2;{r};{g};{b}m{}", c.ch)),
                        None => out.push_str(&format!("\x1b[0m{}", c.ch)),
                    }
                }
                out.push_str("\x1b[0m\n");
            }
            std::fs::write(format!("{dir}/{name}.ans"), out).unwrap();
        };
        // walking into the doghouse and back out
        let d = App::idle_duration(Behavior::Nap);
        for (i, secs) in [0.3, 0.6, 0.9, 1.2, d - 1.2, d - 0.9, d - 0.6, d - 0.3]
            .into_iter()
            .enumerate()
        {
            let mut a = dog(sw, sh, 11);
            a.state = State::IdleBehavior(Behavior::Nap);
            a.st = 0.0;
            a.dog_start_idle(Behavior::Nap);
            for _ in 0..(secs * 16.0) as usize {
                a.update(1.0 / 16.0);
            }
            ansi(&a, &format!("nap_walk{i}"));
        }
        for (n, b, secs) in [
            ("nap", Behavior::Nap, 4.0),
            ("dig", Behavior::Dig, 2.0),
            ("sniff", Behavior::Sniff, 1.5),
            ("ear", Behavior::EarScratch, 1.1),
            ("spin", Behavior::TailSpin, 1.2),
            ("bark", Behavior::Bark, 0.6),
        ] {
            let mut a = dog(sw, sh, 11);
            a.state = State::IdleBehavior(b);
            a.st = 0.0;
            a.dog_start_idle(b);
            for _ in 0..(secs * 16.0) as usize {
                a.update(1.0 / 16.0);
            }
            ansi(&a, n);
        }
    }

    /// Dev helper: `SNAP_DIR=/tmp/x cargo test snap_roll` writes ANSI frames of every roll
    /// pose (the real render at the pose's time) and seated / standing / walking stills.
    #[test]
    fn snap_roll() {
        let Ok(dir) = std::env::var("SNAP_DIR") else {
            return;
        };
        let breed: usize = std::env::var("SNAP_BREED")
            .ok()
            .and_then(|b| b.parse().ok())
            .unwrap_or(0);
        let (w, h) = std::env::var("SNAP_SIZE")
            .ok()
            .and_then(|s| {
                let (a, b) = s.split_once('x')?;
                Some((a.parse().ok()?, b.parse().ok()?))
            })
            .unwrap_or((120u16, 40u16));
        let save = |cv: &crate::art::Canvas, name: &str| {
            let mut out = String::new();
            for y in 0..cv.h {
                for x in 0..cv.w {
                    let c = cv.cells[y * cv.w + x];
                    match c.fg {
                        Some((r, g, b)) => out.push_str(&format!("\x1b[38;2;{r};{g};{b}m{}", c.ch)),
                        None => out.push_str(&format!("\x1b[0m{}", c.ch)),
                    }
                }
                out.push_str("\x1b[0m\n");
            }
            std::fs::write(format!("{dir}/{name}.ans"), out).unwrap();
        };
        let fresh = || {
            let mut a = dog(w, h, 5);
            a.init_color(breed, false);
            a.state = State::Idle;
            a.st = 0.0;
            a.dog.next_wag = 1e9;
            // no drift across the yard, so the frames line up
            a.burst.2 = 99.0;
            a.next_burst = 1e9;
            a
        };
        let mut a = fresh();
        a.press(Key::Pet);
        let mut t = 0.0;
        let names = [
            "roll1_down",
            "roll2_side",
            "roll3_twist",
            "roll4_back",
            "roll5_twist",
            "roll6_side",
            "roll7_stand",
            "roll8_sit",
        ];
        let mut start = 0.0;
        for (name, d) in names.into_iter().zip(ROLL_TIMES) {
            let at = start + 0.6 * d;
            start += d;
            while t < at {
                a.update(1.0 / 16.0);
                a.state = State::Idle;
                t += 1.0 / 16.0;
            }
            assert!(a.dog_pose().roll_pose().is_some(), "{name} not rolling");
            save(&crate::art::render(&a, false), name);
        }
        // stills of one body in three poses, drawn on the yard
        let a = fresh();
        let still = |name: &str, f: &dyn Fn(&mut DogPose)| {
            let mut body = a.dog_host_body();
            body.blink = false;
            body.closed = false;
            f(&mut body.dpose);
            let mut cv = crate::art::Canvas::new(w, h, false);
            crate::dog_art::draw_yard(&mut cv, &a);
            crate::dog_art::draw_dog_into(&mut cv, &body, false, 0, w, h);
            save(&cv, name);
        };
        still("pose_seated", &|_| {});
        still("pose_stand", &|p| p.up = true);
        still("pose_walk", &|p| {
            p.walking = true;
            p.step = 1.0;
            p.happy_on_the_move();
        });
        still("pose_run", &|p| {
            p.running = true;
            p.step = 2.5;
            p.happy_on_the_move();
        });
        still("pose_sniff", &|p| {
            p.head_down = 1.0;
            p.up = true;
        });
    }

    /// Standing is about as tall as sitting (within 10%) in every breed, measured on the
    /// drawn cells at 120x40.
    #[test]
    fn roll_never_runs_on_into_a_walk() {
        // a press just before the first wander burst (t = 6 s) used to end in a walk
        for at in [3.0f32, 4.0, 5.0, 5.9] {
            let mut a = dog(120, 40, 5);
            a.state = State::Idle;
            while a.t < at {
                a.update(1.0 / 16.0);
                a.state = State::Idle;
            }
            a.press(Key::Pet);
            let mut seated_since = None;
            for _ in 0..(10 * 16) {
                a.update(1.0 / 16.0);
                a.state = State::Idle;
                let p = a.dog_pose();
                if a.any_react() {
                    continue;
                }
                let since = *seated_since.get_or_insert(a.t);
                if a.t - since < 2.0 {
                    assert!(
                        p.seated() && a.burst.2 >= BURST,
                        "press at {at}: the dog got up {:.2} s after the roll",
                        a.t - since
                    );
                }
            }
        }
    }

    #[test]
    fn standing_matches_seated_size() {
        let extent = |cv: &crate::art::Canvas| {
            let (mut y0, mut y1, mut x0, mut x1) = (usize::MAX, 0, usize::MAX, 0);
            for y in 0..cv.h {
                for x in 0..cv.w {
                    if cv.cells[y * cv.w + x].ch != ' ' {
                        (y0, y1, x0, x1) = (y0.min(y), y1.max(y), x0.min(x), x1.max(x));
                    }
                }
            }
            (y1 + 1 - y0, x1 + 1 - x0)
        };
        for (breed, br) in BREEDS.iter().enumerate() {
            let mut a = dog(120, 40, 5);
            a.init_color(breed, false);
            let draw = |f: &dyn Fn(&mut DogPose)| {
                let mut body = a.dog_host_body();
                f(&mut body.dpose);
                let mut cv = crate::art::Canvas::new(120, 40, false);
                crate::dog_art::draw_dog_into(&mut cv, &body, false, 0, 120, 40);
                extent(&cv)
            };
            let (sit_h, _) = draw(&|_| {});
            let (stand_h, _) = draw(&|p| p.up = true);
            let ratio = stand_h as f32 / sit_h as f32;
            eprintln!("{}: seated {sit_h} standing {stand_h}", br.name);
            assert!(
                (0.9..=1.1).contains(&ratio),
                "{}: seated {sit_h} rows, standing {stand_h}",
                br.name
            );
        }
    }

    use crate::link::Msg;

    fn hello(a: &mut App, from: u32, away: bool) {
        a.on_msg(from, Msg::Hello { away, accept: true });
    }

    /// A squid guest in the yard after `secs` seconds (rendered for real).
    fn squid_guest(w: u16, h: u16, secs: f32) -> App {
        guest_of(w, h, secs, Msg::Arrive(0))
    }

    /// A guest that arrived with `arrive` in the yard after `secs` seconds.
    fn guest_of(w: u16, h: u16, secs: f32, arrive: Msg) -> App {
        let mut a = dog(w, h, 5);
        a.enable_social(1);
        hello(&mut a, 2, false);
        a.on_msg(2, arrive);
        a.state = State::Idle;
        a.dog.next_wag = 1e9;
        for _ in 0..(secs * 16.0) as usize {
            a.update(1.0 / 16.0);
            a.state = State::Idle;
            hello(&mut a, 2, false);
        }
        a
    }

    #[test]
    fn a_cat_visiting_at_split_size_shrinks_nobody() {
        // the user's split panes: the dog keeps its solo size, the cat is about 3/4 of it
        for (w, h) in [(65u16, 23u16), (86, 22)] {
            let a = guest_of(w, h, 6.0, Msg::ArriveCat(0));
            assert_eq!(a.guests.len(), 1);
            assert!(a.scale > 0.99, "{w}x{h}: visit scale {}", a.scale);
            let (fit, _) = a.guest_fit(Kind::Cat);
            let (_, dog_h) = crate::dog_art::seat_dims(w, h, a.scale);
            let cat_h = h as f32 * 0.68 * a.scale * fit;
            let k = cat_h / dog_h;
            assert!((0.68..=0.82).contains(&k), "{w}x{h}: guest/dog height {k}");
        }
    }

    #[test]
    fn seated_dog_is_level_with_the_cat() {
        // same-size panes: the dog's drawn top and paw rows match the cat's (top row 3,
        // paws at 3 + 68% of the rows)
        for (w, h) in [(65u16, 23u16), (86, 22), (60, 28), (80, 40), (122, 56)] {
            let a = dog(w, h, 5);
            let mut cv = crate::art::Canvas::new(w, h, false);
            crate::dog_art::draw_dog(&mut cv, &a.host_body());
            let row = |y: usize| (0..cv.w).any(|x| cv.cells[y * cv.w + x].fg.is_some());
            let top = (0..cv.h).find(|&y| row(y)).unwrap() as i32;
            let bottom = (0..cv.h).rev().find(|&y| row(y)).unwrap() as i32;
            let cat = crate::cat_art::geo_cat(w, h, 1.0);
            let cat_bottom = cat.top + cat.mh + cat.hh + cat.al - 1;
            assert!(
                (top - cat.top).abs() <= 1,
                "{w}x{h}: dog top {top} cat {}",
                cat.top
            );
            assert!(
                (bottom - cat_bottom).abs() <= 1,
                "{w}x{h}: dog bottom {bottom} cat {cat_bottom}"
            );
        }
    }

    #[test]
    fn a_visitor_sits_beside_the_dog() {
        let mut checked = 0;
        for (w, h) in [(80u16, 24u16), (120, 40), (200, 50)] {
            for arrive in [Msg::Arrive(0), Msg::ArriveCat(0)] {
                let mut a = guest_of(w, h, 6.0, arrive);
                for step in 0..12 {
                    for _ in 0..8 {
                        a.update(1.0 / 16.0);
                        a.state = State::Idle;
                        hello(&mut a, 2, false);
                    }
                    assert_eq!(a.guests.len(), 1);
                    // the resting layout (a play move slides both pets for a moment)
                    if a.fx.dx != 0.0 || a.guests[0].fx.dx != 0.0 {
                        continue;
                    }
                    checked += 1;
                    let cv = crate::art::render(&a, false);
                    let span = |id: u8| {
                        let xs: Vec<i32> = (0..cv.h * cv.w)
                            .filter(|&i| cv.owner[i] == id)
                            .map(|i| (i % cv.w) as i32)
                            .collect();
                        (*xs.iter().min().unwrap(), *xs.iter().max().unwrap())
                    };
                    let bodies = a.bodies();
                    let host = bodies.iter().position(|b| b.kind == Kind::Dog).unwrap() as u8;
                    let (gl, gr) = span(1 - host);
                    let (dl, dr) = span(host);
                    let tag =
                        format!("{w}x{h} {arrive:?} step {step}: guest {gl}..{gr} dog {dl}..{dr}");
                    assert!(gr < dl, "{tag}: guest not beside the dog");
                }
            }
        }
        assert!(checked > 30, "only {checked} resting frames");
    }

    #[test]
    fn fetch_with_a_guest_stays_in_the_yard() {
        let mut guest_frames = 0;
        let arrivals = [Msg::Arrive(0), Msg::ArriveCat(0)];
        for ((w, h), arrive) in [(120u16, 40u16), (200, 50)]
            .into_iter()
            .flat_map(|s| arrivals.iter().map(move |m| (s, *m)))
        {
            let mut a = guest_of(w, h, 2.0, arrive);
            let house = crate::dog_art::house_right(w, h);
            for throw in 0..12 {
                a.press(Key::Play);
                for _ in 0..(9 * 16) {
                    a.update(1.0 / 16.0);
                    if a.guests.is_empty() {
                        a.on_msg(2, arrive);
                    }
                    hello(&mut a, 2, false);
                    let body = a.host_body();
                    let mut cv = crate::art::Canvas::new(w, h, false);
                    crate::dog_art::draw_dog(&mut cv, &body);
                    let col = |x: usize| (0..cv.h).any(|y| cv.cells[y * cv.w + x].fg.is_some());
                    let left = (0..cv.w).find(|&x| col(x)).unwrap_or(cv.w) as i32;
                    let right = (0..cv.w).rev().find(|&x| col(x)).unwrap_or(0);
                    let standing = !body.dpose.seated() && body.dpose.roll == 0.0;
                    assert!(
                        !standing || right + 1 < cv.w,
                        "{w}x{h} guests {:?} throw {throw} t {:.2}: dog runs off the right edge; clear {} ball {} dx {} pose {:?}",
                        (
                            a.guests
                                .iter()
                                .map(|g| (g.x, g.phase, g.startle))
                                .collect::<Vec<_>>(),
                            a.dog.guest_half,
                            crate::dog_art::house_right(w, h)
                        ),
                        a.t,
                        a.dog.clear_dx,
                        a.dog.ball_dx,
                        body.dx,
                        body.dpose
                    );
                    if !body.dpose.seated() && body.dpose.roll == 0.0 {
                        assert!(
                            left > house + 1,
                            "{w}x{h} throw {throw} t {:.2}: dog at col {left} clear {} ball {} pose {:?}",
                            a.t,
                            a.dog.clear_dx,
                            a.dog.ball_dx,
                            body.dpose
                        );
                        if let Some(g) = a.guests.first() {
                            guest_frames += 1;
                            let sq = guest_drawn_x(g) + guest_reach(&a, g.kind).1;
                            assert!(
                                left as f32 > sq,
                                "{w}x{h} throw {throw} t {:.2}: dog at col {left} over the guest (ends {sq}) {:?} x {} fx {} half {} host {} scale {}",
                                a.t,
                                g.kind,
                                g.x,
                                g.fx.dx,
                                guest_reach(&a, g.kind).1,
                                a.host_dx,
                                a.scale
                            );
                        }
                    }
                }
            }
        }
        assert!(
            guest_frames > 1000,
            "guest present in only {guest_frames} standing frames"
        );
    }

    /// Dev helper: `SNAP_DIR=/tmp/x cargo test snap_guest` writes the dog hosting a squid.
    #[test]
    fn snap_guest() {
        let Ok(dir) = std::env::var("SNAP_DIR") else {
            return;
        };
        for (w, h) in [(120u16, 40u16), (200, 50)] {
            let a = squid_guest(w, h, 4.0);
            let cv = crate::art::render(&a, false);
            let mut out = String::new();
            for y in 0..cv.h {
                for x in 0..cv.w {
                    let c = cv.cells[y * cv.w + x];
                    match c.fg {
                        Some((r, g, b)) => out.push_str(&format!("\x1b[38;2;{r};{g};{b}m{}", c.ch)),
                        None => out.push_str(&format!("\x1b[0m{}", c.ch)),
                    }
                }
                out.push_str("\x1b[0m\n");
            }
            std::fs::write(format!("{dir}/guest_{w}x{h}.ans"), out).unwrap();
        }
    }

    /// Dev helper: `SNAP_DIR=/tmp/x cargo test snap_social` writes the dog chasing a toy
    /// mouse and fetching (barking) with a squid or a cat guest, a few frames each.
    #[test]
    fn snap_social() {
        let Ok(dir) = std::env::var("SNAP_DIR") else {
            return;
        };
        let save = |a: &App, name: &str| {
            let cv = crate::art::render(a, false);
            let mut out = String::new();
            for y in 0..cv.h {
                for x in 0..cv.w {
                    let c = cv.cells[y * cv.w + x];
                    match c.fg {
                        Some((r, g, b)) => out.push_str(&format!("\x1b[38;2;{r};{g};{b}m{}", c.ch)),
                        None => out.push_str(&format!("\x1b[0m{}", c.ch)),
                    }
                }
                out.push_str("\x1b[0m\n");
            }
            std::fs::write(format!("{dir}/{name}.ans"), out).unwrap();
        };
        let mut a = dog(120, 40, 1);
        a.enable_social(1);
        a.on_msg(2, Msg::Mouse);
        for i in 0..40 {
            for _ in 0..3 {
                a.update(1.0 / 16.0);
            }
            save(&a, &format!("mouse{i:02}"));
        }
        for (name, arrive) in [("squid", Msg::Arrive(0)), ("cat", Msg::ArriveCat(2))] {
            let mut a = guest_of(120, 40, 8.0, arrive);
            a.press(Key::Play);
            for i in 0..16 {
                for _ in 0..3 {
                    a.update(1.0 / 16.0);
                    hello(&mut a, 2, false);
                }
                save(&a, &format!("bark_{name}{i:02}"));
            }
        }
    }

    #[test]
    fn dog_space_with_neighbor_sends_bark_not_ink() {
        let mut a = dog(120, 40, 1);
        a.enable_social(1);
        hello(&mut a, 2, false);
        let mut barks = 0;
        for _ in 0..30 {
            a.press(Key::Play);
            for _ in 0..3 {
                a.update(0.0625);
                hello(&mut a, 2, false);
            }
            for (_, m) in a.outbox.drain(..) {
                assert!(!matches!(m, Msg::Ink | Msg::Mouse));
                barks += (m == Msg::Bark) as u32;
            }
        }
        assert!((1..=3).contains(&barks), "barks {barks}");
    }

    #[test]
    fn cat_arches_at_bark() {
        let mut c = App::new(120, 40, 1);
        c.init_cat();
        c.init_color(0, false);
        c.enable_social(1);
        c.on_msg(2, Msg::Bark);
        let mut seen = false;
        for _ in 0..60 {
            c.update(0.0625);
            seen |= c.note.as_ref().is_some_and(|n| n.0 == "Hsss!");
        }
        assert!(seen);
    }

    #[test]
    fn squid_jets_off_at_bark_and_returns() {
        let mut s = App::new(120, 40, 2);
        s.enable_social(2);
        s.on_msg(1, Msg::Bark);
        let mut far = 0.0f32;
        for _ in 0..200 {
            s.update(0.0625);
            far = far.max(s.offset().0.abs());
        }
        assert!(far > 10.0, "far {far}");
        assert!(s.scare == 0.0);
    }

    #[test]
    fn dog_chases_mouse_and_shakes_off_ink() {
        let mut a = dog(120, 40, 1);
        a.enable_social(1);
        a.on_msg(2, Msg::Mouse);
        let mut chased = false;
        for _ in 0..80 {
            a.update(0.0625);
            chased |= a.dog.chase.1 > 0.0;
        }
        assert!(chased);
        a.on_msg(2, Msg::Ink);
        let (mut splat, mut shake, mut gone) = (false, false, false);
        for _ in 0..120 {
            a.update(0.0625);
            let had = splat;
            splat |= a.splat.is_some();
            shake |= a.dog_pose().shake > 0.0;
            gone |= had && a.splat.is_none();
        }
        assert!(splat && shake && gone, "{splat} {shake} {gone}");
    }

    #[test]
    fn dog_guest_arrives_by_wire() {
        let mut h = App::new(120, 40, 1);
        h.enable_social(1);
        h.on_msg(2, Msg::ArriveDog(3));
        assert!(h.guests.iter().any(|g| g.kind == Kind::Dog));
    }

    /// Every non-seated frame is the seated chibi dog: the pasted head has the seated head's
    /// rows and columns and shows the seated eyes (white shine) — two facing the viewer, one
    /// in profile; the profile body is as wide as the seated body; the lying dog is about as
    /// long, nose to rump, as the seated dog is tall (a row looks two columns tall); and the
    /// legs are as many columns thick as the seated front leg.
    #[test]
    fn walk_eye_keeps_its_shine_and_the_head_turns_gradually() {
        let (w, h) = (120u16, 40u16);
        let shine = |body: &Body| {
            let mut cv = crate::art::Canvas::new(w, h, false);
            crate::dog_art::draw_dog_into(&mut cv, body, false, 0, w, h);
            cv.cells
                .iter()
                .filter(|c| c.ch == '\u{25cf}' && c.fg == Some((255, 255, 255)))
                .count()
        };
        for breed in 0..BREEDS.len() {
            let mut a = dog(w, h, 5);
            a.init_color(breed, false);
            let base = a.dog_host_body();
            for k in 0..8 {
                let mut b = base.clone();
                b.blink = true;
                b.dpose.walking = k % 2 == 0;
                b.dpose.running = k % 2 == 1;
                b.dpose.step = k as f32 * 0.4;
                assert_eq!(
                    shine(&b),
                    1,
                    "breed {breed}: walk frame {k} lost the eye shine"
                );
            }
        }
        // after a fetch run the head turns over several frames, not in one swap
        let mut a = dog(w, h, 5);
        a.state = State::Idle;
        a.dog.sideways = 1.0;
        a.dog.turn_up = true;
        let mut turns = vec![];
        for _ in 0..16 {
            a.update(1.0 / 16.0);
            a.state = State::Idle;
            turns.push(a.dog_pose().turn);
        }
        let mid = turns.iter().filter(|&&t| t > 0.2 && t < 0.9).count();
        assert!(mid >= 2, "turn frames {turns:?}");
        assert!(turns.windows(2).all(|w| w[1] >= w[0]), "{turns:?}");
        // the body turns with the head: mid-turn it is narrower than the side view
        let span = |t: f32| {
            let mut b = dog(w, h, 5).dog_host_body();
            b.dpose.up = true;
            b.dpose.turn = t;
            let mut cv = crate::art::Canvas::new(w, h, false);
            crate::dog_art::draw_dog(&mut cv, &b);
            let xs: Vec<usize> = (0..cv.cells.len())
                .filter(|&i| cv.cells[i].fg.is_some())
                .map(|i| i % cv.w)
                .collect();
            xs.iter().max().unwrap() - xs.iter().min().unwrap()
        };
        assert!(span(0.5) < span(0.0), "{} {}", span(0.5), span(0.0));
    }

    #[test]
    fn every_frame_is_the_seated_dog() {
        use crate::dog_art::{PASTED, leg_frac, seat_dims, seated_head};
        let (w, h) = (120u16, 40u16);
        let (ww, hh) = seat_dims(w, h, 1.0);
        let feet = crate::dog_art::ground(w, h).0;
        let bbox = |c: &[(i32, i32, crate::art::Cell)]| {
            let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
            for &(x, y, _) in c {
                (x0, x1, y0, y1) = (x0.min(x), x1.max(x), y0.min(y), y1.max(y));
            }
            (y1 - y0 + 1, x1 - x0 + 1, x1)
        };
        let run = |cv: &crate::art::Canvas, y: i32, x: i32| {
            let on = |x: i32| {
                x >= 0
                    && (x as usize) < cv.w
                    && cv.cells[y as usize * cv.w + x as usize].fg.is_some()
            };
            let (mut a, mut b) = (x, x);
            while on(a - 1) {
                a -= 1;
            }
            while on(b + 1) {
                b += 1;
            }
            (a, b)
        };
        // the seated leg is measured on the plain golden coat (spots and patches hide it);
        // its width does not depend on the breed
        let mut sit_leg = 0;
        for (breed, br) in BREEDS.iter().enumerate() {
            let mut a = dog(w, h, 5);
            a.init_color(breed, false);
            let base = a.dog_host_body();
            let cx = base.cx + base.dx.round() as i32;
            let render = |f: &dyn Fn(&mut DogPose)| {
                let mut body = base.clone();
                f(&mut body.dpose);
                body.blink = false;
                body.closed = false;
                PASTED.with(|p| p.borrow_mut().clear());
                let mut cv = crate::art::Canvas::new(w, h, false);
                crate::dog_art::draw_dog_into(&mut cv, &body, false, 0, w, h);
                let pasted = PASTED.with(|p| p.borrow().clone());
                (cv, pasted, body)
            };
            let shines = |cv: &crate::art::Canvas| {
                cv.cells
                    .iter()
                    .filter(|c| c.ch == '\u{25cf}' && c.fg == Some((255, 255, 255)))
                    .count()
            };
            let (front_rows, front_cols, _) = bbox(&seated_head(&base, w, h, false));
            let (side_rows, side_cols, _) = bbox(&seated_head(&base, w, h, true));
            // the profile snout pokes 1-2 columns past the round head (half-width hru)
            let hru = (0.42 * ww).max(4.0);
            let tip = seated_head(&base, w, h, true)
                .iter()
                .filter(|c| c.2.ch != ')')
                .map(|c| c.0)
                .max()
                .unwrap();
            let past = tip - hru.floor() as i32;
            assert!(
                (1..=2).contains(&past),
                "{}: snout {past} columns past the head",
                br.name
            );
            // the seated dog itself: its width across the haunches, its front leg
            let (sit, _, _) = render(&|_| {});
            assert_eq!(shines(&sit), 2, "{}: seated eyes", br.name);
            let hy = (feet - 1 - (0.2 * hh).round() as i32) as usize;
            let l = (0..sit.w)
                .find(|&x| sit.cells[hy * sit.w + x].fg.is_some())
                .unwrap() as i32;
            let sit_w = 2 * (cx - l) + 1;
            // the seated front leg (raw, before the soft edge): on a row inside the torso
            // its cells are the one leg colour (fur speckles aside), unlike the torso around it
            let ls = (0.14 * ww).max(2.3).round() as i32;
            let (pa, pb) = run(&sit, feet - 1, cx - ls);
            let sit_paw = pb - pa + 1;
            let mut raw_sit = crate::art::Canvas::new(w, h, false);
            crate::dog_art::draw_dog(&mut raw_sit, &base);
            let ly = (feet - 1 - (0.36 * hh).round() as i32) as usize;
            let at = |x: i32| raw_sit.cells[ly * raw_sit.w + x as usize];
            let lc = at(cx - ls).fg;
            let leg_cols: Vec<i32> = ((cx - ls - 4)..=(cx - ls + 4))
                .filter(|&x| {
                    at(x).fg == lc
                        || at(x).ch == '\u{2593}' && at(x - 1).fg == lc && at(x + 1).fg == lc
                })
                .collect();
            if breed == 0 {
                sit_leg = leg_cols.len() as i32;
                assert!(
                    leg_cols.windows(2).all(|p| p[1] == p[0] + 1) && sit_leg >= 1,
                    "seated leg not one run: {leg_cols:?}"
                );
            }
            type Pose<'a> = (&'a str, Box<dyn Fn(&mut DogPose)>, bool);
            let poses: Vec<Pose> = vec![
                ("stand", Box::new(|p| p.up = true), false),
                (
                    "walk",
                    Box::new(|p| {
                        p.walking = true;
                        p.happy_on_the_move();
                    }),
                    true,
                ),
                (
                    "run",
                    Box::new(|p| {
                        p.running = true;
                        p.step = 0.8;
                        p.happy_on_the_move();
                    }),
                    true,
                ),
                (
                    "carry",
                    Box::new(|p| {
                        p.running = true;
                        p.step = 2.4;
                        p.carry = 2;
                    }),
                    true,
                ),
                ("down", Box::new(|p| p.roll = 1.5), true),
                ("sideA", Box::new(|p| p.roll = 2.5), true),
                ("twistA", Box::new(|p| p.roll = 3.5), true),
                ("back", Box::new(|p| p.roll = 4.5), true),
                ("twistB", Box::new(|p| p.roll = 5.5), true),
                ("sideB", Box::new(|p| p.roll = 6.5), true),
                ("up", Box::new(|p| p.roll = 7.5), true),
            ];
            for (name, f, side) in &poses {
                let (cv, pasted, _) = render(f.as_ref());
                let (rows, cols, nose) = bbox(&pasted);
                let (er, ec) = if *side {
                    (side_rows, side_cols)
                } else {
                    (front_rows, front_cols)
                };
                eprintln!(
                    "{} {name}: head {rows}x{cols} (seated {front_rows}x{front_cols}, profile sprite {side_rows}x{side_cols}) shines {}",
                    br.name,
                    shines(&cv)
                );
                assert!(
                    (rows - er).abs() <= 2 && cols == ec,
                    "{} {name}: head {rows}x{cols}, seated {er}x{ec}",
                    br.name
                );
                assert!(
                    rows >= front_rows - 2,
                    "{} {name}: head shorter than seated",
                    br.name
                );
                // every pasted head cell is still there (nothing draws over the face)
                let kept = pasted
                    .iter()
                    .filter(|&&(x, y, _)| {
                        x >= 0
                            && y >= 0
                            && (x as usize) < cv.w
                            && (y as usize) < cv.h
                            && cv.cells[y as usize * cv.w + x as usize].fg.is_some()
                    })
                    .count();
                assert!(
                    kept * 100 >= pasted.len() * 97,
                    "{} {name}: head overdrawn",
                    br.name
                );
                assert_eq!(
                    shines(&cv),
                    if *side { 1 } else { 2 },
                    "{} {name}: seated eyes",
                    br.name
                );
                if name.starts_with("walk") {
                    // body width in profile, at the body's middle row
                    let withers = 0.54 * hh;
                    let ll = (leg_frac(br) * withers).max(2.0);
                    let vb = ll + (withers - ll) / 2.0;
                    let (l, r) = run(&cv, feet - 1 - vb.round() as i32, cx);
                    let bw = r - l + 1;
                    eprintln!("{}: profile body {bw} cols, seated {sit_w}", br.name);
                    assert!(
                        (bw as f32 / sit_w as f32 - 1.0).abs() <= 0.15,
                        "{}: profile body {bw} cols, seated {sit_w}",
                        br.name
                    );
                }
                if ["down", "sideA", "twistA", "back", "twistB", "sideB"].contains(name) {
                    let mut rump = i32::MAX;
                    for v in 4..=9 {
                        let y = (feet - 1 - v) as usize;
                        for x in 0..cv.w {
                            if cv.cells[y * cv.w + x].fg.is_some() {
                                rump = rump.min(x as i32);
                                break;
                            }
                        }
                    }
                    let len = nose - rump + 1;
                    eprintln!(
                        "{} {name}: nose to rump {len} cols, seated height {hh} rows",
                        br.name
                    );
                    assert!(
                        (len as f32 / (2.0 * hh) - 1.0).abs() <= 0.1,
                        "{} {name}: {len} cols long, seated {hh} rows tall",
                        br.name
                    );
                }
                if *name == "stand" {
                    // the near front leg, mid-way down, without the darker far leg behind it
                    let ru = 0.46 * ww;
                    let withers = 0.54 * hh;
                    let ll = (leg_frac(br) * withers).max(2.0);
                    let mut raw = crate::art::Canvas::new(w, h, false);
                    let mut body = base.clone();
                    body.dpose.up = true;
                    crate::dog_art::draw_dog(&mut raw, &body);
                    let y = feet - 1 - (0.45 * ll).round() as i32;
                    let x = cx + (0.58 * ru).round() as i32;
                    let (la, lb) = run(&raw, y, x);
                    let far = crate::dog_art::far_leg(br);
                    let near = (la..=lb)
                        .filter(|&x| raw.cells[y as usize * raw.w + x as usize].fg != Some(far))
                        .count() as i32;
                    eprintln!("{}: near front leg {near} cols, seated {sit_leg}", br.name);
                    if sit_leg >= 2 {
                        assert_eq!(near, sit_leg, "{}: leg columns", br.name);
                    }
                    // and the same paw, five columns on the grass
                    let x = cx + (0.58 * ru + 1.2).round() as i32;
                    let (pa, pb) = run(&raw, feet - 1, x);
                    let paw = (pa..=pb)
                        .filter(|&x| {
                            raw.cells[(feet - 1) as usize * raw.w + x as usize].fg
                                != Some(crate::dog_art::far_paw(br))
                        })
                        .count() as i32;
                    eprintln!("{}: near front paw {paw} cols, seated {sit_paw}", br.name);
                    assert!(
                        (paw - sit_paw).abs() <= 1,
                        "{}: paw columns {paw} vs {sit_paw}",
                        br.name
                    );
                }
            }
        }
    }

    /// Dev aid: `DUMP=/path BREED=0 cargo test dump_poses -- --ignored` writes the seated,
    /// standing, walking and roll frames side by side as ANSI text for `scripts/ansi2png.py`.
    #[test]
    #[ignore]
    fn dump_poses() {
        let Ok(path) = std::env::var("DUMP") else {
            return;
        };
        let breed: usize = std::env::var("BREED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let (w, h) = (120u16, 40u16);
        let mut a = dog(w, h, 5);
        a.init_color(breed, false);
        type Set = Box<dyn Fn(&mut DogPose)>;
        let poses: Vec<(&str, Set)> = vec![
            ("sit", Box::new(|_| {})),
            ("stand", Box::new(|p| p.up = true)),
            (
                "walk0",
                Box::new(|p| {
                    p.walking = true;
                    p.step = 0.0;
                    p.happy_on_the_move();
                }),
            ),
            (
                "walk1",
                Box::new(|p| {
                    p.walking = true;
                    p.step = 1.6;
                    p.happy_on_the_move();
                }),
            ),
            (
                "run",
                Box::new(|p| {
                    p.running = true;
                    p.step = 0.8;
                    p.happy_on_the_move();
                }),
            ),
            (
                "carry",
                Box::new(|p| {
                    p.running = true;
                    p.step = 2.4;
                    p.carry = 2;
                }),
            ),
            ("roll1", Box::new(|p| p.roll = 1.5)),
            ("roll2", Box::new(|p| p.roll = 2.5)),
            ("roll3", Box::new(|p| p.roll = 3.5)),
            ("roll4", Box::new(|p| p.roll = 4.5)),
            ("roll5", Box::new(|p| p.roll = 5.5)),
            ("roll6", Box::new(|p| p.roll = 6.5)),
            ("roll7", Box::new(|p| p.roll = 7.5)),
        ];
        let mut panels = vec![];
        for (name, f) in &poses {
            let mut body = a.dog_host_body();
            f(&mut body.dpose);
            let mut cv = crate::art::Canvas::new(w, h, false);
            crate::dog_art::draw_dog_into(&mut cv, &body, false, 0, w, h);
            let (mut x0, mut x1) = (usize::MAX, 0);
            for y in 0..cv.h {
                for x in 0..cv.w {
                    if cv.cells[y * cv.w + x].fg.is_some() {
                        (x0, x1) = (x0.min(x), x1.max(x));
                    }
                }
            }
            panels.push((name.to_string(), cv, x0, x1));
        }
        let mut out = String::new();
        for row in panels.chunks(3) {
            out.push_str(
                &row.iter()
                    .map(|p| format!("{:<80}", p.0))
                    .collect::<String>(),
            );
            out.push('\n');
            for y in 2..h as usize - 9 {
                for (_, cv, x0, x1) in row {
                    let mut n = 0;
                    for x in *x0..=(*x1).min(*x0 + 79) {
                        let c = cv.cells[y * cv.w + x];
                        match c.fg {
                            Some((r, g, b)) => {
                                out.push_str(&format!("\x1b[38;2;{r};{g};{b}m{}\x1b[0m", c.ch))
                            }
                            None => out.push(' '),
                        }
                        n += 1;
                    }
                    for _ in n..80 {
                        out.push(' ');
                    }
                }
                out.push('\n');
            }
        }
        std::fs::write(path, out).unwrap();
    }
}
