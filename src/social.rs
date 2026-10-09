//! Visits, messages between pets, the shared-window layout and play moves.
//! Pure state logic: sockets live in `link`, drawing in `art`.
use crate::app::Kind;
use crate::app::{App, Behavior, Flyer, Heart, Key, State, ease};
use crate::art::{Body, PRESETS};
use crate::link::Msg;
use rand::Rng;
use rand::seq::SliceRandom;

/// Seconds to swim out of / into a window.
pub const LEAVE_T: f32 = 1.5;
/// Seconds a guest stays (from arrival) before swimming home.
pub const VISIT_LEN: f32 = 18.0;
const MOVE_LEN: f32 = 4.0;
/// Width of one squid at rest, in multiples of `maxw`, used to decide when to shrink.
const FOOT: f32 = 3.7;
const HELLO: f32 = 0.3;
const PEER_TIMEOUT: f32 = 1.0;
/// Minimum seconds between sent fish, and between sent ink puffs.
pub const SEND_GAP: f32 = 5.0;
const AWAY_FAILSAFE: f32 = 45.0;

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum Home {
    Home,
    Pending { to: u32, t0: f32 },
    Leaving { to: u32, u: f32, side: f32 },
    Away { host: u32, since: f32 },
    Returning { u: f32, dur: f32, side: f32 },
}

