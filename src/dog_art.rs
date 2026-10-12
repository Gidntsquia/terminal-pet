//! Dog and backyard drawing (step 3 of the wiki's Adding-a-pet.md: `src/<pet>_art.rs`).
//! The dog is a side view built from ellipses in a local frame: `u` runs forward along
//! the facing direction (columns), `v` runs up from the feet (rows).
use crate::app::{App, Kind};
use crate::art::{Body, Canvas, Cell, Geo, NONE, Rgb, lerp};
use crate::dog::{BREEDS, Breed, HOLE_FILL, HOLE_STAY, Pat, RollPose, Tail};

const GRASS: Rgb = (0x3E, 0x9B, 0x4A);
const GRASS_D: Rgb = (0x2A, 0x6E, 0x36);
const WOOD: Rgb = (0xB8, 0x8A, 0x5A);
const WOOD_D: Rgb = (0x84, 0x5E, 0x38);
const HOUSE: Rgb = (0xB0, 0x70, 0x40);
const ROOF: Rgb = (0x9A, 0x3C, 0x30);
const TRUNK: Rgb = (0x6B, 0x4A, 0x2A);
const LEAF: Rgb = (0x2E, 0x8B, 0x3E);
const LEAF2: Rgb = (0x44, 0xA8, 0x54);
const SUN: Rgb = (0xF8, 0xD0, 0x50);
const CLOUD: Rgb = (0xC8, 0xD0, 0xE0);
const DIRT: Rgb = (0x6A, 0x46, 0x2C);
const BALL: Rgb = (0xE8, 0x50, 0x30);
const BISCUIT: Rgb = (0xE0, 0xA8, 0x60);
const TONGUE: Rgb = (0xE8, 0x6A, 0x7A);
const INK: Rgb = (0x38, 0x28, 0x68);
const ZZZ: Rgb = (0xB0, 0xC0, 0xF0);

fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

// ------------------------------------------------------------ layout

/// (feet row, top row of the fence). The dog's feet touch row `feet - 1`.
pub fn ground(w: u16, h: u16) -> (i32, i32) {
    let _ = w;
    let h = h as i32;
    let fh = if h < 20 { 2 } else { 4 };
    let fence_top = h - 2 - fh;
    let feet = ((h as f32 * 0.70).round() as i32).min(fence_top - 2).max(5);
    (feet, fence_top)
}

/// Seated dog: (width in columns, height in rows). Height follows the pane rows like the
/// cat's `geo_cat`; width follows the height but never more than a third of the pane.
pub fn seat_dims(w: u16, h: u16, scale: f32) -> (f32, f32) {
    let (feet, _) = ground(w, h);
    let hh = ((h as f32 * 0.68 * scale).round() as i32).clamp(6, (feet - 2).max(6)) as f32;
    let ww = (hh * 1.2).min(w as f32 * 0.32).max(8.0);
    (ww, hh)
}

pub fn geo_dog(w: u16, h: u16, scale: f32) -> Geo {
    let (feet, _) = ground(w, h);
    let (_, total) = seat_dims(w, h, scale);
    // the standing side view is scaled so it is roughly as tall as the seated dog
    let stand = (1.5 * total).min(w as f32 * 0.6).max(8.0);
    let total = total as i32;
    let mh = (total as f32 * 0.35).round() as i32;
    let hh = (total as f32 * 0.30).round() as i32;
    let maxw = stand / 1.6;
    Geo {
        cx: w as i32 / 2,
        top: feet - total,
        mh,
        hh,
        al: (total - mh - hh).max(1),
        maxw,
        hwh: maxw * 0.7,
    }
}

/// Doghouse: (left column, width, wall height, roof height).
fn house(w: u16, h: u16) -> (i32, i32, i32, i32) {
    let (_, sh) = seat_dims(w, h, 1.0);
    let hw = ((w as f32 * 0.2).round() as i32).clamp(9, 36);
    let wall = ((sh * 0.3).round() as i32).clamp(4, 10);
    let roof = ((hw / 4).clamp(2, 6)).min((ground(w, h).0 - wall - 4).max(2));
    (2, hw, wall, roof)
}

/// Rightmost column of the doghouse (roof overhang included).
pub fn house_right(w: u16, h: u16) -> i32 {
    let (x0, hw, _, _) = house(w, h);
    x0 + hw
}

/// Column of the doghouse door.
pub fn door_x(w: u16, h: u16) -> f32 {
    let (x0, hw, _, _) = house(w, h);
    (x0 + hw / 2) as f32
}

// ------------------------------------------------------------ the yard

pub fn draw_yard(cv: &mut Canvas, app: &App) {
    let (w, h) = (cv.w as i32, cv.h as i32);
    let (feet, fence_top) = ground(app.w, app.h);
    let g = geo_dog(app.w, app.h, 1.0);
    let t = app.t;
    // sun and a drifting cloud
    let sx = (w as f32 * 0.55) as i32;
    cv.text(sx - 2, 1, " \\|/ ", SUN);
    cv.text(sx - 2, 2, "-(@)-", SUN);
    cv.text(sx - 2, 3, " /|\\ ", SUN);
    let cx = ((t * 1.1 + w as f32 * 0.2) % (w as f32 + 20.0)) as i32 - 10;
    cv.text(cx, 3, " .--~~--. ", CLOUD);
    cv.text(cx, 4, "(________)", CLOUD);
    // grass: a horizon line, then tufts down to the fence
    let horizon = (feet - 3).max(3);
    for x in 0..w {
        cv.put(x, horizon, '\u{2591}', GRASS_D);
    }
    for y in (horizon + 1)..fence_top {
        for x in 0..w {
            let k = hash(x, y);
            let ch = match k % 13 {
                0 => '"',
                1 => ',',
                2 => '\'',
                3 => '`',
                _ => continue,
            };
            let c = if k % 5 < 2 { GRASS } else { GRASS_D };
            cv.put(x, y, ch, c);
        }
    }
    // tree on the right
    let tw = if w >= 100 { 4 } else { 3 };
    let rv = (g.mh as f32 * 0.7).clamp(2.5, 6.0);
    let ru = (rv * 2.2).min((w as f32 * 0.1).max(3.0));
    let tx = w - ru as i32 - 2;
    let ccy = (feet as f32 - g.maxw * 0.5 - rv).max(rv + 5.0);
    for y in (ccy as i32)..(feet - 1) {
        for x in 0..tw {
            cv.put(tx - tw / 2 + x, y, '\u{2588}', TRUNK);
        }
    }
    cv.text(tx - tw / 2 - 1, feet - 2, "/", TRUNK);
    cv.text(tx - tw / 2 + tw, feet - 2, "\\", TRUNK);
    for y in ((ccy - rv).floor() as i32)..=((ccy + rv).ceil() as i32) {
        let dy = (y as f32 - ccy) / rv;
        if dy.abs() > 1.0 {
            continue;
        }
        let half = ru * (1.0 - dy * dy).sqrt();
        for x in ((tx as f32 - half).round() as i32)..=((tx as f32 + half).round() as i32) {
            let k = hash(x, y);
            let (ch, c) = match k % 4 {
                0 => ('@', LEAF2),
                1 => ('#', LEAF),
                2 => ('\u{2593}', LEAF),
                _ => ('\u{2588}', LEAF),
            };
            cv.put(x, y, ch, c);
        }
    }
    // doghouse on the left
    let (x0, hw, wall, roof) = house(app.w, app.h);
    let base = feet - 1;
    for y in (base - wall + 1)..=base {
        for x in x0..(x0 + hw) {
            let plank = (x - x0) % 4 == 3;
            cv.put(
                x,
                y,
                if plank { '|' } else { '\u{2588}' },
                if plank { WOOD_D } else { HOUSE },
            );
        }
    }
    let shrink = ((hw + 2) as f32 - 4.0) / roof as f32;
    for r in 0..roof {
        let wd = ((hw + 2) as f32 - shrink * r as f32).round() as i32;
        let xs = x0 - 1 + ((hw + 2) - wd) / 2;
        let y = base - wall - r;
        for x in xs..(xs + wd) {
            cv.put(x, y, '\u{2593}', ROOF);
        }
    }
    let dw = ((hw as f32 * 0.42).round() as i32).max(6);
    let dh = (wall - 1).max(3);
    let dx0 = x0 + (hw - dw) / 2;
    for i in 0..dh {
        let inset = if i >= dh - 2 {
            (i - (dh - 3)).max(0)
        } else {
            0
        };
        for x in (dx0 + inset)..(dx0 + dw - inset) {
            let (xx, yy) = (x as usize, (base - i) as usize);
            if xx < cv.w && yy < cv.h {
                cv.cells[yy * cv.w + xx] = Cell { ch: ' ', fg: None };
            }
        }
    }
    // fence along the bottom
    let fh = h - 2 - fence_top;
    for y in fence_top..(fence_top + fh) {
        for x in 0..w {
            let board = x % 4 < 2;
            let rail = y == fence_top + 1 || y == fence_top + fh - 1;
            if board {
                let ch = if y == fence_top { '^' } else { '\u{2588}' };
                cv.put(x, y, ch, if x % 4 == 0 { WOOD } else { WOOD_D });
            } else if rail {
                cv.put(x, y, '=', WOOD_D);
            }
        }
    }
}

// ------------------------------------------------------------ the dog

#[derive(Clone, Copy)]
struct Rig<'a> {
    br: &'a Breed,
    ox: f32,
    oy: f32,
    f: f32,
    sq: f32,
    m: f32,
    ll: f32,
    rv: f32,
    ru: f32,
    vb: f32,
    hu: f32,
    hv: f32,
    hru: f32,
    hrv: f32,
    flip: bool,
    shake: f32,
    t: f32,
}

