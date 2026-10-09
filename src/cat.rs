//! The cat: breeds, per-cat animation state and the `App` methods that drive it.
//! Drawing lives in `cat_art`. Everything here only runs when `App::kind` is `Cat`.
use super::{App, BURST, Behavior, Key, State, ease};
use crate::art::Body;
use crate::art::Rgb;
use rand::Rng;
use rand::seq::SliceRandom;

/// Coat marking style. Markings are drawn with glyphs as well as color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pat {
    Solid,
    Tabby,
    Striped,
    Calico,
    Pointed,
    Tuxedo,
    Hairless,
}

#[derive(Clone, Copy, Debug)]
pub struct Breed {
    pub name: &'static str,
    pub base: Rgb,
    /// Stripe / patch / point color.
    pub dark: Rgb,
    /// Muzzle, bib and paw color where the pattern uses one.
    pub light: Rgb,
    pub eye: Rgb,
    pub nose: Rgb,
    pub inner: Rgb,
    pub pat: Pat,
    /// Size factor (maine-coon is bigger).
    pub size: f32,
    /// Fur fringe beyond the outline, in columns.
    pub fluff: f32,
    pub shaggy: bool,
    pub tufts: bool,
    pub bushy: bool,
    pub fold: bool,
    pub flat: bool,
    pub ear: f32,
}

const fn b(name: &'static str, base: Rgb, dark: Rgb, light: Rgb, eye: Rgb, pat: Pat) -> Breed {
    Breed {
        name,
        base,
        dark,
        light,
        eye,
        nose: (0xE0, 0x78, 0x88),
        inner: (0xE8, 0x98, 0xA0),
        pat,
        size: 1.0,
        fluff: 0.4,
        shaggy: false,
        tufts: false,
        bushy: false,
        fold: false,
        flat: false,
        ear: 1.0,
    }
}

/// Cycle order for `c`.
pub const BREEDS: [Breed; 12] = [
    b(
        "tabby",
        (0x9A, 0x6B, 0x3C),
        (0x42, 0x28, 0x12),
        (0xD8, 0xC0, 0x9A),
        (0xC8, 0xA0, 0x20),
        Pat::Tabby,
    ),
    b(
        "black",
        (0x2C, 0x2C, 0x38),
        (0x2C, 0x2C, 0x38),
        (0x3C, 0x3C, 0x4A),
        (0x50, 0xE0, 0x50),
        Pat::Solid,
    ),
    b(
        "calico",
        (0xF2, 0xEE, 0xE6),
        (0x26, 0x26, 0x2C),
        (0xE0, 0x8A, 0x30),
        (0xD0, 0xA8, 0x30),
        Pat::Calico,
    ),
    b(
        "siamese",
        (0xEC, 0xDC, 0xBC),
        (0x4A, 0x34, 0x2C),
        (0xF6, 0xEC, 0xD6),
        (0x40, 0xA0, 0xFF),
        Pat::Pointed,
    ),
    b(
        "orange",
        (0xE8, 0x8C, 0x2C),
        (0xFA, 0xC8, 0x80),
        (0xFA, 0xDC, 0xB0),
        (0xD8, 0xB0, 0x30),
        Pat::Striped,
    ),
    b(
        "tuxedo",
        (0x26, 0x26, 0x30),
        (0x26, 0x26, 0x30),
        (0xFA, 0xFA, 0xFA),
        (0xC0, 0xD8, 0x40),
        Pat::Tuxedo,
    ),
    b(
        "gray",
        (0x8A, 0x90, 0x9C),
        (0x8A, 0x90, 0x9C),
        (0xB8, 0xBC, 0xC6),
        (0x90, 0xD0, 0x50),
        Pat::Solid,
    ),
    b(
        "white",
        (0xF6, 0xF6, 0xFA),
        (0xF6, 0xF6, 0xFA),
        (0xFF, 0xFF, 0xFF),
        (0x40, 0xA0, 0xFF),
        Pat::Solid,
    ),
    Breed {
        size: 1.15,
        fluff: 1.6,
        shaggy: true,
        tufts: true,
        bushy: true,
        ..b(
            "maine-coon",
            (0x86, 0x5A, 0x32),
            (0x3A, 0x22, 0x10),
            (0xC8, 0xB0, 0x8C),
            (0xC8, 0xB0, 0x30),
            Pat::Tabby,
        )
    },
    Breed {
        fluff: 0.0,
        ear: 1.6,
        nose: (0xD0, 0x70, 0x78),
        ..b(
            "sphynx",
            (0xE8, 0xB4, 0xA4),
            (0xBC, 0x7C, 0x70),
            (0xF0, 0xC8, 0xB8),
            (0x98, 0xD8, 0x50),
            Pat::Hairless,
        )
    },
    Breed {
        fold: true,
        ear: 0.7,
        ..b(
            "scottish-fold",
            (0x98, 0x9C, 0xA8),
            (0x98, 0x9C, 0xA8),
            (0xC0, 0xC4, 0xCE),
            (0xE8, 0x98, 0x20),
            Pat::Solid,
        )
    },
    Breed {
        flat: true,
        fluff: 1.8,
        ear: 0.6,
        ..b(
            "persian",
            (0xF8, 0xF2, 0xEA),
            (0xF8, 0xF2, 0xEA),
            (0xFF, 0xFF, 0xFF),
            (0xE8, 0x80, 0x28),
            Pat::Solid,
        )
    },
];

