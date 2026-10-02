//! Animation state machine. No rendering here; `art` draws from this state.
use crate::art::geo;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Behavior {
    Yawn,
    LookLeft,
    LookRight,
    InkPuff,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    Feeding,
    Petting,
    Playing,
    IdleBehavior(Behavior),
}

pub struct Bubble {
    pub x: f32,
    pub y: f32,
    pub vy: f32,
    pub ch: char,
    pub phase: f32,
}
pub struct Heart {
    pub x: f32,
    pub y: f32,
    pub age: f32,
}
pub struct Ink {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub age: f32,
    pub life: f32,
}

pub struct App {
    pub state: State,
    pub st: f32,
    pub t: f32,
    pub w: u16,
    pub h: u16,
    pub msg: Option<&'static str>,
    pub bubbles: Vec<Bubble>,
    pub hearts: Vec<Heart>,
    pub ink: Vec<Ink>,
    pub target: (f32, f32),
    pub next_idle: f32,
    pub next_blink: f32,
    pub quit: bool,
    /// Horizontal drift: position in -1..=1, current burst (from, to, seconds in).
    pos: f32,
    burst: (f32, f32, f32),
    next_burst: f32,
    rng: StdRng,
}

const BURST: f32 = 1.8;

fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl App {
    pub fn new(w: u16, h: u16, seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let next_idle = rng.random_range(10.0..25.0);
        let next_blink = rng.random_range(3.0..7.0);
        App {
            state: State::Idle,
            st: 0.0,
            t: 0.0,
            w,
            h,
            msg: None,
            bubbles: vec![],
            hearts: vec![],
            ink: vec![],
            target: (0.0, 0.0),
            next_idle,
            next_blink,
            quit: false,
            pos: 0.0,
            burst: (0.0, 0.0, BURST),
            next_burst: 6.0,
            rng,
        }
    }

    pub fn resize(&mut self, w: u16, h: u16) {
        self.w = w;
        self.h = h;
    }

    pub fn duration(s: State) -> f32 {
        match s {
            State::Idle => f32::MAX,
            State::Feeding => 3.0,
            State::Petting => 2.6,
            State::Playing => 3.4,
            State::IdleBehavior(Behavior::InkPuff) => 1.4,
            State::IdleBehavior(_) => 1.8,
        }
    }

    /// Enter a state (restarts it if already in it).
    pub fn start(&mut self, s: State) {
        self.state = s;
        self.st = 0.0;
        self.msg = match s {
            State::Feeding => Some("Yum!"),
            State::Petting => Some("Blub blub \u{2665}"),
            State::Playing => Some("Wheee!"),
            _ => None,
        };
        let g = geo(self.w, self.h);
        match s {
            State::Petting => {
                let n = self.rng.random_range(3..=6);
                for _ in 0..n {
                    let x = g.cx as f32 + self.wander() + self.rng.random_range(-g.hwh..=g.hwh);
                    let y = (g.top + g.mh / 2) as f32;
                    let age = -self.rng.random_range(0.0..0.6);
                    self.hearts.push(Heart { x, y, age });
                }
            }
            State::Playing => {
                let lim = (self.w as f32 / 2.0 - g.maxw * 2.2).max(0.0);
                self.target = (
                    self.rng.random_range(-lim..=lim),
                    self.rng.random_range(-1.0..=2.0),
                );
                self.spawn_ink(g.cx as f32 + self.wander(), (g.top + g.mh) as f32, 50, 1.0);
            }
            State::IdleBehavior(Behavior::InkPuff) => {
                self.spawn_ink(g.cx as f32 + self.wander(), (g.top + g.mh) as f32, 14, 0.5)
            }
            _ => {}
        }
    }

    fn spawn_ink(&mut self, x: f32, y: f32, n: usize, speed: f32) {
        for _ in 0..n {
            let a = self.rng.random_range(0.0..std::f32::consts::TAU);
            let s = self.rng.random_range(3.0..14.0) * speed;
            self.ink.push(Ink {
                x,
                y,
                vx: a.cos() * s * 2.0,
                vy: a.sin() * s,
                age: 0.0,
                life: self.rng.random_range(1.0..2.2),
            });
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.t += dt;
        self.st += dt;
        // Fish-like drift: mostly still, an occasional short burst to a new spot.
        if self.burst.2 < BURST {
            self.burst.2 += dt;
            self.pos = self.burst.0 + (self.burst.1 - self.burst.0) * ease(self.burst.2 / BURST);
            if self.burst.2 >= BURST {
                self.pos = self.burst.1;
                self.next_burst = self.t + self.rng.random_range(10.0..25.0);
            }
        } else if self.t >= self.next_burst {
            let mut to = self.rng.random_range(-1.0..=1.0f32);
            if (to - self.pos).abs() < 0.4 {
                to = if self.pos > 0.0 {
                    self.pos - 0.6
                } else {
                    self.pos + 0.6
                };
            }
            self.burst = (self.pos, to.clamp(-1.0, 1.0), 0.0);
        }
        if self.st >= Self::duration(self.state) {
            self.state = State::Idle;
            self.msg = None;
            self.st = 0.0;
            self.next_idle = self.t + self.rng.random_range(10.0..25.0);
        }
        if self.state == State::Idle && self.t >= self.next_idle {
            let b = match self.rng.random_range(0..4) {
                0 => Behavior::Yawn,
                1 => Behavior::LookLeft,
                2 => Behavior::LookRight,
                _ => Behavior::InkPuff,
            };
            self.start(State::IdleBehavior(b));
        }
        if self.t >= self.next_blink + 0.15 {
            self.next_blink = self.t + self.rng.random_range(3.0..7.0);
        }
        // bubbles
        let g = geo(self.w, self.h);
        if self.bubbles.len() < 40 && self.rng.random_range(0.0..1.0) < 3.0 * dt {
            let x =
                g.cx as f32 + self.wander() + self.rng.random_range(-g.maxw * 1.4..=g.maxw * 1.4);
            self.bubbles.push(Bubble {
                x,
                y: (g.top + g.mh) as f32,
                vy: self.rng.random_range(2.0..5.0),
                ch: ['o', 'O', '\u{b0}'][self.rng.random_range(0..3)],
                phase: self.rng.random_range(0.0..std::f32::consts::TAU),
            });
        }
        for b in &mut self.bubbles {
            b.y -= b.vy * dt;
        }
        self.bubbles.retain(|b| b.y >= -1.0);
        for h in &mut self.hearts {
            h.age += dt;
            if h.age > 0.0 {
                h.y -= 4.0 * dt;
            }
        }
        self.hearts.retain(|h| h.age < 2.0);
        for i in &mut self.ink {
            i.age += dt;
            i.x += i.vx * dt;
            i.y += i.vy * dt;
            let k = (1.0 - 2.5 * dt).max(0.0);
            i.vx *= k;
            i.vy *= k;
        }
        self.ink.retain(|i| i.age < i.life);
    }

    // ---- derived animation parameters ----
    pub fn puff(&self) -> f32 {
        match self.state {
            State::Feeding if (1.2..2.2).contains(&self.st) => {
                1.0 + 0.25 * (std::f32::consts::PI * (self.st - 1.2)).sin()
            }
            _ => 1.0,
        }
    }
    pub fn curl(&self) -> f32 {
        match self.state {
            State::Feeding => ease((self.st - 1.0) / 0.4) * (1.0 - ease((self.st - 2.4) / 0.5)),
            _ => 0.0,
        }
    }
    pub fn blush(&self) -> f32 {
        match self.state {
            State::Petting => ease(self.st / 0.3) * (1.0 - ease((self.st - 2.1) / 0.5)),
            _ => 0.0,
        }
    }
    pub fn eyes_closed(&self) -> bool {
        self.state == State::Petting
    }
    pub fn blinking(&self) -> bool {
        self.t >= self.next_blink && self.t < self.next_blink + 0.15
    }
    pub fn look(&self) -> i32 {
        match self.state {
            State::IdleBehavior(Behavior::LookLeft) if (0.2..1.6).contains(&self.st) => -1,
            State::IdleBehavior(Behavior::LookRight) if (0.2..1.6).contains(&self.st) => 1,
            _ => 0,
        }
    }
    pub fn yawning(&self) -> bool {
        matches!(self.state, State::IdleBehavior(Behavior::Yawn)) && (0.2..1.6).contains(&self.st)
    }
    /// (dx, dy) in cells: play dart offset plus idle bob.
    pub fn offset(&self) -> (f32, f32) {
        let bob = 1.0 + 0.6 * (self.t * 0.35).sin();
        let (px, py) = match self.state {
            State::Playing => {
                let s = self.st;
                let k = if s < 0.3 {
                    0.0
                } else if s < 0.7 {
                    ease((s - 0.3) / 0.4)
                } else if s < 1.2 {
                    1.0
                } else {
                    1.0 - ease((s - 1.2) / 2.0)
                };
                (self.target.0 * k, self.target.1 * k)
            }
            _ => (0.0, 0.0),
        };
        (px + self.wander(), bob + py)
    }
    /// Horizontal offset in cells: rests, then drifts in a short burst.
    pub fn wander(&self) -> f32 {
        let g = geo(self.w, self.h);
        let room = (self.w as f32 / 2.0 - g.maxw * 2.2).max(0.0);
        room * self.pos
    }
    /// Diagonal lean in columns per row; tilts toward the swim direction.
    pub fn lean(&self) -> f32 {
        let (from, to, t) = self.burst;
        let bell = if t < BURST {
            (std::f32::consts::PI * t / BURST).sin()
        } else {
            0.0
        };
        0.2 + 0.4 * (to - from).signum() * bell
    }
    /// Food position while feeding.
    pub fn food(&self) -> Option<(f32, f32)> {
        if self.state != State::Feeding || self.st > 2.4 {
            return None;
        }
        let g = geo(self.w, self.h);
        let fy = (g.top + g.mh + g.hh) as f32 + g.al as f32 * 0.5;
        let k = ease(self.st / 1.2);
        Some((g.cx as f32, -1.0 + (fy + 1.0) * k))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drift_is_mostly_still_with_short_bursts() {
        let mut a = App::new(120, 40, 3);
        let (mut moving, mut bursts, mut last) = (0, 0, a.wander());
        let mut was = false;
        for _ in 0..(300 * 16) {
            a.update(0.0625);
            let w = a.wander();
            let m = (w - last).abs() > 1e-4;
            moving += m as usize;
            bursts += (m && !was) as usize;
            was = m;
            last = w;
        }
        assert!(moving < 300 * 16 / 5, "moving {moving}");
        assert!((5..=30).contains(&bursts), "bursts {bursts}");
    }

    #[test]
    fn keys_start_states_and_return_to_idle() {
        let mut a = App::new(80, 24, 1);
        for (s, msg) in [
            (State::Feeding, "Yum!"),
            (State::Petting, "Blub blub \u{2665}"),
            (State::Playing, "Wheee!"),
        ] {
            a.start(s);
            assert_eq!(a.state, s);
            assert_eq!(a.msg, Some(msg));
            for _ in 0..((App::duration(s) / 0.0625) as usize + 2) {
                a.update(0.0625);
            }
            assert_eq!(a.state, State::Idle);
            assert_eq!(a.msg, None);
        }
    }

    #[test]
    fn restart_during_animation_never_panics() {
        let mut a = App::new(80, 24, 2);
        for i in 0..500 {
            a.start([State::Feeding, State::Petting, State::Playing][i % 3]);
            a.update(0.03);
        }
    }

    #[test]
    fn idle_behavior_within_30s() {
        let mut a = App::new(80, 24, 3);
        let mut seen = false;
        for _ in 0..(30 * 16) {
            a.update(1.0 / 16.0);
            seen |= matches!(a.state, State::IdleBehavior(_));
        }
        assert!(seen);
    }

    #[test]
    fn bubbles_rise_and_leave() {
        let mut a = App::new(80, 24, 4);
        for _ in 0..800 {
            a.update(0.0625);
        }
        assert!(a.bubbles.len() <= 40);
        assert!(a.bubbles.iter().all(|b| b.y >= -1.0));
    }
}