impl<'a> Rig<'a> {
    fn new(body: &Body, w: u16, h: u16) -> Rig<'a> {
        let g = geo_dog(w, h, body.scale);
        let (feet, _) = ground(w, h);
        let br: &'a Breed = &BREEDS[body.breed.min(BREEDS.len() - 1)];
        let p = &body.dpose;
        let m = g.maxw * br.size * 1.6;
        // mid-roll the dog is on its side: body flattened, legs foreshortened
        let tum = 1.0;
        let ll = (0.15 * m * br.leg * (0.35 + 0.65 * tum)).max(1.0);
        let rv = 0.13 * m * (0.6 + 0.4 * tum);
        let vb = ll + rv;
        let hru = 0.22 * m;
        let hrv = 0.17 * m;
        let hd = p.head_down.max(0.5 * p.crouch);
        let hv0 = vb + rv * 0.75;
        let hv = hv0 - hd * (hv0 - 1.4 - hrv * 0.5) + p.look_up * 0.12 * m - p.tilt.abs() * 0.8;
        let hu = 0.5 * m + hd * 0.1 * m;
        let mut oy = (feet - 1) as f32 + body.dy;
        let flip = false;
        oy += p.crouch;
        Rig {
            br,
            ox: body.cx as f32 + body.dx,
            oy,
            f: if p.spin < 0.0 { -1.0 } else { 1.0 },
            sq: p.spin.abs().max(0.35),
            m,
            ll,
            rv,
            ru: 0.42 * m,
            vb,
            hu,
            hv,
            hru,
            hrv,
            flip,
            shake: p.shake,
            t: body.t,
        }
    }

    fn shear(&self, y: i32) -> i32 {
        if self.shake > 0.0 {
            (self.shake * 1.6 * (self.t * 38.0 + y as f32 * 0.9).sin()).round() as i32
        } else {
            0
        }
    }

    /// Local to screen.
    fn xy(&self, u: f32, v: f32) -> (i32, i32) {
        let y = (self.oy - v).round() as i32;
        let x = (self.ox + self.f * self.sq * u).round() as i32 + self.shear(y);
        (x, y)
    }

    /// Mirror v when rolled over so the legs point up.
    fn fv(&self, v: f32) -> f32 {
        if self.flip {
            2.0 * self.vb - v - self.ll
        } else {
            v
        }
    }

    fn fill(
        &self,
        cv: &mut Canvas,
        (u0, u1): (f32, f32),
        (v0, v1): (f32, f32),
        test: &dyn Fn(f32, f32, i32, i32) -> Option<(char, Rgb)>,
    ) {
        let (v0, v1) = if self.flip {
            (self.fv(v1), self.fv(v0))
        } else {
            (v0, v1)
        };
        let ya = (self.oy - v1).floor() as i32;
        let yb = (self.oy - v0).ceil() as i32;
        let xa = self.ox + self.f * self.sq * u0;
        let xb = self.ox + self.f * self.sq * u1;
        let (xa, xb) = (xa.min(xb).floor() as i32 - 3, xa.max(xb).ceil() as i32 + 3);
        for y in ya..=yb {
            let sh = self.shear(y);
            let v = self.fv(self.oy - y as f32);
            for x in xa..=xb {
                let u = ((x - sh) as f32 - self.ox) * self.f / self.sq;
                if u < u0 || u > u1 || v < v0.min(v1) - 0.01 && false {
                    continue;
                }
                if let Some((ch, c)) = test(u, v, x, y) {
                    cv.put(x, y, ch, c);
                }
            }
        }
    }

    fn ell(
        &self,
        cv: &mut Canvas,
        (cu, cvv): (f32, f32),
        (ru, rv): (f32, f32),
        paint: &dyn Fn(f32, f32, i32, i32) -> (char, Rgb),
    ) {
        let rv = rv.max(0.75);
        self.fill(
            cv,
            (cu - ru, cu + ru),
            (cvv - rv, cvv + rv),
            &|u, v, x, y| {
                let (a, b) = ((u - cu) / ru, (v - cvv) / rv);
                (a * a + b * b <= 1.0).then(|| paint(u, v, x, y))
            },
        );
    }

    fn stamp(&self, cv: &mut Canvas, u: f32, v: f32, ch: char, c: Rgb) {
        let v = self.fv(v);
        let (x, y) = self.xy(u, v);
        cv.put(x, y, ch, c);
    }

    fn mouth(&self) -> (f32, f32) {
        let tip = self.hu + self.hru * 0.85 + 0.11 * self.m;
        (tip, self.hv - 0.4)
    }
}

fn shade(c: Rgb, k: f32) -> Rgb {
    if k >= 0.0 {
        lerp(c, (255, 255, 255), k * 0.15)
    } else {
        lerp(c, (0, 0, 0), -k * 0.22)
    }
}

/// Fur at a body point: glyph and color for this breed's markings.
fn coat(r: &Rig, u: f32, v: f32, x: i32, y: i32) -> (char, Rgb) {
    let (ch, c) = coat_flat(r, u, v, x, y);
    // same fur speckle as the cat: every seventh cell a darker, lighter-weight glyph
    if ch == '\u{2588}' && (x + 3 * y).rem_euclid(7) == 0 && r.br.pat != Pat::Poodle {
        ('\u{2593}', shade(c, -0.6))
    } else {
        (ch, c)
    }
}

fn coat_flat(r: &Rig, u: f32, v: f32, x: i32, y: i32) -> (char, Rgb) {
    let b = r.br;
    let rel = ((v - r.vb) / r.rv).clamp(-1.0, 1.0);
    let uu = u / r.m;
    let base = shade(b.coat, rel * 0.3);
    match b.pat {
        Pat::Plain => ('\u{2588}', base),
        Pat::Husky => {
            if rel < -0.25 || uu > 0.34 {
                ('\u{2588}', shade(b.light, rel * 0.4))
            } else {
                ('\u{2593}', shade(b.coat, rel * 0.6))
            }
        }
        Pat::Dalmatian => {
            let k = hash((u / 3.0).floor() as i32, (v / 2.0).floor() as i32);
            if k.is_multiple_of(4) {
                ('\u{2593}', b.dark)
            } else {
                ('\u{2588}', base)
            }
        }
        Pat::Beagle => {
            if rel < -0.45 || uu > 0.33 {
                ('\u{2588}', shade(b.light, rel * 0.4))
            } else if rel > -0.2 && (-0.38..0.22).contains(&uu) {
                ('\u{2593}', b.dark)
            } else {
                ('\u{2588}', base)
            }
        }
        Pat::Shepherd => {
            if rel > -0.3 && uu < 0.28 {
                ('\u{2593}', shade(b.dark, rel * 0.4))
            } else {
                ('\u{2588}', base)
            }
        }
        Pat::Corgi => {
            if rel < -0.4 || uu > 0.36 {
                ('\u{2588}', shade(b.light, rel * 0.3))
            } else {
                ('\u{2588}', base)
            }
        }
        Pat::Poodle => (if (x + y) % 3 == 0 { '@' } else { '\u{2588}' }, base),
        Pat::Heeler => {
            if rel < -0.4 || uu > 0.36 {
                ('\u{2588}', shade(b.light, rel * 0.3))
            } else if hash((u / 2.0).floor() as i32, v.floor() as i32).is_multiple_of(3) {
                ('\u{2593}', b.dark)
            } else {
                ('\u{2588}', base)
            }
        }
        Pat::Dachshund => {
            if rel < -0.5 || uu > 0.38 {
                ('\u{2588}', shade(b.light, rel * 0.3))
            } else {
                ('\u{2588}', base)
            }
        }
    }
}

fn draw_tail(r: &Rig, cv: &mut Canvas, p: &crate::dog::DogPose) {
    let b = r.br;
    // tail lengths scaled to the body; the root sits just inside the rump
    let m = 0.8 * r.ru / 0.42;
    let (bu, bv) = (-0.42 * m, r.vb + r.rv * 0.5);
    let wag = (p.wag_ph).sin() * p.wag.clamp(0.0, 1.0) * 0.55;
    // happy on the move: the tail rides high
    let up = if p.walking || p.running { 0.45 } else { 0.0 };
    let col = b.coat;
    let stampc = |cv: &mut Canvas, u: f32, v: f32, ch: char, c: Rgb| {
        r.stamp(cv, u, v, ch, c);
    };
    let line = |cv: &mut Canvas, ang: f32, len: f32, thick: bool, ch: char, c: Rgb| {
        let n = (len * 1.2).max(2.0) as i32;
        for k in 0..=n {
            let d = k as f32 / n as f32 * len;
            let (u, v) = (bu - d * ang.cos(), bv + d * ang.sin() * 0.5);
            stampc(cv, u, v, ch, c);
            if thick {
                stampc(cv, u, v + 1.0, ch, c);
            }
        }
    };
    match b.tail {
        Tail::Straight => line(cv, 0.6 + up + wag, 0.30 * m, true, '\u{2588}', col),
        Tail::Plume => {
            line(cv, 0.7 + up + wag, 0.32 * m, true, '\u{2588}', col);
            line(
                cv,
                0.45 + up + wag,
                0.3 * m,
                false,
                '\u{2593}',
                shade(col, -0.4),
            );
        }
        Tail::Curl => {
            // an arc curling forward over the back
            for k in 0..=10 {
                let a = 2.9 - k as f32 * 0.38 + wag * 0.4;
                let (u, v) = (
                    -0.40 * m + 0.12 * m * a.cos(),
                    r.vb + r.rv + 0.1 * m * a.sin() * 0.7,
                );
                stampc(cv, u, v, '\u{2588}', if k > 7 { b.light } else { col });
            }
        }
        Tail::Stub => {
            let o = wag * 1.2;
            stampc(cv, -0.47 * m, bv + o * 0.5, '\u{2588}', col);
            stampc(cv, -0.5 * m, bv + 0.5 + o, '\u{2588}', col);
        }
        Tail::Pom => {
            line(cv, 0.9 + up + wag, 0.14 * m, false, '\u{2588}', col);
            let (cu, cv0) = (
                bu - 0.14 * m * (0.9 + up + wag).cos(),
                bv + 0.07 * m * (0.9 + up + wag).sin() + 1.0,
            );
            r.ell(cv, (cu, cv0), (0.06 * m, 0.04 * m), &|_, _, x, y| {
                (if (x + y) % 2 == 0 { '@' } else { '\u{2588}' }, b.light)
            });
        }
        Tail::Bushy => {
            line(
                cv,
                -0.5 + 2.0 * up + wag,
                0.34 * m,
                true,
                '\u{2593}',
                shade(col, -0.2),
            );
            line(cv, -0.5 + 2.0 * up + wag, 0.34 * m, false, '\u{2588}', col);
        }
    }
}

