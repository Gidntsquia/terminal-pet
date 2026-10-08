//! Procedural drawing of the cat and its cozy room. All shapes are ellipses and
//! curves in cell space (a cell is half as wide as it is tall, so a round shape
//! has `a ≈ 2b`), painted back to front onto a scratch canvas.
use crate::app::App;
use crate::art::{Body, Canvas, Geo, Rgb, lerp};
use crate::cat::{BREEDS, Breed, Pat};

const WHITE: Rgb = (0xFF, 0xFF, 0xFF);
const WHISKER: Rgb = (0xE8, 0xE8, 0xF2);
const FLUSH: Rgb = (0xFF, 0x6E, 0x8C);

/// Cat layout for a terminal size. `mh` is the head, `hh` the chest, `al` the rest.
pub fn geo_cat(w: u16, h: u16, scale: f32) -> Geo {
    let total = ((h as f32 * 0.68 * scale).round() as i32).max(8);
    let mh = (total as f32 * 0.35).round() as i32;
    let hh = (total as f32 * 0.30).round() as i32;
    let al = (total - mh - hh).max(2);
    let maxw = (w as f32 * 0.27 * scale).min(total as f32 * 0.8).max(3.0);
    Geo {
        cx: w as i32 / 2,
        top: 3,
        mh,
        hh,
        al,
        maxw,
        hwh: maxw * 0.7,
    }
}

fn hash(x: i32, y: i32, s: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ s.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

fn shade(c: Rgb, k: f32) -> Rgb {
    if k >= 0.0 {
        lerp(c, WHITE, k)
    } else {
        lerp(c, (0, 0, 0), -k)
    }
}

fn lum(c: Rgb) -> f32 {
    0.3 * c.0 as f32 + 0.59 * c.1 as f32 + 0.11 * c.2 as f32
}

#[derive(Clone, Copy, PartialEq)]
enum Reg {
    Head,
    Ear,
    Body,
    Chest,
    Paw,
    Muzzle,
    Tail(f32),
}

/// Coat colors (blended while fading between breeds) plus the pattern/shape breed.
struct Look {
    sb: &'static Breed,
    base: Rgb,
    dark: Rgb,
    light: Rgb,
    eye: Rgb,
    nose: Rgb,
    inner: Rgb,
}

fn look_of(body: &Body) -> Look {
    let from = &BREEDS[body.breed_from.min(BREEDS.len() - 1)];
    let to = &BREEDS[body.breed.min(BREEDS.len() - 1)];
    let k = body.bfade;
    let sb = if k >= 0.5 || body.bfade >= 1.0 {
        to
    } else {
        from
    };
    let tone = |c: Rgb| {
        let c = lerp(c, WHITE, 0.4 * body.bright);
        lerp(c, (0x30, 0x30, 0x38), 0.8 * body.dark)
    };
    let mix = |a: Rgb, b: Rgb| tone(lerp(a, b, k));
    Look {
        sb,
        base: mix(from.base, to.base),
        dark: mix(from.dark, to.dark),
        light: mix(from.light, to.light),
        eye: mix(from.eye, to.eye),
        nose: mix(from.nose, to.nose),
        inner: mix(from.inner, to.inner),
    }
}

struct P<'a> {
    cv: &'a mut Canvas,
    l: Look,
    cx: f32,
    y0: f32,
    t: f32,
    m: f32,
    time: f32,
    ruffle: f32,
    ruffle_y: f32,
}

