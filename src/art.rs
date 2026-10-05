//! Procedural drawing of the squid and its world onto a `Canvas`.
use crate::app::App;

pub type Rgb = (u8, u8, u8);
pub const MANTLE: Rgb = (0xB5, 0x40, 0x1F);
pub const ARM: Rgb = (0xD4, 0x52, 0x1E);
pub const PEACH: Rgb = (0xE8, 0xA0, 0x80);
pub const FLUSH: Rgb = (0xFF, 0x5A, 0x2A);

/// A full body palette. `rings` > 0 means blue rings are drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub mantle: Rgb,
    pub arm: Rgb,
    pub peach: Rgb,
    pub flush: Rgb,
    pub ring: Rgb,
    pub ring_hi: Rgb,
    pub edge: Rgb,
    pub rings: f32,
}

const fn body(mantle: Rgb, arm: Rgb, peach: Rgb, flush: Rgb) -> Palette {
    Palette {
        mantle,
        arm,
        peach,
        flush,
        ring: arm,
        ring_hi: arm,
        edge: mantle,
        rings: 0.0,
    }
}

/// Cycle order for the `c` key (random is not a preset).
pub const PRESETS: [(&str, Palette); 8] = [
    ("red", body(MANTLE, ARM, PEACH, FLUSH)),
    (
        "purple",
        body(
            (0x6A, 0x2A, 0x9E),
            (0x8A, 0x44, 0xC4),
            (0xD8, 0xB4, 0xF0),
            (0xB8, 0x3C, 0xFF),
        ),
    ),
    (
        "blue",
        body(
            (0x1C, 0x3E, 0x9C),
            (0x2C, 0x62, 0xD4),
            (0x9C, 0xC4, 0xF4),
            (0x2A, 0x7A, 0xFF),
        ),
    ),
    (
        "pink",
        body(
            (0xC0, 0x3C, 0x7C),
            (0xE0, 0x58, 0x98),
            (0xF8, 0xC0, 0xD8),
            (0xFF, 0x3C, 0x9C),
        ),
    ),
    (
        "green",
        body(
            (0x2A, 0x82, 0x2E),
            (0x3C, 0xB0, 0x3C),
            (0xA8, 0xE0, 0xA0),
            (0x2A, 0xE0, 0x50),
        ),
    ),
    (
        "gold",
        body(
            (0xB0, 0x84, 0x10),
            (0xD8, 0xA8, 0x20),
            (0xF8, 0xE4, 0x98),
            (0xFF, 0xC8, 0x00),
        ),
    ),
    (
        "ghost",
        body(
            (0xBE, 0xC4, 0xDA),
            (0xDC, 0xDE, 0xEE),
            (0xFF, 0xFF, 0xFF),
            (0xFF, 0xB0, 0xD0),
        ),
    ),
    (
        "blue-ringed",
        Palette {
            mantle: (0xA8, 0x78, 0x14),
            arm: (0xE0, 0xB4, 0x30),
            peach: (0xFA, 0xE6, 0x9A),
            flush: (0xFF, 0xCC, 0x20),
            ring: (0x34, 0x58, 0xA8),
            ring_hi: (0x10, 0x90, 0xFF),
            edge: (0x4A, 0x2C, 0x10),
            rings: 1.0,
        },
    ),
];

pub fn lerp_pal(a: &Palette, b: &Palette, t: f32) -> Palette {
    Palette {
        mantle: lerp(a.mantle, b.mantle, t),
        arm: lerp(a.arm, b.arm, t),
        peach: lerp(a.peach, b.peach, t),
        flush: lerp(a.flush, b.flush, t),
        ring: lerp(a.ring, b.ring, t),
        ring_hi: lerp(a.ring_hi, b.ring_hi, t),
        edge: lerp(a.edge, b.edge, t),
        rings: a.rings + (b.rings - a.rings) * t.clamp(0.0, 1.0),
    }
}

/// 256-color cube index for an RGB value.
pub fn to_indexed((r, g, b): Rgb) -> u8 {
    let q = |v: u8| (v as f32 / 255.0 * 5.0).round() as u8;
    16 + 36 * q(r) + 6 * q(g) + q(b)
}