fn draw_head(r: &Rig, cv: &mut Canvas, body: &Body, neck: bool) {
    let b = r.br;
    let p = &body.dpose;
    let m = r.m;
    let (hu, hv) = (r.hu, r.hv);
    let mu = hu + r.hru * 0.85;
    let mv = hv - 0.25 * r.hrv;
    let mru = 0.12 * m;
    let mrv = (0.07 * m).max(1.0);
    // far ear (behind the head)
    let ear = |cv: &mut Canvas, far: bool| {
        let c = if far {
            shade(b.dark, -1.5)
        } else {
            shade(b.dark, -0.8)
        };
        if b.ears_up {
            let base = hu - r.hru * (if far { 0.15 } else { 0.55 });
            let h = (0.1 * m).max(2.0);
            for i in 0..(h as i32) {
                let wd = ((h - i as f32) / h * 2.0).ceil();
                for k in 0..(wd as i32) {
                    r.stamp(
                        cv,
                        base + k as f32 + p.tilt * i as f32 * 0.4,
                        hv + r.hrv + i as f32,
                        '\u{2593}',
                        c,
                    );
                }
            }
        } else if b.pat == Pat::Poodle {
            r.ell(
                cv,
                (hu - r.hru * 0.6, hv - 1.0),
                (0.07 * m, 0.1 * m),
                &|_, _, x, y| (if (x + y) % 2 == 0 { '@' } else { '\u{2588}' }, c),
            );
        } else {
            let sw = p.tilt * 0.4;
            let u = hu - r.hru * (if far { 0.1 } else { 0.5 }) + sw;
            r.ell(
                cv,
                (u, hv - 0.2 * m * 0.5),
                (0.06 * m, 0.13 * m),
                &|_, _, _, _| ('\u{2593}', c),
            );
        }
    };
    ear(cv, true);
    // neck
    for k in 0..=(if neck { 6 } else { -1 }) {
        let a = k as f32 / 6.0;
        let (nu, nv) = (
            0.3 * m + (hu - 0.3 * m) * a,
            r.vb + r.rv * 0.5 + (hv - r.vb - r.rv * 0.5) * a,
        );
        r.ell(
            cv,
            (nu, nv),
            (0.1 * m, (0.06 * m).max(1.2)),
            &|u, v, x, y| coat(r, u, v.min(r.vb + r.rv), x, y),
        );
    }
    // head
    let head_c = |v: f32| -> (char, Rgb) {
        let rel = ((v - hv) / r.hrv).clamp(-1.0, 1.0);
        match b.pat {
            Pat::Husky => {
                if rel < 0.1 {
                    ('\u{2588}', b.light)
                } else {
                    ('\u{2593}', b.coat)
                }
            }
            Pat::Poodle => ('@', b.light),
            _ => ('\u{2588}', shade(b.coat, rel * 0.5)),
        }
    };
    r.ell(cv, (hu, hv), (r.hru, r.hrv), &|_, v, x, y| {
        let (c, col) = head_c(v);
        if b.pat == Pat::Poodle && (x + y) % 2 == 1 {
            ('\u{2588}', col)
        } else {
            (c, col)
        }
    });
    if b.pat == Pat::Poodle {
        r.ell(
            cv,
            (hu - 0.02 * m, hv + r.hrv * 0.9),
            (r.hru * 0.7, 0.07 * m),
            &|_, _, x, y| (if (x + y) % 2 == 0 { '@' } else { '\u{2588}' }, b.light),
        );
    }
    // muzzle
    let muz = match b.pat {
        Pat::Shepherd => b.dark,
        _ => b.light,
    };
    r.ell(cv, (mu, mv), (mru, mrv), &|_, _, _, _| ('\u{2588}', muz));
    // mouth and tongue
    if p.open > 0.15 {
        let (tu, tv) = (mu + 0.2 * mru, mv - mrv * 0.9);
        r.stamp(cv, tu, tv, '\u{2584}', (0x70, 0x20, 0x30));
        r.stamp(cv, tu + 1.0, tv, '\u{2584}', (0x70, 0x20, 0x30));
        if p.tongue > 0.3 {
            r.stamp(cv, tu + 0.5, tv - 1.0, '~', TONGUE);
        }
    }
    // nose, a big shiny eye and a rosy cheek, like the seated face
    r.stamp(cv, mu + mru * 0.9, mv + mrv * 0.5, '\u{2588}', b.nose);
    r.stamp(cv, mu + mru * 0.9 - 1.0, mv + mrv * 0.5, '\u{2588}', b.nose);
    let (eu, ev) = (hu + 0.3 * r.hru, hv + 0.18 * r.hrv);
    let dark = (0x18, 0x12, 0x14);
    if body.closed || body.blink {
        r.stamp(cv, eu - 0.5, ev, '\u{2580}', dark);
        r.stamp(cv, eu + 0.5, ev, '\u{2580}', dark);
    } else {
        let iris = if b.eye.0 as u32 + b.eye.1 as u32 + b.eye.2 as u32 > 200 {
            b.eye
        } else {
            dark
        };
        r.ell(
            cv,
            (eu, ev),
            ((0.1 * r.hru).max(1.3), (0.17 * r.hrv).max(1.3)),
            &|_, _, _, _| ('\u{2588}', iris),
        );
        r.stamp(cv, eu - 0.5, ev + 0.9, '\u{25cf}', (255, 255, 255));
    }
    r.stamp(
        cv,
        hu - 0.1 * r.hru,
        hv - 0.4 * r.hrv,
        '\u{2592}',
        (0xF0, 0x90, 0xA0),
    );
    if b.pat == Pat::Dalmatian {
        r.stamp(cv, hu - 0.4 * r.hru, hv + 0.3, '\u{25cf}', b.dark);
    }
    ear(cv, false);
    if p.bark > 0.25 {
        let (tu, tv) = r.mouth();
        for k in 0..3 {
            r.stamp(
                cv,
                tu + 1.5 + 2.2 * k as f32,
                tv + 0.4 * k as f32,
                ')',
                (0xFF, 0xF0, 0xA0),
            );
        }
    }
}

// ------------------------------------------------------------ seated, facing the viewer

/// A rig for the seated front view: `u` is columns from the centre, `v` rows up from the
/// feet; everything is sized from the seated width and height.
fn sit_rig<'a>(body: &Body, w: u16, h: u16) -> Rig<'a> {
    let mut r = Rig::new(body, w, h);
    let (ww, hh) = seat_dims(w, h, body.scale);
    let p = &body.dpose;
    let (feet, _) = ground(w, h);
    r.m = ww;
    r.vb = 0.34 * hh;
    r.rv = 0.30 * hh;
    r.ru = 0.40 * ww;
    r.ll = 0.0;
    r.flip = false;
    r.sq = 1.0;
    r.f = 1.0;
    r.hru = (0.42 * ww).max(4.0);
    r.hrv = 0.18 * hh;
    r.hu = p.tilt * 0.04 * ww;
    r.hv = 0.745 * hh + p.look_up * 0.03 * hh - p.tilt.abs() * 0.5;
    r.ox = body.cx as f32 + body.dx;
    r.oy = (feet - 1) as f32 + body.dy;
    r
}

fn draw_sit_tail(r: &Rig, cv: &mut Canvas, p: &crate::dog::DogPose, side: f32, hh: f32) {
    let b = r.br;
    let ww = r.m;
    let wag = p.wag_ph.sin() * p.wag.clamp(0.0, 1.0) * 0.5;
    let (bu, bv) = (0.40 * ww, 0.06 * hh + 0.5);
    let (len, thick, ch, col) = match b.tail {
        Tail::Straight => (0.34 * ww, false, '\u{2588}', b.coat),
        Tail::Plume => (0.40 * ww, true, '\u{2593}', shade(b.coat, -0.2)),
        Tail::Curl => (0.26 * ww, true, '\u{2588}', b.coat),
        Tail::Stub => (0.12 * ww, true, '\u{2588}', b.coat),
        Tail::Pom => (0.22 * ww, false, '\u{2588}', b.coat),
        Tail::Bushy => (0.42 * ww, true, '\u{2593}', shade(b.coat, -0.2)),
    };
    let n = (len * 1.6).max(3.0) as i32;
    let mut end = (bu, bv);
    for k in 0..=n {
        let t = k as f32 / n as f32;
        // out along the grass, then curling up
        let ang = t * 1.5 + wag;
        let u = bu + len * ang.sin() * 0.9 * (0.4 + 0.6 * t);
        let v = bv + 0.22 * hh * (1.0 - ang.cos()) * if b.tail == Tail::Curl { 1.5 } else { 1.0 };
        r.stamp(cv, side * u, v, ch, col);
        if thick {
            r.stamp(cv, side * (u + 0.7), v, ch, col);
            r.stamp(cv, side * u, v + 1.0, ch, col);
        }
        end = (u, v);
    }
    if b.tail == Tail::Pom {
        r.ell(
            cv,
            (side * end.0, end.1 + 0.5),
            (1.6, 1.0),
            &|_, _, x, y| (if (x + y) % 2 == 0 { '@' } else { '\u{2588}' }, b.light),
        );
    }
    if matches!(b.tail, Tail::Curl) {
        r.stamp(cv, side * end.0, end.1 + 0.5, '\u{2588}', b.light);
    }
}

fn draw_sit(cv: &mut Canvas, body: &Body) {
    let (w, h) = (cv.w as u16, cv.h as u16);
    let r = sit_rig(body, w, h);
    let (ww, hh) = seat_dims(w, h, body.scale);
    let p = &body.dpose;
    let b = r.br;
    let side = if p.spin < 0.0 { -1.0 } else { 1.0 };
    draw_sit_tail(&r, cv, p, side, hh);
    // haunches (the hind thighs), then the body, chest bib and front legs
    let tu = 0.26 * ww;
    for s in [-1.0f32, 1.0] {
        r.ell(
            cv,
            (s * tu, 0.2 * hh),
            (0.2 * ww, 0.2 * hh),
            &|u, v, x, y| {
                let (c, col) = coat(&r, u, v, x, y);
                (c, shade(col, -0.5))
            },
        );
        // hind paws poke out to the sides
        r.ell(
            cv,
            (s * (tu + 0.12 * ww), 0.04 * hh + 0.5),
            (0.1 * ww + 0.5, 0.5),
            &|_, _, _, _| {
                (
                    '\u{2588}',
                    if b.pat == Pat::Plain { b.coat } else { b.light },
                )
            },
        );
    }
    r.ell(
        cv,
        (0.0, 0.44 * hh),
        (0.3 * ww, 0.24 * hh),
        &|u, v, x, y| coat(&r, u, v, x, y),
    );
    // chest bib
    r.ell(
        cv,
        (0.0, 0.46 * hh),
        (0.13 * ww, 0.17 * hh),
        &|_, v, _, _| {
            (
                '\u{2588}',
                shade(b.light, ((v - 0.38 * hh) / (0.16 * hh)) * 0.3),
            )
        },
    );
    // front legs and paws
    let ls = (0.14 * ww).max(2.3);
    let lw = (0.05 * ww).max(1.0);
    let paw = (0.06 * hh).max(1.0);
    for s in [-1.0f32, 1.0] {
        r.fill(
            cv,
            (s * ls - lw - 0.8, s * ls + lw + 0.8),
            (0.0, 0.42 * hh),
            &|u, v, x, y| {
                let du = (u - s * ls).abs();
                let is_paw = v < paw;
                if du > lw + if is_paw { 0.7 } else { 0.0 } {
                    return None;
                }
                if is_paw {
                    let c = if matches!(
                        b.pat,
                        Pat::Corgi | Pat::Beagle | Pat::Husky | Pat::Heeler | Pat::Dachshund
                    ) {
                        b.light
                    } else {
                        shade(b.coat, 0.6)
                    };
                    return Some(('\u{2588}', c));
                }
                let (c, col) = coat(&r, u, v.min(0.3 * hh), x, y);
                Some((c, shade(col, 0.4)))
            },
        );
    }
    // scratching: a hind foot lifted to the ear
    if p.scratch > 0.0 {
        let k = (p.fast_ph * 0.5).sin() * 0.5 + 0.5;
        for i in 0..7 {
            let a = i as f32 / 6.0 * p.scratch;
            let (u, v) = (
                tu + (r.hu + r.hru * 0.9 - tu + 0.8 * k) * a,
                0.3 * hh + (r.hv - 0.3 * hh) * a,
            );
            r.stamp(cv, u, v, '\u{2588}', b.coat);
        }
    }
    draw_sit_head(&r, cv, body, hh);
    // ink on the coat
    if let Some((left, ox)) = body.splat {
        let k = if left > 1.4 {
            1.0
        } else {
            (left / 1.4).clamp(0.0, 1.0)
        };
        if k > 0.0 {
            r.ell(
                cv,
                (ox.clamp(-0.2 * ww, 0.2 * ww), 0.38 * hh),
                (0.14 * ww * k, (0.08 * hh * k).max(0.6)),
                &|_, _, _, _| ('\u{2593}', INK),
            );
        }
    }
    if p.carry == 1 {
        r.stamp(cv, r.hu, r.hv - r.hrv * 0.9, '\u{25cf}', BISCUIT);
    }
}