#[derive(Clone, Copy, Debug)]
pub struct Peer {
    pub seen: f32,
    pub away: bool,
    pub accept: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GPhase {
    Entering,
    Present,
    Leaving,
}

/// Per-squid play offsets for the current move.
#[derive(Clone, Copy, Default, Debug)]
pub struct Fx {
    pub dx: f32,
    pub dy: f32,
    pub bright: f32,
    pub dark: f32,
}

pub struct Guest {
    pub pid: u32,
    pub kind: Kind,
    pub color: usize,
    pub phase: GPhase,
    pub side: f32,
    pub u: f32,
    pub age: f32,
    pub x: f32,
    pub from_x: f32,
    pub to_x: f32,
    pub t0: f32,
    pub tag: f32,
    pub fx: Fx,
    pub touch: crate::app::Touch,
    /// Seconds left of a startle at a toy mouse, and the mouse's direction.
    pub startle: f32,
    pub startle_dir: f32,
    /// Ink splat on a cat guest: (seconds left, x offset).
    pub splat: Option<(f32, f32)>,
}

pub enum In {
    Fish,
    Ink,
    Mouse,
    Bubbles(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mv {
    Chase,
    HighFive,
    InkTag,
    Flash,
}

pub struct PlayCtl {
    pub mv: Mv,
    pub u: f32,
    /// Participants left to right; 0 is the host.
    parts: Vec<u32>,
    lead: usize,
    dir: f32,
    a: usize,
    fired: u8,
}

/// Own-visit interval (seconds) and cooldown after any visit.
const VISIT_MIN: f32 = 1200.0;
const VISIT_MAX: f32 = 1800.0;
const VISIT_COOLDOWN: f32 = 600.0;

fn sin_pulse(s: f32, len: f32) -> f32 {
    if (0.0..len).contains(&s) {
        (std::f32::consts::PI * s / len).sin()
    } else {
        0.0
    }
}

impl Guest {
    /// Sideways jerk (cells) of a startle.
    fn jerk(&self) -> f32 {
        if self.startle > 0.0 {
            2.0 * ease(((crate::app::STARTLE_T - self.startle) / 0.15).min(1.0))
        } else {
            0.0
        }
    }
}

impl App {
    /// Turn on visits and messages for this process.
    pub fn enable_social(&mut self, pid: u32) {
        self.social = true;
        self.pid = pid;
        self.next_visit = self.t + self.vrng.random_range(VISIT_MIN..VISIT_MAX);
        self.next_bubble = self.t + self.vrng.random_range(45.0..90.0);
        let h = self.hello();
        self.outbox.push((None, h));
    }

    /// Tell neighbors we are going (quit): guests are sent home, hosts drop us.
    pub fn shutdown(&mut self) {
        if self.social {
            self.outbox.push((None, Msg::Bye));
        }
    }

    fn hello(&self) -> Msg {
        Msg::Hello {
            away: self.home != Home::Home,
            accept: self.w >= 40 && self.h >= 12,
        }
    }

    pub fn is_away(&self) -> bool {
        matches!(self.home, Home::Away { .. })
    }

    /// Unscaled layout of this process's own pet.
    fn geo_full(&self) -> crate::art::Geo {
        if self.kind == Kind::Cat {
            crate::cat_art::geo_cat(self.w, self.h, 1.0)
        } else {
            crate::art::geo(self.w, self.h)
        }
    }

    pub fn kind_of(&self, pid: u32) -> Kind {
        if pid == 0 {
            self.kind
        } else {
            self.guests
                .iter()
                .find(|g| g.pid == pid)
                .map_or(self.kind, |g| g.kind)
        }
    }

    fn note(&mut self, s: &str, secs: f32) {
        self.note = Some((s.to_string(), secs));
    }

    fn rand_side(&mut self) -> f32 {
        if self.vrng.random_bool(0.5) {
            1.0
        } else {
            -1.0
        }
    }

    /// Cells beyond the window edge where a squid is fully out of sight.
    fn margin(&self) -> f32 {
        3.0 * self.geo_full().maxw + self.h as f32 * 0.5 + 4.0
    }

    fn off_x(&self, side: f32) -> f32 {
        if side < 0.0 {
            -self.margin()
        } else {
            self.w as f32 + self.margin()
        }
    }

    fn exit_dist(&self) -> f32 {
        let g = self.geo_full();
        let room = (self.w as f32 / 2.0 - g.maxw * 2.2).max(0.0);
        self.w as f32 / 2.0 + self.margin() + room
    }

    /// Host squid's extra horizontal offset while swimming out of / into its window.
    pub fn home_ox(&self) -> f32 {
        match self.home {
            Home::Leaving { u, side, .. } => side * self.exit_dist() * ease(u / LEAVE_T),
            Home::Returning { u, dur, side } => side * self.exit_dist() * (1.0 - ease(u / dur)),
            _ => 0.0,
        }
    }

    fn cands(&self) -> Vec<u32> {
        self.peers
            .iter()
            .filter(|(_, p)| !p.away && p.accept)
            .map(|(k, _)| *k)
            .collect()
    }

    fn pick_cand(&mut self) -> Option<u32> {
        let c = self.cands();
        if c.is_empty() {
            None
        } else {
            Some(c[self.vrng.random_range(0..c.len())])
        }
    }

    // ---- keys ----

    /// The `v` key.
    pub fn visit_key(&mut self) {
        if !self.social {
            self.note("No one around...", 2.0);
            return;
        }
        match self.home {
            Home::Home => {
                if !self.guests.is_empty() {
                    self.recall_guests();
                } else if self.reserved.is_empty() {
                    match self.pick_cand() {
                        Some(p) => self.begin_visit(p),
                        None => self.note("No one around...", 2.0),
                    }
                }
            }
            Home::Away { host, .. } => self.outbox.push((Some(host), Msg::Recall)),
            _ => {}
        }
    }

    fn begin_visit(&mut self, to: u32) {
        self.home = Home::Pending { to, t0: self.t };
        self.outbox.push((Some(to), Msg::Req));
    }

    fn recall_guests(&mut self) {
        for i in 0..self.guests.len() {
            self.leave_guest(i);
        }
    }

    fn leave_guest(&mut self, i: usize) {
        let to_x = self.off_x(self.guests[i].side);
        let g = &mut self.guests[i];
        if g.phase != GPhase::Leaving {
            g.phase = GPhase::Leaving;
            g.u = 0.0;
            g.from_x = g.x;
            g.to_x = to_x;
        }
    }

    /// Called after a counted feed / space press: maybe send something to a neighbor.
    pub(crate) fn after_press(&mut self, k: Key) {
        if self.home != Home::Home {
            return;
        }
        let g = self.geo();
        let y = (g.top + g.mh + g.hh / 2) as f32;
        match k {
            Key::Feed => {
                if self.t - self.last_fish_send >= SEND_GAP
                    && !self.cands().is_empty()
                    && self.vrng.random_range(0..3) == 0
                {
                    self.last_fish_send = self.t;
                    let side = self.rand_side();
                    self.flyers.push(Flyer {
                        x: self.mouth_x(),
                        y,
                        vx: side * 45.0,
                        ink: false,
                        mouse: false,
                        hit_x: None,
                        target: None,
                    });
                    if let Some(p) = self.pick_cand() {
                        self.outbox.push((Some(p), Msg::Fish));
                    }
                }
            }
            Key::Play => {
                let mouse = self.kind == Kind::Cat;
                // A guest of the other kind in this scene is the local target.
                let want = if mouse { Kind::Squid } else { Kind::Cat };
                let tgt = self
                    .guests
                    .iter()
                    .find(|g| g.kind == want)
                    .map(|g| (g.pid, g.x));
                let cand = self.pick_cand();
                if self.t - self.last_ink_send >= SEND_GAP && (cand.is_some() || tgt.is_some()) {
                    self.last_ink_send = self.t;
                    let mx = self.mouth_x();
                    let side = match tgt {
                        Some((_, gx)) => {
                            if gx >= mx {
                                1.0
                            } else {
                                -1.0
                            }
                        }
                        None => self.rand_side(),
                    };
                    self.flyers.push(Flyer {
                        x: mx,
                        y: if mouse { g_floor(self) as f32 } else { y },
                        vx: side * 45.0,
                        ink: !mouse,
                        mouse,
                        hit_x: None,
                        target: tgt.map(|t| t.0),
                    });
                    let m = if mouse { Msg::Mouse } else { Msg::Ink };
                    if let Some(p) = cand {
                        self.outbox.push((Some(p), m));
                    }
                }
            }
            Key::Pet => {}
        }
    }

    // ---- incoming messages ----

    pub fn on_msg(&mut self, from: u32, m: Msg) {
        if !self.social || from == self.pid {
            return;
        }
        let t = self.t;
        match m {
            Msg::Hello { away, accept } => {
                self.peers.insert(
                    from,
                    Peer {
                        seen: t,
                        away,
                        accept,
                    },
                );
                return;
            }
            Msg::Bye => {
                self.peers.remove(&from);
                self.drop_peer(from);
                return;
            }
            _ => {
                self.peers.entry(from).or_insert(Peer {
                    seen: t,
                    away: false,
                    accept: true,
                });
                if let Some(p) = self.peers.get_mut(&from) {
                    p.seen = t;
                }
            }
        }
        match m {
            Msg::Req => self.on_req(from),
            Msg::Ack => {
                if let Home::Pending { to, .. } = self.home
                    && to == from
                {
                    let side = self.rand_side();
                    self.home = Home::Leaving { to, u: 0.0, side };
                }
            }
            Msg::Nak => {
                if let Home::Pending { to, .. } = self.home
                    && to == from
                {
                    self.home = Home::Home;
                }
            }
            Msg::Arrive(c) => self.on_arrive(from, c as usize, Kind::Squid),
            Msg::ArriveCat(c) => self.on_arrive(from, c as usize, Kind::Cat),
            Msg::Color(c) => {
                if let Some(g) = self.guests.iter_mut().find(|g| g.pid == from) {
                    let n = if g.kind == Kind::Cat {
                        crate::cat::BREEDS.len()
                    } else {
                        PRESETS.len()
                    };
                    g.color = (c as usize).min(n - 1);
                }
            }
            Msg::Recall => {
                if let Some(i) = self.guests.iter().position(|g| g.pid == from) {
                    self.leave_guest(i);
                }
            }
            Msg::Home => {
                if let Home::Away { host, .. } = self.home
                    && host == from
                {
                    let side = self.rand_side();
                    self.home = Home::Returning {
                        u: 0.0,
                        dur: LEAVE_T,
                        side,
                    };
                }
            }
            Msg::Fish => self.queue_in(In::Fish, 0.6),
            Msg::Ink => self.queue_in(In::Ink, 0.6),
            Msg::Mouse => self.queue_in(In::Mouse, 0.6),
            Msg::Bubbles(x) => self.queue_in(In::Bubbles(x), 0.8),
            Msg::Hello { .. } | Msg::Bye => {}
        }
    }

    fn queue_in(&mut self, i: In, delay: f32) {
        if self.home == Home::Home {
            self.inbox.push((delay, i));
        }
    }

    /// A neighbor is gone (quit or lost): bring guests / visitors home.
    fn drop_peer(&mut self, pid: u32) {
        self.guests.retain(|g| g.pid != pid);
        self.reserved.retain(|r| r.0 != pid);
        match self.home {
            Home::Away { host, .. } if host == pid => {
                let side = self.rand_side();
                self.home = Home::Returning {
                    u: 0.0,
                    dur: 0.5,
                    side,
                };
            }
            Home::Pending { to, .. } if to == pid => self.home = Home::Home,
            _ => {}
        }
    }

    fn can_host(&self) -> bool {
        self.w >= 40 && self.h >= 12
    }

    fn on_req(&mut self, from: u32) {
        let ack = match self.home {
            Home::Home => self.can_host(),
            Home::Pending { to, .. } if to == from => {
                // Both set off at once: the lower pid keeps its trip, the other hosts.
                if self.pid < from {
                    false
                } else {
                    self.home = Home::Home;
                    self.can_host()
                }
            }
            _ => false,
        };
        if ack {
            self.reserved.push((from, self.t + 4.0));
            self.outbox.push((Some(from), Msg::Ack));
        } else {
            self.outbox.push((Some(from), Msg::Nak));
        }
    }

    fn on_arrive(&mut self, from: u32, color: usize, kind: Kind) {
        if self.home != Home::Home || !self.can_host() {
            self.outbox.push((Some(from), Msg::Home));
            return;
        }
        self.reserved.retain(|r| r.0 != from);
        self.guests.retain(|g| g.pid != from);
        let n = if kind == Kind::Cat {
            crate::cat::BREEDS.len()
        } else {
            PRESETS.len()
        };
        let color = color.min(n - 1);
        let left = self.guests.iter().filter(|g| g.side < 0.0).count();
        let right = self.guests.len() - left;
        let side = if left <= right { -1.0 } else { 1.0 };
        let x = self.off_x(side);
        let t0 = self.vrng.random_range(0.0..20.0);
        self.guests.push(Guest {
            pid: from,
            kind,
            color,
            phase: GPhase::Entering,
            side,
            u: 0.0,
            age: 0.0,
            x,
            from_x: x,
            to_x: x,
            t0,
            tag: 3.0,
            fx: Fx::default(),
            touch: Default::default(),
            startle: 0.0,
            startle_dir: 0.0,
            splat: None,
        });
        let name = if kind == Kind::Cat {
            crate::cat::BREEDS[color].name
        } else {
            PRESETS[color].0
        };
        self.note(&format!("A {name} friend is here!"), 3.0);
    }

    // ---- per-frame ----

    fn go_away(&mut self) {
        self.feed.on = false;
        self.pet.on = false;
        self.play.on = false;
        self.fish.clear();
        self.hearts.clear();
        self.ink.clear();
        self.flyers.clear();
        self.inbox.clear();
        self.msg = None;
        self.note = None;
        self.jolt = 0.0;
    }

    pub(crate) fn update_flyers(&mut self, dt: f32) {
        if self.flyers.is_empty() {
            return;
        }
        let w = self.w as f32;
        let mut hits = vec![];
        let mut mice = vec![];
        for f in &mut self.flyers {
            f.x += f.vx * dt;
            if let Some(hx) = f.hit_x
                && ((f.vx > 0.0 && f.x >= hx) || (f.vx < 0.0 && f.x <= hx))
            {
                f.hit_x = None;
                if f.mouse {
                    // the mouse keeps running off-screen
                    mice.push(f.vx.signum());
                } else {
                    f.vx = 0.0;
                    hits.push((f.x, f.y));
                }
            }
        }
        let mut gh = vec![];
        for f in &mut self.flyers {
            if let Some(pid) = f.target
                && let Some(g) = self.guests.iter().find(|g| g.pid == pid)
                && ((f.vx > 0.0 && f.x >= g.x) || (f.vx < 0.0 && f.x <= g.x))
            {
                f.target = None;
                if !f.mouse {
                    f.vx = 0.0;
                }
                gh.push((pid, f.vx.signum(), f.mouse));
            }
        }
        for (pid, d, mouse) in gh {
            if let Some(g) = self.guests.iter_mut().find(|g| g.pid == pid) {
                if mouse {
                    g.startle = crate::app::STARTLE_T;
                    g.startle_dir = d;
                } else {
                    g.splat = Some((2.0, 0.0));
                    self.note = Some(("Pfft!".to_string(), 2.0));
                }
            }
        }
        for d in mice {
            self.startle = crate::app::STARTLE_T;
            self.startle_dir = d;
        }
        self.flyers
            .retain(|f| (f.vx != 0.0 || f.hit_x.is_some()) && f.x > -8.0 && f.x < w + 8.0);
        for (x, y) in hits {
            if self.kind == Kind::Cat {
                // inked: dark splat on the fur, sneeze, shake it off
                self.splat = Some((2.0, x - self.mouth_x()));
                self.state = State::IdleBehavior(Behavior::InkPuff);
                self.st = 0.0;
                self.cat_sneeze();
                self.note("Pfft!", 2.0);
                continue;
            }
            self.spawn_ink_v(x, y, 16, 0.0, 14.0);
            self.jolt = 0.4;
            self.note("Eep!", 2.0);
        }
    }

    pub(crate) fn social_update(&mut self, dt: f32) {
        let t = self.t;
        self.hello_t -= dt;
        if self.hello_t <= 0.0 {
            self.hello_t = HELLO;
            let h = self.hello();
            self.outbox.push((None, h));
        }
        // Neighbors that stopped talking are gone.
        let lost: Vec<u32> = self
            .peers
            .iter()
            .filter(|(_, p)| t - p.seen > PEER_TIMEOUT)
            .map(|(k, _)| *k)
            .collect();
        for k in lost {
            self.peers.remove(&k);
            self.drop_peer(k);
        }
        self.reserved.retain(|r| r.1 > t);
        self.home_update(dt);
        self.guests_update(dt);
        self.play_update(dt);
        self.inbox_update(dt);
        self.timers_update();
    }

    fn home_update(&mut self, dt: f32) {
        match self.home {
            Home::Home => {}
            Home::Pending { to, t0 } => {
                if self.t - t0 > 0.8 || !self.peers.contains_key(&to) {
                    self.home = Home::Home;
                }
            }
            Home::Leaving { to, u, side } => {
                let u = u + dt;
                if u < LEAVE_T {
                    self.home = Home::Leaving { to, u, side };
                } else if self.peers.contains_key(&to) {
                    let m = if self.kind == Kind::Cat {
                        Msg::ArriveCat(self.color_idx() as u8)
                    } else {
                        Msg::Arrive(self.color_idx() as u8)
                    };
                    self.outbox.push((Some(to), m));
                    self.home = Home::Away {
                        host: to,
                        since: self.t,
                    };
                    self.go_away();
                } else {
                    self.home = Home::Returning {
                        u: 0.0,
                        dur: 0.5,
                        side,
                    };
                }
            }
            Home::Away { host, since } => {
                if !self.peers.contains_key(&host) || self.t - since > AWAY_FAILSAFE {
                    let side = self.rand_side();
                    self.home = Home::Returning {
                        u: 0.0,
                        dur: 0.5,
                        side,
                    };
                }
            }
            Home::Returning { u, dur, side } => {
                let u = u + dt;
                if u >= dur {
                    self.home = Home::Home;
                    self.next_visit = self.t + self.vrng.random_range(VISIT_MIN..VISIT_MAX);
                } else {
                    self.home = Home::Returning { u, dur, side };
                }
            }
        }
    }

    /// Left-to-right layout: `None` is the host, `Some(i)` is `guests[i]`.
    pub fn order(&self) -> Vec<Option<usize>> {
        let mut v: Vec<Option<usize>> = (0..self.guests.len())
            .filter(|&i| self.guests[i].side < 0.0)
            .rev()
            .map(Some)
            .collect();
        v.push(None);
        v.extend(
            (0..self.guests.len())
                .filter(|&i| self.guests[i].side >= 0.0)
                .map(Some),
        );
        v
    }

    fn guests_update(&mut self, dt: f32) {
        let t = self.t;
        // Drop guests whose home went quiet.
        let gone: Vec<u32> = self
            .guests
            .iter()
            .filter(|g| !self.peers.contains_key(&g.pid))
            .map(|g| g.pid)
            .collect();
        self.guests.retain(|g| !gone.contains(&g.pid));
        let _ = t;
        for i in 0..self.guests.len() {
            let g = &mut self.guests[i];
            g.age += dt;
            g.tag = (g.tag - dt).max(0.0);
            g.startle = (g.startle - dt).max(0.0);
            if let Some((l, x)) = g.splat {
                g.splat = (l > dt).then_some((l - dt, x));
            }
            if g.phase == GPhase::Present && g.age >= VISIT_LEN {
                self.leave_guest(i);
            }
        }
        let w = self.w as f32;
        let n = 1 + self.guests.len();
        let cx = self.geo().cx as f32;
        let order = self.order();
        let mut done = vec![];
        for (k, who) in order.iter().enumerate() {
            let sx = w * (k as f32 + 0.5) / n as f32;
            match who {
                None => {
                    let target = if n == 1 { 0.0 } else { sx - cx };
                    self.host_dx += (target - self.host_dx) * (1.0 - (-6.0 * dt).exp());
                    if (target - self.host_dx).abs() < 0.01 {
                        self.host_dx = target;
                    }
                }
                Some(i) => {
                    let g = &mut self.guests[*i];
                    match g.phase {
                        GPhase::Entering => {
                            g.u += dt;
                            g.x = g.from_x + (sx - g.from_x) * ease(g.u / LEAVE_T);
                            if g.u >= LEAVE_T {
                                g.phase = GPhase::Present;
                            }
                        }
                        GPhase::Present => {
                            g.x += (sx - g.x) * (1.0 - (-6.0 * dt).exp());
                        }
                        GPhase::Leaving => {
                            g.u += dt;
                            g.x = g.from_x + (g.to_x - g.from_x) * ease(g.u / LEAVE_T);
                            if g.u >= LEAVE_T {
                                done.push(g.pid);
                            }
                        }
                    }
                }
            }
        }
        for pid in done {
            self.outbox.push((Some(pid), Msg::Home));
            self.guests.retain(|g| g.pid != pid);
        }
        // Shrink so everyone fits; ease about 0.7 s.
        let n = 1 + self.guests.len();
        let full = self.geo_full().maxw;
        let target = if n == 1 {
            1.0
        } else {
            (w / (n as f32 * FOOT * full)).min(1.0)
        };
        self.scale = target + (self.scale - target) * (-6.5 * dt).exp();
        if (self.scale - target).abs() < 0.002 {
            self.scale = target;
        }
    }

    fn pos_of(&self, pid: u32) -> f32 {
        if pid == 0 {
            self.geo().cx as f32 + self.host_dx
        } else {
            self.guests
                .iter()
                .find(|g| g.pid == pid)
                .map_or(0.0, |g| g.x)
        }
    }

    fn start_move(&mut self) {
        if self.bag.is_empty() {
            let mut b = vec![Mv::Chase, Mv::HighFive, Mv::InkTag, Mv::Flash];
            b.shuffle(&mut self.vrng);
            self.bag = b;
        }
        let mv = self.bag.pop().unwrap_or(Mv::Chase);
        let mut parts = vec![];
        for who in self.order() {
            match who {
                None => parts.push(0),
                Some(i) if self.guests[i].phase == GPhase::Present => {
                    parts.push(self.guests[i].pid)
                }
                _ => {}
            }
        }
        let n = parts.len().max(2);
        let lead = self.vrng.random_range(0..n);
        let dir = if self.vrng.random_bool(0.5) {
            1.0
        } else {
            -1.0
        };
        let a = self.vrng.random_range(0..(parts.len().max(2) - 1));
        self.play_ctl = Some(PlayCtl {
            mv,
            u: 0.0,
            parts,
            lead,
            dir,
            a,
            fired: 0,
        });
    }

    fn play_update(&mut self, dt: f32) {
        let present = self.guests.iter().any(|g| g.phase == GPhase::Present);
        self.fx = Fx::default();
        for g in &mut self.guests {
            g.fx = Fx::default();
        }
        if !present || self.home != Home::Home {
            self.play_ctl = None;
            self.bag.clear();
            return;
        }
        if self.play_ctl.is_none() {
            self.start_move();
        }
        let Some(mut ctl) = self.play_ctl.take() else {
            return;
        };
        // Someone may have left since the move began.
        ctl.parts.retain(|&p| {
            p == 0
                || self
                    .guests
                    .iter()
                    .any(|g| g.pid == p && g.phase == GPhase::Present)
        });
        ctl.u += dt;
        let n = ctl.parts.len();
        if n >= 2 {
            let pos: Vec<f32> = ctl.parts.iter().map(|&p| self.pos_of(p)).collect();
            let g = self.geo();
            let slotw = self.w as f32 / (1 + self.guests.len()) as f32;
            let amp = (slotw * 0.3).max(4.0);
            self.move_events(&mut ctl, &pos);
            for i in 0..n {
                let fx = mv_fx(&ctl, i, &pos, amp, 2.6 * g.maxw);
                let pid = ctl.parts[i];
                if pid == 0 {
                    self.fx = fx;
                } else if let Some(gu) = self.guests.iter_mut().find(|g| g.pid == pid) {
                    gu.fx = fx;
                }
            }
        }
        if ctl.u < MOVE_LEN {
            self.play_ctl = Some(ctl);
        }
    }

    fn move_events(&mut self, ctl: &mut PlayCtl, pos: &[f32]) {
        let n = ctl.parts.len();
        let g = self.geo();
        let a = ctl.a.min(n - 2);
        let b = a + 1;
        match ctl.mv {
            Mv::HighFive => {
                let mid = (pos[a] + pos[b]) / 2.0;
                let y = (g.top + g.mh + g.hh) as f32 + g.al as f32 * 0.45;
                for (bit, at, count) in [(1u8, 1.3, 5), (2u8, 2.2, 4)] {
                    if ctl.u >= at && ctl.fired & bit == 0 {
                        ctl.fired |= bit;
                        for _ in 0..count {
                            let x = mid + self.vrng.random_range(-3.0..=3.0);
                            let age = -self.vrng.random_range(0.0..0.5);
                            self.hearts.push(Heart {
                                x,
                                y,
                                age,
                                life: 2.0,
                            });
                        }
                    }
                }
            }
            Mv::InkTag => {
                let y = (g.top + g.mh + g.hh / 2) as f32;
                for (bit, at, from, to) in [(1u8, 0.3, a, b), (2u8, 2.3, b, a)] {
                    if ctl.u >= at && ctl.fired & bit == 0 {
                        ctl.fired |= bit;
                        let d = pos[to] - pos[from];
                        let sign = if d >= 0.0 { 1.0 } else { -1.0 };
                        let dist = d.abs().max(10.0);
                        let from_cat = self.kind_of(ctl.parts[from]) == Kind::Cat;
                        let to_pid = ctl.parts[to];
                        if from_cat {
                            // the cat's turn: it throws the toy mouse
                            self.flyers.push(Flyer {
                                x: pos[from] + sign * g.maxw * 0.6,
                                y: g_floor(self) as f32,
                                vx: sign * 45.0,
                                ink: false,
                                mouse: true,
                                hit_x: (to_pid == self.pid).then_some(pos[to]),
                                target: (to_pid != self.pid).then_some(to_pid),
                            });
                        } else {
                            let saved = self.kind;
                            self.kind = Kind::Squid;
                            self.spawn_ink_v(
                                pos[from] + sign * g.maxw * 0.6,
                                y,
                                26,
                                sign * dist * 2.4,
                                3.0,
                            );
                            self.kind = saved;
                            if self.kind_of(to_pid) == Kind::Cat {
                                if to_pid == self.pid {
                                    self.splat = Some((2.0, 0.0));
                                    self.state = State::IdleBehavior(Behavior::InkPuff);
                                    self.st = 0.0;
                                    self.cat_sneeze();
                                } else if let Some(gu) =
                                    self.guests.iter_mut().find(|gu| gu.pid == to_pid)
                                {
                                    gu.splat = Some((2.0, 0.0));
                                }
                                self.note("Pfft!", 2.0);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn inbox_update(&mut self, dt: f32) {
        if self.inbox.is_empty() {
            return;
        }
        let ready = self.home == Home::Home;
        let mut due = vec![];
        for (d, _) in &mut self.inbox {
            *d -= dt;
        }
        let mut i = 0;
        while i < self.inbox.len() {
            if self.inbox[i].0 <= 0.0 {
                due.push(self.inbox.remove(i).1);
            } else {
                i += 1;
            }
        }
        if !ready {
            return;
        }
        for item in due {
            self.receive(item);
        }
    }

    fn receive(&mut self, item: In) {
        let g = self.geo();
        let w = self.w as f32;
        let y = (g.top + g.mh + g.hh / 2) as f32;
        match item {
            In::Fish => {
                let side = self.rand_side();
                let x = if side < 0.0 { -3.0 } else { w + 3.0 };
                self.add_fish((x, y * 0.6));
                self.msg = Some("Yum!");
                self.state = State::Idle;
            }
            In::Ink => {
                let side = self.rand_side();
                let x = if side < 0.0 { -2.0 } else { w + 2.0 };
                let hit = self.mouth_x() - side * g.maxw * 1.4;
                self.flyers.push(Flyer {
                    x,
                    y,
                    vx: -side * 40.0,
                    ink: true,
                    mouse: false,
                    hit_x: Some(hit),
                    target: None,
                });
            }
            In::Mouse => {
                let side = self.rand_side();
                let x = if side < 0.0 { -2.0 } else { w + 2.0 };
                let hit = self.mouth_x() - side * g.maxw * 1.4;
                self.flyers.push(Flyer {
                    x,
                    y: g_floor(self) as f32,
                    vx: -side * 45.0,
                    ink: false,
                    mouse: true,
                    hit_x: Some(hit),
                    target: None,
                });
            }
            In::Bubbles(xf) => {
                for k in 0..6 {
                    let x = w * xf.clamp(0.05, 0.95) + self.vrng.random_range(-4.0..=4.0);
                    self.bubbles.push(crate::app::Bubble {
                        x,
                        y: self.h as f32 - 3.0 + k as f32 * 1.6,
                        vy: self.vrng.random_range(2.5..5.0),
                        ch: if self.kind == Kind::Cat {
                            ['.', '*', '\u{b0}'][self.vrng.random_range(0..3)]
                        } else {
                            ['o', 'O', '\u{b0}'][self.vrng.random_range(0..3)]
                        },
                        phase: self.vrng.random_range(0.0..std::f32::consts::TAU),
                    });
                }
                if self.state == State::Idle && !self.any_react() {
                    let b = if w * xf < self.mouth_x() {
                        Behavior::LookLeft
                    } else {
                        Behavior::LookRight
                    };
                    self.state = State::IdleBehavior(b);
                    self.st = 0.0;
                }
            }
        }
    }

    fn timers_update(&mut self) {
        let t = self.t;
        if self.home != Home::Home || !self.guests.is_empty() || !self.reserved.is_empty() {
            // Involved in a visit (as guest or host): no own visit for the cooldown after it.
            self.next_visit = self.next_visit.max(t + VISIT_COOLDOWN);
        }
        if t >= self.next_visit {
            let free = self.home == Home::Home
                && self.guests.is_empty()
                && self.reserved.is_empty()
                && !self.any_react();
            match if free { self.pick_cand() } else { None } {
                Some(p) => {
                    self.begin_visit(p);
                    self.next_visit = t + self.vrng.random_range(VISIT_MIN..VISIT_MAX);
                }
                None => self.next_visit = t + 3.0,
            }
        }
        if t >= self.next_bubble {
            self.next_bubble = t + self.vrng.random_range(45.0..90.0);
            if self.home == Home::Home
                && let Some(p) = self.pick_cand()
            {
                let xf = self.vrng.random_range(0.15..0.85f32);
                self.outbox.push((Some(p), Msg::Bubbles(xf)));
                let g = self.geo();
                for k in 0..6 {
                    let x = self.spawn_x() + self.vrng.random_range(-2.0..=2.0);
                    self.bubbles.push(crate::app::Bubble {
                        x,
                        y: (g.top + g.mh) as f32 + k as f32 * 1.6,
                        vy: self.vrng.random_range(3.0..5.0),
                        ch: if self.kind == Kind::Cat { '*' } else { 'o' },
                        phase: self.vrng.random_range(0.0..std::f32::consts::TAU),
                    });
                }
            }
        }
    }

    // ---- drawing data ----

    pub fn bodies(&self) -> Vec<Body> {
        let mut v: Vec<Body> = self.guests.iter().map(|g| self.guest_body(g)).collect();
        if !self.is_away() {
            v.push(self.host_body());
        }
        v
    }

    fn guest_body(&self, g: &Guest) -> Body {
        let cx = self.geo().cx;
        let tt = self.t + g.t0;
        if g.kind == Kind::Cat {
            return Body {
                kind: Kind::Cat,
                breed: g.color,
                breed_from: g.color,
                bfade: 1.0,
                boop: (g.touch.boop / crate::app::BOOP).clamp(0.0, 1.0),
                head_dy: g.touch.dy(),
                dir: g.touch.dir,
                tail: g.touch.tail_side(),
                pose: crate::cat::CatPose {
                    tail_tw: if (tt % 6.0) < 0.5 {
                        (tt * 30.0).sin()
                    } else {
                        0.0
                    },
                    sneeze: g.splat.map_or(0.0, |(l, _)| sin_pulse(1.7 - l, 0.45)),
                    wiggle: g.splat.map_or(0.0, |(l, _)| {
                        if l < 1.4 {
                            (tt * 40.0).sin() * l / 1.4
                        } else {
                            0.0
                        }
                    }),
                    ..Default::default()
                },
                splat: g.splat,
                cx,
                dx: g.x - cx as f32 + g.fx.dx,
                dy: g.fx.dy,
                scale: self.scale,
                blush: (g.touch.stroke / 0.5).min(1.0),
                bright: g.fx.bright,
                dark: g.fx.dark,
                closed: g.touch.stroke > 0.0,
                blink: tt % 5.0 < 0.15,
                lean: 0.25 * g.touch.lean(),
                food_x: g.x,
                t: tt,
                touch: ease(g.touch.touch),
                touch_at: g.touch.at,
                ruffle: g.touch.ruffle_k(),
                ruffle_at: g.touch.ruffle_at,
                tag: (g.tag > 0.0).then_some("guest"),
                ..Body::default()
            };
        }
        Body {
            pal: PRESETS[g.color].1,
            cx,
            dx: g.x - cx as f32 + g.fx.dx - g.startle_dir * g.jerk(),
            bang: g.startle > 0.0,
            dy: 1.0 + 0.6 * (tt * 0.35).sin() + g.fx.dy + g.touch.dy(),
            scale: self.scale,
            puff: 1.0
                + 0.3
                    * sin_pulse(crate::app::STARTLE_T - g.startle, crate::app::STARTLE_T)
                    * (g.startle > 0.0) as u8 as f32,
            curl: 0.0,
            blush: (g.touch.stroke / 0.5).min(1.0),
            flash: 0.0,
            bright: g.fx.bright,
            dark: g.fx.dark,
            look: 0,
            closed: g.touch.stroke > 0.0,
            blink: tt % 5.0 < 0.15,
            yawn: false,
            lean: 0.2 + g.touch.lean(),
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
}

/// Where participant `i` is in the current play move.
fn mv_fx(ctl: &PlayCtl, i: usize, pos: &[f32], amp: f32, want: f32) -> Fx {
    let n = ctl.parts.len();
    let u = ctl.u;
    let mut fx = Fx::default();
    let a = ctl.a.min(n - 2);
    let b = a + 1;
    match ctl.mv {
        Mv::Chase => {
            let ph = (u >= 2.0) as usize;
            let local = u - 2.0 * ph as f32;
            let leader = (ctl.lead % n + ph) % n;
            let d = if ph == 1 { -ctl.dir } else { ctl.dir };
            let sh = |x: f32| ease(x / 0.6) * (1.0 - ease((x - 1.4) / 0.6));
            if i == leader {
                let s = sh(local);
                fx.dx = d * amp * s;
                fx.dy = -s;
            } else {
                let rank = ((i + n - leader) % n) as f32;
                let s = sh(local - 0.35 * rank);
                fx.dx = d * amp * 0.8 * s;
                fx.dy = -0.5 * s;
            }
        }
        Mv::HighFive => {
            if i == a || i == b {
                let other = if i == a { b } else { a };
                let sep = pos[other] - pos[i];
                let shape = ease(u / 1.2) * (1.0 - ease((u - 3.0) / 0.8));
                let mv = (sep.abs() - want).max(0.0) / 2.0;
                fx.dx = sep.signum() * mv * shape;
                fx.dy = -0.5 * shape;
                fx.bright = 0.5 * sin_pulse(u - 1.2, 1.4);
            }
        }
        Mv::InkTag => {
            let env = |s: f32| ease((u - s) / 0.2) * (1.0 - ease((u - s - 0.8) / 0.3));
            if i == b {
                fx.dark = env(0.7);
                fx.dy = 1.5 * fx.dark;
            } else if i == a {
                fx.dark = env(2.7);
                fx.dy = 1.5 * fx.dark;
            }
        }
        Mv::Flash => {
            let turn = 0.7;
            let cycle = turn * n as f32;
            fx.bright = sin_pulse((u - 0.2 - turn * i as f32).rem_euclid(cycle), turn);
            if u < 0.2 {
                fx.bright = 0.0;
            }
        }
    }
    fx
}

/// Row the toy mouse runs along (the floor, just above the status lines).
fn g_floor(a: &App) -> i32 {
    a.h as i32 - 5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art::geo_s;

    fn mk(pid: u32, w: u16, h: u16) -> App {
        let mut a = App::new(w, h, pid as u64);
        a.enable_social(pid);
        a
    }

    /// Deliver every queued message until the network is quiet.
    fn pump(apps: &mut [App]) {
        loop {
            let mut msgs = vec![];
            for a in apps.iter_mut() {
                let me = a.pid;
                for (to, m) in a.outbox.drain(..) {
                    msgs.push((me, to, m));
                }
            }
            if msgs.is_empty() {
                break;
            }
            for (from, to, m) in msgs {
                for a in apps.iter_mut() {
                    if a.pid != from && to.is_none_or(|p| p == a.pid) {
                        a.on_msg(from, m);
                    }
                }
            }
        }
    }

    fn step(apps: &mut [App], secs: f32) {
        for _ in 0..((secs / 0.0625).round() as usize) {
            for a in apps.iter_mut() {
                a.update(0.0625);
            }
            pump(apps);
        }
    }

    fn pair() -> Vec<App> {
        let mut v = vec![mk(10, 120, 40), mk(20, 120, 40)];
        step(&mut v, 1.0);
        v
    }

    fn app(v: &mut [App], pid: u32) -> &mut App {
        v.iter_mut().find(|a| a.pid == pid).unwrap()
    }

    #[test]
    fn neighbors_found_within_2s() {
        let v = pair();
        assert!(v[0].peers.contains_key(&20) && v[1].peers.contains_key(&10));
    }

    #[test]
    fn no_neighbor_says_no_one_around() {
        let mut a = mk(1, 120, 40);
        a.visit_key();
        assert_eq!(a.note.as_ref().unwrap().0, "No one around...");
        step(std::slice::from_mut(&mut a), 2.2);
        assert!(a.note.is_none());
        assert_eq!(a.home, Home::Home);
        assert!(a.bodies().len() == 1);
    }

    #[test]
    fn visit_goes_out_plays_and_comes_home() {
        let mut v = pair();
        app(&mut v, 10).init_color(1, false);
        app(&mut v, 10).visit_key();
        step(&mut v, 0.5);
        // Swimming out: still drawn at home, not yet in B.
        assert!(matches!(v[0].home, Home::Leaving { .. }));
        assert!(v[1].guests.is_empty());
        step(&mut v, 1.3);
        // Never in two places: A has no body once B has the guest.
        assert!(v[0].is_away());
        assert_eq!(v[1].guests.len(), 1);
        assert!(v[0].bodies().is_empty());
        assert_eq!(v[1].bodies().len(), 2);
        assert_eq!(v[1].note.as_ref().unwrap().0, "A purple friend is here!");
        assert!(v[1].guests[0].tag > 0.0);
        // At no sampled moment is the squid present in both.
        let mut seen_moves = std::collections::HashSet::new();
        for _ in 0..(21.0 / 0.0625) as usize {
            step(&mut v, 0.0625);
            assert!(
                !(v[0].bodies().len() == 1 && v[0].home == Home::Home && !v[1].guests.is_empty())
            );
            if let Some(c) = &v[1].play_ctl {
                seen_moves.insert(c.mv);
            }
        }
        assert!(seen_moves.len() >= 3, "{seen_moves:?}");
        step(&mut v, 4.0);
        assert_eq!(v[0].home, Home::Home);
        assert!(v[1].guests.is_empty());
        assert!((v[1].scale - 1.0).abs() < 1e-6);
        assert_eq!(v[0].bodies().len(), 1);
    }

    #[test]
    fn visit_ends_on_its_own_around_20s() {
        let mut v = pair();
        app(&mut v, 10).visit_key();
        step(&mut v, 3.0);
        let mut t = 3.0;
        while v[0].home != Home::Home && t < 40.0 {
            step(&mut v, 0.25);
            t += 0.25;
        }
        assert!((18.0..=26.0).contains(&(t - 1.5)), "ended at {t}");
    }

    #[test]
    fn v_at_either_end_recalls_within_3s() {
        for host_presses in [false, true] {
            let mut v = pair();
            app(&mut v, 10).visit_key();
            step(&mut v, 6.0);
            assert!(v[0].is_away());
            let who = if host_presses { 20 } else { 10 };
            app(&mut v, who).visit_key();
            step(&mut v, 3.1);
            assert_eq!(v[0].home, Home::Home, "host_presses={host_presses}");
            assert!(v[1].guests.is_empty());
        }
    }

    #[test]
    fn simultaneous_visits_send_exactly_one() {
        let mut v = pair();
        v[0].visit_key();
        v[1].visit_key();
        step(&mut v, 2.0);
        let away = v.iter().filter(|a| a.is_away()).count();
        let guests: usize = v.iter().map(|a| a.guests.len()).sum();
        assert_eq!((away, guests), (1, 1));
    }

    #[test]
    fn host_killed_mid_visit_guest_is_home_within_2s() {
        let mut v = pair();
        v[0].visit_key();
        step(&mut v, 6.0);
        assert!(v[0].is_away());
        v.pop(); // kill -9: no goodbye
        step(&mut v, 2.0);
        assert_eq!(v[0].home, Home::Home);
        assert_eq!(v[0].bodies().len(), 1);
    }

    #[test]
    fn guest_home_killed_guest_leaves_host_within_2s() {
        let mut v = pair();
        v[0].visit_key();
        step(&mut v, 6.0);
        assert_eq!(v[1].guests.len(), 1);
        v.remove(0);
        step(&mut v, 2.0);
        assert!(v[0].guests.is_empty());
        step(&mut v, 3.0);
        assert!((v[0].scale - 1.0).abs() < 1e-6);
        assert!(v[0].peers.is_empty());
    }

    #[test]
    fn clean_quit_sends_guest_home_at_once() {
        let mut v = pair();
        v[0].visit_key();
        step(&mut v, 6.0);
        v[1].shutdown();
        pump(&mut v);
        assert!(matches!(v[0].home, Home::Returning { .. }));
    }

    #[test]
    fn party_shrinks_and_recovers() {
        let mut v = vec![mk(1, 120, 40), mk(2, 120, 40), mk(3, 120, 40)];
        step(&mut v, 1.0);
        v[0].visit_key();
        v[2].visit_key();
        // Pick B (pid 2) for both: only B is a candidate once the others go away.
        step(&mut v, 8.0);
        let away: Vec<u32> = v.iter().filter(|a| a.is_away()).map(|a| a.pid).collect();
        assert!(!away.is_empty());
        let host = v.iter().find(|a| a.guests.len() == 2);
        if let Some(h) = host {
            assert!(h.scale < 0.9, "scale {}", h.scale);
            let xs: Vec<f32> = h
                .order()
                .iter()
                .map(|w| w.map_or(h.pos_of(0), |i| h.guests[i].x))
                .collect();
            assert!(xs.windows(2).all(|p| p[0] < p[1]));
        }
        step(&mut v, 30.0);
        assert!(
            v.iter()
                .all(|a| (a.scale - 1.0).abs() < 1e-6 && a.guests.is_empty())
        );
    }

    #[test]
    fn two_stay_full_size_in_a_wide_window_but_shrink_in_a_normal_one() {
        for (w, h, full) in [(300u16, 80u16, true), (120, 40, false)] {
            let mut v = vec![mk(1, w, h), mk(2, w, h)];
            step(&mut v, 1.0);
            v[0].visit_key();
            step(&mut v, 6.0);
            assert_eq!(v[1].guests.len(), 1);
            assert_eq!(v[1].scale >= 0.999, full, "{w}x{h} scale {}", v[1].scale);
        }
    }

    #[test]
    fn too_small_host_accepts_none() {
        let mut v = vec![mk(1, 120, 40), mk(2, 30, 10)];
        step(&mut v, 1.0);
        v[0].visit_key();
        step(&mut v, 1.0);
        assert_eq!(v[0].home, Home::Home);
        assert!(v[1].guests.is_empty());
    }

    #[test]
    fn away_squid_ignores_reaction_keys_and_incoming() {
        let mut v = pair();
        v[0].visit_key();
        step(&mut v, 6.0);
        assert!(!v[0].press(Key::Feed));
        assert!(!v[0].any_react());
        v[0].on_msg(20, Msg::Fish);
        assert!(v[0].inbox.is_empty());
    }

    #[test]
    fn auto_visit_happens_without_keys() {
        let mut v = pair();
        let mut visited = false;
        for _ in 0..(900.0 / 0.25) as usize {
            step(&mut v, 0.25);
            assert!(
                v.iter().all(|a| a.guests.is_empty()),
                "visited before 15 min"
            );
        }
        for _ in 0..(1000.0 / 0.25) as usize {
            step(&mut v, 0.25);
            visited |= v.iter().any(|a| !a.guests.is_empty());
        }
        assert!(visited);
    }

    #[test]
    fn no_own_visit_for_10_min_after_a_visit() {
        let mut v = pair();
        for a in v.iter_mut() {
            a.next_visit = a.t + 1.0; // would visit on its own right away
        }
        v[0].visit_key();
        step(&mut v, 45.0); // visit runs and ends
        assert!(
            v.iter()
                .all(|a| a.guests.is_empty() && a.home == Home::Home)
        );
        for _ in 0..(540.0 / 0.25) as usize {
            step(&mut v, 0.25);
            assert!(
                v.iter()
                    .all(|a| a.guests.is_empty() && a.home == Home::Home)
            );
        }
    }

    #[test]
    fn fish_send_is_rate_limited_and_arrives_as_yum() {
        // Count Fish messages with a fresh run so we can see the outbox.
        let mut a = mk(1, 120, 40);
        a.peers.insert(
            2,
            Peer {
                seen: 0.0,
                away: false,
                accept: true,
            },
        );
        let mut fish = 0;
        for i in 0..20 {
            a.press(Key::Feed);
            a.peers.get_mut(&2).unwrap().seen = a.t;
            for _ in 0..4 {
                a.update(0.0625);
                a.peers.get_mut(&2).unwrap().seen = a.t;
            }
            let _ = i;
            fish += a.outbox.drain(..).filter(|(_, m)| *m == Msg::Fish).count();
        }
        assert!(fish <= 1, "sent {fish} fish in 5 s");
        // Spaced presses eventually send, and the receiver reacts with Yum!.
        let mut b = mk(2, 120, 40);
        b.on_msg(1, Msg::Fish);
        for _ in 0..16 {
            b.update(0.0625);
        }
        assert!(b.feed.on && b.msg == Some("Yum!"));
        let foods = b.foods();
        assert!(!foods.is_empty());
    }

    #[test]
    fn ink_send_limited_and_receiver_startles() {
        let mut a = mk(1, 120, 40);
        a.peers.insert(
            2,
            Peer {
                seen: 0.0,
                away: false,
                accept: true,
            },
        );
        let mut n = 0;
        for _ in 0..30 {
            a.press(Key::Play);
            for _ in 0..3 {
                a.update(0.0625);
                a.peers.get_mut(&2).unwrap().seen = a.t;
            }
            n += a.outbox.drain(..).filter(|(_, m)| *m == Msg::Ink).count();
        }
        assert!(n <= 2, "ink sends {n} in ~5.6 s");
        assert!(n >= 1);
        let mut b = mk(2, 120, 40);
        b.on_msg(1, Msg::Ink);
        let mut got = false;
        for _ in 0..80 {
            b.update(0.0625);
            got |= b.note.as_ref().is_some_and(|n| n.0 == "Eep!");
        }
        assert!(got, "never startled");
    }

    #[test]
    fn bubble_stream_arrives_and_squid_looks() {
        let mut b = mk(2, 120, 40);
        let before = b.bubbles.len();
        b.on_msg(1, Msg::Bubbles(0.2));
        for _ in 0..20 {
            b.update(0.0625);
        }
        assert!(b.bubbles.len() >= before + 5);
        assert!(matches!(
            b.state,
            State::IdleBehavior(Behavior::LookLeft | Behavior::LookRight)
        ));
    }

    #[test]
    fn nothing_sent_without_neighbors_or_when_solo() {
        let mut a = App::new(120, 40, 5); // solo: social never enabled
        for _ in 0..50 {
            a.press(Key::Feed);
            a.press(Key::Play);
            a.update(0.25);
        }
        assert!(a.outbox.is_empty() && a.guests.is_empty() && a.home == Home::Home);
        let mut b = mk(9, 120, 40);
        b.press(Key::Play);
        assert!(
            b.outbox
                .iter()
                .all(|(_, m)| !matches!(m, Msg::Ink | Msg::Fish))
        );
    }

    #[test]
    fn geo_scale_one_is_solo() {
        let a = crate::art::geo(120, 40);
        let b = geo_s(120, 40, 1.0);
        assert_eq!((a.mh, a.hh, a.al, a.maxw), (b.mh, b.hh, b.al, b.maxw));
    }
}