const EYE: Rgb = (0x12, 0x08, 0x06);
const EYE_RING: Rgb = (0x40, 0x16, 0x0A);
const WHITE: Rgb = (0xFF, 0xFF, 0xFF);
const HEART: Rgb = (0xE0, 0x20, 0x30);
const BUBBLE: Rgb = (0x9F, 0xE8, 0xF0);
const WEED: Rgb = (0x1E, 0x6B, 0x35);
const WAVE: Rgb = (0x24, 0x44, 0x8C);
const INK: Rgb = (0x50, 0x50, 0x58);
const FOOD: Rgb = (0xF0, 0x9A, 0x3C);
const DIM: Rgb = (0x80, 0x80, 0x80);

#[derive(Clone, Copy)]
pub struct Cell {
    pub ch: char,
    pub fg: Option<Rgb>,
}

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<Cell>,
    pub ascii: bool,
}

impl Canvas {
    pub fn new(w: u16, h: u16, ascii: bool) -> Self {
        Canvas {
            w: w as usize,
            h: h as usize,
            cells: vec![Cell { ch: ' ', fg: None }; w as usize * h as usize],
            ascii,
        }
    }
    pub fn put(&mut self, x: i32, y: i32, ch: char, c: Rgb) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        let ch = if self.ascii { asciify(ch) } else { ch };
        self.cells[y as usize * self.w + x as usize] = Cell { ch, fg: Some(c) };
    }
    pub fn text_center(&mut self, y: i32, s: &str, c: Rgb) {
        let n = s.chars().count() as i32;
        let x0 = (self.w as i32 - n) / 2;
        for (i, ch) in s.chars().enumerate() {
            self.put(x0 + i as i32, y, ch, c);
        }
    }
    pub fn text(&mut self, x: i32, y: i32, s: &str, c: Rgb) {
        for (i, ch) in s.chars().enumerate() {
            self.put(x + i as i32, y, ch, c);
        }
    }
}

fn asciify(c: char) -> char {
    match c {
        '\u{2588}' => '#',
        '\u{2593}' => '%',
        '\u{2592}' => ':',
        '\u{2591}' => '.',
        '\u{2665}' => 'v',
        '\u{25e0}' => '^',
        '\u{25cf}' => '*',
        '\u{25cb}' => 'o',
        '\u{b0}' => '\'',
        c if c.is_ascii() => c,
        _ => '?',
    }
}

fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    (f(a.0, b.0), f(a.1, b.1), f(a.2, b.2))
}

/// Squid layout for a terminal size. Pure function of (w, h).
#[derive(Clone, Copy, Debug)]
pub struct Geo {
    pub cx: i32,
    pub top: i32,
    pub mh: i32,
    pub hh: i32,
    pub al: i32,
    pub maxw: f32,
    pub hwh: f32,
}

pub fn geo(w: u16, h: u16) -> Geo {
    geo_s(w, h, 1.0)
}

/// Layout at a size factor (1.0 = solo size; smaller when guests share the window).
pub fn geo_s(w: u16, h: u16, scale: f32) -> Geo {
    let total = ((h as f32 * 0.8 * scale).round() as i32).max(8);
    let mh = (total as f32 * 0.42).round() as i32;
    let hh = ((total as f32 * 0.14).round() as i32).max(3);
    let al = (total - mh - hh).max(2);
    let maxw = (w as f32 * 0.24 * scale).min(total as f32 * 0.6).max(3.0);
    Geo {
        cx: w as i32 / 2,
        top: 0,
        mh,
        hh,
        al,
        maxw,
        hwh: maxw * 0.7,
    }
}

/// Everything needed to draw one squid (the host's or a guest's).
#[derive(Clone, Debug)]
pub struct Body {
    pub pal: Palette,
    pub cx: i32,
    pub dx: f32,
    pub dy: f32,
    pub scale: f32,
    pub puff: f32,
    pub curl: f32,
    pub blush: f32,
    pub flash: f32,
    pub bright: f32,
    pub dark: f32,
    pub look: i32,
    pub closed: bool,
    pub blink: bool,
    pub yawn: bool,
    pub lean: f32,
    pub food_x: f32,
    pub t: f32,
    /// Floating label (e.g. "guest") and its top-row x.
    pub tag: Option<&'static str>,
}