pub fn breed_list() -> String {
    let mut v: Vec<&str> = BREEDS.iter().map(|b| b.name).collect();
    v.push("random");
    v.join(", ")
}

/// Breed index for `--breed NAME`; `Err(true)` is random.
pub fn parse_breed(s: &str) -> Option<Result<usize, ()>> {
    let s = s.to_ascii_lowercase();
    if s == "random" {
        return Some(Err(()));
    }
    BREEDS.iter().position(|b| b.name == s).map(Ok)
}

/// Breed picked by `--breed random` for a seed.
pub fn random_breed(seed: u64) -> usize {
    use rand::SeedableRng;
    rand::rngs::StdRng::seed_from_u64(seed).random_range(0..BREEDS.len())
}

/// Per-cat animation timers that the squid does not have.
#[derive(Default)]
pub struct CatState {
    pub next_ear: f32,
    pub ear_t: f32,
    pub ear_side: f32,
    pub next_tail: f32,
    pub tail_t: f32,
    pub bag: Vec<Behavior>,
    /// Laser waypoints (fractions of the window) and zoomie targets (cells from center).
    pub way: [(f32, f32); 5],
    pub dashes: [f32; 4],
}

/// Everything about the cat's pose that is not already in `Body`.
#[derive(Clone, Copy, Debug, Default)]
pub struct CatPose {
    pub walking: bool,
    /// Walk / run cycle phase (radians).
    pub step: f32,
    pub stretch: f32,
    pub groom: f32,
    pub groom_ph: f32,
    pub sneeze: f32,
    /// -1 left / 1 right ear flicking, magnitude 0..1.
    pub ear: f32,
    pub tail_tw: f32,
    pub slow: bool,
    pub crouch: f32,
    pub wiggle: f32,
    pub pounce: f32,
    pub swat: f32,
    pub eat: f32,
    pub puffy: bool,
    pub running: bool,
    /// Pupil direction, each -1..1 (y > 0 is down).
    pub gaze: (f32, f32),
}

pub const ZOOM_AT: f32 = 3.2;
const ZOOM_SEG: f32 = 0.7;
pub const PLAY_LEN: f32 = 7.4;

fn tri(s: f32, a: f32, b: f32) -> f32 {
    // 0 outside a..b, rises and falls inside (sin bump)
    if (a..b).contains(&s) {
        (std::f32::consts::PI * (s - a) / (b - a)).sin()
    } else {
        0.0
    }
}

impl App {
    pub fn is_cat(&self) -> bool {
        self.kind == super::Kind::Cat
    }

    /// Switch this process to the cat (call right after `App::new`).
    pub fn init_cat(&mut self) {
        self.kind = super::Kind::Cat;
        self.cat.next_ear = self.t + self.rng.random_range(3.0..8.0);
        self.cat.next_tail = self.t + self.rng.random_range(2.0..6.0);
        self.cat.ear_side = 1.0;
    }

    fn refill_bag(&mut self) {
        let mut v = vec![
            Behavior::Stretch,
            Behavior::Groom,
            Behavior::LookLeft,
            Behavior::LookRight,
            Behavior::InkPuff, // the sneeze
            Behavior::SlowBlink,
        ];
        v.shuffle(&mut self.rng);
        self.cat.bag = v;
    }

    pub(crate) fn cat_pick_idle(&mut self) -> Behavior {
        if self.cat.bag.is_empty() {
            self.refill_bag();
        }
        self.cat.bag.pop().unwrap_or(Behavior::LookLeft)
    }

