//! Animation state machine. No rendering here; `art` draws from this state.
use crate::art::{Body, PRESETS, Palette, geo_s, lerp_pal};
use crate::link::Msg;
use crate::social::{Fx, Guest, Home, In, Peer};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Behavior {
    Yawn,
    LookLeft,
    LookRight,
    InkPuff,
}

/// Idle-time state. Key reactions are layered separately (`React`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Idle,
    IdleBehavior(Behavior),
}

/// The three stackable key reactions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Feed,
    Pet,
    Play,
}

impl Key {
    fn idx(self) -> usize {
        self as usize
    }
    pub fn base(self) -> f32 {
        match self {
            Key::Feed => 3.0,
            Key::Pet => 2.6,
            Key::Play => 3.4,
        }
    }
    fn msg(self) -> &'static str {
        match self {
            Key::Feed => "Yum!",
            Key::Pet => "Blub blub \u{2665}",
            Key::Play => "Wheee!",
        }
    }
}

/// A running reaction: seconds in, and total length (grows with repeat presses).
#[derive(Clone, Copy, Default, Debug)]
pub struct React {
    pub on: bool,
    pub st: f32,
    pub dur: f32,
}

/// A fish on its way to the mouth; `born` is in feed-reaction time.
pub struct Fish {
    pub born: f32,
    pub from: (f32, f32),
}

