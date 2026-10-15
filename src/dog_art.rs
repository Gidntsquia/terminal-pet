//! Dog and backyard drawing (step 3 of the wiki's Adding-a-pet.md: `src/<pet>_art.rs`).
//! The dog is a side view built from ellipses in a local frame: `u` runs forward along
//! the facing direction (columns), `v` runs up from the feet (rows).
use crate::app::{App, Kind};
use crate::art::{Body, Canvas, Cell, Geo, Rgb, lerp};
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
pub(crate) const SUN: Rgb = (0xF8, 0xD0, 0x50);
const FLOWER: Rgb = (0xF0, 0x70, 0xA0);
const FLOWER2: Rgb = (0xF8, 0xE0, 0x70);
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

/// (feet row, top row of the fence). The dog's feet touch row `feet - 1`, the cat's paw
/// row in a same-size pane (`geo_cat`: top row 3, about 68% of the rows tall), so a dog and
/// a cat side by side sit level.
pub fn ground(w: u16, h: u16) -> (i32, i32) {
    let _ = w;
    let cat = (h as f32 * 0.68).round() as i32;
    let h = h as i32;
    let fh = if h < 20 { 2 } else { 4 };
    let fence_top = h - 2 - fh;
    let feet = (3 + cat).min(h - 3).max(5);
    (feet, fence_top)
}

/// Row of the back horizon (grass line, fence bottom, house/tree/bush feet): about 55% of
/// the pane, at least 3 rows above the dog's feet row.
pub fn horizon(w: u16, h: u16) -> i32 {
    let (feet, _) = ground(w, h);
    ((h as f32 * 0.55).round() as i32).min(feet - 3).max(3)
}