fn tone(p: &Palette, bright: f32, dark: f32) -> Palette {
    let f = |c: Rgb| {
        let c = lerp(c, WHITE, 0.4 * bright);
        lerp(c, (0x30, 0x30, 0x38), 0.8 * dark)
    };
    Palette {
        mantle: f(p.mantle),
        arm: f(p.arm),
        peach: f(p.peach),
        flush: f(p.flush),
        ring: f(p.ring),
        ring_hi: f(p.ring_hi),
        edge: f(p.edge),
        rings: p.rings,
    }
}

pub fn render(app: &App, ascii: bool) -> Canvas {
    let (w, h) = (app.w, app.h);
    let mut cv = Canvas::new(w, h, ascii);
    if w < 40 || h < 12 {
        cv.text_center(h as i32 / 2, "terminal too small (min 40x12)", PEACH);
        return cv;
    }
    draw_world(&mut cv, app);
    draw_bubbles(&mut cv, app);
    let bodies = app.bodies();
    for b in &bodies {
        draw_squid(&mut cv, b, app);
    }
    draw_fx(&mut cv, app);
    for b in &bodies {
        if let Some(tag) = b.tag {
            let x = b.cx + b.dx.round() as i32 - tag.len() as i32 / 2;
            cv.text(x, 0, tag, PEACH);
        }
    }
    let hh = h as i32;
    if let Some((m, _)) = &app.cmsg {
        cv.text_center(hh - 2, m, PEACH);
    } else if let Some((m, _)) = &app.note {
        cv.text_center(hh - 2, m, PEACH);
    } else if let Some(m) = app.msg {
        cv.text_center(hh - 2, m, PEACH);
    } else if app.is_away() {
        cv.text_center(hh - 2, "Out visiting!", PEACH);
    }
    cv.text_center(
        hh - 1,
        if app.social {
            "[f] feed   [p] pet   [space] play   [v] visit   [c] color   [q] quit"
        } else {
            "[f] feed   [p] pet   [space] play   [c] color   [q] quit"
        },
        DIM,
    );
    cv
}

fn draw_world(cv: &mut Canvas, app: &App) {
    let (w, h) = (cv.w as i32, cv.h as i32);
    let t = app.t;
    for x in 0..w {
        if ((x as f32) * 0.6 + t * 2.0).sin() > 0.0 {
            cv.put(x, h - 3, '~', WAVE);
        }
    }
    let mut x = 3;
    let mut i = 0;
    while x < w - 2 {
        let sw = ((t * 1.3 + i as f32).sin() * 1.2).round() as i32;
        cv.put(x + sw, h - 5, '(', WEED);
        cv.put(x, h - 4, ')', WEED);
        x += 8 + (i % 3) * 2;
        i += 1;
    }
}

fn draw_bubbles(cv: &mut Canvas, app: &App) {
    for b in &app.bubbles {
        let x = b.x + (b.y * 0.7 + b.phase).sin() * 0.8;
        cv.put(x.round() as i32, b.y.round() as i32, b.ch, BUBBLE);
    }
}

fn draw_fx(cv: &mut Canvas, app: &App) {
    for i in &app.ink {
        let k = i.age / i.life;
        let ch = if k < 0.4 {
            '\u{2588}'
        } else if k < 0.7 {
            '\u{2593}'
        } else {
            '\u{2592}'
        };
        cv.put(
            i.x.round() as i32,
            i.y.round() as i32,
            ch,
            lerp(INK, (0x18, 0x18, 0x1c), k),
        );
    }
    for hr in &app.hearts {
        if hr.age >= 0.0 {
            let c = lerp(HEART, (0x40, 0x08, 0x10), (hr.age - 1.0) / 1.0);
            cv.put(hr.x.round() as i32, hr.y.round() as i32, '\u{2665}', c);
        }
    }
    for (fx, fy) in app.foods() {
        let (x, y) = (fx.round() as i32 - 2, fy.round() as i32);
        cv.text(x, y, "~(o>", FOOD);
        cv.put(x + 2, y, 'o', WHITE);
    }
    for f in &app.flyers {
        let (x, y) = (f.x.round() as i32, f.y.round() as i32);
        if f.ink {
            cv.text(x - 1, y, "\u{2592}\u{2593}\u{2588}", INK);
        } else if f.vx > 0.0 {
            cv.text(x - 2, y, "~(o>", FOOD);
        } else {
            cv.text(x - 2, y, "<o)~", FOOD);
        }
    }
}