impl P<'_> {
    fn amp(&self, y: i32) -> f32 {
        if self.ruffle <= 0.0 {
            return 0.0;
        }
        let prox = (1.0 - (y as f32 - self.ruffle_y).abs() / (self.t * 0.4)).max(0.0);
        self.ruffle * prox
    }

    /// Fur glyph and color for a cell of a body part.
    fn fur(&self, reg: Reg, x: i32, y: i32) -> (char, Rgb) {
        let l = &self.l;
        let (ux, uy) = (x as f32 - self.cx, y as f32 - self.y0);
        let (nx, ny) = (ux / self.m, uy / self.t);
        let hair = l.sb.pat == Pat::Hairless;
        let tex = |c: Rgb| {
            if !hair && (x + 3 * y).rem_euclid(7) == 0 {
                ('\u{2593}', shade(c, -0.14))
            } else {
                ('\u{2588}', c)
            }
        };
        let paw_c = lerp(l.base, WHITE, 0.12);
        let r = match (l.sb.pat, reg) {
            (Pat::Tuxedo, Reg::Paw | Reg::Muzzle) => ('\u{2588}', l.light),
            (Pat::Tuxedo, Reg::Chest) => ('\u{2588}', l.light),
            (Pat::Tuxedo, Reg::Body) if nx.abs() < 0.34 && ny < 0.82 => ('\u{2588}', l.light),
            (Pat::Pointed, Reg::Muzzle | Reg::Ear | Reg::Paw | Reg::Tail(_)) => {
                ('\u{2593}', l.dark)
            }
            (Pat::Pointed, Reg::Head) if nx.abs() < 0.3 && (0.17..0.31).contains(&ny) => {
                ('\u{2593}', l.dark)
            }
            (Pat::Pointed, Reg::Body | Reg::Chest) if nx.abs() > 0.78 => {
                ('\u{2592}', shade(l.base, -0.22))
            }
            (Pat::Tabby | Pat::Striped, _) => {
                let stripe = match reg {
                    Reg::Head => {
                        (ny < 0.15 && (ux.round() as i32).rem_euclid(4) == 0)
                            || (nx.abs() > 0.45 && (ny * self.t) as i32 % 3 == 0 && ny > 0.17)
                    }
                    Reg::Body | Reg::Chest => nx.abs() > 0.3 && (uy as i32).rem_euclid(3) == 0,
                    Reg::Paw => (uy as i32).rem_euclid(3) == 0 && nx.abs() > 0.3,
                    Reg::Tail(s) => ((s * 9.0) as i32) % 2 == 1,
                    _ => false,
                };
                if stripe {
                    ('\u{2593}', l.dark)
                } else if reg == Reg::Muzzle {
                    ('\u{2588}', l.light)
                } else {
                    tex(l.base)
                }
            }
            (Pat::Calico, _) => {
                let n = (ux * 0.21 + 1.3).sin()
                    + (uy * 0.55 + 0.4).cos()
                    + (ux * 0.11 + uy * 0.37).sin();
                let orange = l.light;
                match reg {
                    Reg::Ear => {
                        if ux < 0.0 {
                            ('\u{2592}', orange)
                        } else {
                            ('\u{2593}', l.dark)
                        }
                    }
                    Reg::Head if ux < -0.12 * self.m && ny < 0.2 => ('\u{2592}', orange),
                    Reg::Head if ux > 0.34 * self.m && ny < 0.21 => ('\u{2593}', l.dark),
                    Reg::Muzzle | Reg::Paw | Reg::Head => tex(l.base),
                    _ if n > 0.8 => ('\u{2592}', orange),
                    _ if n < -1.0 => ('\u{2593}', l.dark),
                    _ => tex(l.base),
                }
            }
            (Pat::Hairless, _) => {
                let wr = match reg {
                    Reg::Head => {
                        (0.06..0.135).contains(&ny) && (uy as i32) % 2 == 0 && nx.abs() < 0.5
                    }
                    Reg::Body | Reg::Chest => nx.abs() > 0.28 && (uy as i32).rem_euclid(4) == 0,
                    _ => false,
                };
                if wr {
                    ('~', l.dark)
                } else {
                    ('\u{2588}', l.base)
                }
            }
            (_, Reg::Muzzle) => ('\u{2588}', lerp(l.base, WHITE, 0.4)),
            (_, Reg::Paw) => tex(paw_c),
            _ => tex(l.base),
        };
        let mut r = r;
        let a = self.amp(y);
        if a > 0.3 && hash(x, y, (self.time * 12.0) as u32).is_multiple_of(3) {
            r.0 = ['\u{2592}', '\u{2591}', '\u{2593}'][(hash(y, x, 5) % 3) as usize];
        }
        r
    }

    fn ell(&mut self, cx: f32, cy: f32, a: f32, b: f32, reg: Reg) {
        let fl_max = if matches!(reg, Reg::Tail(_) | Reg::Muzzle | Reg::Paw) {
            0.0
        } else {
            self.l.sb.fluff
        };
        let pad_x = a + fl_max * 2.0 + 3.0;
        let pad_y = b + fl_max + 2.0;
        for y in (cy - pad_y).floor() as i32..=(cy + pad_y).ceil() as i32 {
            let wob = self.amp(y) * 1.6 * (self.time * 14.0 + y as f32 * 0.9).sin();
            for x in (cx - pad_x).floor() as i32..=(cx + pad_x).ceil() as i32 {
                let dx = (x as f32 - cx - wob) / a;
                let dy = (y as f32 - cy) / b;
                let n = (dx * dx + dy * dy).sqrt();
                if n <= 1.0 {
                    let (ch, c) = self.fur(reg, x, y);
                    self.cv.put(x, y, ch, c);
                } else if fl_max > 0.0 && self.empty(x, y) {
                    let d = (n - 1.0) * a.min(b * 2.0);
                    let lim = if self.l.sb.shaggy {
                        fl_max * (0.25 + 1.1 * (hash(x, y, 9) % 100) as f32 / 100.0)
                    } else {
                        fl_max
                    };
                    if d <= lim {
                        let ch = if d < lim * 0.5 {
                            '\u{2592}'
                        } else {
                            '\u{2591}'
                        };
                        self.cv.put(x, y, ch, self.l.base);
                    }
                }
            }
        }
    }

    fn empty(&self, x: i32, y: i32) -> bool {
        x >= 0
            && y >= 0
            && (x as usize) < self.cv.w
            && (y as usize) < self.cv.h
            && self.cv.cells[y as usize * self.cv.w + x as usize]
                .fg
                .is_none()
    }

    fn ear(&mut self, tipx: f32, tipy: f32, basex: f32, basey: f32, hw: f32) {
        let rows = (basey - tipy).ceil() as i32;
        for r in 0..=rows {
            let f = (r as f32 / rows.max(1) as f32).clamp(0.0, 1.0);
            let y = tipy.round() as i32 + r;
            let c = tipx + (basex - tipx) * f;
            let half = (hw * f).max(0.6);
            let n = half.round().max(1.0) as i32;
            for dx in -n..=n {
                let x = (c + dx as f32).round() as i32;
                let (ch, col) = self.fur(Reg::Ear, x, y);
                let inner = f > 0.4 && (dx.abs() as f32) < half * 0.5 && !self.l.sb.fold;
                if inner {
                    self.cv.put(x, y, '\u{2588}', self.l.inner);
                } else {
                    self.cv.put(x, y, ch, col);
                }
            }
        }
        if self.l.sb.tufts {
            let (x, y) = (tipx.round() as i32, tipy.round() as i32);
            self.cv.put(x, y - 1, '|', self.l.dark);
            self.cv.put(
                x + if tipx > self.cx { 1 } else { -1 },
                y,
                '\'',
                self.l.dark,
            );
        }
    }
}