/// A thing flying sideways across the screen (tossed fish or ink puff).
pub struct Flyer {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub ink: bool,
    /// Incoming ink: bursts and startles the squid on reaching this x.
    pub hit_x: Option<f32>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorChoice {
    Preset(usize),
    Random,
}

pub fn color_list() -> String {
    let mut v: Vec<&str> = PRESETS.iter().map(|p| p.0).collect();
    v.push("random");
    v.join(", ")
}

/// Case-insensitive color name lookup.
pub fn parse_color(s: &str) -> Option<ColorChoice> {
    let s = s.to_ascii_lowercase();
    if s == "random" {
        return Some(ColorChoice::Random);
    }
    PRESETS
        .iter()
        .position(|p| p.0 == s)
        .map(ColorChoice::Preset)
}

/// Preset picked by `--color random` for a seed.
pub fn random_preset(seed: u64) -> usize {
    StdRng::seed_from_u64(seed).random_range(0..PRESETS.len())
}

const FADE: f32 = 0.5;
/// Counted presses per key within `PRESS_WINDOW` seconds.
pub const PRESS_CAP: usize = 5;
pub const PRESS_WINDOW: f32 = 3.0;
/// Seconds a repeat press adds to a running reaction.
pub const EXTEND: f32 = 1.0;

pub struct App {
    pub mono: bool,
    /// Color-name message and seconds left.
    pub cmsg: Option<(String, f32)>,
    /// Timed message from the visit/message system.
    pub note: Option<(String, f32)>,
    color: usize,
    pal_from: Palette,
    fade: f32,
    pub state: State,
    pub st: f32,
    pub t: f32,
    pub w: u16,
    pub h: u16,
    pub msg: Option<&'static str>,
    pub bubbles: Vec<Bubble>,
    pub hearts: Vec<Heart>,
    pub ink: Vec<Ink>,
    pub flyers: Vec<Flyer>,
    pub feed: React,
    pub pet: React,
    pub play: React,
    pub fish: Vec<Fish>,
    presses: [Vec<f32>; 3],
    dart_s: f32,
    dart_from: (f32, f32),
    pub target: (f32, f32),
    pub jolt: f32,
    pub next_idle: f32,
    pub next_blink: f32,
    pub quit: bool,
    /// Horizontal drift: position in -1..=1, current burst (from, to, seconds in).
    pos: f32,
    burst: (f32, f32, f32),
    next_burst: f32,
    rng: StdRng,
    // ---- shared window / visits (see social.rs) ----
    pub scale: f32,
    pub host_dx: f32,
    pub fx: Fx,
    pub social: bool,
    pub pid: u32,
    pub peers: BTreeMap<u32, Peer>,
    pub home: Home,
    pub guests: Vec<Guest>,
    pub reserved: Vec<(u32, f32)>,
    pub inbox: Vec<(f32, In)>,
    pub outbox: Vec<(Option<u32>, Msg)>,
    pub(crate) vrng: StdRng,
    pub(crate) next_visit: f32,
    pub(crate) next_bubble: f32,
    pub(crate) last_fish_send: f32,
    pub(crate) last_ink_send: f32,
    pub(crate) hello_t: f32,
    pub(crate) play_ctl: Option<crate::social::PlayCtl>,
    pub(crate) bag: Vec<crate::social::Mv>,
}

const BURST: f32 = 1.8;

pub fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl App {
    pub fn new(w: u16, h: u16, seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let next_idle = rng.random_range(10.0..25.0);
        let next_blink = rng.random_range(3.0..7.0);
        App {
            mono: false,
            cmsg: None,
            note: None,
            color: 0,
            pal_from: PRESETS[0].1,
            fade: 1.0,
            state: State::Idle,
            st: 0.0,
            t: 0.0,
            w,
            h,
            msg: None,
            bubbles: vec![],
            hearts: vec![],
            ink: vec![],
            flyers: vec![],
            feed: React::default(),
            pet: React::default(),
            play: React::default(),
            fish: vec![],
            presses: [vec![], vec![], vec![]],
            dart_s: 99.0,
            dart_from: (0.0, 0.0),
            target: (0.0, 0.0),
            jolt: 0.0,
            next_idle,
            next_blink,
            quit: false,
            pos: 0.0,
            burst: (0.0, 0.0, BURST),
            next_burst: 6.0,
            rng,
            scale: 1.0,
            host_dx: 0.0,
            fx: Fx::default(),
            social: false,
            pid: 0,
            peers: BTreeMap::new(),
            home: Home::Home,
            guests: vec![],
            reserved: vec![],
            inbox: vec![],
            outbox: vec![],
            vrng: StdRng::seed_from_u64(seed ^ 0x5eed_5eed_5eed),
            next_visit: f32::MAX,
            next_bubble: f32::MAX,
            last_fish_send: -1000.0,
            last_ink_send: -1000.0,
            hello_t: 0.0,
            play_ctl: None,
            bag: vec![],
        }
    }

    pub fn geo(&self) -> crate::art::Geo {
        geo_s(self.w, self.h, self.scale)
    }

    /// Set the starting preset; `announce` shows "I'm <name> today!".
    pub fn init_color(&mut self, idx: usize, announce: bool) {
        self.color = idx;
        self.pal_from = PRESETS[idx].1;
        self.fade = 1.0;
        if announce {
            self.cmsg = Some((format!("I'm {} today!", PRESETS[idx].0), 2.0));
        }
    }

    /// Fade to the next preset (the `c` key).
    pub fn next_color(&mut self) {
        self.pal_from = self.palette();
        self.color = (self.color + 1) % PRESETS.len();
        self.fade = 0.0;
        self.cmsg = Some((format!("{}!", PRESETS[self.color].0), 2.0));
        if let Home::Away { host, .. } = self.home {
            self.outbox.push((Some(host), Msg::Color(self.color as u8)));
        }
    }

    /// Current preset index (the target one while fading).
    pub fn color_idx(&self) -> usize {
        self.color
    }

    pub fn palette(&self) -> Palette {
        if self.fade >= 1.0 {
            PRESETS[self.color].1
        } else {
            lerp_pal(&self.pal_from, &PRESETS[self.color].1, ease(self.fade))
        }
    }

    fn react_flash(r: &React) -> f32 {
        if !r.on {
            return 0.0;
        }
        ease(r.st / 0.3) * (1.0 - ease((r.st - (r.dur - 0.3)) / 0.3))
    }

    /// 0..1 ring brightness: ramps in 0.3 s on pet/play, out at the end.
    pub fn flash(&self) -> f32 {
        Self::react_flash(&self.pet).max(Self::react_flash(&self.play))
    }

    pub fn resize(&mut self, w: u16, h: u16) {
        self.w = w;
        self.h = h;
    }

    pub fn any_react(&self) -> bool {
        self.feed.on || self.pet.on || self.play.on
    }

    fn react(&mut self, k: Key) -> &mut React {
        match k {
            Key::Feed => &mut self.feed,
            Key::Pet => &mut self.pet,
            Key::Play => &mut self.play,
        }
    }

    /// Seconds of unused window: true if the press counts (at most 5 per key per 3 s).
    fn count_press(&mut self, k: Key) -> bool {
        let t = self.t;
        let p = &mut self.presses[k.idx()];
        p.retain(|&x| t - x < PRESS_WINDOW);
        if p.len() >= PRESS_CAP {
            return false;
        }
        p.push(t);
        true
    }

    /// A feed / pet / play key press. Reactions layer; repeats add and extend.
    /// Returns whether the press counted.
    pub fn press(&mut self, k: Key) -> bool {
        if matches!(self.home, Home::Away { .. } | Home::Leaving { .. }) {
            return false;
        }
        if !self.count_press(k) {
            return false;
        }
        self.msg = Some(k.msg());
        self.state = State::Idle;
        let g = self.geo();
        match k {
            Key::Feed => {
                let from = (self.mouth_x(), -1.0);
                self.add_fish(from);
            }
            Key::Pet => {
                self.bump(k, 0.0);
                let n = self.rng.random_range(3..=6);
                for _ in 0..n {
                    let x = self.spawn_x() + self.rng.random_range(-g.hwh..=g.hwh);
                    let y = (g.top + g.mh / 2) as f32;
                    let age = -self.rng.random_range(0.0..0.6);
                    self.hearts.push(Heart { x, y, age });
                }
            }
            Key::Play => {
                self.bump(k, 3.4);
                let (px, py) = self.dart_offset();
                self.dart_from = (px, py);
                self.dart_s = 0.0;
                let lim = (self.w as f32 / 2.0 - g.maxw * 2.2).max(0.0);
                self.target = (
                    self.rng.random_range(-lim..=lim),
                    self.rng.random_range(-1.0..=2.0),
                );
                self.spawn_ink(self.spawn_x(), (g.top + g.mh) as f32, 50, 1.0);
            }
        }
        if self.social {
            self.after_press(k);
        }
        true
    }

    /// Start the reaction, or extend a running one. `need` = minimum seconds left.
    fn bump(&mut self, k: Key, need: f32) {
        let base = k.base();
        let r = self.react(k);
        if r.on {
            r.dur = (r.dur + EXTEND).max(r.st + need);
        } else {
            *r = React {
                on: true,
                st: 0.0,
                dur: base,
            };
        }
    }

    pub(crate) fn add_fish(&mut self, from: (f32, f32)) {
        self.bump(Key::Feed, 3.0);
        let born = self.feed.st;
        self.fish.push(Fish { born, from });
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

    /// Ink puff for effects that are not tied to the main rng stream.
    pub(crate) fn spawn_ink_v(&mut self, x: f32, y: f32, n: usize, vx: f32, spread: f32) {
        for _ in 0..n {
            let s = self.vrng.random_range(0.6..1.0);
            self.ink.push(Ink {
                x,
                y,
                vx: vx * s + self.vrng.random_range(-spread..=spread),
                vy: self.vrng.random_range(-4.0..4.0),
                age: 0.0,
                life: self.vrng.random_range(1.0..1.8),
            });
        }
    }

    /// Where a spawned effect appears: the squid's resting center.
    pub fn spawn_x(&self) -> f32 {
        self.geo().cx as f32 + self.host_dx + self.wander()
    }

    fn start_idle(&mut self, b: Behavior) {
        self.state = State::IdleBehavior(b);
        self.st = 0.0;
        let g = self.geo();
        if b == Behavior::InkPuff {
            self.spawn_ink(self.spawn_x(), (g.top + g.mh) as f32, 14, 0.5)
        }
    }

    pub fn idle_duration(b: Behavior) -> f32 {
        if b == Behavior::InkPuff { 1.4 } else { 1.8 }
    }

    pub fn update(&mut self, dt: f32) {
        self.t += dt;
        self.st += dt;
        if self.fade < 1.0 {
            self.fade = (self.fade + dt / FADE).min(1.0);
        }
        if let Some((_, left)) = &mut self.cmsg {
            *left -= dt;
            if *left <= 0.0 {
                self.cmsg = None;
            }
        }
        if let Some((_, left)) = &mut self.note {
            *left -= dt;
            if *left <= 0.0 {
                self.note = None;
            }
        }
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
        if let State::IdleBehavior(b) = self.state
            && self.st >= Self::idle_duration(b)
        {
            self.state = State::Idle;
            self.st = 0.0;
            self.next_idle = self.t + self.rng.random_range(10.0..25.0);
        }
        // Layered key reactions: each runs its own clock.
        let was = self.any_react();
        for r in [&mut self.feed, &mut self.pet, &mut self.play] {
            if r.on {
                r.st += dt;
                if r.st >= r.dur {
                    r.on = false;
                }
            }
        }
        self.dart_s += dt;
        if self.feed.on {
            let st = self.feed.st;
            self.fish.retain(|f| st - f.born <= 2.9);
        } else {
            self.fish.clear();
        }
        if was && !self.any_react() {
            self.msg = None;
            self.next_idle = self.t + self.rng.random_range(10.0..25.0);
        }
        if self.state == State::Idle && !self.any_react() && self.t >= self.next_idle {
            let b = match self.rng.random_range(0..4) {
                0 => Behavior::Yawn,
                1 => Behavior::LookLeft,
                2 => Behavior::LookRight,
                _ => Behavior::InkPuff,
            };
            self.start_idle(b);
        }
        if self.t >= self.next_blink + 0.15 {
            self.next_blink = self.t + self.rng.random_range(3.0..7.0);
        }
        // bubbles
        let g = self.geo();
        if self.bubbles.len() < 40 && self.rng.random_range(0.0..1.0) < 3.0 * dt {
            let x = self.spawn_x() + self.rng.random_range(-g.maxw * 1.4..=g.maxw * 1.4);
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
        self.jolt = (self.jolt - dt).max(0.0);
        self.update_flyers(dt);
        if self.social {
            self.social_update(dt);
        }
    }

    // ---- derived animation parameters ----
    pub fn fish_age(&self, f: &Fish) -> f32 {
        self.feed.st - f.born
    }
    pub fn puff(&self) -> f32 {
        if !self.feed.on {
            return 1.0;
        }
        self.fish
            .iter()
            .map(|f| self.fish_age(f))
            .filter(|a| (1.2..2.2).contains(a))
            .map(|a| 1.0 + 0.25 * (std::f32::consts::PI * (a - 1.2)).sin())
            .fold(1.0, f32::max)
    }
    pub fn curl(&self) -> f32 {
        if !self.feed.on {
            return 0.0;
        }
        self.fish
            .iter()
            .map(|f| {
                let a = self.fish_age(f);
                ease((a - 1.0) / 0.4) * (1.0 - ease((a - 2.4) / 0.5))
            })
            .fold(0.0, f32::max)
    }
    pub fn blush(&self) -> f32 {
        let r = &self.pet;
        if !r.on {
            return 0.0;
        }
        ease(r.st / 0.3) * (1.0 - ease((r.st - (r.dur - 0.5)) / 0.5))
    }
    pub fn eyes_closed(&self) -> bool {
        self.pet.on
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
    /// Dart offset in cells from the play reaction.
    fn dart_offset(&self) -> (f32, f32) {
        if !self.play.on {
            return (0.0, 0.0);
        }
        let s = self.dart_s;
        let k = if s < 0.3 {
            0.0
        } else if s < 0.7 {
            ease((s - 0.3) / 0.4)
        } else if s < 1.2 {
            1.0
        } else {
            1.0 - ease((s - 1.2) / 2.0)
        };
        let keep = 1.0 - ease(s / 0.5);
        (
            self.target.0 * k + self.dart_from.0 * keep,
            self.target.1 * k + self.dart_from.1 * keep,
        )
    }
    /// (dx, dy) in cells: play dart offset plus idle bob.
    pub fn offset(&self) -> (f32, f32) {
        let bob = 1.0 + 0.6 * (self.t * 0.35).sin();
        let (px, py) = self.dart_offset();
        let jolt = if self.jolt > 0.0 {
            -2.0 * (self.jolt / 0.4).min(1.0)
        } else {
            0.0
        };
        (
            px + self.wander() + self.host_dx + self.home_ox() + self.fx.dx,
            bob + py + jolt + self.fx.dy,
        )
    }
    /// Horizontal offset in cells: rests, then drifts in a short burst.
    pub fn wander(&self) -> f32 {
        let g = self.geo();
        let n = 1 + self.guests.len();
        let room = (self.w as f32 / n as f32 / 2.0 - g.maxw * 2.2).max(0.0);
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
    /// Where the host squid's mouth is (columns).
    pub fn mouth_x(&self) -> f32 {
        self.geo().cx as f32 + self.offset().0
    }
    /// Food positions while feeding (one per fish still on screen).
    pub fn foods(&self) -> Vec<(f32, f32)> {
        if !self.feed.on {
            return vec![];
        }
        let g = self.geo();
        let fy = (g.top + g.mh + g.hh) as f32 + g.al as f32 * 0.5;
        let mx = self.mouth_x();
        self.fish
            .iter()
            .filter_map(|f| {
                let a = self.fish_age(f);
                if !(0.0..=2.4).contains(&a) {
                    return None;
                }
                let k = ease(a / 1.2);
                Some((
                    f.from.0 + (mx - f.from.0) * k,
                    f.from.1 + (fy - f.from.1) * k,
                ))
            })
            .collect()
    }
    /// Draw data for the host squid.
    pub fn host_body(&self) -> Body {
        let g = self.geo();
        let (dx, dy) = self.offset();
        Body {
            pal: self.palette(),
            cx: g.cx,
            dx,
            dy,
            scale: self.scale,
            puff: self.puff(),
            curl: self.curl(),
            blush: self.blush(),
            flash: self.flash(),
            bright: self.fx.bright,
            dark: self.fx.dark,
            look: self.look(),
            closed: self.eyes_closed(),
            blink: self.blinking(),
            yawn: self.yawning(),
            lean: self.lean(),
            food_x: self.foods().last().map_or(g.cx as f32 + dx, |f| f.0),
            t: self.t,
            tag: None,
        }
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
    fn color_parsing() {
        assert_eq!(parse_color("red"), Some(ColorChoice::Preset(0)));
        assert_eq!(parse_color("Blue-Ringed"), Some(ColorChoice::Preset(7)));
        assert_eq!(parse_color("RANDOM"), Some(ColorChoice::Random));
        assert_eq!(parse_color("teal"), None);
        assert_eq!(parse_color(""), None);
    }

    #[test]
    fn random_is_deterministic_and_varies() {
        assert_eq!(random_preset(42), random_preset(42));
        let set: std::collections::HashSet<_> = (1..12).map(random_preset).collect();
        assert!(set.len() > 3);
    }

    #[test]
    fn c_cycles_with_fade_and_wraps() {
        let mut a = App::new(80, 24, 1);
        a.next_color();
        assert_eq!(a.cmsg.as_ref().unwrap().0, "purple!");
        a.update(0.25);
        let mid = a.palette();
        assert!(mid != PRESETS[0].1 && mid != PRESETS[1].1);
        for _ in 0..7 {
            a.next_color();
        }
        a.update(0.6);
        assert_eq!(a.palette(), PRESETS[0].1);
        a.update(2.1);
        assert!(a.cmsg.is_none());
    }

    #[test]
    fn keys_start_reactions_and_return_to_idle() {
        let mut a = App::new(80, 24, 1);
        for (k, msg) in [
            (Key::Feed, "Yum!"),
            (Key::Pet, "Blub blub \u{2665}"),
            (Key::Play, "Wheee!"),
        ] {
            assert!(a.press(k));
            assert_eq!(a.msg, Some(msg));
            assert!(a.any_react());
            for _ in 0..(12 * 16) {
                a.update(0.0625);
            }
            assert!(!a.any_react());
            assert_eq!(a.state, State::Idle);
            assert_eq!(a.msg, None);
        }
    }

    #[test]
    fn restart_during_animation_never_panics() {
        let mut a = App::new(80, 24, 2);
        for i in 0..500 {
            a.press([Key::Feed, Key::Pet, Key::Play][i % 3]);
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

    #[test]
    fn presses_stack_extend_and_cap() {
        let mut a = App::new(80, 24, 7);
        assert!(a.press(Key::Feed));
        let d1 = a.feed.dur;
        assert!(a.press(Key::Pet));
        assert!(a.press(Key::Play));
        assert!(a.feed.on && a.pet.on && a.play.on, "reactions run together");
        assert!(a.press(Key::Feed));
        assert!(
            (a.feed.dur - d1 - EXTEND).abs() < 1e-4,
            "repeat extends ~1 s"
        );
        assert_eq!(a.fish.len(), 2);
        let hearts = a.hearts.len();
        a.press(Key::Pet);
        assert!(a.hearts.len() > hearts);
        // 5 counted per key per 3 s; the 6th is silently ignored.
        let mut b = App::new(80, 24, 8);
        let n = (0..10).filter(|_| b.press(Key::Feed)).count();
        assert_eq!(n, 5);
        assert_eq!(b.fish.len(), 5);
        for _ in 0..(3.2 * 16.0) as usize {
            b.update(0.0625);
        }
        assert!(b.press(Key::Feed), "counts again after the window");
        // The feelings line shows the latest counted press.
        let mut c = App::new(80, 24, 9);
        c.press(Key::Feed);
        c.press(Key::Pet);
        assert_eq!(c.msg, Some(Key::Pet.msg()));
        c.press(Key::Play);
        assert_eq!(c.msg, Some(Key::Play.msg()));
    }
}