fn draw_sit_head(r: &Rig, cv: &mut Canvas, body: &Body, hh: f32) {
    let b = r.br;
    let p = &body.dpose;
    let (hu, hv, hru, hrv) = (r.hu, r.hv, r.hru, r.hrv);
    let face = if p.spin < 0.0 { -1.0 } else { 1.0 };
    // ears behind the head: pointed and up, or floppy at the sides
    let eard = 0.0;
    for s in [-1.0f32, 1.0] {
        let tilt = if s == face { p.tilt * 0.8 } else { 0.0 };
        let c = shade(b.dark, -0.8);
        if b.ears_up {
            let eh = (0.17 * hh).max(2.0);
            let eu = hu + s * hru * 0.6;
            let n = eh.ceil() as i32;
            for i in 0..n {
                let wd = ((n - i) as f32 / n as f32 * (hru * 0.45).max(1.5)).max(1.0);
                r.fill(
                    cv,
                    (eu - wd, eu + wd),
                    (
                        hv + hrv * 0.55 + i as f32 - 0.5 + eard,
                        hv + hrv * 0.55 + i as f32 + 0.5,
                    ),
                    &|u, _, _, _| ((u - eu).abs() <= wd * 0.5 + 0.4).then_some(('\u{2593}', c)),
                );
            }
        } else if b.pat == Pat::Poodle {
            r.ell(
                cv,
                (hu + s * hru * 1.05, hv - hrv * 0.2),
                (hru * 0.32, hrv * 0.8),
                &|_, _, x, y| (if (x + y) % 2 == 0 { '@' } else { '\u{2588}' }, c),
            );
        } else {
            r.ell(
                cv,
                (
                    hu + s * hru * 1.0,
                    hv - hrv * (if b.pat == Pat::Dachshund { 0.9 } else { 0.35 } + tilt * 0.2),
                ),
                if b.pat == Pat::Dachshund {
                    (hru * 0.34, hrv * 1.6)
                } else {
                    (hru * 0.3, hrv * 0.95)
                },
                &|_, _, _, _| ('\u{2593}', c),
            );
        }
    }
    // head
    r.ell(cv, (hu, hv), (hru, hrv), &|_, v, x, y| {
        let rel = ((v - hv) / hrv).clamp(-1.0, 1.0);
        match b.pat {
            Pat::Husky => {
                if rel < 0.2 {
                    ('\u{2588}', b.light)
                } else {
                    ('\u{2593}', b.coat)
                }
            }
            Pat::Poodle => (if (x + y) % 2 == 0 { '@' } else { '\u{2588}' }, b.light),
            _ => ('\u{2588}', shade(b.coat, rel * 0.5)),
        }
    });
    if b.pat == Pat::Poodle {
        r.ell(
            cv,
            (hu, hv + hrv * 0.9),
            (hru * 0.6, hrv * 0.5),
            &|_, _, x, y| (if (x + y) % 2 == 0 { '@' } else { '\u{2588}' }, b.light),
        );
    }
    if b.pat == Pat::Dalmatian {
        r.stamp(cv, hu - hru * 0.6, hv + hrv * 0.4, '\u{25cf}', b.dark);
    }
    // muzzle, nose, mouth
    let muz = if b.pat == Pat::Shepherd {
        b.dark
    } else {
        b.light
    };
    let (mv, mru, mrv) = (hv - hrv * 0.42, hru * 0.38, (hrv * 0.36).max(1.0));
    r.ell(cv, (hu, mv), (mru, mrv), &|_, _, _, _| ('\u{2588}', muz));
    r.stamp(cv, hu - 0.5, mv + mrv * 0.55, '\u{2588}', b.nose);
    r.stamp(cv, hu + 0.5, mv + mrv * 0.55, '\u{2588}', b.nose);
    if p.open > 0.15 || p.tongue > 0.3 {
        let m = (0x70, 0x20, 0x30);
        r.stamp(cv, hu - 0.5, mv - mrv + 0.2, '\u{2584}', m);
        r.stamp(cv, hu + 0.5, mv - mrv + 0.2, '\u{2584}', m);
        if p.tongue > 0.3 {
            r.stamp(cv, hu - 0.5, mv - mrv - 0.8, '\u{2588}', TONGUE);
            r.stamp(cv, hu + 0.5, mv - mrv - 0.8, '\u{2588}', TONGUE);
        }
    }
    // big wide-set eyes with a shine, rosy cheeks; eyes follow the look direction
    let ex = (0.5 * hru).max(2.0);
    let ev = hv + 0.12 * hrv;
    let shift = 0.5 * face;
    let dark = (0x18, 0x12, 0x14);
    let iris = if b.eye.0 as u32 + b.eye.1 as u32 + b.eye.2 as u32 > 200 {
        b.eye
    } else {
        dark
    };
    for s in [-1.0f32, 1.0] {
        let eu = hu + s * ex + shift;
        if body.closed || body.blink {
            r.stamp(cv, eu - 0.5, ev, '\u{2580}', dark);
            r.stamp(cv, eu + 0.5, ev, '\u{2580}', dark);
        } else {
            let (eru, erv) = ((0.11 * hru).max(1.0), (0.13 * hrv).max(1.0));
            r.ell(cv, (eu, ev), (eru, erv), &|_, _, _, _| ('\u{2588}', iris));
            r.stamp(cv, eu - 0.4, ev + erv * 0.6, '\u{25cf}', (255, 255, 255));
        }
        r.stamp(
            cv,
            hu + s * (ex + 0.28 * hru),
            hv - 0.3 * hrv,
            '\u{2592}',
            (0xF0, 0x90, 0xA0),
        );
    }
    if p.bark > 0.25 {
        for k in 0..3 {
            r.stamp(
                cv,
                hu + face * (hru + 1.5 + 2.2 * k as f32),
                mv + 0.4 * k as f32,
                ')',
                (0xFF, 0xF0, 0xA0),
            );
        }
    }
}

// ------------------------------------------------------------ rolling over

/// The seated dog's head and body sizes, shared by every roll pose so the head never shrinks.
struct LieDims {
    ww: f32,
    hh: f32,
    hru: f32,
    hrv: f32,
    /// Lying body: centre column (local), half-length, half-height.
    bu: f32,
    ru: f32,
    rv: f32,
}

fn lie_dims(w: u16, h: u16, scale: f32) -> LieDims {
    let (ww, hh) = seat_dims(w, h, scale);
    let hru = (0.42 * ww).max(4.0);
    let hrv = 0.18 * hh;
    LieDims {
        ww,
        hh,
        hru,
        hrv,
        bu: -0.3 * ww,
        ru: 0.6 * ww,
        rv: (0.19 * hh).max(2.0),
    }
}

fn paw_col(b: &Breed) -> Rgb {
    if matches!(
        b.pat,
        Pat::Corgi | Pat::Beagle | Pat::Husky | Pat::Heeler | Pat::Dachshund
    ) {
        b.light
    } else {
        shade(b.coat, 0.6)
    }
}

/// A rig for one roll pose: local `u` columns toward the head, `v` rows up from the grass.
/// `coat()` reads the body centre from `vb`/`rv` and the pattern scale from `m`.
fn lie_rig<'a>(body: &Body, w: u16, h: u16, d: &LieDims, bv: f32, rv: f32) -> Rig<'a> {
    let mut r = sit_rig(body, w, h);
    r.f = if body.dpose.spin < 0.0 { -1.0 } else { 1.0 };
    r.sq = 1.0;
    r.vb = bv;
    r.rv = rv;
    r.m = d.ru / 0.42;
    r.hru = d.hru;
    r.hrv = d.hrv;
    r
}

/// A thick limb from `a` to `b` (local coordinates), `hw` columns either side, paw at the end.
fn limb(r: &Rig, cv: &mut Canvas, a: (f32, f32), b: (f32, f32), hw: f32, fur: Rgb, paw: Rgb) {
    let len = ((b.0 - a.0).powi(2) + (2.0 * (b.1 - a.1)).powi(2)).sqrt();
    let n = (len * 1.5).max(2.0) as i32;
    for k in 0..=n {
        let t = k as f32 / n as f32;
        let (u, v) = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        let c = if t > 0.8 { paw } else { fur };
        let mut du = -hw;
        while du <= hw + 0.01 {
            r.stamp(cv, u + du, v, '\u{2588}', c);
            du += 0.5;
        }
    }
}

/// The seated dog's face, drawn at (screen column, local v) with the seated head size.
/// `flip` draws it upside down (on its back), mirrored around the head centre row.
fn lie_head(
    cv: &mut Canvas,
    r: &Rig,
    body: &Body,
    d: &LieDims,
    at: (f32, f32),
    tilt: f32,
    flip: bool,
) {
    let mut hb = body.clone();
    hb.dpose.tilt = tilt;
    hb.dpose.bark = 0.0;
    hb.dpose.spin = r.f;
    if flip {
        hb.dpose.tongue = 0.0;
        hb.dpose.open = 0.0;
    }
    let mut hr = Rig {
        f: 1.0,
        sq: 1.0,
        shake: 0.0,
        ..*r
    };
    let (sx, _) = r.xy(at.0, 0.0);
    hr.ox = sx as f32;
    hr.hu = 0.0;
    hr.hv = at.1;
    hr.hru = d.hru;
    hr.hrv = d.hrv;
    if !flip {
        draw_sit_head(&hr, cv, &hb, d.hh);
        return;
    }
    let mut tmp = Canvas::new(cv.w as u16, cv.h as u16, cv.ascii);
    draw_sit_head(&hr, &mut tmp, &hb, d.hh);
    let cy = (hr.oy - at.1).round() as i32;
    for y in 0..tmp.h as i32 {
        for x in 0..tmp.w as i32 {
            let c = tmp.cells[(y * tmp.w as i32 + x) as usize];
            if let Some(col) = c.fg {
                let ch = match c.ch {
                    '\u{2580}' => '\u{2584}',
                    '\u{2584}' => '\u{2580}',
                    k => k,
                };
                // ears that would hang below the crown fold flat along it instead
                let ground = (hr.oy - (at.1 - d.hrv)).round() as i32;
                cv.put(x, (2 * cy - y).min(ground), ch, col);
            }
        }
    }
    // upside down the tongue flops out of the side of the mouth
    let mv = at.1 + d.hrv * 0.42;
    for k in 0..3 {
        hr.stamp(
            cv,
            r.f * (d.hru * 0.3 + k as f32),
            mv + 0.6 + (k as f32 * 0.5).floor(),
            '\u{2588}',
            TONGUE,
        );
    }
}

/// The tail lying on the grass behind, wagging.
fn lie_tail(r: &Rig, cv: &mut Canvas, p: &crate::dog::DogPose, root: (f32, f32), ww: f32) {
    let b = r.br;
    let wag = p.wag_ph.sin() * p.wag.clamp(0.0, 1.0);
    let len = match b.tail {
        Tail::Stub => 0.1 * ww,
        Tail::Pom | Tail::Curl => 0.2 * ww,
        _ => 0.32 * ww,
    }
    .max(2.0);
    let thick = matches!(b.tail, Tail::Plume | Tail::Bushy | Tail::Curl);
    let n = (len * 1.4) as i32;
    let mut end = root;
    for k in 0..=n {
        let t = k as f32 / n as f32;
        let u = root.0 - len * t;
        let v = root.1 + (0.12 * ww * t * t * (0.6 + 0.4 * wag)).max(0.0) * 0.5;
        r.stamp(cv, u, v, '\u{2588}', b.coat);
        if thick {
            r.stamp(cv, u, v + 1.0, '\u{2593}', shade(b.coat, -0.2));
        }
        end = (u, v);
    }
    if b.tail == Tail::Pom {
        r.ell(cv, (end.0, end.1 + 0.5), (1.6, 1.0), &|_, _, x, y| {
            (if (x + y) % 2 == 0 { '@' } else { '\u{2588}' }, b.light)
        });
    }
}