pub fn draw_cat(cv: &mut Canvas, body: &Body, mono: bool) {
    let g = geo_cat(cv.w as u16, cv.h as u16, body.scale);
    let l = look_of(body);
    let tot0 = (g.mh + g.hh + g.al) as f32;
    let size = l.sb.size;
    let t = (tot0 * size).min(cv.h as f32 - 5.0).max(tot0);
    let m = (g.maxw * size)
        .min(cv.w as f32 * 0.26)
        .max(g.maxw.min(cv.w as f32 * 0.26));
    let pose = body.pose;
    let sb = l.sb;
    let walk_bob = if pose.walking && pose.step.sin() > 0.75 {
        -1.0
    } else {
        0.0
    };
    let cx = (body.cx as f32 + body.dx).round()
        + if pose.walking {
            (pose.step * 0.5).sin() * 0.6
        } else {
            0.0
        };
    let feet0 = g.top as f32 + tot0;
    let y0 = (feet0 - t).max(3.0) + body.dy.round() + walk_bob;
    let ruff_ear = if body.ruffle > 0.3 {
        (body.t * 20.0).sin()
    } else {
        0.0
    };
    let mut p = P {
        cv,
        l,
        cx,
        y0,
        t,
        m,
        time: body.t,
        ruffle: body.ruffle,
        ruffle_y: body.ruffle_at.1 - 0.0,
    };
    let sq = 1.0 + 0.5 * (body.puff - 1.0);
    let touch = body.touch;
    let py = |f: f32| y0 + f * t;
    let px = |f: f32| cx + f * m;

    // ---- tail (behind the body) ----
    {
        let sx = body.tail;
        let sway = (body.t * 1.6).sin() * 0.05
            + if pose.walking {
                (pose.step * 0.5).sin() * 0.12
            } else {
                0.0
            };
        let curl = touch * 0.12 + pose.stretch * 0.15;
        let p0 = (0.80, 0.94);
        let p1 = (1.75, 0.99);
        let p2 = (1.62 + sway, 0.52);
        let p3 = (1.20 + sway - 0.25 * touch, 0.50 - curl);
        let bez = |s: f32| {
            let u = 1.0 - s;
            let f = |a: f32, b: f32, c: f32, d: f32| {
                u * u * u * a + 3.0 * u * u * s * b + 3.0 * u * s * s * c + s * s * s * d
            };
            (f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1))
        };
        let puffy = if pose.puffy { 1.7 } else { 1.0 };
        let (ta, tb) = (
            (0.085 * m * if sb.bushy { 1.8 } else { 1.0 } * puffy).max(1.3),
            (0.026 * t * if sb.bushy { 1.8 } else { 1.0 } * puffy).max(0.8),
        );
        for i in 0..=40 {
            let s = i as f32 / 40.0;
            let (bx, by) = bez(s);
            let tw = if s > 0.85 { pose.tail_tw * 1.2 } else { 0.0 };
            p.ell(cx + sx * bx * m + tw, py(by), ta, tb, Reg::Tail(s));
        }
    }

    // ---- body ----
    let crouch = pose.crouch;
    let stretch = pose.stretch;
    let hdy_rows =
        0.13 * t * stretch + 0.07 * t * crouch + 0.10 * t * pose.pounce + 0.05 * t * pose.eat
            - 1.5 * pose.sneeze
            - 0.55 * body.head_dy;
    let wig = pose.wiggle * 1.2;
    p.ell(
        cx + wig,
        py(0.70 - 0.05 * stretch + 0.03 * crouch),
        0.90 * m * sq * (1.0 + 0.08 * crouch),
        0.27 * t * (1.0 + 0.1 * stretch - 0.08 * crouch),
        Reg::Body,
    );
    p.ell(
        cx,
        py(0.52 + 0.05 * crouch) + hdy_rows * 0.4,
        0.60 * m * sq,
        0.16 * t,
        Reg::Chest,
    );

    // ---- front paws ----
    let spread = 0.26 + 0.38 * stretch + 0.1 * pose.pounce;
    let lift = |side: f32| {
        if pose.walking || pose.running {
            0.05 * t * (pose.step * side).sin().max(0.0)
        } else {
            0.0
        }
    };
    let pounce_up = 0.2 * t * pose.pounce;
    let (pa, pb) = (0.17 * m * (1.0 + 0.2 * stretch), 0.075 * t);
    let left = (px(-spread), py(0.90) - lift(1.0) - pounce_up);
    p.ell(left.0, left.1, pa, pb, Reg::Paw);
    // right paw: rests, bats a treat, or washes the face
    let rest = (px(spread), py(0.90) - lift(-1.0) - pounce_up);
    let face = (px(0.30), py(0.29) + 0.025 * t * pose.groom_ph.sin());
    let swat = (px(0.62), py(0.60) + 0.03 * t * (body.t * 30.0).sin());
    let rp = {
        let mut q = rest;
        for (k, tgt) in [(pose.groom, face), (pose.swat, swat)] {
            if k > 0.0 {
                q = (q.0 + (tgt.0 - q.0) * k, q.1 + (tgt.1 - q.1) * k);
            }
        }
        q
    };
    let raised_late = pose.groom > 0.0 || pose.swat > 0.0;
    if !raised_late {
        p.ell(rp.0, rp.1, pa, pb, Reg::Paw);
    }
    for (c, _) in [(left, 0), (rest, 1)] {
        if raised_late && c == rest {
            continue;
        }
        let toe = shade(p.l.base, -0.4);
        for dx in [-1.0, 1.0] {
            p.cv.put(
                (c.0 + dx).round() as i32,
                (c.1 + pb * 0.4).round() as i32,
                '|',
                toe,
            );
        }
    }

    // ---- head ----
    let hshift = pose.gaze.0.round()
        * (pose.walking as i32 as f32 * 0.0 + 1.0)
        * if pose.gaze.1 > 0.5 { 0.0 } else { 1.0 };
    let hcx = cx + hshift + wig * 0.5;
    let hcy = py(0.27) + hdy_rows;
    let (ha, hb) = if sb.fold {
        (0.80 * m, 0.20 * t)
    } else if sb.flat {
        (0.84 * m, 0.18 * t)
    } else {
        (0.82 * m, 0.19 * t)
    };
    let ha = ha * (1.0 + 0.3 * (body.puff - 1.0)).max(1.0);
    // ears first (behind the head)
    let ear_h = 0.15 * t * sb.ear;
    let ear_w = 0.19 * m * sb.ear.sqrt();
    for side in [-1.0f32, 1.0] {
        let f = if pose.ear.signum() == side {
            pose.ear.abs()
        } else {
            0.0
        }
        .max(ruff_ear.abs());
        if sb.fold {
            continue;
        }
        let tipx = hcx + side * (0.60 * ha + 0.8 * f);
        let tipy = (hcy - hb - ear_h * 0.55 + f * 1.2).max(y0 - 1.0);
        let basex = hcx + side * 0.42 * ha;
        let basey = hcy - hb * 0.55;
        p.ear(tipx, tipy, basex, basey, ear_w);
    }
    p.ell(hcx, hcy, ha, hb, Reg::Head);
    if sb.fold {
        for side in [-1.0f32, 1.0] {
            let f = if pose.ear.signum() == side {
                pose.ear.abs()
            } else {
                0.0
            }
            .max(ruff_ear.abs());
            let ex = hcx + side * 0.62 * ha;
            let ey = hcy - hb * 0.78 + f;
            p.ell(ex, ey, 0.17 * m, 0.05 * t + 0.4, Reg::Ear);
            let c = shade(p.l.base, -0.4);
            for dx in -2..=2 {
                p.cv.put(
                    (ex + dx as f32).round() as i32,
                    (ey + 0.8).round() as i32,
                    '_',
                    c,
                );
            }
        }
    }
    // muzzle
    let nose_y = hcy + (if sb.flat { 0.05 } else { 0.06 }) * t;
    p.ell(hcx, nose_y + 0.4, 0.2 * m, 0.045 * t + 0.3, Reg::Muzzle);

    // forehead M for tabbies
    if sb.pat == Pat::Tabby {
        let my = (hcy - hb * 0.62).round() as i32;
        let mc = shade(p.l.dark, 0.0);
        if ha >= 9.0 {
            p.cv.text(hcx.round() as i32 - 2, my, "|\\/|", mc);
        } else {
            p.cv.put(hcx.round() as i32, my, 'M', mc);
        }
    }

    // ---- face ----
    let ea = (0.13 * m * if sb.flat { 1.2 } else { 1.0 }).max(1.5);
    let eb = (0.026 * t * if sb.flat { 1.15 } else { 1.0 }).max(0.9);
    let ey = hcy - 0.02 * t + if sb.flat { 0.025 * t } else { 0.0 };
    let lid = if lum(p.l.base) < 90.0 {
        (0xB8, 0xB8, 0xC0)
    } else {
        (0x30, 0x20, 0x18)
    };
    let squint = body.boop > 0.15;
    let closed = body.closed || squint || pose.groom > 0.3;
    for s in [-1.0f32, 1.0] {
        let ex = hcx + s * 0.36 * ha;
        let (exi, eyi) = (ex.round() as i32, ey.round() as i32);
        let w = ea.round() as i32;
        if closed {
            for x in -w..=w {
                p.cv.put(exi + x, eyi, '\u{25e0}', lid);
            }
        } else if body.blink {
            for x in -w..=w {
                p.cv.put(exi + x, eyi, '-', lid);
            }
        } else {
            let slow = pose.slow;
            let rows = eb.ceil() as i32;
            for y in -rows..=rows {
                for x in -w..=w {
                    let d = (x as f32 / ea).powi(2) + (y as f32 / (eb + 0.4)).powi(2);
                    if d <= 1.05 {
                        if slow && y < 0 {
                            continue;
                        }
                        p.cv.put(exi + x, eyi + y, '\u{2588}', p.l.eye);
                    }
                }
            }
            if slow {
                for x in -w..=w {
                    p.cv.put(exi + x, eyi, '-', lid);
                }
            }
            // slit pupil follows the gaze
            let gx = (pose.gaze.0 * (ea * 0.45)).round() as i32;
            let pw = if ea > 3.8 { 1 } else { 0 };
            for y in -rows..=rows {
                for x in -pw..=pw {
                    if !(slow && y < 0) {
                        p.cv.put(exi + gx + x, eyi + y, '\u{2588}', (0x10, 0x08, 0x08));
                    }
                }
            }
            if !slow {
                p.cv.put(exi - w / 2 - 1, eyi - rows.min(1), '\u{2588}', WHITE);
            }
        }
    }
    // nose + mouth
    let (nx, ny) = (hcx.round() as i32, nose_y.round() as i32);
    if squint {
        p.cv.put(nx - 1, ny, '>', p.l.dark);
        p.cv.put(nx, ny, '\u{25bc}', p.l.nose);
        p.cv.put(nx + 1, ny, '<', p.l.dark);
        p.cv.put(nx, ny - 1, '~', p.l.dark);
    } else {
        p.cv.put(nx, ny, '\u{25bc}', p.l.nose);
    }
    let mouth_c = if lum(p.l.base) < 90.0 {
        (0xB0, 0xA0, 0xA8)
    } else {
        shade(p.l.base, -0.55)
    };
    let chew = pose.eat > 0.2 && ((body.t * 8.0) as i32) % 2 == 0;
    p.cv.put(nx, ny + 1, if chew { 'o' } else { 'w' }, mouth_c);
    // blush
    if body.blush > 0.0 {
        for s in [-1.0f32, 1.0] {
            p.cv.put(
                (hcx + s * 0.58 * ha).round() as i32,
                ey.round() as i32 + 2,
                '\u{25cf}',
                FLUSH,
            );
        }
    }
    // whiskers
    let wy = ny;
    let from = 0.72 * ha;
    let to = ha + 0.14 * m;
    for s in [-1i32, 1] {
        for (dy, ch_out) in [
            (-1i32, if s < 0 { '\\' } else { '/' }),
            (0, '-'),
            (1, if s < 0 { '/' } else { '\\' }),
        ] {
            let mut x = from;
            while x <= to {
                let xi = hcx.round() as i32 + s * x.round() as i32;
                let last = x + 1.0 > to;
                p.cv.put(xi, wy + dy, if last { ch_out } else { '-' }, WHISKER);
                x += 1.0;
            }
        }
    }
    // the washing / batting paw goes on top of the face
    if raised_late {
        p.ell(rp.0, rp.1, pa, pb, Reg::Paw);
        let toe = shade(p.l.base, -0.4);
        p.cv.put(
            (rp.0 - 1.0).round() as i32,
            (rp.1 + pb * 0.4).round() as i32,
            '|',
            toe,
        );
        p.cv.put(
            (rp.0 + 1.0).round() as i32,
            (rp.1 + pb * 0.4).round() as i32,
            '|',
            toe,
        );
    }
    let _ = mono;
    let _ = touch;
}