    pub(crate) fn cat_sneeze(&mut self) {
        let g = self.geo();
        for _ in 0..8 {
            let x = self.spawn_x() + self.rng.random_range(-3.0..=3.0);
            let y = (g.top + g.mh / 2 + 2) as f32;
            self.bubbles.push(super::Bubble {
                x,
                y: y + self.rng.random_range(0.0..2.0),
                vy: self.rng.random_range(3.0..6.0),
                ch: ['.', '*', '\u{b0}'][self.rng.random_range(0..3)],
                phase: self.rng.random_range(0.0..std::f32::consts::TAU),
            });
        }
        self.set_msg("Achoo!");
    }

    /// Dust motes drifting up through the sunbeam.
    pub(crate) fn cat_motes(&mut self, dt: f32) {
        if self.bubbles.len() < 40 && self.rng.random_range(0.0..1.0) < 2.0 * dt {
            let g = self.geo();
            let floor = (g.top + g.mh + g.hh + g.al - 2) as f32;
            let y = self.rng.random_range(12.0..floor.max(13.0));
            let (lo, hi) = crate::cat_art::beam_at(self.w as i32, self.h as i32, y);
            self.bubbles.push(super::Bubble {
                x: self.rng.random_range(lo..=hi.max(lo + 1.0)),
                y,
                vy: self.rng.random_range(1.0..2.2),
                ch: ['.', '*', '\u{b0}'][self.rng.random_range(0..3)],
                phase: self.rng.random_range(0.0..std::f32::consts::TAU),
            });
        }
        for b in &mut self.bubbles {
            b.y -= b.vy * dt * 0.45;
            b.x -= 0.5 * b.vy * dt * 0.45;
        }
        self.bubbles.retain(|b| b.y > 2.0);
    }

    pub(crate) fn cat_update(&mut self, dt: f32) {
        let t = self.t;
        if self.cat.ear_t > 0.0 {
            self.cat.ear_t -= dt;
        } else if t >= self.cat.next_ear {
            self.cat.ear_t = 0.35;
            self.cat.ear_side = if self.rng.random_bool(0.5) { 1.0 } else { -1.0 };
            self.cat.next_ear = t + self.rng.random_range(4.0..9.0);
        }
        if self.cat.tail_t > 0.0 {
            self.cat.tail_t -= dt;
        } else if t >= self.cat.next_tail {
            self.cat.tail_t = 0.5;
            self.cat.next_tail = t + self.rng.random_range(3.0..7.0);
        }
    }

    pub(crate) fn cat_play_start(&mut self) {
        let g = self.geo();
        self.dart_from = (self.cat_dx(), 0.0);
        self.dart_s = 0.0;
        let lim = (self.w as f32 / 2.0 - g.maxw * 1.7).max(0.0);
        let mut side = if self.rng.random_bool(0.5) { 1.0 } else { -1.0 };
        for d in &mut self.cat.dashes {
            *d = side * lim * self.rng.random_range(0.7..=1.0);
            side = -side;
        }
        for w in &mut self.cat.way {
            *w = (
                self.rng.random_range(0.12..0.88),
                self.rng.random_range(0.2..0.75),
            );
        }
        self.play.dur = self.play.dur.max(self.play.st + PLAY_LEN);
    }

    /// Horizontal play offset: pounce lunge, then zoomies across the screen.
    pub fn cat_dx(&self) -> f32 {
        if !self.play.on {
            return 0.0;
        }
        let s = self.dart_s;
        let from = self.dart_from.0 * (1.0 - ease(s / 0.6));
        let dot_dir = (self.laser_pos().map_or(0.0, |p| p.0) - self.w as f32 / 2.0).signum();
        let lunge = dot_dir * 5.0 * tri(s, 2.3, 2.9);
        if s < ZOOM_AT {
            return from + lunge;
        }
        let k = (((s - ZOOM_AT) / ZOOM_SEG) as usize).min(3);
        let end = ZOOM_AT + ZOOM_SEG * 4.0;
        if s >= end {
            return self.cat.dashes[3] * (1.0 - ease((s - end) / 1.2));
        }
        let a = if k == 0 { 0.0 } else { self.cat.dashes[k - 1] };
        let u = ease((s - ZOOM_AT - ZOOM_SEG * k as f32) / ZOOM_SEG);
        a + (self.cat.dashes[k] - a) * u
    }

    fn zooming(&self) -> bool {
        self.play.on && (ZOOM_AT..ZOOM_AT + ZOOM_SEG * 4.0).contains(&self.dart_s)
    }