/// A jointed limb through `pts` (local coordinates), `hw` columns either side, a paw at the end.
fn joint_limb(r: &Rig, cv: &mut Canvas, pts: &[(f32, f32)], hw: f32, fur: Rgb, paw: Rgb) {
    for (i, s) in pts.windows(2).enumerate() {
        let last = i + 2 == pts.len();
        limb(r, cv, s[0], s[1], hw, fur, if last { paw } else { fur });
    }
    if let Some(&(u, v)) = pts.last() {
        for k in 0..3 {
            r.stamp(cv, u - 1.0 + k as f32, v, '\u{2588}', paw);
        }
    }
}

/// Body lying on its side or back: a level ellipse (the spine stays horizontal). `belly` is
/// how much of it shows the light belly: 0 none (back toward the viewer), 1 the lower half
/// (belly toward the viewer), 2 the upper half (belly up).
fn lie_torso(r: &Rig, cv: &mut Canvas, (bu, bv): (f32, f32), (ru, rv): (f32, f32), belly: f32) {
    let b = r.br;
    r.ell(cv, (bu, bv), (ru, rv), &|u, v, x, y| {
        let rel = (v - bv) / rv;
        let along = ((u - bu) / ru).abs() < 0.82;
        let light = match belly {
            x if x <= 0.0 => false,
            x if x <= 1.0 => rel < 0.4 - 1.0 * x,
            x => rel > 1.6 - x,
        };
        if light && along {
            let spot = b.pat == Pat::Dalmatian && hash(x / 3, y).is_multiple_of(7);
            if spot {
                ('\u{2593}', b.dark)
            } else {
                ('\u{2588}', shade(b.light, 0.2))
            }
        } else {
            coat(r, u - bu, v, x, y)
        }
    });
}

/// Roll-over for `p`, as a real dog does it: down on its belly (sphinx), flat on one side
/// with the legs out, half over, on its back with the legs bent and tucked, half over, flat
/// on the other side, up on its feet, and sitting. Every frame uses the seated dog's head at
/// the seated size and palette; the spine stays level while lying.
fn draw_roll(cv: &mut Canvas, body: &Body, pose: RollPose, k: f32) {
    let (w, h) = (cv.w as u16, cv.h as u16);
    let body = &roll_body(body, w, h);
    if pose == RollPose::Sit {
        draw_sit(cv, body);
        return;
    }
    let d = lie_dims(w, h, body.scale);
    let p = &body.dpose;
    let ww = d.ww;
    let wig = |ph: f32| (body.t * 9.0 + ph).sin();
    let (bu, ru, rv) = (d.bu, d.ru, d.rv);
    let lw = 1.0;
    match pose {
        RollPose::Down => {
            // sphinx: belly on the grass, front legs forward, hind legs tucked, head up
            let bv = rv + 0.2;
            let r = lie_rig(body, w, h, &d, bv, rv);
            let b = r.br;
            let paw = paw_col(b);
            lie_tail(&r, cv, p, (bu - ru + 1.0, 0.8), ww);
            // far front leg, a column behind
            let reach = bu + ru + d.hru + 1.0;
            for v in [1.6, 0.6] {
                joint_limb(
                    &r,
                    cv,
                    &[(bu + 0.55 * ru, v + 0.4), (reach - 2.0, v)],
                    lw,
                    shade(b.coat, -0.9),
                    shade(paw, -0.6),
                );
            }
            lie_torso(&r, cv, (bu, bv), (ru, rv), 0.0);
            // the hind leg folded under the haunch: thigh, hock flat, paw forward
            r.ell(
                cv,
                (bu - 0.5 * ru, bv + 0.3),
                (0.36 * ru, rv * 0.95),
                &|u, v, x, y| {
                    let (c, col) = coat(&r, u - bu, v, x, y);
                    (c, shade(col, -0.35))
                },
            );
            joint_limb(
                &r,
                cv,
                &[(bu - 0.75 * ru, 0.5), (bu - 0.25 * ru, 0.5)],
                lw,
                shade(b.coat, 0.15),
                paw,
            );
            // near front leg forward along the grass
            for v in [1.0, 0.0] {
                joint_limb(
                    &r,
                    cv,
                    &[(bu + 0.6 * ru, v + 0.4), (reach, v)],
                    lw,
                    shade(b.coat, 0.15),
                    paw,
                );
            }
            let hv = 1.2 * rv + d.hrv * (1.0 + 0.15 * k);
            lie_head(cv, &r, body, &d, (bu + ru + 0.15 * d.hru, hv), 0.0, false);
        }
        RollPose::SideA | RollPose::SideB => {
            let a = pose == RollPose::SideA;
            let bv = rv + if a { 1.2 } else { 0.2 };
            let r = lie_rig(body, w, h, &d, bv, rv);
            let b = r.br;
            let paw = paw_col(b);
            lie_tail(&r, cv, p, (bu - ru + 1.0, 0.8), ww);
            let fur = shade(b.coat, 0.15);
            if !a {
                // back toward the viewer: the legs lie on the far side, paws peek over
                for (i, u0) in [0.62f32, 0.38, -0.45, -0.7].iter().enumerate() {
                    let up = 2.0 * rv + bv - rv + 0.4 + 0.3 * wig(i as f32);
                    r.ell(cv, (bu + u0 * ru, up), (1.6, 0.8), &|_, _, _, _| {
                        ('\u{2588}', shade(paw, -0.4))
                    });
                }
            }
            lie_torso(&r, cv, (bu, bv), (ru, rv), if a { 0.55 } else { 0.0 });
            if a {
                // belly toward the viewer: all four legs out on the grass in front of it,
                // loosely bent, front legs toward the head, hind legs toward the tail
                let s = |i: f32| 0.4 * wig(i * 1.7);
                let fl = |u0: f32, dir: f32, i: f32| {
                    [
                        (u0, bv - 0.3 * rv),
                        (u0 + dir * (0.1 * ww) + s(i), 0.0),
                        (u0 + dir * (0.2 * ww) + s(i), -2.2),
                    ]
                };
                joint_limb(&r, cv, &fl(bu + 0.25 * ru, 1.0, 0.0), lw, fur, paw);
                joint_limb(&r, cv, &fl(bu + 0.02 * ru, 1.0, 1.0), lw, fur, paw);
                // hind legs: thigh forward to the knee, then back to the paw
                for (i, u0) in [-0.4f32, -0.65].iter().enumerate() {
                    let u0 = bu + u0 * ru;
                    joint_limb(
                        &r,
                        cv,
                        &[
                            (u0, bv - 0.2 * rv),
                            (u0 + 0.08 * ww, 0.2),
                            (u0 - 0.1 * ww + s(i as f32 + 2.0), -2.2),
                        ],
                        lw,
                        fur,
                        paw,
                    );
                }
            }
            // head lying on the grass at the front, turned on its side
            let hu = bu + ru + 0.45 * d.hru;
            lie_head(
                cv,
                &r,
                body,
                &d,
                (hu, d.hrv + 0.6 + if a { 0.6 } else { 0.0 }),
                if a { 0.8 } else { -0.8 },
                false,
            );
        }
        RollPose::TwistA | RollPose::TwistB => {
            // half way over: belly turning up, legs bent and tucked against it
            let a = pose == RollPose::TwistA;
            let bv = rv + 0.4;
            let r = lie_rig(body, w, h, &d, bv, rv);
            let b = r.br;
            let paw = paw_col(b);
            lie_tail(&r, cv, p, (bu - ru + 1.0, 0.8), ww);
            let top = bv + rv;
            let fur = shade(b.coat, if a { 0.15 } else { -0.6 });
            let pc = if a { paw } else { shade(paw, -0.4) };
            let legs = |cv: &mut Canvas| {
                // front legs folded toward the chest, hind legs bent at the hock
                for (i, u0) in [0.55f32, 0.35].iter().enumerate() {
                    let u0 = bu + u0 * ru;
                    let o = i as f32;
                    joint_limb(
                        &r,
                        cv,
                        &[
                            (u0, top - 2.5),
                            (u0 + 2.5 + o, top + 0.8),
                            (u0 + 0.5 + o, top + 1.4),
                        ],
                        lw,
                        fur,
                        pc,
                    );
                }
                for (i, u0) in [-0.4f32, -0.62].iter().enumerate() {
                    let u0 = bu + u0 * ru;
                    let o = i as f32;
                    joint_limb(
                        &r,
                        cv,
                        &[
                            (u0, top - 2.5),
                            (u0 + 2.0, top + 1.2),
                            (u0 - 2.5 - o, top + 1.6),
                        ],
                        lw + 0.5 * (1 - i) as f32,
                        fur,
                        pc,
                    );
                }
            };
            if !a {
                legs(cv);
            }
            lie_torso(&r, cv, (bu, bv), (ru, rv), if a { 1.55 } else { 1.2 });
            if a {
                legs(cv);
            }
            // head rolling with the body
            let hu = bu + ru + 0.5 * d.hru;
            lie_head(
                cv,
                &r,
                body,
                &d,
                (hu, d.hrv + 0.8),
                if a { 1.3 } else { -1.3 },
                false,
            );
        }
        RollPose::Back => {
            let rv = rv * 0.95;
            let bv = rv + 0.2;
            let r = lie_rig(body, w, h, &d, bv, rv);
            let b = r.br;
            let paw = paw_col(b);
            // tail out to one side on the grass
            lie_tail(&r, cv, p, (bu - ru + 1.0, 0.8), ww);
            let top = bv + rv;
            let far = shade(b.coat, -0.6);
            let fur = shade(b.coat, 0.15);
            let s = |i: f32| 0.5 * wig(i * 1.9);
            // far legs first, a column behind
            let front = |u0: f32, i: f32| {
                // upper arm up from the shoulder, forearm folded back down onto the chest
                [
                    (u0, top - 1.5),
                    (u0 + 2.0, top + 2.3 + 0.3 * s(i)),
                    (u0 - 1.0 + s(i), top + 0.8),
                ]
            };
            let hind = |u0: f32, out: f32, i: f32| {
                // thigh up and forward to the knee, bent at the hock, paw splayed back
                [
                    (u0, top - 1.5),
                    (u0 + 2.5, top + 2.0),
                    (u0 - 2.5 - out + s(i), top + 2.4 + 0.3 * s(i + 1.0)),
                ]
            };
            joint_limb(
                &r,
                cv,
                &front(bu + 0.68 * ru, 0.0),
                lw,
                far,
                shade(paw, -0.4),
            );
            joint_limb(
                &r,
                cv,
                &hind(bu - 0.35 * ru, -1.0, 1.0),
                lw,
                far,
                shade(paw, -0.4),
            );
            lie_torso(&r, cv, (bu, bv), (ru, rv), 1.9);
            joint_limb(&r, cv, &front(bu + 0.52 * ru, 2.0), lw, fur, paw);
            joint_limb(&r, cv, &hind(bu - 0.55 * ru, 1.5, 3.0), lw + 0.5, fur, paw);
            // head tilted back on the grass, upside down, tongue out
            let ear = if b.ears_up { 0.17 * d.hh } else { 0.0 };
            let hv = (0.55 * d.hrv + ear + 0.5).max(d.hrv * 0.9 + 0.5);
            lie_head(cv, &r, body, &d, (bu + ru + 0.55 * d.hru, hv), 0.0, true);
        }
        RollPose::Stand => draw_stand(cv, body, true),
        RollPose::Sit => draw_sit(cv, body),
    }
}