/// Copy a cat drawn on a scratch canvas into the scene, shearing it about the feet.
pub fn draw_cat_into(cv: &mut Canvas, body: &Body, mono: bool, id: u8, w: u16, h: u16) {
    let mut sc = Canvas::new(w, h, cv.ascii);
    draw_cat(&mut sc, body, mono);
    let g = geo_cat(w, h, body.scale);
    let feet = (g.top + g.mh + g.hh + g.al) as f32;
    for y in 0..sc.h {
        let s = ((y as f32 - feet) * body.lean).round() as i32;
        for x in 0..sc.w {
            let c = sc.cells[y * sc.w + x];
            if c.fg.is_some() {
                let nx = x as i32 + s;
                if nx >= 0 && (nx as usize) < cv.w {
                    cv.cells[y * cv.w + nx as usize] = c;
                    cv.owner[y * cv.w + nx as usize] = id;
                }
            }
        }
    }
}

// ---------------------------------------------------------------- the room

const WOOD: Rgb = (0x8C, 0x64, 0x3C);
const BEAM: Rgb = (0xB0, 0x98, 0x3C);
const LEAF: Rgb = (0x2E, 0x8B, 0x3E);
const POT: Rgb = (0xB0, 0x58, 0x38);
const CURTAIN: Rgb = (0x7A, 0x48, 0x78);