    /// The red laser dot (cell coordinates) while it is on screen.
    pub fn laser_pos(&self) -> Option<(f32, f32)> {
        if !self.play.on || self.dart_s >= 3.0 {
            return None;
        }
        let s = self.dart_s;
        let (w, h) = (self.w as f32, self.h as f32);
        let cell = |f: (f32, f32)| (2.0 + f.0 * (w - 4.0), 2.0 + f.1 * (h - 10.0));
        if s < 2.0 {
            let i = ((s / 0.5) as usize).min(3);
            let u = ease(((s - 0.5 * i as f32) / 0.5) * 3.0);
            let (a, b) = (cell(self.cat.way[i]), cell(self.cat.way[i + 1]));
            Some((a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u))
        } else if s < 2.5 {
            // it stops, the cat pounces
            let p = cell(self.cat.way[4]);
            Some((p.0 + 0.8 * (s * 20.0).sin(), p.1))
        } else {
            // missed: it skitters away
            let p = cell(self.cat.way[4]);
            let k = ease((s - 2.5) / 0.4);
            let far = if p.0 > w / 2.0 { 2.0 } else { w - 2.0 };
            Some((p.0 + (far - p.0) * k, p.1 - 6.0 * k))
        }
    }

    pub fn cat_pose(&self) -> CatPose {
        let t = self.t;
        let mut p = CatPose {
            walking: self.burst.2 < BURST && self.home_ox() == 0.0,
            step: t * 9.0,
            ..Default::default()
        };
        let gx = self.geo().cx as f32 + self.offset().0;
        // idle behaviors
        if let State::IdleBehavior(b) = self.state {
            let st = self.st;
            match b {
                Behavior::Stretch => {
                    let d = App::idle_duration(b);
                    p.stretch = ease(st / 0.8) * (1.0 - ease((st - (d - 0.8)) / 0.8));
                }
                Behavior::Groom => {
                    let d = App::idle_duration(b);
                    p.groom = ease(st / 0.4) * (1.0 - ease((st - (d - 0.4)) / 0.4));
                    p.groom_ph = st * std::f32::consts::TAU * 1.7;
                }
                Behavior::SlowBlink => p.slow = (0.2..1.1).contains(&st),
                Behavior::InkPuff => p.sneeze = tri(st, 0.3, 0.75),
                Behavior::LookLeft if (0.2..1.6).contains(&st) => p.gaze.0 = -1.0,
                Behavior::LookRight if (0.2..1.6).contains(&st) => p.gaze.0 = 1.0,
                _ => {}
            }
        }
        if let Some((left, _)) = self.splat
            && left < 1.4
        {
            // shaking the ink off
            p.wiggle = (t * 40.0).sin() * (left / 1.4).min(1.0);
        }
        if self.cat.ear_t > 0.0 {
            p.ear =
                self.cat.ear_side * (std::f32::consts::PI * (1.0 - self.cat.ear_t / 0.35)).sin();
        }
        if self.cat.tail_t > 0.0 {
            p.tail_tw = (t * 30.0).sin();
        }
        // treat: watch it fall, bat it, eat it
        if self.feed.on
            && let Some(f) = self.fish.last()
        {
            let a = self.fish_age(f);
            if a < 1.4 {
                let fx = self.foods().last().map_or(gx, |f| f.0);
                p.gaze = (((fx - gx) / self.geo().maxw).clamp(-1.0, 1.0), 1.0);
            }
            p.swat = tri(a, 0.8, 1.5);
            p.eat = tri(a, 1.4, 2.4);
        }
        // laser dot, pounce, zoomies
        if self.play.on {
            let s = self.dart_s;
            if let Some(d) = self.laser_pos() {
                p.gaze = (((d.0 - gx) / (self.w as f32 / 2.0)).clamp(-1.0, 1.0), 0.6);
            }
            p.crouch = ease((s - 0.4) / 0.5) * (1.0 - ease((s - 2.9) / 0.3));
            p.wiggle = if (1.1..2.3).contains(&s) {
                (s * 22.0).sin()
            } else {
                0.0
            };
            p.pounce = tri(s, 2.3, 2.9);
            if self.zooming() {
                p.running = true;
                p.step = t * 22.0;
                p.puffy = true;
            } else if s > ZOOM_AT && s < ZOOM_AT + ZOOM_SEG * 4.0 + 0.8 {
                p.puffy = true;
            }
        }
        p
    }

