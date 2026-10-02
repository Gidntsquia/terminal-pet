//! Procedural drawing of the squid and its world onto a `Canvas`.
use crate::app::App;

pub type Rgb = (u8, u8, u8);
pub const MANTLE: Rgb = (0xB5, 0x40, 0x1F);
pub const ARM: Rgb = (0xD4, 0x52, 0x1E);
pub const PEACH: Rgb = (0xE8, 0xA0, 0x80);
pub const FLUSH: Rgb = (0xFF, 0x5A, 0x2A);
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
    let total = ((h as f32 * 0.8).round() as i32).max(8);
    let mh = (total as f32 * 0.42).round() as i32;
    let hh = ((total as f32 * 0.14).round() as i32).max(3);
    let al = (total - mh - hh).max(2);
    let maxw = (w as f32 * 0.24).min(total as f32 * 0.6).max(3.0);
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

pub fn render(app: &App, ascii: bool) -> Canvas {
    let (w, h) = (app.w, app.h);
    let mut cv = Canvas::new(w, h, ascii);
    if w < 40 || h < 12 {
        cv.text_center(h as i32 / 2, "terminal too small (min 40x12)", PEACH);
        return cv;
    }
    draw_world(&mut cv, app);
    draw_bubbles(&mut cv, app);
    draw_squid(&mut cv, app);
    draw_fx(&mut cv, app);
    let hh = h as i32;
    if let Some(m) = app.msg {
        cv.text_center(hh - 2, m, PEACH);
    }
    cv.text_center(hh - 1, "[f] feed   [p] pet   [space] play   [q] quit", DIM);
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
    if let Some((fx, fy)) = app.food() {
        let (x, y) = (fx.round() as i32 - 2, fy.round() as i32);
        cv.text(x, y, "~(o>", FOOD);
        cv.put(x + 2, y, 'o', WHITE);
    }
}

/// Draw the squid on a scratch canvas, then shear it (diagonal lean) onto `cv`.
fn draw_squid(cv: &mut Canvas, app: &App) {
    let mut sq = Canvas::new(app.w, app.h, cv.ascii);
    draw_squid_upright(&mut sq, app);
    let mid = app.h as f32 * 0.4;
    let lean = app.lean();
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

fn draw_squid_upright(cv: &mut Canvas, app: &App) {
    let g = geo(app.w, app.h);
    let (dx, dy) = app.offset();
    let cx = g.cx + dx.round() as i32;
    let top = g.top + dy.round() as i32;
    let puff = app.puff();
    let blush = app.blush();
    let mantle = lerp(MANTLE, FLUSH, blush);
    let arm = lerp(ARM, FLUSH, blush * 0.6);
    let t = app.t;

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
            let c = if r == g.mh - 1 { PEACH } else { mantle };
            cv.put(cx + x, y, ch, c);
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
            cv.put(bx, top + g.mh + ry + 1, '\u{25cf}', FLUSH);
        }
    }

    // eyes
    let ry = (g.hh / 2).max(1);
    let rx = ry * 2;
    let ey = top + g.mh + g.hh / 2;
    let look = app.look();
    for s in [-1i32, 1] {
        let ex = cx + s * (hwh * 0.95).round() as i32;
        if app.eyes_closed() {
            for x in -rx..=rx {
                cv.put(ex + x, ey, '\u{25e0}', EYE);
            }
        } else if app.blinking() {
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
    if app.yawning() {
        cv.put(cx, y0 - 1, 'O', EYE);
    }

    // arms + 2 long tentacles
    let curl = app.curl();
    let (fx, _) = app.food().unwrap_or((g.cx as f32, 0.0));
    let fx = fx + dx;
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
                ('o', PEACH)
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
    use crate::app::State;

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

    #[test]
    fn too_small_message() {
        let app = App::new(30, 10, 1);
        let cv = render(&app, false);
        assert!(cv.cells.iter().any(|c| c.ch == 'm'));
    }

    #[test]
    fn ascii_mode_has_no_unicode() {
        let mut app = App::new(100, 30, 1);
        app.start(State::Petting);
        for _ in 0..20 {
            app.update(0.05);
        }
        let cv = render(&app, true);
        assert!(cv.cells.iter().all(|c| c.ch.is_ascii()));
    }
}