/// The body moved sideways, if needed, so the lying dog stays clear of the doghouse and
/// inside the pane.
fn roll_body(body: &Body, w: u16, h: u16) -> Body {
    let full = roll_body_full(body, w, h);
    // shuffle over while lying down, and back while sitting up, so nothing jumps
    let s = match body.dpose.roll_pose() {
        Some((RollPose::Down, k)) => ease_k(k / 0.6),
        Some((RollPose::Sit, k)) => 1.0 - ease_k(k / 0.8),
        _ => 1.0,
    };
    Body {
        dx: body.dx + (full.dx - body.dx) * s,
        ..body.clone()
    }
}

fn ease_k(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn roll_body_full(body: &Body, w: u16, h: u16) -> Body {
    let face = if body.dpose.spin < 0.0 { -1.0 } else { 1.0 };
    let (l, r) = roll_extent(w, h, body.scale, face);
    let (x0, hw, _, _) = house(w, h);
    let c = body.cx as f32 + body.dx;
    let mut dx = body.dx;
    let min_c = (x0 + hw + 1) as f32 - l;
    let max_c = (w as f32 - 2.0) - r;
    if c < min_c {
        dx += min_c - c;
    } else if c > max_c && max_c >= min_c {
        dx -= c - max_c;
    }
    Body { dx, ..body.clone() }
}

/// Screen columns the roll pose covers (left, right) relative to the dog's centre column,
/// so the lying dog can be kept clear of the doghouse.
fn roll_extent(w: u16, h: u16, scale: f32, face: f32) -> (f32, f32) {
    let d = lie_dims(w, h, scale);
    let back = d.bu - d.ru - 0.32 * d.ww - 1.0;
    let front = (d.bu + d.ru + 0.55 * d.hru + d.hru * 1.35).max(0.3 * d.ww + 0.42 * d.ww + 2.0);
    if face > 0.0 {
        (back, front)
    } else {
        (-front, -back)
    }
}

// ------------------------------------------------------------ standing, side view

/// Leg length as a share of the withers height (the top of the back): short for the dachshund
/// and corgi, long for the shepherd, dalmatian and poodle.
pub fn leg_frac(b: &Breed) -> f32 {
    match b.pat {
        Pat::Corgi | Pat::Dachshund => 0.3,
        Pat::Shepherd | Pat::Dalmatian | Pat::Poodle => 0.52,
        _ => 0.41,
    }
}

/// The standing dog is the seated dog on its feet: the same head at the same size and
/// height. Local `u` runs forward (columns), `v` up from the grass (rows). The body is about
/// 1.3 head-widths long; `ll` is the leg length below the chest, `vb`/`rv` the body's middle
/// and half-depth at the chest, so the withers sit at `vb + rv`.
fn stand_rig<'a>(body: &Body, w: u16, h: u16) -> Rig<'a> {
    let mut r = Rig::new(body, w, h);
    let (ww, hh) = seat_dims(w, h, body.scale);
    let p = &body.dpose;
    let hru = (0.42 * ww).max(4.0);
    let hrv = 0.18 * hh;
    // chibi like the seated dog: a big round head on a small, round body with stubby legs
    let withers = 0.54 * hh;
    let ll = (leg_frac(r.br) * withers).max(2.0);
    let ru = hru
        * if r.br.pat == Pat::Dachshund {
            1.35
        } else {
            1.0
        };
    r.m = ru / 0.42;
    r.ru = ru;
    r.ll = ll;
    r.rv = (withers - ll) / 2.0;
    r.vb = ll + r.rv;
    r.hru = hru;
    r.hrv = hrv;
    let hv0 = 0.745 * hh;
    let hd = p.head_down.max(0.5 * p.crouch);
    let low = hrv * 0.95 + 0.5;
    r.hv = hv0 + p.look_up * 0.03 * hh - hd * (hv0 - low);
    r.hu = 0.62 * ru + hd * 0.25 * hru;
    // a bouncy, happy step: the body and head rise twice per stride, paws stay down
    if p.walking || p.running {
        let amp = if p.running { 1.0 } else { 0.7 };
        let bob = amp * (0.5 - 0.5 * (2.0 * p.step).cos());
        r.ll += bob;
        r.vb += bob;
        r.hv += bob;
    }
    r.flip = false;
    r
}

/// Facing the way it goes (profile) while moving or spinning; facing the viewer otherwise.
fn profile(p: &crate::dog::DogPose) -> bool {
    p.walking || p.running || p.spin.abs() < 0.999
}

/// Half-width of the profile skull (columns).
fn skull_ru(r: &Rig) -> f32 {
    0.88 * r.hru
}

/// How far the profile muzzle reaches past the skull (columns, at least 2).
fn muzzle_len(r: &Rig) -> f32 {
    (0.14 * r.hru).max(2.0)
}

/// Rows (top, bottom) of the profile muzzle in local `v`.
fn muzzle_rows(r: &Rig) -> (f32, f32) {
    let top = r.hv - 0.18 * r.hrv;
    let bot = (r.hv - 0.78 * r.hrv).min(top - 2.0);
    (top, bot)
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A straight run of cells from `a` to `b` (local), `hw` columns either side of the line.
fn seg(r: &Rig, cv: &mut Canvas, a: (f32, f32), b: (f32, f32), hw: f32, c: &dyn Fn(f32) -> Rgb) {
    let len = ((b.0 - a.0).powi(2) + (2.0 * (b.1 - a.1)).powi(2)).sqrt();
    let n = (len * 1.5).max(2.0) as i32;
    for k in 0..=n {
        let t = k as f32 / n as f32;
        let (u, v) = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        let mut du = -hw;
        while du <= hw + 0.01 {
            r.stamp(cv, u + du, v, '\u{2588}', c(v));
            du += 0.5;
        }
    }
}

/// Socks: breeds with white feet get light lower legs.
fn socks(b: &Breed) -> bool {
    matches!(
        b.pat,
        Pat::Corgi | Pat::Beagle | Pat::Husky | Pat::Heeler | Pat::Dachshund
    )
}

/// One leg in profile, jointed: the front leg runs from the shoulder past the elbow, nearly
/// straight down with a slight bend at the wrist; the hind leg from the hip forward to the
/// knee, back to the hock, then down. Both end in a paw pointing forward. `ph` is the walk
/// phase; walking swings the leg from its top joint and lifts the paw on the way forward.
fn stand_leg(
    r: &Rig,
    cv: &mut Canvas,
    p: &crate::dog::DogPose,
    front: bool,
    u0: f32,
    ph: f32,
    far: bool,
) {
    let b = r.br;
    let l = r.ll;
    let d = 2.0 * r.rv;
    let moving = p.running || p.walking;
    let amp = if p.running {
        0.55
    } else if moving {
        0.3
    } else {
        0.0
    };
    let s = (p.step + ph).sin();
    let a = amp * s;
    let lift = if moving {
        (p.step + ph).cos().max(0.0) * if p.running { 0.35 } else { 0.22 }
    } else {
        0.0
    };
    // joints: anchor, upper joint, lower joint, paw (u columns, v rows)
    let j = if front {
        [
            (u0, l + 0.45 * d),
            (u0 - 0.4, l - 0.2),
            (u0 + 0.1, 0.22 * l),
            (u0 + 0.6, 0.0),
        ]
    } else {
        [
            (u0, l + 0.5 * d),
            (u0 + 0.3 * l + 0.5, 0.9 * l),
            (u0 - 0.4 * l - 0.8, 0.36 * l),
            (u0 - 0.3 * l - 0.6, 0.0),
        ]
    };
    // swing about the anchor in visual space (a row is two columns tall)
    let (sa, ca) = a.sin_cos();
    let rot = |q: (f32, f32)| {
        let (dx, dy) = (q.0 - j[0].0, 2.0 * (q.1 - j[0].1));
        (
            j[0].0 + dx * ca - dy * sa,
            j[0].1 + (dx * sa + dy * ca) / 2.0,
        )
    };
    let mut q = [j[0], rot(j[1]), rot(j[2]), rot(j[3])];
    // the paw comes up and folds back while the leg swings forward
    q[2].1 += lift * l * 0.6;
    q[3].1 += lift * l;
    q[3].0 -= lift * l * if front { 1.4 } else { 0.4 };
    let floor = q[3].1.min(q[2].1 - 1.0).min(0.0);
    for k in q.iter_mut() {
        k.1 -= floor;
    }
    let fur = if far {
        shade(b.coat, -1.1)
    } else {
        shade(b.coat, 0.15)
    };
    let sock = socks(b);
    let lower = |v: f32| -> Rgb {
        if sock && v < 0.45 * l {
            if far { shade(b.light, -0.9) } else { b.light }
        } else {
            fur
        }
    };
    let top = |_: f32| fur;
    // two columns thick below the body; the upper leg is hidden in the body and thigh
    let hw = if front { 1.5 } else { 2.0 };
    let lw = if far { 1.0 } else { 1.5 };
    seg(r, cv, q[0], q[1], hw, &top);
    seg(r, cv, q[1], q[2], lw, &lower);
    seg(r, cv, q[2], q[3], lw, &lower);
    // paw: three columns, toes forward
    let pc = if far {
        shade(paw_col(b), -0.9)
    } else {
        paw_col(b)
    };
    let (pu, pv) = q[3];
    for k in 0..5 {
        r.stamp(cv, pu - 1.5 + k as f32 * 0.9, pv, '\u{2588}', pc);
    }
    if b.pat == Pat::Poodle {
        // pompoms at the ankle and the paw
        let pom = if far { shade(b.light, -0.6) } else { b.light };
        r.ell(cv, (q[2].0, q[2].1 * 0.6), (1.6, 0.9), &|_, _, x, y| {
            (if (x + y) % 2 == 0 { '@' } else { '\u{2588}' }, pom)
        });
    }
}

/// Body in profile: chest deeper than the belly (the belly line rises toward the hind legs),
/// the back sloping a little down to the tail, round at both ends, and the hind thigh.
fn stand_body(r: &Rig, cv: &mut Canvas) {
    let (ll, d, ru) = (r.ll, 2.0 * r.rv, r.ru);
    let wh = ll + d;
    let croup = wh - (0.06 * d).max(0.8);
    let tuck = (0.18 * d).max(1.0);
    let cf = (0.55 * ru).min(1.2 * d);
    let cb = (0.5 * ru).min(1.2 * d);
    let (uf, ub) = (ru - cf, -ru + cb);
    let t_of = |u: f32| ((u - ub) / (uf - ub)).clamp(0.0, 1.0);
    let top = |u: f32| croup + (wh - croup) * t_of(u);
    let bot = |u: f32| ll + tuck * (1.0 - smooth(0.3, 0.8, t_of(u)));
    let rear_bot = bot(ub);
    let under = shade(r.br.coat, -0.2);
    r.fill(cv, (-ru, ru), (ll, wh), &|u, v, x, y| {
        let inside = if u > uf {
            let (a, b) = ((u - uf) / cf, (v - (ll + wh) / 2.0) / (d / 2.0));
            a * a + b * b <= 1.0
        } else if u < ub {
            let (cy, ry) = ((rear_bot + croup) / 2.0, (croup - rear_bot) / 2.0);
            let (a, b) = ((u - ub) / cb, (v - cy) / ry);
            a * a + b * b <= 1.0
        } else {
            v >= bot(u) - 0.5 && v <= top(u) + 0.5
        };
        inside.then(|| {
            if v < bot(u.clamp(ub, uf)) + 0.5 {
                ('\u{2588}', under)
            } else {
                coat(r, u, v, x, y)
            }
        })
    });
    // the seated dog's light chest bib, seen from the side
    let light = r.br.light;
    let (bu, bv) = (ru - 0.3 * cf, ll + 0.45 * d);
    r.ell(cv, (bu, bv), (0.32 * cf, 0.32 * d), &|_, v, _, _| {
        ('\u{2588}', shade(light, ((v - bv) / d) * 0.6))
    });
}

/// The hind thigh: a muscle over the top of the near hind leg.
fn stand_thigh(r: &Rig, cv: &mut Canvas, uh: f32) {
    let (ll, d) = (r.ll, 2.0 * r.rv);
    r.ell(
        cv,
        (uh + 0.08 * r.ru, ll + 0.32 * d),
        (0.3 * r.ru, 0.42 * d + 0.12 * ll),
        &|u, v, x, y| {
            let (c, col) = coat(r, u, v.max(ll + 0.5 * d), x, y);
            (c, shade(col, -0.25))
        },
    );
}

/// The standing head: the seated face (front view), or the head in profile.
fn draw_stand_head(r: &Rig, cv: &mut Canvas, body: &Body, hh: f32, side: bool) {
    if !side {
        let (sx, _) = r.xy(r.hu, 0.0);
        let hr = Rig {
            f: 1.0,
            sq: 1.0,
            hu: 0.0,
            ox: sx as f32,
            ..*r
        };
        draw_sit_head(&hr, cv, body, hh);
        return;
    }
    draw_profile_head(r, cv, body, hh);
}

/// A dog's head seen from the side: round skull, a muzzle sticking out toward the travel
/// direction and ending in a dark nose, one shiny eye, one ear at the back, a mouth line.
/// Same height as the seated head.
fn draw_profile_head(r: &Rig, cv: &mut Canvas, body: &Body, hh: f32) {
    let b = r.br;
    let p = &body.dpose;
    let (hu, hv, hrv) = (r.hu, r.hv, r.hrv);
    let su = skull_ru(r);
    let ml = muzzle_len(r);
    let tip = hu + su + ml;
    let (mt, mb) = muzzle_rows(r);
    let ear_c = shade(b.dark, -0.8);
    let ascii = cv.ascii;
    let up_ear = |cv: &mut Canvas, back: f32, c: Rgb| {
        let eh = (0.17 * hh).max(2.0);
        let eu = hu - back * su;
        let n = eh.ceil() as i32;
        for i in 0..n {
            let wd = ((n - i) as f32 / n as f32 * (su * 0.45).max(1.5)).max(1.0);
            let lean = p.tilt * i as f32 * 0.3 - 0.25 * i as f32;
            r.fill(
                cv,
                (eu - wd + lean, eu + wd + lean),
                (
                    hv + hrv * 0.6 + i as f32 - 0.5,
                    hv + hrv * 0.6 + i as f32 + 0.5,
                ),
                &|u, _, _, _| ((u - eu - lean).abs() <= wd * 0.5 + 0.4).then_some(('\u{2593}', c)),
            );
        }
    };
    // far ear peeks up behind the near one
    if b.ears_up {
        up_ear(cv, 0.05, shade(ear_c, -0.5));
    }
    // neck: from the withers up into the back of the skull
    let (nu0, nv0) = (0.55 * r.ru, r.vb + 0.4 * r.rv);
    let (nu1, nv1) = (hu - 0.3 * su, hv - 0.35 * hrv);
    for k in 0..=6 {
        let a = k as f32 / 6.0;
        r.ell(
            cv,
            (nu0 + (nu1 - nu0) * a, nv0 + (nv1 - nv0) * a),
            (0.3 * r.ru, (0.4 * r.rv).max(1.5)),
            &|u, v, x, y| coat(r, u, v.min(r.vb + r.rv), x, y),
        );
    }
    // skull
    r.ell(cv, (hu, hv), (su, hrv), &|_, v, x, y| {
        let rel = ((v - hv) / hrv).clamp(-1.0, 1.0);
        match b.pat {
            Pat::Husky => {
                if rel < 0.2 {
                    ('\u{2588}', b.light)
                } else {
                    ('\u{2593}', b.coat)
                }
            }
            Pat::Poodle => (if (x + y) % 2 == 0 { '@' } else { '\u{2588}' }, b.light),
            _ => ('\u{2588}', shade(b.coat, rel * 0.5)),
        }
    });
    if b.pat == Pat::Poodle {
        r.ell(
            cv,
            (hu - 0.15 * su, hv + hrv * 0.9),
            (su * 0.7, hrv * 0.5),
            &|_, _, x, y| (if (x + y) % 2 == 0 { '@' } else { '\u{2588}' }, b.light),
        );
    }
    if b.pat == Pat::Dalmatian {
        r.stamp(cv, hu - su * 0.3, hv + hrv * 0.5, '\u{25cf}', b.dark);
    }
    // muzzle: a block reaching past the skull, the lower front corner rounded off
    let muz = if b.pat == Pat::Shepherd {
        b.dark
    } else {
        b.light
    };
    let open = p.open > 0.15 || p.tongue > 0.3;
    // a short round snout like the seated dog's: an ellipse from the cheek past the skull
    let mu0 = hu + 0.42 * su;
    let (mcu, mcv) = ((mu0 + tip) / 2.0, (mt + mb) / 2.0);
    let (mru, mrv) = ((tip - mu0) / 2.0 + 0.5, (mt - mb) / 2.0 + 0.6);
    r.fill(
        cv,
        (mu0 - 1.0, tip + 1.0),
        (mb - 1.0, mt + 1.0),
        &|u, v, _, _| {
            let (a, b) = ((u - mcu) / mru, (v - mcv) / mrv);
            (a * a + b * b <= 1.05).then_some(('\u{2588}', muz))
        },
    );
    // mouth line along the lower jaw: the dark top half of '▄' cells; open, a dark gap
    let mouth = (0x70, 0x20, 0x30);
    let mrow = mb + 1.0;
    let mut u = mu0 + 0.3 * su;
    while u <= tip - 1.5 {
        if open {
            r.stamp(cv, u, mrow - 1.0, '\u{2588}', mouth);
        } else {
            r.stamp(cv, u, mrow - 1.0, if ascii { '_' } else { '\u{2584}' }, muz);
        }
        u += 1.0;
    }
    if open {
        // the corner of the mouth turns up into a grin
        r.stamp(cv, mu0 + 0.3 * su - 1.0, mrow, '\u{2588}', mouth);
        // chin dropped a row, tongue hanging out of the side
        let mut u = mu0 + 0.2 * su;
        while u <= tip - 2.5 {
            r.stamp(cv, u, mb - 1.0, '\u{2588}', muz);
            u += 1.0;
        }
        if p.tongue > 0.3 {
            r.stamp(cv, tip - 4.0, mb - 1.0, '\u{2588}', TONGUE);
            r.stamp(cv, tip - 3.0, mb - 1.0, '\u{2588}', TONGUE);
            r.stamp(cv, tip - 3.5, mb - 2.0, '\u{2588}', TONGUE);
        }
    }
    // the nose: dark, at the top of the muzzle tip and one column past it ('▇' is left
    // alone by the soft edge pass)
    let nose = if ascii { '#' } else { '\u{2587}' };
    let nc = lerp(b.nose, (0x60, 0x50, 0x54), 0.45);
    for k in 0..2 {
        r.stamp(cv, tip - k as f32, mt, nose, nc);
    }
    r.stamp(cv, tip, mt - 1.0, nose, nc);
    // one big shiny eye and a rosy cheek
    let (eu, ev) = (hu + 0.3 * su, hv + 0.15 * hrv);
    let dark = (0x18, 0x12, 0x14);
    if body.closed || body.blink {
        r.stamp(cv, eu - 0.5, ev, '\u{2580}', dark);
        r.stamp(cv, eu + 0.5, ev, '\u{2580}', dark);
    } else {
        let iris = if b.eye.0 as u32 + b.eye.1 as u32 + b.eye.2 as u32 > 200 {
            b.eye
        } else {
            dark
        };
        let (eru, erv) = ((0.11 * r.hru).max(1.0), (0.13 * hrv).max(1.0));
        r.ell(cv, (eu, ev), (eru, erv), &|_, _, _, _| ('\u{2588}', iris));
        r.stamp(cv, eu - 0.4, ev + erv * 0.6, '\u{25cf}', (255, 255, 255));
    }
    r.stamp(
        cv,
        hu + 0.1 * su,
        hv - 0.4 * hrv,
        '\u{2592}',
        (0xF0, 0x90, 0xA0),
    );
    // near ear at the back of the head
    if b.ears_up {
        up_ear(cv, 0.4, ear_c);
    } else if b.pat == Pat::Poodle {
        r.ell(
            cv,
            (hu - 0.45 * su, hv - hrv * 0.3),
            (su * 0.35, hrv * 0.85),
            &|_, _, x, y| (if (x + y) % 2 == 0 { '@' } else { '\u{2588}' }, ear_c),
        );
    } else {
        let long = if b.pat == Pat::Dachshund { 1.5 } else { 0.95 };
        let swing = if p.running { 0.6 * p.step.sin() } else { 0.0 };
        r.ell(
            cv,
            (hu - 0.35 * su - swing, hv - hrv * 0.3),
            (r.hru * 0.3, hrv * long),
            &|_, _, _, _| ('\u{2593}', ear_c),
        );
    }
    if p.bark > 0.25 {
        for k in 0..3 {
            r.stamp(
                cv,
                tip + 2.0 + 2.2 * k as f32,
                mt - 0.5 + 0.4 * k as f32,
                ')',
                (0xFF, 0xF0, 0xA0),
            );
        }
    }
}

/// Where the standing dog's mouth is (local coordinates).
fn stand_mouth(r: &Rig, p: &crate::dog::DogPose) -> (f32, f32) {
    if profile(p) {
        let (_, mb) = muzzle_rows(r);
        (r.hu + skull_ru(r) + muzzle_len(r) - 2.0, mb - 0.5)
    } else {
        (r.hu, r.hv - 0.42 * r.hrv - 0.6)
    }
}

pub fn draw_dog(cv: &mut Canvas, body: &Body) {
    let (w, h) = (cv.w as u16, cv.h as u16);
    let p = &body.dpose;
    if p.inside {
        let r = Rig::new(body, w, h);
        draw_inside(cv, body, &r);
        return;
    }
    if let Some((pose, k)) = p.roll_pose() {
        draw_roll(cv, body, pose, k);
        return;
    }
    if p.seated() {
        draw_sit(cv, body);
        return;
    }
    draw_stand(cv, body, profile(p));
}

/// The standing dog; `side` draws the head in profile, else the seated front face.
fn draw_stand(cv: &mut Canvas, body: &Body, side: bool) {
    let (w, h) = (cv.w as u16, cv.h as u16);
    let p = &body.dpose;
    let r = stand_rig(body, w, h);
    let m = r.m;
    let (_, hh) = seat_dims(w, h, body.scale);
    let (uf, uh) = (0.58 * r.ru, -0.6 * r.ru);
    // front legs: dig alternates them backwards
    let dig = if p.dig > 0.0 {
        (p.fast_ph).sin() * p.dig
    } else {
        0.0
    };
    // far legs (darker, a column behind), tail, body and thigh, near legs, head
    let pi = std::f32::consts::PI;
    stand_leg(&r, cv, p, true, uf - 1.0, pi, true);
    stand_leg(&r, cv, p, false, uh - 1.0, 0.0, true);
    draw_tail(&r, cv, p);
    stand_body(&r, cv);
    stand_thigh(&r, cv, uh);
    if p.scratch <= 0.0 {
        stand_leg(&r, cv, p, false, uh, pi, false);
    }
    stand_leg(&r, cv, p, true, uf - 0.06 * m * dig, 0.0, false);
    // hind leg raised to scratch the ear
    if p.scratch > 0.0 {
        let k = (p.fast_ph * 0.5).sin() * 0.5 + 0.5;
        for i in 0..6 {
            let a = i as f32 / 5.0 * p.scratch;
            let (u, v) = (
                uh + (r.hu - uh + 0.5 * k) * a,
                r.vb - 0.5 + (r.hv - r.vb - 0.5) * a,
            );
            r.stamp(cv, u, v, '\u{2588}', r.br.coat);
            r.stamp(cv, u + 1.0, v, '\u{2588}', r.br.coat);
        }
    }
    draw_stand_head(&r, cv, body, hh, side);
    // dirt flying from the front paws
    if p.dig > 0.2 {
        for i in 0..5 {
            let k = (body.t * 9.0 + i as f32 * 1.7) % 2.0;
            let (u, v) = (
                uf - 1.0 - k * 2.4 - i as f32 * 0.8,
                r.ll * 0.4 + 2.5 * k * (2.0 - k),
            );
            r.stamp(cv, u, v, if i % 2 == 0 { '\'' } else { '.' }, DIRT);
        }
    }
    if p.carry == 2 {
        let (u, v) = stand_mouth(&r, p);
        r.stamp(cv, u, v, '\u{25cf}', BALL);
    }
    // ink on the coat: a blot that shrinks as it is shaken off, with flecks
    if let Some((left, ox)) = body.splat {
        let k = if left > 1.4 {
            1.0
        } else {
            (left / 1.4).clamp(0.0, 1.0)
        };
        if k > 0.0 {
            let u = ox * r.f / r.sq.max(0.3);
            r.ell(
                cv,
                (u - 0.02 * m, r.vb + r.rv * 0.2),
                (0.12 * m * k, (0.08 * m * k).max(0.6)),
                &|_, _, _, _| ('\u{2593}', INK),
            );
        }
        if left < 1.4 {
            for i in 0..6 {
                let a = i as f32 * 1.1 + body.t * 3.0;
                let rad = (1.4 - left) * 0.2 * m + i as f32;
                r.stamp(
                    cv,
                    ox + rad * a.cos(),
                    r.vb + rad * a.sin() * 0.6 + 1.0,
                    '*',
                    INK,
                );
            }
        }
    }
}

/// Napping in the doghouse: just the head at the door and the tail on the grass beside it.
fn draw_inside(cv: &mut Canvas, body: &Body, r0: &Rig) {
    let (w, h) = (cv.w as u16, cv.h as u16);
    let (feet, _) = ground(w, h);
    let (x0, hw, _, _) = house(w, h);
    let dx = door_x(w, h);
    let mut b2 = body.clone();
    b2.dpose.spin = 1.0;
    b2.dpose.head_down = 0.0;
    b2.dpose.look_up = 0.0;
    b2.dpose.tilt = 0.0;
    b2.dpose.bark = 0.0;
    b2.dpose.open = 0.0;
    b2.dpose.shake = 0.0;
    b2.closed = true;
    b2.dx = 0.0;
    b2.cx = 0;
    let mut r = Rig::new(&b2, w, h);
    r.ox = dx - r.hu;
    r.oy = (feet - 3) as f32 + r.hv;
    r.shake = 0.0;
    draw_head(&r, cv, &b2, false);
    for y in ((feet - 1) as usize + 1)..cv.h {
        for x in 0..cv.w {
            cv.cells[y * cv.w + x] = Cell { ch: ' ', fg: None };
        }
    }
    // the paws rest on the sill
    let sill = feet - 1;
    for x in -2..3 {
        cv.put(dx.round() as i32 + x + 1, sill, '\u{2584}', r.br.light);
    }
    // tail beside the house
    let tx = x0 + hw + 1;
    let wag = (body.dpose.wag_ph).sin() * body.dpose.wag;
    let tl = (0.26 * r0.m).max(4.0) as i32;
    for k in 0..tl {
        let up = ((k as f32 / tl as f32).powi(2) * 2.0 + wag * 1.5 * k as f32 / tl as f32).round()
            as i32;
        cv.put(tx + k, sill - up, '\u{2588}', r.br.coat);
        if r.br.tail == Tail::Plume || r.br.tail == Tail::Bushy {
            cv.put(tx + k, sill - up + 1, '\u{2593}', r.br.coat);
        }
    }
    // Zzz
    for i in 0..3 {
        let k = (body.t * 0.8 + i as f32 * 0.35) % 1.0;
        let ch = if i % 2 == 0 { 'z' } else { 'Z' };
        cv.put(
            dx.round() as i32 + 6 + (k * 4.0) as i32 + i * 2,
            sill - 4 - (k * 4.0) as i32 - i,
            ch,
            ZZZ,
        );
    }
}

/// Soft darker rim (`▒`) on cells that border empty space, like the cat's gray edge.
fn edge_pass(sc: &mut Canvas) {
    let (w, h) = (sc.w as i32, sc.h as i32);
    let src: Vec<Cell> = sc.cells.clone();
    let at = |x: i32, y: i32| -> bool {
        x >= 0 && y >= 0 && x < w && y < h && src[(y * w + x) as usize].fg.is_some()
    };
    for y in 0..h {
        for x in 0..w {
            let c = src[(y * w + x) as usize];
            let Some(col) = c.fg else { continue };
            if c.ch != '\u{2588}' && c.ch != '\u{2593}' {
                continue;
            }
            if !(at(x - 1, y) && at(x + 1, y) && at(x, y - 1) && at(x, y + 1)) {
                sc.cells[(y * w + x) as usize] = Cell {
                    ch: '\u{2592}',
                    fg: Some(lerp(col, (0, 0, 0), 0.28)),
                };
            }
        }
    }
}

pub fn draw_dog_into(cv: &mut Canvas, body: &Body, _mono: bool, id: u8, w: u16, h: u16) {
    let mut sc = Canvas::new(w, h, cv.ascii);
    draw_dog(&mut sc, body);
    edge_pass(&mut sc);
    let p = &body.dpose;
    let (x0, hw, _, _) = house(w, h);
    let hide = p.nap > 0.0 && !p.inside;
    // rolling dog shuffling past the house stays behind it
    let behind = p.roll_pose().is_some();
    let (feet, _) = ground(w, h);
    for y in 0..sc.h {
        for x in 0..sc.w {
            let c = sc.cells[y * sc.w + x];
            if c.fg.is_none() {
                continue;
            }
            // walking into the house: the body vanishes behind its wall
            if hide && (x as i32) > x0 && (x as i32) < x0 + hw - 1 {
                continue;
            }
            if behind && (x as i32) <= x0 + hw && (y as i32) < feet {
                continue;
            }
            cv.cells[y * cv.w + x] = c;
            cv.owner[y * cv.w + x] = id;
        }
    }
    let _ = NONE;
}

/// Where the dog's mouth is (cell coordinates), for biscuits and the ball.
pub fn mouth_pos(app: &App, body: &Body) -> (f32, f32) {
    if body.dpose.inside {
        return (door_x(app.w, app.h), ground(app.w, app.h).0 as f32 - 3.0);
    }
    if body.dpose.seated() {
        let r = sit_rig(body, app.w, app.h);
        let (x, y) = r.xy(r.hu, r.hv - r.hrv * 0.4);
        return (x as f32, y as f32);
    }
    match body.dpose.roll_pose() {
        Some((RollPose::Sit, _)) => {
            let r = sit_rig(body, app.w, app.h);
            let (x, y) = r.xy(r.hu, r.hv - r.hrv * 0.4);
            return (x as f32, y as f32);
        }
        Some((RollPose::Stand, _)) => {
            let body = &roll_body(body, app.w, app.h);
            let r = stand_rig(body, app.w, app.h);
            let mut p = body.dpose;
            p.walking = true;
            let (u, v) = stand_mouth(&r, &p);
            let (x, y) = r.xy(u, v);
            return (x as f32, y as f32);
        }
        _ => {}
    }
    if let Some((pose, _)) = body.dpose.roll_pose() {
        let d = lie_dims(app.w, app.h, body.scale);
        let body = &roll_body(body, app.w, app.h);
        let r = lie_rig(body, app.w, app.h, &d, d.rv, d.rv);
        let hv = if pose == RollPose::Back {
            d.hrv
        } else {
            d.hrv * 0.6
        };
        let (x, y) = r.xy(d.bu + d.ru + 0.3 * d.hru, hv);
        return (x as f32, y as f32);
    }
    let r = stand_rig(body, app.w, app.h);
    let (u, v) = stand_mouth(&r, &body.dpose);
    let (x, y) = r.xy(u, v);
    (x as f32, y as f32)
}

// ------------------------------------------------------------ props

pub fn draw_props(cv: &mut Canvas, app: &App) {
    if app.kind != Kind::Dog {
        // a dog guest in a cat/squid room carries no props of its own
        return;
    }
    let (feet, _) = ground(app.w, app.h);
    // dug hole
    if let Some((x, age, len)) = app.dog.hole {
        let depth = if len == 0.0 {
            (age / 1.2).min(1.0)
        } else {
            let a = age - len;
            if a > HOLE_STAY {
                1.0 - ((a - HOLE_STAY) / HOLE_FILL).min(1.0)
            } else {
                1.0
            }
        };
        let wd = (2.0 + depth * 5.0).round() as i32;
        let xi = x.round() as i32;
        let y = feet;
        if depth > 0.05 {
            for i in -wd..=wd {
                cv.put(xi + i, y, '\u{2584}', (0x2A, 0x1C, 0x12));
            }
            for i in -(wd - 1)..=(wd - 1) {
                cv.put(xi + i, y - 1, '\u{2580}', DIRT);
            }
            if depth > 0.5 {
                cv.put(xi + wd + 2, y - 1, '.', DIRT);
                cv.put(xi + wd + 3, y, '\u{2584}', DIRT);
            }
        }
    }
    if let Some((x, y)) = app.ball_pos() {
        cv.put(x.round() as i32, y.round() as i32, '\u{25cf}', BALL);
    }
    for (fx, fy) in app.foods() {
        cv.text(fx.round() as i32 - 1, fy.round() as i32, "(o)", BISCUIT);
    }
}