    /// Draw data for the host cat.
    pub fn cat_host_body(&self) -> Body {
        let g = self.geo();
        let (dx, dy) = self.offset();
        let mut pose = self.cat_pose();
        pose.walking &= !pose.running;
        Body {
            kind: super::Kind::Cat,
            breed: self.color_idx(),
            breed_from: self.breed_from,
            bfade: ease(self.fade),
            boop: (self.touch.boop / super::BOOP).clamp(0.0, 1.0),
            pose,
            pal: crate::art::PRESETS[0].1,
            cx: g.cx,
            dx,
            dy,
            scale: self.scale,
            puff: self.puff(),
            curl: 0.0,
            blush: self.blush(),
            flash: self.flash(),
            bright: self.fx.bright,
            dark: self.fx.dark,
            look: 0,
            closed: self.eyes_closed(),
            blink: self.blinking(),
            yawn: false,
            lean: 0.25 * self.touch.lean() + if self.pet.on { 0.04 } else { 0.0 },
            food_x: g.cx as f32 + dx,
            t: self.t,
            touch: ease(self.touch.touch),
            touch_at: self.touch.at,
            ruffle: self.touch.ruffle_k(),
            ruffle_at: self.touch.ruffle_at,
            tag: None,
            head_dy: self.touch.dy(),
            dir: self.touch.dir,
            tail: self.touch.tail_side(),
            bang: false,
            splat: self.splat,
        }
    }