fn win_geo(w: i32, h: i32) -> (i32, i32, i32) {
    let ww = ((w as f32 * 0.2) as i32).clamp(8, 28);
    let wh = ((h as f32 * 0.22) as i32).clamp(4, 9);
    (3, ww, wh)
}

/// Sunbeam span (columns) at a row.
pub fn beam_at(w: i32, h: i32, y: f32) -> (f32, f32) {
    let (x0, ww, wh) = win_geo(w, h);
    let k = (y - (wh + 2) as f32).max(0.0) * 0.5;
    ((x0 + 1) as f32 + k, (x0 + ww - 2) as f32 + k)
}

pub fn draw_room(cv: &mut Canvas, app: &App) {
    let (w, h) = (cv.w as i32, cv.h as i32);
    let g = geo_cat(app.w, app.h, 1.0);
    let feet = g.top + g.mh + g.hh + g.al;
    let floor_y = (feet - 4).max(4);
    let t = app.t;
    // sunbeam
    let (x0, ww, wh) = win_geo(w, h);
    for y in (wh + 2)..(feet.min(h - 3)) {
        let (lo, hi) = beam_at(w, h, y as f32);
        for x in lo.round() as i32..=hi.round() as i32 {
            if (x + y) % 2 == 0 || y % 3 == 0 {
                cv.put(x, y, '\u{2591}', BEAM);
            }
        }
    }
    // window
    let wc = (0x70, 0x90, 0xB8);
    for x in x0..x0 + ww {
        cv.put(x, 0, '=', WOOD);
        cv.put(x, wh, '=', WOOD);
        cv.put(x, wh + 1, '=', WOOD);
    }
    for y in 0..=wh {
        cv.put(x0, y, '|', WOOD);
        cv.put(x0 + ww - 1, y, '|', WOOD);
        cv.put(x0 + ww / 2, y, '|', WOOD);
    }
    for x in x0 + 1..x0 + ww - 1 {
        if x == x0 + ww / 2 {
            continue;
        }
        cv.put(x, wh / 2, '-', WOOD);
        for y in 1..wh {
            if y != wh / 2 && (x + y) % 3 == 0 {
                cv.put(x, y, '.', wc);
            }
        }
    }
    cv.put(x0 + ww / 2, wh / 2, '+', WOOD);
    for x in x0 - 1..=x0 + ww {
        cv.put(x, wh + 2, '_', WOOD);
    }
    // floor
    for x in 0..w {
        cv.put(x, floor_y, '_', (0x5A, 0x40, 0x2C));
    }
    for y in floor_y + 1..(feet + 3).min(h - 3) {
        for x in 0..w {
            if (x + 5 * y).rem_euclid(12) < 3 {
                cv.put(x, y, '-', (0x3E, 0x2C, 0x20));
            }
        }
    }
    // rug
    let m = g.maxw;
    let ra = (w as f32 * 0.42).min(2.4 * m);
    let rc = (w / 2) as f32;
    let ry = (feet - 1) as f32;
    for y in (feet - 5)..(feet + 3).min(h - 2) {
        for x in (rc - ra - 1.0) as i32..=(rc + ra + 1.0) as i32 {
            let n = (((x as f32 - rc) / ra).powi(2) + ((y as f32 - ry) / 3.0).powi(2)).sqrt();
            if n <= 1.0 {
                let (ch, c) = if n > 0.86 {
                    ('\u{2593}', (0xA8, 0x58, 0x44))
                } else if (0.62..0.7).contains(&n) {
                    ('\u{2591}', (0xE0, 0xC8, 0xA0))
                } else {
                    ('\u{2592}', (0x7A, 0x2A, 0x38))
                };
                cv.put(x, y, ch, c);
            }
        }
    }
    // plant, left
    let pot_top = feet - 3;
    for r in 0..4 {
        let inset = (r > 1) as i32;
        for x in (4 + inset)..(12 - inset) {
            cv.put(x, pot_top + r, if r == 0 { '=' } else { '\u{2588}' }, POT);
        }
    }
    let fronds = [
        (-1.0f32, 0.9f32),
        (-0.5, 1.0),
        (0.0, 1.15),
        (0.5, 1.0),
        (1.0, 0.9),
    ];
    let len = ((feet as f32) * 0.38) as i32;
    for (i, (dir, k)) in fronds.iter().enumerate() {
        let l = (len as f32 * k) as i32;
        for s in 0..l {
            let sway = (t * 1.2 + i as f32).sin() * 0.04 * s as f32;
            let x = 8.0 + dir * s as f32 * 0.55 + sway;
            let y = pot_top - 1 - s;
            let ch = if *dir < -0.2 {
                '\\'
            } else if *dir > 0.2 {
                '/'
            } else {
                '|'
            };
            cv.put(x.round() as i32, y, ch, LEAF);
            if s > l / 3 && s % 3 == 0 {
                cv.put(x.round() as i32 + 1, y, 'o', (0x3C, 0xA8, 0x50));
                cv.put(x.round() as i32 - 1, y, 'o', (0x3C, 0xA8, 0x50));
            }
        }
    }
    // curtain, right
    let cw = ((w as f32 * 0.07) as i32).max(5);
    for x in (w - cw)..w {
        let col = x - (w - cw);
        let ch = ['\u{2593}', '\u{2592}', '\u{2588}'][(col % 3) as usize];
        let sway = ((t * 0.8 + col as f32).sin() * 0.7).round() as i32;
        let end = floor_y + 3 + sway;
        for y in 1..end.min(h - 3) {
            let c = if col % 3 == 2 {
                CURTAIN
            } else {
                lerp(CURTAIN, (0, 0, 0), 0.35)
            };
            cv.put(x, y, ch, c);
        }
    }
    for x in (w - cw - 1)..w {
        cv.put(x, 0, '=', WOOD);
    }
    let tie = (feet / 2).max(3);
    for x in (w - cw)..w {
        cv.put(x, tie, '=', (0xD0, 0xB0, 0x60));
    }
}