/// Draw the squid on a scratch canvas, then shear it (diagonal lean) onto `cv`.
fn draw_squid(cv: &mut Canvas, body: &Body, app: &App) {
    let mut sq = Canvas::new(app.w, app.h, cv.ascii);
    draw_squid_upright(&mut sq, body, app.mono);
    let mid = app.h as f32 * 0.4;
    let lean = body.lean;
    for y in 0..sq.h {
        let s = ((y as f32 - mid) * lean).round() as i32;
        for x in 0..sq.w {
            let c = sq.cells[y * sq.w + x];
            if c.fg.is_some() {
                let nx = x as i32 + s;
                if nx >= 0 && (nx as usize) < cv.w {
                    cv.cells[y * cv.w + nx as usize] = c;
                }
            }
        }
    }
}

fn draw_squid_upright(cv: &mut Canvas, body: &Body, mono: bool) {
    let g = geo_s(cv.w as u16, cv.h as u16, body.scale);
    let (dx, dy) = (body.dx, body.dy);
    let cx = body.cx + dx.round() as i32;
    let top = g.top + dy.round() as i32;
    let puff = body.puff;
    let blush = body.blush;
    let pal = tone(&body.pal, body.bright, body.dark);
    let rings_on = pal.rings > 0.0 && !mono;
    let ring = lerp(pal.ring, pal.ring_hi, body.flash.max(body.bright));
    let mantle = lerp(pal.mantle, pal.flush, blush);
    let arm = lerp(pal.arm, pal.flush, blush * 0.6);
    let t = body.t;

    // mantle + fins
    let mh = g.mh as f32;
    for r in 0..g.mh {
        let tt = (r as f32 + 0.5) / mh;
        let hw = (g.maxw * puff * tt.powf(0.5) * (1.0 - 0.3 * tt * tt)).max(0.5);
        let y = top + r;
        let n = hw.round() as i32;
        for x in -n..=n {
            let edge = hw - (x.abs() as f32);
            let ch = if r == g.mh - 1 || edge < 1.0 {
                '\u{2592}'
            } else if (r + x).rem_euclid(7) == 0 {
                '\u{2593}'
            } else {
                '\u{2588}'
            };
            let c = if r == g.mh - 1 { pal.peach } else { mantle };
            cv.put(cx + x, y, ch, c);
        }
        if rings_on && r < g.mh - 1 {
            for x in (1 - n)..n {
                if (x + 3 * r).rem_euclid(7) == 0 {
                    cv.put(cx + x - 1, y, '\u{2593}', pal.edge);
                    cv.put(cx + x + 1, y, '\u{2593}', pal.edge);
                    cv.put(cx + x, y, '\u{25cb}', ring);
                }
            }
        }
        if tt < 0.55 {
            let fw = g.maxw * 0.4 * puff * (1.0 - (tt - 0.28).abs() / 0.28).max(0.0);
            for k in 1..=(fw.round() as i32) {
                cv.put(cx - n - k, y, '\u{2592}', arm);
                cv.put(cx + n + k, y, '\u{2592}', arm);
            }
        }
    }

    // head
    let y0 = top + g.mh + g.hh;
    let hwh = g.hwh * (1.0 + 0.5 * (puff - 1.0));
    for r in 0..g.hh {
        let t2 = r as f32 / g.hh as f32;
        let n = (hwh * (1.0 - 0.25 * t2)).round() as i32;
        for x in -n..=n {
            cv.put(cx + x, top + g.mh + r, '\u{2588}', mantle);
        }
    }
    if blush > 0.0 {
        let ry = (g.hh / 2).max(1);
        for s in [-1, 1] {
            let bx = cx + s * (hwh * 0.9).round() as i32;
            cv.put(bx, top + g.mh + ry + 1, '\u{25cf}', pal.flush);
        }
    }

    // eyes
    let ry = (g.hh / 2).max(1);
    let rx = ry * 2;
    let ey = top + g.mh + g.hh / 2;
    let look = body.look;
    for s in [-1i32, 1] {
        let ex = cx + s * (hwh * 0.95).round() as i32;
        if body.closed {
            for x in -rx..=rx {
                cv.put(ex + x, ey, '\u{25e0}', EYE);
            }
        } else if body.blink {
            for x in -rx..=rx {
                cv.put(ex + x, ey, '-', EYE);
            }
        } else {
            for y in -ry..=ry {
                for x in -rx..=rx {
                    let d = (x as f32 / rx as f32).powi(2) + (y as f32 / ry as f32).powi(2);
                    if d <= 1.15 {
                        let c = if d > 0.7 { EYE_RING } else { EYE };
                        cv.put(ex + x, ey + y, '\u{2588}', c);
                    }
                }
            }
            cv.put(
                ex - rx / 2 + look * (rx / 2).max(1),
                ey - ry / 2,
                '\u{25cf}',
                WHITE,
            );
        }
    }
    if body.yawn {
        cv.put(cx, y0 - 1, 'O', EYE);
    }

    // arms + 2 long tentacles
    let curl = body.curl;
    let fx = body.food_x;
    for i in 0..10 {
        let tentacle = i >= 8;
        let (base, len) = if tentacle {
            let s = if i == 8 { -0.3 } else { 0.3 };
            (s * hwh, g.al as f32)
        } else {
            let b = (i as f32 - 3.5) / 3.5 * hwh * 0.9;
            (b, g.al as f32 * (0.7 + 0.05 * (i % 3) as f32))
        };
        let n = len.round() as i32;
        let amp = g.maxw * 0.5;
        let mut prev = cx as f32 + base;
        for k in 0..n {
            let d = k as f32 / len;
            let mut x = cx as f32
                + base * (1.0 + 2.4 * d)
                + amp * d * (t * 2.2 - d * 5.0 + i as f32 * 0.9).sin();
            x += (fx - x) * curl * d;
            let xi = x.round() as i32;
            let slope = x - prev;
            prev = x;
            let y = y0 + k;
            let club = tentacle && d > 0.85;
            let (ch, c) = if k % 3 == 2 && d > 0.2 {
                if rings_on {
                    ('\u{25cb}', ring)
                } else {
                    ('o', pal.peach)
                }
            } else if d > 0.8 {
                ('\u{2593}', arm)
            } else {
                let _ = slope;
                ('\u{2588}', arm)
            };
            cv.put(xi, y, ch, c);
            if club {
                cv.put(xi - 1, y, '\u{2593}', arm);
                cv.put(xi + 1, y, '\u{2593}', arm);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn squid_rows(w: u16, h: u16) -> (usize, usize, Vec<usize>) {
        let app = App::new(w, h, 7);
        let cv = render(&app, false);
        let mut rows = vec![];
        let (mut lo, mut hi) = (usize::MAX, 0);
        for y in 0..cv.h {
            let mut xs = vec![];
            for x in 0..cv.w {
                let c = cv.cells[y * cv.w + x];
                if matches!(c.fg, Some(MANTLE) | Some(ARM)) {
                    xs.push(x);
                }
            }
            if !xs.is_empty() {
                lo = lo.min(y);
                hi = hi.max(y);
                rows.push(xs[0] + xs[xs.len() - 1]);
            }
        }
        (lo, hi, rows)
    }

    #[test]
    fn fills_about_80_percent_and_centered() {
        for (w, h) in [(80u16, 24u16), (120, 40), (200, 60), (300, 80), (40, 12)] {
            let (lo, hi, sums) = squid_rows(w, h);
            let span = (hi - lo + 1) as f32 / h as f32;
            assert!((0.6..=0.92).contains(&span), "{w}x{h}: span {span}");
            let mid: i32 = sums.iter().map(|&s| s as i32).sum::<i32>() / sums.len() as i32;
            assert!(
                (mid - w as i32).abs() <= w as i32 / 3,
                "{w}x{h}: center {mid}"
            );
        }
    }

    fn ring_count(app: &App, ascii: bool) -> usize {
        let ch = if ascii { 'o' } else { '\u{25cb}' };
        let cv = render(app, ascii);
        cv.cells
            .iter()
            .filter(|c| c.ch == ch && c.fg.is_some_and(|f| f.2 > f.0))
            .count()
    }

    #[test]
    fn blue_ringed_has_about_60_rings_and_flashes() {
        let mut app = App::new(120, 40, 7);
        app.init_color(7, false);
        let n = ring_count(&app, false);
        assert!((50..=70).contains(&n), "rings {n}");
        let small = App::new(80, 24, 7);
        let mut small = small;
        small.init_color(7, false);
        assert!(ring_count(&small, false) < n);
        let rc = |app: &App| {
            let cv = render(app, false);
            cv.cells
                .iter()
                .find(|c| c.ch == '\u{25cb}')
                .unwrap()
                .fg
                .unwrap()
        };
        let before = rc(&app);
        app.press(crate::app::Key::Pet);
        for _ in 0..8 {
            app.update(0.0625);
        }
        let after = rc(&app);
        assert!(
            after.2 > before.2 && after.0 < before.0,
            "{before:?} {after:?}"
        );
        app.mono = true;
        assert_eq!(ring_count(&app, false), 0);
    }

    #[test]
    fn red_is_the_old_constants() {
        let p = PRESETS[0].1;
        assert_eq!(
            (p.mantle, p.arm, p.peach, p.flush),
            (
                (0xB5, 0x40, 0x1F),
                (0xD4, 0x52, 0x1E),
                (0xE8, 0xA0, 0x80),
                (0xFF, 0x5A, 0x2A)
            )
        );
    }

    #[test]
    fn presets_are_shaded_and_distinct_in_256() {
        let lum = |c: Rgb| 0.3 * c.0 as f32 + 0.59 * c.1 as f32 + 0.11 * c.2 as f32;
        let mut seen = std::collections::HashSet::new();
        for (name, p) in PRESETS {
            assert!(lum(p.mantle) < lum(p.arm), "{name}");
            assert!(lum(p.peach) > lum(p.arm) + 15.0, "{name}");
            let sat = |c: Rgb| c.0.max(c.1).max(c.2) as i32 - c.0.min(c.1).min(c.2) as i32;
            assert!(sat(p.flush) >= sat(p.arm), "{name}");
            assert!(
                seen.insert((to_indexed(p.mantle), to_indexed(p.arm), to_indexed(p.peach))),
                "{name}"
            );
        }
    }

    #[test]
    fn too_small_message() {
        let app = App::new(30, 10, 1);
        let cv = render(&app, false);
        assert!(cv.cells.iter().any(|c| c.ch == 'm'));
    }

    #[test]
    fn ascii_mode_has_no_unicode() {
        let mut app = App::new(100, 30, 1);
        app.press(crate::app::Key::Pet);
        for _ in 0..20 {
            app.update(0.05);
        }
        let cv = render(&app, true);
        assert!(cv.cells.iter().all(|c| c.ch.is_ascii()));
    }
}

#[cfg(test)]
mod golden {
    use super::*;
    /// Solo with a seed must render exactly as the build before visits existed.
    #[test]
    fn solo_seed_matches_old_build() {
        let mut app = App::new(120, 40, 42);
        let mut h: u64 = 1469598103934665603;
        for i in 0..(60 * 16) {
            app.update(1.0 / 16.0);
            if i % 8 == 0 {
                let cv = render(&app, false);
                for c in &cv.cells {
                    let v = c.ch as u64
                        ^ c.fg.map_or(0, |f| {
                            ((f.0 as u64) << 16) | ((f.1 as u64) << 8) | f.2 as u64
                        }) << 8;
                    h = (h ^ v).wrapping_mul(1099511628211);
                }
            }
        }
        assert_eq!(h, 10832562962011645795);
    }
}