    pub(crate) fn cat_msg(k: Key) -> &'static str {
        match k {
            Key::Feed => "Yum!",
            Key::Pet => "Purrrr \u{2665}",
            Key::Play => "Wheee!",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{App, Key, Kind, PRESS_CAP, Ptr, State};
    use super::*;
    use crate::art::NONE;

    fn cat(w: u16, h: u16, seed: u64) -> App {
        let mut a = App::new(w, h, seed);
        a.init_cat();
        a.init_color(0, false);
        a
    }

    /// Draw a frame; returns a cat cell and an empty-room cell.
    fn framed(a: &mut App) -> ((u16, u16), (u16, u16)) {
        let cv = crate::art::render(a, false);
        a.set_hit(cv.owner.clone());
        let w = a.w as usize;
        let row = (a.geo().top + a.geo().mh + a.geo().hh / 2) as usize;
        let xs: Vec<usize> = (0..w).filter(|&x| cv.owner[row * w + x] != NONE).collect();
        ((xs[xs.len() / 2] as u16, row as u16), (w as u16 - 30, 2))
    }

    #[test]
    fn cats_do_not_glow() {
        let mut a = cat(120, 40, 1);
        let b0 = a.cat_host_body();
        let mut b1 = b0.clone();
        b1.bright = 1.0;
        let draw = |b: &crate::art::Body| {
            let mut cv = crate::art::Canvas::new(120, 40, false);
            crate::cat_art::draw_cat(&mut cv, b, false);
            cv.cells.iter().map(|c| c.fg).collect::<Vec<_>>()
        };
        assert_eq!(draw(&b0), draw(&b1));
        let _ = &mut a;
    }

    #[test]
    fn breeds_in_spec_order_and_cycle() {
        let names: Vec<_> = BREEDS.iter().map(|b| b.name).collect();
        assert_eq!(
            names,
            [
                "tabby",
                "black",
                "calico",
                "siamese",
                "orange",
                "tuxedo",
                "gray",
                "white",
                "maine-coon",
                "sphynx",
                "scottish-fold",
                "persian"
            ]
        );
        assert_eq!(parse_breed("Maine-Coon"), Some(Ok(8)));
        assert_eq!(parse_breed("random"), Some(Err(())));
        assert_eq!(parse_breed("dog"), None);
        assert_eq!(random_breed(7), random_breed(7));
        let mut a = cat(120, 40, 1);
        a.next_color();
        assert_eq!(a.cmsg.as_ref().unwrap().0, "I'm a black today!");
        for _ in 0..11 {
            a.next_color();
        }
        assert_eq!(a.color_idx(), 0);
    }

    #[test]
    fn cat_press_caps_equal_squid_caps() {
        for k in [Key::Feed, Key::Pet, Key::Play] {
            let mut c = cat(120, 40, 1);
            let mut s = App::new(120, 40, 1);
            let nc = (0..10).filter(|_| c.press(k)).count();
            let ns = (0..10).filter(|_| s.press(k)).count();
            assert_eq!(nc, ns);
            assert_eq!(nc, PRESS_CAP);
        }
        let mut c = cat(120, 40, 1);
        assert!(c.press(Key::Feed) && c.press(Key::Pet) && c.press(Key::Play));
        assert!(c.feed.on && c.pet.on && c.play.on);
        assert_eq!(c.fish.len(), 1);
        c.press(Key::Feed);
        assert_eq!(c.fish.len(), 2);
    }

    #[test]
    fn cat_key_messages() {
        let mut a = cat(120, 40, 1);
        a.press(Key::Feed);
        assert_eq!(a.msg, Some("Yum!"));
        a.press(Key::Pet);
        assert_eq!(a.msg, Some("Purrrr \u{2665}"));
        assert!(a.eyes_closed() && !a.hearts.is_empty());
        a.press(Key::Play);
        assert_eq!(a.msg, Some("Wheee!"));
        assert!(a.laser_pos().is_some());
        for _ in 0..(12 * 16) {
            a.update(1.0 / 16.0);
        }
        assert!(!a.any_react());
        assert!(a.laser_pos().is_none());
    }

    #[test]
    fn click_drag_scroll_on_cat_and_misses() {
        let mut a = cat(120, 40, 1);
        let (c, off) = framed(&mut a);
        a.pointer(Ptr::Down, off.0, off.1);
        a.pointer(Ptr::Up, off.0, off.1);
        a.pointer(Ptr::Scroll, off.0, off.1);
        assert!(a.msg.is_none() && a.hearts.is_empty() && a.touch.boop == 0.0);
        // click = boop
        a.pointer(Ptr::Down, c.0, c.1);
        a.pointer(Ptr::Up, c.0, c.1);
        assert_eq!(a.msg, Some("Mrrp?"));
        assert_eq!(a.hearts.len(), 1);
        assert!(a.touch.boop > 0.0);
        // drag = stroke
        let mut b = cat(120, 40, 1);
        let (c, _) = framed(&mut b);
        b.pointer(Ptr::Down, c.0, c.1);
        for i in 1..=10 {
            b.pointer(Ptr::Drag, c.0 + i, c.1);
            b.update(0.06);
        }
        assert_eq!(b.msg, Some("Purrrr \u{2665}"));
        assert!(b.hearts.len() >= 4 && b.eyes_closed() && b.blush() > 0.0);
        assert!(b.cat_host_body().lean != 0.0);
        // scroll = ruffle, no hearts
        let mut d = cat(120, 40, 1);
        let (c, _) = framed(&mut d);
        d.pointer(Ptr::Scroll, c.0, c.1);
        assert_eq!(d.msg, Some("Mrow~"));
        assert!(d.hearts.is_empty() && d.touch.ruffle > 0.0 && !d.pet.on);
    }

    #[test]
    fn idle_behaviors_all_appear_within_three_minutes() {
        let mut a = cat(120, 40, 1);
        let (mut stretch, mut groom, mut ear, mut sneeze, mut moved) =
            (false, false, false, false, false);
        let x0 = a.offset().0;
        for _ in 0..(180 * 16) {
            a.update(1.0 / 16.0);
            stretch |= a.state == State::IdleBehavior(Behavior::Stretch);
            groom |= a.state == State::IdleBehavior(Behavior::Groom);
            sneeze |= a.state == State::IdleBehavior(Behavior::InkPuff);
            ear |= a.cat.ear_t > 0.0;
            moved |= (a.offset().0 - x0).abs() > 3.0;
        }
        assert!(stretch && groom && ear && sneeze && moved);
    }

    #[test]
    fn cat_walks_sideways_over_two_minutes() {
        let mut a = cat(120, 40, 1);
        let mut xs = vec![];
        for i in 0..(120 * 16) {
            a.update(1.0 / 16.0);
            if i % 16 == 0 {
                xs.push(a.offset().0);
            }
        }
        let (lo, hi) = xs
            .iter()
            .fold((f32::MAX, f32::MIN), |(l, h), &x| (l.min(x), h.max(x)));
        assert!(hi - lo > 4.0, "{lo} {hi}");
    }

    #[test]
    fn ascii_has_no_wide_glyphs_and_markings_survive() {
        for (i, br) in BREEDS.iter().enumerate() {
            let mut a = cat(120, 40, 1);
            a.init_color(i, false);
            for _ in 0..40 {
                a.update(0.0625);
            }
            let cv = crate::art::render(&a, true);
            assert!(cv.cells.iter().all(|c| c.ch.is_ascii()), "{}", br.name);
        }
        let mut a = cat(120, 40, 1);
        a.mono = true;
        let cv = crate::art::render(&a, true);
        let s: String = cv.cells.iter().map(|c| c.ch).collect();
        assert!(s.matches('%').count() > 40, "tabby stripes as glyphs");
        let mut b = cat(120, 40, 1);
        b.init_color(1, false);
        let s2: String = crate::art::render(&b, true)
            .cells
            .iter()
            .map(|c| c.ch)
            .collect();
        assert!(s.matches('%').count() > s2.matches('%').count() + 20);
    }

    #[test]
    fn every_breed_fits_without_clipping() {
        for (w, h) in [(120u16, 40u16), (300, 90), (80, 24)] {
            for (i, br) in BREEDS.iter().enumerate() {
                let mut a = cat(w, h, 1);
                a.init_color(i, false);
                let cv = crate::art::render(&a, false);
                for y in 0..h as usize {
                    for x in 0..w as usize {
                        if cv.owner[y * w as usize + x] != NONE {
                            assert!(
                                y >= 1 && y < h as usize - 2 && x >= 1 && x < w as usize - 1,
                                "{} at {w}x{h}: cell {x},{y}",
                                br.name
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn breeds_draw_differently() {
        let mut seen = std::collections::HashSet::new();
        for (i, br) in BREEDS.iter().enumerate() {
            let mut a = cat(120, 40, 1);
            a.init_color(i, false);
            let cv = crate::art::render(&a, false);
            let sig: Vec<_> = cv.cells.iter().map(|c| (c.ch, c.fg)).collect();
            assert!(seen.insert(format!("{sig:?}")), "{}", br.name);
        }
    }

    #[test]
    fn cat_and_squid_visit_each_other() {
        let mut h = App::new(120, 40, 1);
        h.enable_social(1);
        h.on_msg(2, crate::link::Msg::ArriveCat(2));
        for _ in 0..200 {
            h.on_msg(
                2,
                crate::link::Msg::Hello {
                    away: true,
                    accept: true,
                },
            );
            h.update(0.06);
        }
        assert_eq!(h.guests.len(), 1);
        assert_eq!(h.guests[0].kind, Kind::Cat);
        let cv = crate::art::render(&h, false);
        assert!(cv.owner.contains(&0) && cv.owner.contains(&1));
        let mut c = cat(120, 40, 1);
        c.enable_social(3);
        c.on_msg(4, crate::link::Msg::Arrive(2));
        for _ in 0..200 {
            c.on_msg(
                4,
                crate::link::Msg::Hello {
                    away: true,
                    accept: true,
                },
            );
            c.update(0.06);
        }
        assert_eq!(c.guests[0].kind, Kind::Squid);
        assert!(c.note.is_none() || c.note.as_ref().unwrap().0.contains("friend"));
        let cv = crate::art::render(&c, false);
        assert!(cv.owner.contains(&0) && cv.owner.contains(&1));
    }

    #[test]
    fn tail_sweeps_without_jumping_during_a_drag() {
        let mut a = cat(120, 40, 1);
        let (cat_cell, _) = framed(&mut a);
        let g = a.geo();
        let row = cat_cell.1;
        let cv = crate::art::render(&a, false);
        let w = a.w as usize;
        let xs: Vec<usize> = (0..w)
            .filter(|&x| cv.owner[row as usize * w + x] != NONE)
            .collect();
        let (l, r) = (xs[1] as u16, xs[xs.len() - 2] as u16);
        let reach = 1.2 * g.maxw;
        let tip = |a: &App| a.host_body().tail * reach;
        a.pointer(Ptr::Down, l, row);
        for _ in 0..10 {
            a.pointer(Ptr::Drag, l, row);
            a.update(0.0625);
        }
        let mut prev = tip(&a);
        let mut worst = 0.0f32;
        let mut seen = vec![prev];
        for i in 0..=30 {
            let col = if i < 10 {
                l + (r - l) * (i as u16) / 10
            } else {
                r
            };
            a.pointer(Ptr::Drag, col, row);
            a.update(0.0625);
            let t = tip(&a);
            worst = worst.max((t - prev).abs());
            prev = t;
            seen.push(t);
        }
        // A full swing is 2*reach cells in 0.4 s: ~1/6 of it per 62 ms frame, never the
        // old instant flip. (The spec's "3 cells" cannot hold with a 0.4 s swing at this width.)
        assert!(
            worst <= 0.2 * 2.0 * reach,
            "tail tip jumped {worst} cells: {seen:?}"
        );
        // it did swing across, and passes through the middle on the way
        let (lo, hi) = seen
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), &x| (a.min(x), b.max(x)));
        assert!(lo < -0.5 * reach && hi > 0.5 * reach, "{seen:?}");
        // after release the tail stays on the side the drag left it
        let before = tip(&a);
        a.pointer(Ptr::Up, r, row);
        for _ in 0..40 {
            a.update(0.0625);
        }
        assert!((tip(&a) - before).abs() < 0.01, "tail moved after release");

        // drag left, release mid-sweep: the tail finishes fully on the left
        a.pointer(Ptr::Down, r, row);
        for _ in 0..10 {
            a.pointer(Ptr::Drag, r, row);
            a.update(0.0625);
        }
        for _ in 0..3 {
            a.pointer(Ptr::Drag, l, row);
            a.update(0.0625);
        }
        let mid = tip(&a);
        assert!(mid.abs() < 0.9 * reach, "not mid-sweep: {mid}");
        a.pointer(Ptr::Up, l, row);
        for _ in 0..40 {
            a.update(0.0625);
        }
        assert!(tip(&a) < -0.95 * reach, "stuck: {}", tip(&a));
        // then drag right: it reaches the right
        a.pointer(Ptr::Down, r, row);
        for _ in 0..15 {
            a.pointer(Ptr::Drag, r, row);
            a.update(0.0625);
        }
        assert!(tip(&a) > 0.95 * reach, "never crossed: {}", tip(&a));
    }

    #[test]
    fn cat_play_sends_mouse_never_ink() {
        let mut a = cat(120, 40, 1);
        a.enable_social(1);
        a.peers.insert(
            2,
            crate::social::Peer {
                seen: 0.0,
                away: false,
                accept: true,
            },
        );
        let mut mice = 0;
        for _ in 0..30 {
            a.press(Key::Play);
            for _ in 0..3 {
                a.update(0.0625);
                a.peers.get_mut(&2).unwrap().seen = a.t;
            }
            for (_, m) in a.outbox.drain(..) {
                assert_ne!(m, crate::link::Msg::Ink);
                mice += (m == crate::link::Msg::Mouse) as u32;
            }
        }
        assert!((1..=2).contains(&mice), "mice {mice}");
        assert!(a.flyers.iter().all(|f| !f.ink));
    }

    #[test]
    fn squid_startles_at_mouse() {
        let mut b = App::new(120, 40, 2);
        b.enable_social(2);
        b.on_msg(1, crate::link::Msg::Mouse);
        let mut peak = 1.0f32;
        let mut bang = false;
        for _ in 0..80 {
            b.update(0.0625);
            peak = peak.max(b.puff());
            if b.startle > 0.0 {
                bang = true;
                let cv = crate::art::render(&b, false);
                bang &= cv.cells.iter().any(|c| c.ch == '!');
            }
        }
        assert!(peak > 1.1 && bang, "peak {peak} bang {bang}");
    }

    #[test]
    fn cat_is_inked_and_sneezes() {
        let mut c = cat(120, 40, 1);
        c.enable_social(1);
        c.on_msg(2, crate::link::Msg::Ink);
        let mut pfft = false;
        let mut splat = false;
        for _ in 0..60 {
            c.update(0.0625);
            pfft |= c.note.as_ref().is_some_and(|n| n.0 == "Pfft!");
            splat |= c.splat.is_some();
        }
        assert!(pfft && splat);
    }

    fn with_guest(host_cat: bool) -> App {
        let mut h = if host_cat {
            cat(120, 40, 1)
        } else {
            App::new(120, 40, 1)
        };
        h.enable_social(1);
        h.on_msg(
            2,
            if host_cat {
                crate::link::Msg::Arrive(2)
            } else {
                crate::link::Msg::ArriveCat(2)
            },
        );
        for _ in 0..200 {
            h.on_msg(
                2,
                crate::link::Msg::Hello {
                    away: true,
                    accept: true,
                },
            );
            h.update(0.06);
        }
        h
    }

    #[test]
    fn guest_squid_startles_at_hosts_mouse() {
        let mut h = with_guest(true);
        h.press(Key::Play);
        let (mut bang, mut puff) = (false, 1.0f32);
        for _ in 0..80 {
            h.on_msg(
                2,
                crate::link::Msg::Hello {
                    away: true,
                    accept: true,
                },
            );
            h.update(0.0625);
            for b in h.bodies() {
                bang |= b.bang;
                puff = puff.max(b.puff);
            }
        }
        assert!(bang && puff > 1.1, "bang {bang} puff {puff}");
    }

    #[test]
    fn guest_cat_is_inked_by_squid_host() {
        let mut h = with_guest(false);
        h.press(Key::Play);
        let (mut splat, mut pfft) = (false, false);
        for _ in 0..80 {
            h.on_msg(
                2,
                crate::link::Msg::Hello {
                    away: true,
                    accept: true,
                },
            );
            h.update(0.0625);
            splat |= h.bodies().iter().any(|b| b.splat.is_some());
            pfft |= h.note.as_ref().is_some_and(|n| n.0 == "Pfft!");
        }
        assert!(splat && pfft);
    }
}