/// Seated dog: (width in columns, height in rows). Height follows the pane rows like the
/// cat's `geo_cat`; width follows the height but never more than a third of the pane.
pub fn seat_dims(w: u16, h: u16, scale: f32) -> (f32, f32) {
    let feet = ground(w, h).0;
    let frac = 0.70 + 0.11 * ((38.0 - h as f32) / 10.0).clamp(0.0, 1.0);
    let hh = ((h as f32 * frac * scale).round() as i32).clamp(6, (feet - 2).max(6)) as f32;
    let wide = 1.2 + 0.25 * ((40.0 - h as f32) / 18.0).clamp(0.0, 1.0);
    let ww = (hh * wide).min(w as f32 * 0.45).max(8.0);
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

/// Scale the napping dog shrinks to while it walks into the doghouse (its head then fits
/// the small door).
pub const NAP_SCALE: f32 = 0.45;

/// Doghouse: (left column, width, wall height, roof height). A small prop at the far left,
/// about two thirds of the seated dog's height; its door is only as big as the shrunken
/// napping head needs.
pub(crate) fn house(w: u16, h: u16) -> (i32, i32, i32, i32) {
    let (ww, hh) = seat_dims(w, h, 1.0);
    let t = ((ww * 0.1).round() as i32).max(2);
    let (_, dw, dh) = door(w, h);
    let hw = dw + 2 * t;
    let total = ((hh * 0.6).round() as i32).max(6);
    let roof = ((hw as f32 * 0.14).round() as i32).clamp(2, 10);
    let wall = (total - roof).max(dh + 2);
    (1, hw, wall, roof)
}

/// Doghouse door: (left column, width, height). Its bottom row is the dog's feet row
/// `ground().0 - 1`; the top two rows are rounded.
pub fn door(w: u16, h: u16) -> (i32, i32, i32) {
    let (ww, _) = seat_dims(w, h, 1.0);
    let t = ((ww * 0.1).round() as i32).max(2);
    let (wn, hn) = seat_dims(w, h, NAP_SCALE);
    let dw = (0.9 * wn).ceil() as i32 + 1;
    let dh = (0.53 * hn).ceil() as i32 + 2;
    (1 + t, dw, dh)
}

/// Is cell (x, y) inside the door opening?
pub(crate) fn in_door(w: u16, h: u16, x: i32, y: i32) -> bool {
    let (dx0, dw, dh) = door(w, h);
    let base = horizon(w, h) - 1;
    let i = base - y;
    if !(0..dh).contains(&i) {
        return false;
    }
    let inset = (i - (dh - 3)).max(0);
    (dx0 + inset..dx0 + dw - inset).contains(&x)
}

/// Rightmost column of the doghouse (roof overhang included).
pub fn house_right(w: u16, h: u16) -> i32 {
    let (x0, hw, _, _) = house(w, h);
    x0 + hw
}

/// Is cell (x, y) covered by the doghouse drawing (walls and roof, not the open door)?
/// Cells beside the roof are open sky, so a dog behind the house still shows there.
fn house_covers(w: u16, h: u16, x: i32, y: i32) -> bool {
    let (x0, hw, wall, roof) = house(w, h);
    let base = horizon(w, h) - 1;
    if y > base {
        return false;
    }
    if y > base - wall {
        return (x0..x0 + hw).contains(&x) && !in_door(w, h, x, y);
    }
    let r = base - wall - y;
    if r >= roof {
        return false;
    }
    let (xs, wd) = roof_row(hw, x0, roof, r);
    (xs..xs + wd).contains(&x)
}

/// Roof row `r` (0 = eave): (left column, width).
fn roof_row(hw: i32, x0: i32, roof: i32, r: i32) -> (i32, i32) {
    let shrink = ((hw + 2) as f32 - 4.0) / roof as f32;
    let wd = ((hw + 2) as f32 - shrink * r as f32).round() as i32;
    (x0 - 1 + ((hw + 2) - wd) / 2, wd)
}

/// Column of the doghouse door.
pub fn door_x(w: u16, h: u16) -> f32 {
    let (dx0, dw, _) = door(w, h);
    dx0 as f32 + (dw - 1) as f32 / 2.0
}

/// Half the width of the seated dog as drawn (paws and haunches, without the tail).
pub fn seat_half(w: u16, h: u16) -> f32 {
    0.56 * seat_dims(w, h, 1.0).0
}

/// Tree: (trunk centre column, trunk width, crown centre row, crown radii in rows / cols).
/// The crown is about 0.8x the seated dog's height and tops the dog's head; it
/// starts right of the seated dog (tail and idle drift included).
pub(crate) fn tree(w: u16, h: u16) -> (i32, i32, f32, f32, f32) {
    let (ww, hh) = seat_dims(w, h, 1.0);
    let feet = horizon(w, h);
    let wi = w as i32;
    let tw = ((ww * 0.13).round() as i32).max(3);
    let tx = wi - 2 - tw / 2;
    let rv = 0.41 * hh;
    let bottom = (feet - 1) as f32 - 0.2 * hh;
    let ccy = bottom - rv;
    // clear of the seated dog: centre + drift + seated half + tail
    let dog_right = wi as f32 / 2.0 + idle_room(w, h) + 0.95 * ww;
    let ru = (1.2 * rv).min((tx as f32 - dog_right - 1.0).max(tw as f32 + 2.0));
    (tx, tw, ccy, rv, ru)
}

/// Leftmost column of the tree's crown.
pub fn tree_left(w: u16, h: u16) -> i32 {
    let (tx, _, _, _, ru) = tree(w, h);
    (tx as f32 - ru).floor() as i32
}

/// How far (columns) the idle drift may take the seated dog from the centre with no guest
/// (as `App::wander` with no guests).
fn idle_room(w: u16, h: u16) -> f32 {
    let g = geo_dog(w, h, 1.0);
    (w as f32 / 2.0 - g.maxw * 2.2).max(0.0)
}

/// Back fence: (top row, bottom row). Pickets are about 0.3 of the seated dog's height and
/// stand on the horizon line behind the dog.
pub(crate) fn fence(w: u16, h: u16) -> (i32, i32) {
    let (_, hh) = seat_dims(w, h, 1.0);
    let horizon = self::horizon(w, h);
    let fh = ((0.3 * hh).round() as i32).clamp(2, horizon);
    (horizon - fh, horizon - 1)
}

// ------------------------------------------------------------ the yard

pub fn draw_yard(cv: &mut Canvas, app: &App) {
    let (w, h) = (cv.w as i32, cv.h as i32);
    let (dfeet, _) = ground(app.w, app.h);
    let feet = horizon(app.w, app.h);
    let (ww, hh) = seat_dims(app.w, app.h, 1.0);
    let t = app.t;
    let (tx, tw, ccy, rv, ru) = tree(app.w, app.h);
    // sun: a disc with rays, in the sky between the dog and the tree, about 0.2 of the
    // seated height (5 rows at 120x40)
    let sr = (0.07 * hh).max(1.0);
    let head_right = w as f32 / 2.0 + idle_room(app.w, app.h) + 0.5 * ww;
    let sx = (head_right + tx as f32 - ru) / 2.0;
    let sy = sr + 1.0;
    for y in 0..=((sy + sr).ceil() as i32) {
        for x in ((sx - sr * 2.0 - 1.0) as i32)..=((sx + sr * 2.0 + 1.0) as i32) {
            let (dx, dy) = ((x as f32 - sx) / 2.0, y as f32 - sy);
            let d = (dx * dx + dy * dy).sqrt();
            if d <= sr {
                cv.put(x, y, if d <= sr * 0.6 { '@' } else { '\u{2588}' }, SUN);
            }
        }
    }
    // eight rays, each two cells long
    for (ux, uy, ch) in [
        (1.0, 0.0, '-'),
        (-1.0, 0.0, '-'),
        (0.0, 1.0, '|'),
        (0.0, -1.0, '|'),
        (0.7, -0.7, '/'),
        (-0.7, 0.7, '/'),
        (0.7, 0.7, '\\'),
        (-0.7, -0.7, '\\'),
    ] {
        for r in [sr + 0.8, sr + 1.6] {
            let x = (sx + ux * r * 2.0).round() as i32;
            let y = (sy + uy * r).round() as i32;
            cv.put(x, y, ch, SUN);
        }
    }
    // clouds drifting across the sky, about twice as wide as before
    let cl = (0.55 * ww).round().clamp(10.0, 32.0) as i32;
    for (i, row) in [(0, 2), (1, 5)] {
        let span = w as f32 + cl as f32 * 2.0;
        let cx =
            ((t * (1.1 - 0.3 * i as f32) + w as f32 * (0.2 + 0.5 * i as f32)) % span) as i32 - cl;
        let top: String = format!(" .{}. ", "-~".repeat(((cl - 4) / 2).max(1) as usize));
        let bot: String = format!("({})", "_".repeat((cl - 2).max(1) as usize));
        cv.text(
            cx + 1,
            row,
            &format!("({})", " ".repeat((cl - 4).max(1) as usize)),
            CLOUD,
        );
        cv.text(cx, row - 1, &top, CLOUD);
        cv.text(cx, row + 1, &bot, CLOUD);
    }
    // back fence on the horizon, behind everything in the yard
    let horizon = feet;
    let (ftop, fbot) = fence(app.w, app.h);
    let fh = fbot - ftop + 1;
    for y in ftop..=fbot {
        for x in 0..w {
            let board = x % 4 < 2;
            let rail = y == ftop + fh / 4 + 1 || y == fbot - fh / 4;
            if board {
                let ch = if y == ftop { '^' } else { '\u{2588}' };
                cv.put(x, y, ch, if x % 4 == 0 { WOOD } else { WOOD_D });
            } else if rail {
                cv.put(x, y, '=', WOOD_D);
            }
        }
    }
    // grass: a horizon line, then tufts down to just above the message row
    for x in 0..w {
        cv.put(x, horizon, '\u{2591}', GRASS_D);
    }
    for y in (horizon + 1)..(h - 2) {
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
    // shadow under the seated dog: darker, denser tufts one row high at the paw row
    {
        let cx = w / 2 + app.dog_dx() as i32;
        let half = (0.5 * ww) as i32;
        for x in (cx - half)..=(cx + half) {
            let k = hash(x + 77, dfeet);
            cv.put(
                x,
                dfeet - 1,
                if k.is_multiple_of(3) { '"' } else { ',' },
                GRASS_D,
            );
        }
    }
    // tree on the right
    for y in (ccy as i32).max(0)..(feet - 1) {
        for x in 0..tw {
            cv.put(tx - tw / 2 + x, y, '\u{2588}', TRUNK);
        }
    }
    cv.text(tx - tw / 2 - 1, feet - 2, "/", TRUNK);
    cv.text(tx - tw / 2 + tw, feet - 2, "\\", TRUNK);
    for y in ((ccy - rv).floor() as i32).max(0)..=((ccy + rv).ceil() as i32) {
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
    // a flowering bush at the foot of the tree, on the dog's side
    let br = (0.3 * hh).max(2.0);
    let bu = (br * 2.4)
        .min((tx - tw / 2) as f32 - (w as f32 / 2.0 + 0.6 * ww))
        .max(br);
    let bx = (tx - tw / 2) as f32 - bu * 0.55;
    let by = (feet - 1) as f32;
    for y in ((by - br).floor() as i32)..=(feet - 1) {
        let dy = (y as f32 - by) / br;
        if dy.abs() > 1.0 {
            continue;
        }
        let half = bu * (1.0 - dy * dy).sqrt();
        for x in ((bx - half).round() as i32)..=((bx + half).round() as i32) {
            let k = hash(x + 911, y);
            let (ch, c) = match k % 9 {
                0 => ('*', FLOWER),
                1 => ('o', FLOWER2),
                2 | 3 => ('#', LEAF),
                4 => ('@', LEAF2),
                _ => ('\u{2593}', LEAF),
            };
            cv.put(x, y, ch, c);
        }
    }
    // doghouse on the left
    let (x0, hw, wall, roof) = house(app.w, app.h);
    let base = feet - 1;
    for y in (base - wall + 1)..=base {
        for x in x0..(x0 + hw) {
            // board seams, kept off the door's rounded corners
            let plank = (x - x0) % 4 == 3
                && !in_door(app.w, app.h, x - 1, y)
                && !in_door(app.w, app.h, x + 1, y);
            cv.put(
                x,
                y,
                if plank { '|' } else { '\u{2588}' },
                if plank { WOOD_D } else { HOUSE },
            );
        }
    }
    for r in 0..roof {
        let (xs, wd) = roof_row(hw, x0, roof, r);
        let y = base - wall - r;
        for x in xs..(xs + wd) {
            cv.put(x, y, '\u{2593}', ROOF);
        }
    }
    for y in (base - wall + 1)..=base {
        for x in x0..(x0 + hw) {
            if in_door(app.w, app.h, x, y) {
                let (xx, yy) = (x as usize, y as usize);
                if xx < cv.w && yy < cv.h {
                    cv.cells[yy * cv.w + xx] = Cell { ch: ' ', fg: None };
                }
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
    // turning from the side view: the face slides toward `face` (three-quarter view) and
    // the eyes close up, easing back to the front face as `turn` reaches 1
    let q = 1.0 - smooth(0.0, 1.0, p.turn.clamp(0.0, 1.0));
    let fo = face * q * 0.45 * hru;
    let squeeze = 1.0 - 0.5 * q;
    // ears behind the head: pointed and up, or floppy at the sides
    let eard = 0.0;
    for s in [-1.0f32, 1.0] {
        let tilt = if s == face { p.tilt * 0.8 } else { 0.0 };
        let c = shade(b.dark, -0.8);
        let hu = hu - 0.5 * fo;
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
    let hu = hu + fo;
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
    let ex = (0.5 * hru).max(2.0) * squeeze;
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
        let nap = body.closed && (body.scale - NAP_SCALE).abs() < 0.01;
        if nap {
            // asleep: a dark `u` lid line per eye, 3 cells on a wide head, 2, or one
            let wide = if ex >= 3.0 {
                8.0
            } else if ex >= 2.5 {
                5.0
            } else {
                0.0
            };
            let cells: &[(f32, char, char)] = if wide >= 8.0 {
                &[
                    (-1.0, '\u{2580}', '\\'),
                    (0.0, '\u{2580}', '_'),
                    (1.0, '\u{2580}', '/'),
                ]
            } else if wide >= 5.0 {
                &[(-0.5, '\u{2580}', '\\'), (0.5, '\u{2580}', '/')]
            } else {
                &[(0.0, '\u{2580}', 'u')]
            };
            // no cell background: put fur above and below so the dark lid has fur around it
            let fur = shade(b.coat, 0.0);
            for &(du, _, _) in cells {
                for dv in [-1.0f32, 1.0] {
                    r.stamp(cv, eu + du, ev + dv, '\u{2588}', fur);
                }
            }
            for &(du, ch, a) in cells {
                r.stamp(
                    cv,
                    eu + du,
                    ev,
                    if cv.ascii { a } else { ch },
                    if cv.ascii { dark } else { fur },
                );
            }
        } else if body.closed || body.blink {
            // closed lid: the cell is lid skin with a thin dark gap along its top edge (the lash line), so
            // it reads as a slit under a lid, not a hole; one `-` per eye in ascii
            let skin = shade(b.coat, 1.0);
            for du in [-0.5f32, 0.5] {
                let ch = if cv.ascii { '-' } else { '\u{2586}' };
                r.stamp(cv, eu + du, ev, ch, if cv.ascii { dark } else { skin });
            }
        } else {
            let (eru, erv) = ((0.11 * hru).max(1.0), (0.13 * hrv).max(1.0));
            r.ell(cv, (eu, ev), (eru, erv), &|_, _, _, _| ('\u{2588}', iris));
            r.stamp(cv, eu - 0.4, ev + erv * 0.6, '\u{25cf}', (255, 255, 255));
        }
        r.stamp(
            cv,
            hu + s * (ex + 0.28 * hru * squeeze),
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

// ------------------------------------------------------------ the seated head, anywhere

/// One head, drawn alone: cells relative to the head's centre cell.
type Sprite = Vec<(i32, i32, Cell)>;

#[cfg(test)]
thread_local! {
    /// Screen cells of every head pasted since the last clear (tests measure them).
    pub(crate) static PASTED: std::cell::RefCell<Vec<(i32, i32, Cell)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// The seated head on its own (tests compare every pose's head with it).
#[cfg(test)]
pub(crate) fn seated_head(body: &Body, w: u16, h: u16, side: bool) -> Vec<(i32, i32, Cell)> {
    head_sprite(body, w, h, false, side, 1.0)
}

/// The seated dog's head drawn on its own at the seated size, as cells relative to its
/// centre, so every pose pastes exactly the seated head: the same rows and columns, eyes,
/// blush, nose, ears and markings. `side` draws the chibi profile facing `face`.
fn head_sprite(body: &Body, w: u16, h: u16, ascii: bool, side: bool, face: f32) -> Sprite {
    let mut b = body.clone();
    b.dx = 0.0;
    b.dy = 0.0;
    b.cx = w as i32 / 2;
    b.dpose.shake = 0.0;
    b.dpose.look_up = 0.0;
    b.dpose.spin = face;
    let (_, hh) = seat_dims(w, h, body.scale);
    let mut r = sit_rig(&b, w, h);
    r.hu = 0.0;
    r.hv = 0.745 * hh;
    r.shake = 0.0;
    let mut tmp = Canvas::new(w, h, ascii);
    if side {
        r.f = face;
        draw_profile_head(&r, &mut tmp, &b, hh);
    } else {
        draw_sit_head(&r, &mut tmp, &b, hh);
    }
    let (cx, cy) = (r.ox.round() as i32, (r.oy - r.hv).round() as i32);
    let mut out = vec![];
    for y in 0..tmp.h as i32 {
        for x in 0..tmp.w as i32 {
            let c = tmp.cells[(y * tmp.w as i32 + x) as usize];
            if c.fg.is_some() {
                out.push((x - cx, y - cy, c));
            }
        }
    }
    out
}

/// Paste a head sprite with its centre at local (`u`, `v`) of `r`. `flip` turns it upside
/// down (on its back); cells that would land below `floor` (local v) fold up onto it.
fn paste(r: &Rig, cv: &mut Canvas, s: &Sprite, (u, v): (f32, f32), flip: bool, floor: f32) {
    let x0 = (r.ox + r.f * r.sq * u).round() as i32;
    let y0 = (r.oy - v).round() as i32;
    let ground = (r.oy - floor).round() as i32;
    for &(dx, dy, c) in s {
        let Some(col) = c.fg else { continue };
        let (y, ch) = if flip {
            let ch = match c.ch {
                '\u{2580}' => '\u{2584}',
                '\u{2584}' => '\u{2580}',
                k => k,
            };
            (y0 - dy, ch)
        } else {
            (y0 + dy, c.ch)
        };
        let y = y.min(ground);
        let x = x0 + dx + r.shear(y);
        cv.put(x, y, ch, col);
        #[cfg(test)]
        PASTED.with(|p| p.borrow_mut().push((x, y, Cell { ch, fg: Some(col) })));
    }
}

/// The seated front leg's half-width (columns): every leg in every pose is this thick.
fn seat_lw(ww: f32) -> f32 {
    (0.05 * ww).max(1.0)
}

/// A straight piece of leg from `a` to `b` (local), as thick as the seated front leg: a
/// cell is in when it is within `lw` columns or 0.8 rows of the line.
fn fat_seg(
    r: &Rig,
    cv: &mut Canvas,
    a: (f32, f32),
    b: (f32, f32),
    lw: f32,
    paint: &dyn Fn(f32, f32, i32, i32) -> (char, Rgb),
) {
    let tv = 0.8;
    let (ax, ay, bx, by) = (a.0 / lw, a.1 / tv, b.0 / lw, b.1 / tv);
    let (dx, dy) = (bx - ax, by - ay);
    let l2 = (dx * dx + dy * dy).max(1e-6);
    r.fill(
        cv,
        (a.0.min(b.0) - lw - 1.0, a.0.max(b.0) + lw + 1.0),
        (a.1.min(b.1) - 1.0, a.1.max(b.1) + 1.0),
        &|u, v, x, y| {
            let (px, py) = (u / lw - ax, v / tv - ay);
            let t = ((px * dx + py * dy) / l2).clamp(0.0, 1.0);
            let (ex, ey) = (px - t * dx, py - t * dy);
            (ex * ex + ey * ey <= 1.0).then(|| paint(u, v, x, y))
        },
    );
}

/// The seated dog's rounded paw (5 columns by 2 rows at the seated size), centred at local
/// (`u`, `v`); `v` should sit half way between two rows.
fn seat_paw(r: &Rig, cv: &mut Canvas, (u, v): (f32, f32), lw: f32, c: Rgb) {
    r.ell(cv, (u, v), (lw + 1.0, 1.0), &|_, _, _, _| ('\u{2588}', c));
}

/// A chunky leg through `pts` (local) with the seated paw at the end, toes `toe` columns
/// forward of the last point.
fn chunky_leg(r: &Rig, cv: &mut Canvas, pts: &[(f32, f32)], lw: f32, fur: Rgb, paw: Rgb, toe: f32) {
    for s in pts.windows(2) {
        fat_seg(r, cv, s[0], s[1], lw, &|_, _, _, _| ('\u{2588}', fur));
    }
    if let Some(&(u, v)) = pts.last() {
        seat_paw(r, cv, (u + toe, v), lw, paw);
    }
}

/// The seated haunch (hind thigh), the same size and shade as on the seated dog.
fn seat_haunch(r: &Rig, cv: &mut Canvas, at: (f32, f32), d: &LieDims, dk: f32) {
    r.ell(cv, at, (0.2 * d.ww, 0.2 * d.hh), &|u, v, x, y| {
        let (c, col) = coat(r, u, v, x, y);
        (c, shade(col, dk))
    });
}

// ------------------------------------------------------------ rolling over

/// The seated dog's sizes, shared by every roll pose so nothing shrinks: the seated head,
/// the seated leg thickness, and a lying body about as long, nose to rump, as the seated
/// dog is tall (in looks: a row is two columns) and as deep as the seated torso is wide.
struct LieDims {
    ww: f32,
    hh: f32,
    hru: f32,
    hrv: f32,
    /// Leg half-width and leg length.
    lw: f32,
    ll: f32,
    /// Lying body: centre column (local), half-length, half-height.
    bu: f32,
    ru: f32,
    rv: f32,
    /// Column of the head's centre.
    hc: f32,
    /// How far pointed ears reach above the head centre (0 for floppy ears).
    ear: f32,
}

fn lie_dims(w: u16, h: u16, scale: f32, b: &Breed) -> LieDims {
    let (ww, hh) = seat_dims(w, h, scale);
    let hru = (0.42 * ww).max(4.0);
    let hrv = 0.18 * hh;
    let ru = ((1.95 * hh - 1.3 * hru) / 2.0).max(0.4 * ww);
    let bu = -0.625 * hru;
    LieDims {
        ww,
        hh,
        hru,
        hrv,
        lw: seat_lw(ww),
        ll: (leg_frac(b) * 0.54 * hh).max(2.0),
        bu,
        ru,
        rv: (0.16 * ww).max(2.0),
        hc: bu + ru + 0.25 * hru,
        ear: if b.ears_up {
            0.55 * hrv + (0.17 * hh).max(2.0).ceil()
        } else {
            0.0
        },
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

/// Height of the head's centre (local v) in each lying pose.
fn lie_head_v(pose: RollPose, d: &LieDims, k: f32) -> f32 {
    match pose {
        RollPose::Down => 1.1 * d.rv + d.hrv + 0.2 + 0.6 * k,
        RollPose::SideA => d.hrv + 1.1,
        RollPose::SideB => d.hrv + 0.5,
        RollPose::TwistA | RollPose::TwistB => d.hrv + 0.8,
        // upside down, pointed ears prop the head up off the grass
        _ => (d.hrv + 0.5).max(d.ear + 0.5),
    }
}

/// The seated dog's face at the seated size, centred at local (`hc`, `v`). `flip` draws it
/// upside down (on its back) with the tongue flopping out of the side of the mouth.
fn lie_head(cv: &mut Canvas, r: &Rig, body: &Body, d: &LieDims, v: f32, tilt: f32, flip: bool) {
    let (w, h) = (cv.w as u16, cv.h as u16);
    let mut hb = body.clone();
    hb.dpose.tilt = tilt;
    hb.dpose.bark = 0.0;
    if flip {
        hb.dpose.tongue = 0.0;
        hb.dpose.open = 0.0;
    }
    let s = head_sprite(&hb, w, h, cv.ascii, true, r.f);
    // long ears may drape onto the lawn in front; upside down they fold on the grass
    // the head is raised if its ears would hang past the last lawn row above the message row
    let low = s.iter().map(|c| c.1).max().unwrap_or(0);
    let bottom = (r.oy - v).round() as i32 + low;
    let over = (bottom - (cv.h as i32 - 3)).max(0) as f32;
    let v = if flip { v } else { v + over };
    paste(r, cv, &s, (d.hc, v), flip, if flip { 0.0 } else { -6.0 });
    if flip {
        let mv = v + d.hrv * 0.42;
        // the tongue flops out past the upside-down snout tip, down toward the grass
        let tip = d.hc + d.hru + 1.6;
        for k in 0..3 {
            r.stamp(
                cv,
                tip - 0.5 + k as f32,
                mv - (k as f32 * 0.5).floor(),
                '\u{2588}',
                TONGUE,
            );
        }
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
/// on the other side, up on its feet, and sitting. Every frame is the seated dog: the
/// seated head pasted at the seated size, a body as bulky as the seated one, and legs as
/// thick as the seated front leg with the seated paw; the spine stays level while lying.
fn draw_roll(cv: &mut Canvas, body: &Body, pose: RollPose, k: f32) {
    let (w, h) = (cv.w as u16, cv.h as u16);
    let body = &roll_body(body, w, h);
    if pose == RollPose::Sit {
        draw_sit(cv, body);
        return;
    }
    if pose == RollPose::Stand {
        draw_stand(cv, body, true);
        return;
    }
    let br = &BREEDS[body.breed.min(BREEDS.len() - 1)];
    let d = lie_dims(w, h, body.scale, br);
    let p = &body.dpose;
    let ww = d.ww;
    let wig = |ph: f32| (body.t * 9.0 + ph).sin();
    let (bu, ru, rv, lw, ll) = (d.bu, d.ru, d.rv, d.lw, d.ll);
    let hv = lie_head_v(pose, &d, k);
    let rump = bu - 0.62 * ru;
    match pose {
        RollPose::Down => {
            // sphinx: belly on the grass, front legs forward under the chin, the hind leg
            // folded under the haunch, head up
            let bv = rv + 0.2;
            let r = lie_rig(body, w, h, &d, bv, rv);
            let b = r.br;
            let paw = paw_col(b);
            lie_tail(&r, cv, p, (bu - ru + 1.0, 0.8), ww);
            let reach = d.hc + 0.25 * d.hru;
            let far = shade(b.coat, -0.9);
            chunky_leg(
                &r,
                cv,
                &[(bu + 0.5 * ru, 1.5), (reach - 3.0, 1.5)],
                lw,
                far,
                shade(paw, -0.6),
                0.5,
            );
            lie_torso(&r, cv, (bu, bv), (ru, rv), 0.0);
            seat_haunch(&r, cv, (rump, 0.2 * d.hh), &d, -0.5);
            // the hind paw pokes out forward under the haunch, as on the seated dog
            seat_paw(&r, cv, (rump + 0.2 * ww, 0.5), lw, paw);
            chunky_leg(
                &r,
                cv,
                &[(bu + 0.55 * ru, 0.5), (reach, 0.5)],
                lw,
                shade(b.coat, 0.15),
                paw,
                0.5,
            );
            lie_head(cv, &r, body, &d, hv, 0.0, false);
        }
        RollPose::SideA | RollPose::SideB => {
            let a = pose == RollPose::SideA;
            let bv = rv + if a { 1.2 } else { 0.2 };
            let r = lie_rig(body, w, h, &d, bv, rv);
            let b = r.br;
            let paw = paw_col(b);
            lie_tail(&r, cv, p, (bu - ru + 1.0, 0.8), ww);
            let fur = shade(b.coat, 0.15);
            let top = bv + rv;
            if !a {
                // back toward the viewer: the legs lie on the far side, paws peek over
                for (i, u0) in [0.62f32, 0.36, -0.42, -0.68].iter().enumerate() {
                    let u = bu + u0 * ru;
                    let up = top + 0.5 + (0.6 * wig(i as f32)).round();
                    fat_seg(&r, cv, (u, top - 2.0), (u + 1.0, up), lw, &|_, _, _, _| {
                        ('\u{2588}', shade(b.coat, -0.6))
                    });
                    seat_paw(&r, cv, (u + 1.5, up), lw, shade(paw, -0.4));
                }
            }
            lie_torso(&r, cv, (bu, bv), (ru, rv), if a { 0.55 } else { 0.0 });
            if a {
                seat_haunch(&r, cv, (rump, bv), &d, -0.5);
                // belly toward the viewer: all four legs out on the grass in front of it,
                // loosely bent, front legs toward the head, hind legs toward the tail
                let s = |i: f32| 0.5 * wig(i * 1.7);
                let low = (bv - 0.4 * ll).min(-1.5);
                for (i, u0) in [0.5f32, 0.22].iter().enumerate() {
                    let u0 = bu + u0 * ru;
                    let o = s(i as f32);
                    chunky_leg(
                        &r,
                        cv,
                        &[
                            (u0, bv - 0.3 * rv),
                            (u0 + 0.5 * ll + o, 0.5 * (bv + low)),
                            (u0 + 1.1 * ll + o, low),
                        ],
                        lw,
                        fur,
                        paw,
                        0.8,
                    );
                }
                for (i, u0) in [-0.55f32, -0.78].iter().enumerate() {
                    let u0 = bu + u0 * ru;
                    let o = s(i as f32 + 2.0);
                    chunky_leg(
                        &r,
                        cv,
                        &[
                            (u0, bv - 0.3 * rv),
                            (u0 + 0.6 * ll, 0.6 * (bv + low)),
                            (u0 - 0.2 * ll + o, low),
                        ],
                        lw,
                        fur,
                        paw,
                        -0.8,
                    );
                }
            }
            // head lying on the grass at the front, turned on its side
            lie_head(cv, &r, body, &d, hv, if a { 0.8 } else { -0.8 }, false);
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
            let fold = 0.5 * ll;
            let legs = |cv: &mut Canvas| {
                // front legs folded toward the chest, hind legs bent at the hock
                for (i, u0) in [0.5f32, 0.25].iter().enumerate() {
                    let u0 = bu + u0 * ru;
                    let o = 2.0 * i as f32;
                    chunky_leg(
                        &r,
                        cv,
                        &[(u0, top - 2.0), (u0 + 2.0 + o, top + fold)],
                        lw,
                        fur,
                        pc,
                        -1.5,
                    );
                }
                seat_haunch(
                    &r,
                    cv,
                    (rump, top - 0.1 * d.hh),
                    &d,
                    if a { -0.5 } else { -1.0 },
                );
                for (i, u0) in [-0.55f32, -0.75].iter().enumerate() {
                    let u0 = bu + u0 * ru;
                    let o = 2.0 * i as f32;
                    chunky_leg(
                        &r,
                        cv,
                        &[(u0 + 2.0, top + 0.5), (u0 + 4.0 - o, top + fold + 0.5)],
                        lw,
                        fur,
                        pc,
                        -1.5,
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
            lie_head(cv, &r, body, &d, hv, if a { 1.3 } else { -1.3 }, false);
        }
        RollPose::Back => {
            let bv = rv + 0.2;
            let r = lie_rig(body, w, h, &d, bv, rv);
            let b = r.br;
            let paw = paw_col(b);
            lie_tail(&r, cv, p, (bu - ru + 1.0, 0.8), ww);
            let top = bv + rv;
            let far = shade(b.coat, -0.6);
            let fur = shade(b.coat, 0.15);
            let s = |i: f32| (0.5 * wig(i * 1.9)).round();
            let fold = (0.5 * ll).max(2.0);
            // forelegs: upper arm up from the chest, forearm folded back, paw curled
            let front = |cv: &mut Canvas, u0: f32, i: f32, fur: Rgb, pc: Rgb| {
                let knee = (u0 + 2.0, top + fold + s(i));
                chunky_leg(&r, cv, &[(u0, top - 1.0), knee], lw, fur, pc, -2.0);
            };
            // hind legs: the thigh (the seated haunch) up, shin bent back toward the tail
            let hind = |cv: &mut Canvas, u0: f32, i: f32, fur: Rgb, pc: Rgb, dk: f32| {
                seat_haunch(&r, cv, (u0, top), &d, dk);
                let knee = (u0 + 2.0, top + 0.1 * d.hh + 1.5);
                chunky_leg(
                    &r,
                    cv,
                    &[knee, (u0 - 0.6 * ll, top + fold + 1.0 + s(i))],
                    lw,
                    fur,
                    pc,
                    -0.8,
                );
            };
            front(cv, bu + 0.7 * ru, 0.0, far, shade(paw, -0.4));
            hind(cv, rump + 3.0, 1.0, far, shade(paw, -0.4), -1.0);
            lie_torso(&r, cv, (bu, bv), (ru, rv), 1.9);
            front(cv, bu + 0.45 * ru, 2.0, fur, paw);
            hind(cv, rump, 3.0, fur, paw, -0.5);
            // head tilted back on the grass, upside down, tongue out
            lie_head(cv, &r, body, &d, hv, 0.0, true);
        }
        RollPose::Stand | RollPose::Sit => {}
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
    let d = lie_dims(w, h, scale, &BREEDS[0]);
    let back = d.bu - d.ru - 0.32 * d.ww - 1.0;
    let front = (d.hc + d.hru * 1.35).max(0.29 * d.ww + d.hru + 3.0);
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
/// height, a body as long as the seated dog is wide (haunch to haunch) and legs as thick as
/// the seated front leg. Local `u` runs forward (columns), `v` up from the grass (rows);
/// `ll` is the leg length below the chest, `vb`/`rv` the body's middle and half-depth at
/// the chest, so the withers sit at `vb + rv`.
fn stand_rig<'a>(body: &Body, w: u16, h: u16) -> Rig<'a> {
    let mut r = Rig::new(body, w, h);
    let (ww, hh) = seat_dims(w, h, body.scale);
    let p = &body.dpose;
    let hru = (0.42 * ww).max(4.0);
    let hrv = 0.18 * hh;
    // chibi like the seated dog: a big round head on a round body with stubby legs
    let withers = 0.54 * hh;
    let ll = (leg_frac(r.br) * withers).max(2.0);
    // as long as the seated dog is wide across the haunches, for every breed
    let ru = 0.46 * ww;
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
    r.hu = 0.6 * ru + hd * 0.25 * hru;
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

/// The profile snout, relative to the head centre: (centre u, centre v, half-width,
/// half-height). It is the seated muzzle seen from the side, poking `1–2` columns past the
/// round head.
fn snout(r: &Rig) -> (f32, f32, f32, f32) {
    let (mru, mrv) = (r.hru * 0.38, (r.hrv * 0.36).max(1.0));
    let tip = r.hru + 1.6;
    (tip - mru, -0.42 * r.hrv, mru, mrv)
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Socks: breeds with white feet get light lower legs.
fn socks(b: &Breed) -> bool {
    matches!(
        b.pat,
        Pat::Corgi | Pat::Beagle | Pat::Husky | Pat::Heeler | Pat::Dachshund
    )
}

/// Colour of the far legs in profile (darker, behind the body).
pub(crate) fn far_leg(b: &Breed) -> Rgb {
    shade(b.coat, -1.1)
}

/// Colour of the far paws in profile.
pub(crate) fn far_paw(b: &Breed) -> Rgb {
    shade(paw_col(b), -0.9)
}

/// One leg in profile, jointed: the front leg runs from the shoulder past the elbow, nearly
/// straight down with a slight bend at the wrist; the hind leg is the seated haunch with
/// the shin going back to the hock, then down. Every leg is as thick as the seated front
/// leg and ends in the seated paw, toes forward. `ph` is the walk phase; walking swings the
/// leg from its top joint and lifts the paw on the way forward.
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
    // the seated width and height, from the seated head this rig carries
    let (ww, hh) = (r.hru.max(4.0) / 0.42, r.hrv / 0.18);
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
            (u0 + 0.25 * l + 0.8, 0.62 * l),
            (u0 - 0.35 * l - 1.0, 0.3 * l),
            (u0 - 0.25 * l - 0.6, 0.0),
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
    let fur = if far { far_leg(b) } else { shade(b.coat, 0.15) };
    let sock = socks(b);
    let lower = move |v: f32| -> Rgb {
        if sock && v < 0.45 * l {
            if far { shade(b.light, -0.9) } else { b.light }
        } else {
            fur
        }
    };
    let lw = seat_lw(ww);
    let leg = |_: f32, v: f32, _: i32, _: i32| ('\u{2588}', lower(v));
    if !front {
        // the seated haunch at the hip, its top at the withers
        let dk = if far { -1.1 } else { -0.5 };
        let (hu, hv) = (q[0].0, (l + d - 0.2 * hh).max(0.2 * hh));
        r.ell(cv, (hu, hv), (0.2 * ww, 0.2 * hh), &|u, v, x, y| {
            let (c, col) = coat(r, u, v, x, y);
            (c, shade(col, dk))
        });
    } else {
        // the upper leg is mostly hidden in the body
        fat_seg(r, cv, q[0], q[1], lw, &leg);
    }
    fat_seg(r, cv, q[1], q[2], lw, &leg);
    fat_seg(r, cv, q[2], q[3], lw, &leg);
    // the seated paw: two rows on the grass, toes forward
    let pc = if far { far_paw(b) } else { paw_col(b) };
    let (pu, pv) = q[3];
    seat_paw(r, cv, (pu + 0.6, pv + 0.5), lw, pc);
    if b.pat == Pat::Poodle {
        // pompoms at the ankle
        let pom = if far { shade(b.light, -0.6) } else { b.light };
        r.ell(
            cv,
            (q[2].0, q[2].1 * 0.6 + 1.0),
            (lw + 1.0, 1.0),
            &|_, _, x, y| (if (x + y) % 2 == 0 { '@' } else { '\u{2588}' }, pom),
        );
    }
}

/// Body in profile: chest deeper than the belly (the belly line rises toward the hind legs),
/// the back sloping a little down to the tail, round at both ends.
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

/// The standing head: the seated head pasted at its seated size, facing the viewer, or the
/// chibi profile on a short neck.
fn draw_stand_head(r: &Rig, cv: &mut Canvas, body: &Body, side: bool) {
    let (w, h) = (cv.w as u16, cv.h as u16);
    if side {
        // neck: from the withers up into the back of the head
        let (nu0, nv0) = (0.5 * r.ru, r.vb + 0.4 * r.rv);
        let (nu1, nv1) = (r.hu - 0.3 * r.hru, r.hv - 0.35 * r.hrv);
        for k in 0..=6 {
            let a = k as f32 / 6.0;
            r.ell(
                cv,
                (nu0 + (nu1 - nu0) * a, nv0 + (nv1 - nv0) * a),
                (0.3 * r.ru, (0.4 * r.rv).max(1.5)),
                &|u, v, x, y| coat(r, u, v.min(r.vb + r.rv), x, y),
            );
        }
    }
    let s = head_sprite(body, w, h, cv.ascii, side, r.f);
    let u = r.hu;
    let r1 = Rig { sq: 1.0, ..*r };
    paste(&r1, cv, &s, (u * r.sq, r.hv), false, -100.0);
}

/// The chibi head in profile, drawn on the seated rig (`hu`, `hv`, `f` set by the caller):
/// the seated round head and colouring, the seated muzzle seen from the side as a short
/// snout poking 1–2 columns past the head and ending in the small dark nose, one seated eye
/// with its white shine, the blush, and one seated ear (floppy or upright, same size).
fn draw_profile_head(r: &Rig, cv: &mut Canvas, body: &Body, hh: f32) {
    let b = r.br;
    let p = &body.dpose;
    let (hu, hv, hru, hrv) = (r.hu, r.hv, r.hru, r.hrv);
    let ear_c = shade(b.dark, -0.8);
    let up_ear = |cv: &mut Canvas, eu: f32, c: Rgb| {
        // the seated pointed ear, leaning back a little
        let eh = (0.17 * hh).max(2.0);
        let n = eh.ceil() as i32;
        for i in 0..n {
            let wd = ((n - i) as f32 / n as f32 * (hru * 0.45).max(1.5)).max(1.0);
            let lean = p.tilt * i as f32 * 0.3 - 0.2 * i as f32;
            r.fill(
                cv,
                (eu - wd + lean, eu + wd + lean),
                (
                    hv + hrv * 0.55 + i as f32 - 0.5,
                    hv + hrv * 0.55 + i as f32 + 0.5,
                ),
                &|u, _, _, _| ((u - eu - lean).abs() <= wd * 0.5 + 0.4).then_some(('\u{2593}', c)),
            );
        }
    };
    // the far ear peeks up behind
    if b.ears_up {
        up_ear(cv, hu + 0.2 * hru, shade(ear_c, -0.6));
    }
    // the seated round head
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
            (hu - 0.1 * hru, hv + hrv * 0.9),
            (hru * 0.6, hrv * 0.5),
            &|_, _, x, y| (if (x + y) % 2 == 0 { '@' } else { '\u{2588}' }, b.light),
        );
    }
    if b.pat == Pat::Dalmatian {
        r.stamp(cv, hu - hru * 0.25, hv + hrv * 0.4, '\u{25cf}', b.dark);
    }
    // the seated muzzle, seen from the side
    let muz = if b.pat == Pat::Shepherd {
        b.dark
    } else {
        b.light
    };
    let (mcu, mcv, mru, mrv) = snout(r);
    let (mu, mv) = (hu + mcu, hv + mcv);
    r.ell(cv, (mu, mv), (mru, mrv), &|_, _, _, _| ('\u{2588}', muz));
    let tip = mu + mru;
    // mouth: a short line under the snout; open, a dark grin with the tongue out
    let mouth = (0x70, 0x20, 0x30);
    let low = mv - mrv + 0.2;
    if p.open > 0.15 || p.tongue > 0.3 {
        for k in 0..3 {
            r.stamp(cv, tip - 1.5 - k as f32, low, '\u{2584}', mouth);
        }
        if p.tongue > 0.3 {
            r.stamp(cv, tip - 2.5, low - 1.0, '\u{2588}', TONGUE);
            r.stamp(cv, tip - 3.5, low - 1.0, '\u{2588}', TONGUE);
        }
    } else if !cv.ascii {
        r.stamp(cv, tip - 2.0, low + 0.6, '\u{2581}', mouth);
    }
    // the small dark nose at the top of the snout tip ('▇' is left alone by the edge pass)
    let nose = if cv.ascii { '#' } else { '\u{2587}' };
    let nv = mv + mrv * 0.55;
    r.stamp(cv, tip - 0.4, nv, nose, b.nose);
    r.stamp(cv, tip - 1.4, nv, nose, b.nose);
    // the seated eye with its white shine, and the blush below and behind it
    let (eu, ev) = (hu + 0.42 * hru, hv + 0.12 * hrv);
    let dark = (0x18, 0x12, 0x14);
    // on the move the eye stays open with its shine (no blink mid-stride)
    let moving = p.walking || p.running;
    if (body.closed || body.blink) && !moving {
        r.stamp(cv, eu - 0.5, ev, '\u{2580}', dark);
        r.stamp(cv, eu + 0.5, ev, '\u{2580}', dark);
    } else {
        let iris = if b.eye.0 as u32 + b.eye.1 as u32 + b.eye.2 as u32 > 200 {
            b.eye
        } else {
            dark
        };
        let (eru, erv) = ((0.11 * hru).max(1.0), (0.13 * hrv).max(1.0));
        r.ell(cv, (eu, ev), (eru, erv), &|_, _, _, _| ('\u{2588}', iris));
        r.stamp(
            cv,
            eu - 0.4 * r.f,
            ev + erv * 0.6,
            '\u{25cf}',
            (255, 255, 255),
        );
    }
    r.stamp(
        cv,
        hu + 0.12 * hru,
        hv - 0.3 * hrv,
        '\u{2592}',
        (0xF0, 0x90, 0xA0),
    );
    // the near ear at the back of the head, the seated size
    if b.ears_up {
        up_ear(cv, hu - 0.3 * hru, ear_c);
    } else if b.pat == Pat::Poodle {
        r.ell(
            cv,
            (hu - 0.6 * hru, hv - hrv * 0.2),
            (hru * 0.32, hrv * 0.8),
            &|_, _, x, y| (if (x + y) % 2 == 0 { '@' } else { '\u{2588}' }, ear_c),
        );
    } else {
        let dach = b.pat == Pat::Dachshund;
        let swing = if p.running { 0.6 * p.step.sin() } else { 0.0 };
        r.ell(
            cv,
            (
                hu - 0.62 * hru - swing,
                hv - hrv * (if dach { 0.9 } else { 0.35 } + p.tilt * 0.2),
            ),
            if dach {
                (hru * 0.34, hrv * 1.6)
            } else {
                (hru * 0.3, hrv * 0.95)
            },
            &|_, _, _, _| ('\u{2593}', ear_c),
        );
    }
    if p.bark > 0.25 {
        for k in 0..3 {
            r.stamp(
                cv,
                tip + 2.0 + 2.2 * k as f32,
                nv - 0.5 + 0.4 * k as f32,
                ')',
                (0xFF, 0xF0, 0xA0),
            );
        }
    }
}

/// Where the standing dog's mouth is (local coordinates).
fn stand_mouth(r: &Rig, p: &crate::dog::DogPose) -> (f32, f32) {
    if profile(p) || p.turn < 0.15 {
        let (mcu, mcv, mru, mrv) = snout(r);
        (r.hu + mcu + mru - 2.0, r.hv + mcv - mrv - 0.3)
    } else {
        (r.hu, r.hv - 0.42 * r.hrv - 0.6)
    }
}

pub fn draw_dog(cv: &mut Canvas, body: &Body) {
    let p = &body.dpose;
    if p.inside {
        draw_inside(cv, body);
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
    draw_stand(cv, body, profile(p) || p.turn < 0.15);
}

/// The standing dog; `side` draws the head in profile, else the seated front face.
fn draw_stand(cv: &mut Canvas, body: &Body, side: bool) {
    let (w, h) = (cv.w as u16, cv.h as u16);
    let p = &body.dpose;
    let mut r = stand_rig(body, w, h);
    if !side {
        // turning to face the viewer: the whole body narrows with the head (three-quarter)
        let q = 1.0 - smooth(0.0, 1.0, p.turn.clamp(0.0, 1.0));
        r.sq = (1.0 - 0.45 * q).max(0.5);
    }
    let r = r;
    let m = r.m;
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
    draw_stand_head(&r, cv, body, side);
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

/// The napping dog: only its head, at the bottom of the door opening (chin on the house
/// floor), eyes closed, body hidden behind the wall.
fn inside_body(body: &Body, w: u16, h: u16) -> Body {
    let mut b2 = body.clone();
    let lift = (horizon(w, h) - ground(w, h).0) as f32;
    b2.dpose = crate::dog::DogPose {
        ..Default::default()
    };
    b2.closed = true;
    b2.blink = false;
    b2.scale = NAP_SCALE;
    let (_, hn) = seat_dims(w, h, NAP_SCALE);
    // the chin sits 0.565 of the seated height above the feet row
    b2.dy = lift + 0.565 * hn - 0.5;
    b2.cx = 0;
    b2.dx = door_x(w, h);
    b2
}

fn draw_inside(cv: &mut Canvas, body: &Body) {
    let (w, h) = (cv.w as u16, cv.h as u16);
    draw_sit(cv, &inside_body(body, w, h));
}

/// Zzz drifting up from the roof top of the doghouse the dog naps in.
fn draw_zzz(cv: &mut Canvas, body: &Body, w: u16, h: u16) {
    let (x0, hw, wall, roof) = house(w, h);
    let base = horizon(w, h) - 1;
    let top = base - wall - roof;
    let cx = x0 + hw / 2;
    for i in 0..3 {
        let k = (body.t / 1.5 + i as f32 * 0.33) % 1.0;
        let ch = ['z', 'Z', 'z'][i as usize];
        cv.put(
            cx + (k * 4.0) as i32 + i * 2,
            top - (k * 4.0) as i32 - i,
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
    let (x0, _, _, _) = house(w, h);
    // walking into the house hides the body behind its walls and roof; asleep inside,
    // only what the door frames shows
    let hide = p.nap > 0.0 || p.inside;
    for y in 0..sc.h {
        for x in 0..sc.w {
            let c = sc.cells[y * sc.w + x];
            if c.fg.is_none() {
                continue;
            }
            // walking into the house: the body vanishes behind its walls and roof
            // (the open sky beside the roof still shows the dog)
            let (xi, yi) = (x as i32, y as i32);
            let covered = house_covers(w, h, xi, yi) || xi < x0 - 1;
            if hide && covered {
                continue;
            }
            // asleep inside: nothing pokes out past the door (tail, paws)
            if p.inside && !in_door(w, h, xi, yi) {
                continue;
            }
            // asleep: everything at about half brightness
            let c = if p.inside {
                Cell {
                    ch: c.ch,
                    fg: c.fg.map(|f| lerp(f, (0, 0, 0), 0.5)),
                }
            } else {
                c
            };
            cv.cells[y * cv.w + x] = c;
            cv.owner[y * cv.w + x] = id;
        }
    }
    if p.inside {
        draw_zzz(cv, body, w, h);
    }
}

/// Where the dog's mouth is (cell coordinates), for biscuits and the ball.
pub fn mouth_pos(app: &App, body: &Body) -> (f32, f32) {
    if body.dpose.inside || body.dpose.seated() {
        let inside = inside_body(body, app.w, app.h);
        let body = if body.dpose.inside { &inside } else { body };
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
    if let Some((pose, k)) = body.dpose.roll_pose() {
        let br = &BREEDS[body.breed.min(BREEDS.len() - 1)];
        let d = lie_dims(app.w, app.h, body.scale, br);
        let body = &roll_body(body, app.w, app.h);
        let r = lie_rig(body, app.w, app.h, &d, d.rv, d.rv);
        let hv = lie_head_v(pose, &d, k);
        let mv = if pose == RollPose::Back {
            hv + 0.42 * d.hrv
        } else {
            hv - 0.42 * d.hrv
        };
        // the profile snout pokes out toward the way the dog faces
        let (x, y) = r.xy(d.hc + 0.8 * d.hru, mv);
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
    // grass in front: tufts over the bottom row of the paws so they sink into the lawn
    let y = feet - 1;
    if y >= 0 && (y as usize) < cv.h {
        for x in 0..cv.w as i32 {
            let i = y as usize * cv.w + x as usize;
            if cv.cells[i].fg.is_some() && cv.cells[i].ch != ' ' && cv.owner[i] != crate::art::NONE
            {
                let k = hash(x + 5, y);
                if k.is_multiple_of(3) {
                    cv.put(x, y, if k.is_multiple_of(2) { '"' } else { ',' }, GRASS);
                }
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
