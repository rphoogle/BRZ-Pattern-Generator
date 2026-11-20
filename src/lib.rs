//! BRZ Pattern Generator core (stage 1: honeycomb).
//!
//! In plain English: the pattern is worked out as a set of "pieces" laid on a
//! honeycomb. Each piece is a short run of walls (and optionally a spoke) that
//! all point the same way. Pieces at 0° go on the normal brick grid; pieces at
//! any other angle go on an entity (a separate brick grid) that is turned to
//! that angle and held by a hidden Micro Rigid Bearing on the main grid.
//! Positions on a turned entity can't always land on whole units, so each piece
//! is nudged to the nearest whole unit (by under ~0.6 units) as one solid unit.
//!
//! Units: Brickadia brick units. 1 micro brick = 2 units, 1 stud = 10 units.

use brdb::{
    assets::LiteralComponent, AsBrdbValue, Brick, BrickSize, BrickType, Direction, Entity, Position,
    Quat4f, Rotation, Vector3f, World,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BEARING: &str = "B_Joint_RigidBearing_Micro";
pub const MICRO_BRICK: &str = "PB_DefaultMicroBrick";
pub const MICRO_WEDGE: &str = "PB_DefaultMicroWedge";

pub const MATERIALS: [(&str, &str); 6] = [
    ("Plastic", "BMC_Plastic"),
    ("Glass", "BMC_Glass"),
    ("Translucent", "BMC_TranslucentPlastic"),
    ("Glow", "BMC_Glow"),
    ("Metallic", "BMC_Metallic"),
    ("Hologram", "BMC_Hologram"),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WallStyle { MicroBrick, MicroWedge }
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpokeMode { None, EveryCell, AltRows, Custom }
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Area { Hexagon, Rectangle, Circle, Oval }
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WayUp { Upright, UpsideDown }
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Turn { WallsUpright, WallsAcross }

/// Every setting, in brick units (2 units = 1 micro brick).
#[derive(Clone, Debug)]
pub struct Settings {
    pub name: String,
    /// Cell size, flat side to flat side.
    pub cell: i32,
    /// Row spacing as a percentage of a perfect honeycomb (stretch V); 100 = perfect.
    pub stretch_v: f64,
    /// Column spacing percentage (stretch H).
    pub stretch_h: f64,
    pub turn: Turn,
    pub style: WallStyle,
    pub wall_height: i32,
    /// Micro brick style only.
    pub wall_thickness: i32,
    /// Use the half-turned version of every piece (wedge slopes face the other way).
    pub flip_wedges: bool,
    pub spokes: SpokeMode,
    /// Spoke directions 0°, 60°, 120° (relative to the pattern).
    pub spoke_dirs: [bool; 3],
    pub spoke_every: i32,
    pub spoke_rows: i32,
    pub spoke_offset: i32,
    pub spoke_stagger: bool,
    /// 0 = wall to wall.
    pub spoke_length: i32,
    pub spoke_thickness: i32,
    pub spoke_top: bool,
    pub area: Area,
    pub width: i32,
    pub length: i32,
    pub way_up: WayUp,
    pub color: [u8; 3],
    pub material: usize,
    pub intensity: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            name: "Honeycomb".into(),
            cell: 20,
            stretch_v: 100.0,
            stretch_h: 100.0,
            turn: Turn::WallsUpright,
            style: WallStyle::MicroWedge,
            wall_height: 30,
            wall_thickness: 2,
            flip_wedges: false,
            spokes: SpokeMode::Custom,
            spoke_dirs: [true; 3],
            spoke_every: 2,
            spoke_rows: 2,
            spoke_offset: 0,
            spoke_stagger: true,
            spoke_length: 0,
            spoke_thickness: 2,
            spoke_top: false,
            area: Area::Hexagon,
            width: 400,
            length: 400,
            way_up: WayUp::Upright,
            color: [0xDB, 0xBE, 0xC0],
            material: 0,
            intensity: 0,
        }
    }
}

/// One brick before it is placed in a grid: size, local position, orientation byte.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct B {
    pub wedge: bool,
    pub size: (i32, i32, i32),
    pub pos: (i32, i32, i32),
    /// direction * 4 + rotation (as in the save file).
    pub ori: u8,
}

/// A grid of bricks: the main grid (yaw None) or an entity at `yaw` degrees.
#[derive(Clone, Debug)]
pub struct Grid {
    pub yaw: Option<f64>,
    pub bricks: Vec<B>,
    /// Entity origin in world units (bearing mounting face). Main grid: zero.
    pub origin: (i32, i32, i32),
    /// Main-grid position of this entity's bearing.
    pub bearing: (i32, i32, i32),
    /// Bearing angle (degrees).
    pub angle: f64,
    pub max_move: f64,
}

#[derive(Clone, Debug)]
pub struct Built {
    pub main: Vec<B>,
    pub entities: Vec<Grid>,
    pub cells: usize,
    pub max_move: f64,
    pub angles: Vec<f64>,
}

impl Built {
    pub fn brick_count(&self) -> usize {
        self.main.len() + self.entities.iter().map(|g| g.bricks.len() + 1).sum::<usize>()
    }
}

// ---------- orientation helpers ----------

/// Half extents in grid axes (same rule as the game / brdb).
pub fn half(b: &B) -> (i32, i32, i32) {
    let (sx, sy, sz) = b.size;
    let dir = b.ori >> 2;
    let rot = b.ori & 3;
    let (a, bb) = if rot & 1 == 1 { (sy, sx) } else { (sx, sy) };
    match dir {
        0 | 1 => (sz, bb, a),
        2 | 3 => (bb, sz, a),
        _ => (a, bb, sz),
    }
}

/// Half turn about the vertical axis (confirmed in game).
fn rot180(b: B) -> B {
    let dir = b.ori >> 2;
    let rot = b.ori & 3;
    let ori = if dir < 4 { ((dir ^ 1) << 2) | rot } else { (dir << 2) | ((rot + 2) & 3) };
    B { pos: (-b.pos.0, -b.pos.1, b.pos.2), ori, ..b }
}

/// Upside down: half turn about the X axis (matches the user's upside-down honeycomb).
fn flip_x(b: B) -> B {
    let dir = b.ori >> 2;
    let rot = b.ori & 3;
    let ori = match dir {
        0 | 1 => (dir << 2) | ((rot + 2) & 3),
        2 | 3 => ((dir ^ 1) << 2) | ((rot + 2) & 3),
        _ => ((dir ^ 1) << 2) | rot,
    };
    B { pos: (b.pos.0, -b.pos.1, -b.pos.2), ori, ..b }
}

// ---------- the piece template (taken from the user's upright honeycomb) ----------

/// Bricks of one wall running along local Y, centred at (cx, cy). `plus` picks
/// which way the wedge pair faces.
fn wall(s: &Settings, cx: i32, cy: i32, zb: i32, lh: i32, plus: bool, out: &mut Vec<B>) {
    let hh = s.wall_height / 2;
    let zc = zb + hh;
    match s.style {
        WallStyle::MicroWedge => {
            let (a, b) = if plus { ((cx + 1, 8u8), (cx - 1, 11u8)) } else { ((cx - 1, 12u8), (cx + 1, 15u8)) };
            out.push(B { wedge: true, size: (hh, 1, lh), pos: (a.0, cy, zc), ori: a.1 });
            out.push(B { wedge: true, size: (1, hh, lh), pos: (b.0, cy, zc), ori: b.1 });
        }
        WallStyle::MicroBrick => {
            out.push(B { wedge: false, size: (s.wall_thickness / 2, lh, hh), pos: (cx, cy, zc), ori: 16 });
        }
    }
}

fn spoke_any(s: &Settings) -> bool {
    s.spokes != SpokeMode::None && s.spoke_dirs.iter().any(|d| *d)
}

fn wall_base(s: &Settings) -> i32 {
    if spoke_any(s) && !s.spoke_top { s.spoke_thickness } else { 0 }
}

/// The four walls of one piece (strip direction = local X).
fn piece_walls(s: &Settings, g: &Geo) -> Vec<B> {
    let mut v = Vec::new();
    let zb = wall_base(s);
    let lh = g.wall_half;
    wall(s, g.side_x, 0, zb, lh, true, &mut v);
    wall(s, -g.side_x, 0, zb, lh, false, &mut v);
    wall(s, 0, g.leg_y, zb, lh, false, &mut v);
    wall(s, 0, -g.leg_y, zb, lh, false, &mut v);
    if s.flip_wedges {
        v = v.into_iter().map(rot180).collect();
    }
    v
}

/// One spoke along local X. `trim_fwd` / `trim_back`: a neighbour also has a
/// spoke that way, so stop at the wall's centre line instead of reaching into it.
fn spoke_brick(s: &Settings, g: &Geo, trim_fwd: bool, trim_back: bool) -> B {
    let t = (s.spoke_thickness / 2).max(1);
    let z = if s.spoke_top { wall_base(s) + s.wall_height + t } else { t };
    let (back, fwd) = if s.spoke_length > 0 {
        let h = (s.spoke_length / 2).max(1).min(g.side_x);
        (h, h)
    } else {
        (if trim_back { g.side_x } else { g.side_x + 1 }, if trim_fwd { g.side_x } else { g.side_x + 1 })
    };
    // Length back+fwd, centred at (fwd-back)/2. A whole-unit centre needs an even total.
    let mut total = back + fwd;
    let mut fwd = fwd;
    if (fwd - back) % 2 != 0 {
        fwd -= 1;
        total -= 1;
    }
    let cx = (fwd - back) / 2;
    // size (t, half, t) with rotation 3 runs along local X (as in the user's file).
    B { wedge: false, size: (t, total / 2, t), pos: (cx, 0, z), ori: 19 }
}

// ---------- honeycomb geometry ----------

struct Geo {
    /// Distance between cell centres along a row.
    col: f64,
    /// Distance between rows.
    row: f64,
    side_x: i32,
    leg_y: i32,
    wall_half: i32,
    across: bool,
}

fn geo(s: &Settings) -> Geo {
    let f = s.cell as f64;
    let side = f / 3f64.sqrt();
    Geo {
        col: f * s.stretch_h / 100.0,
        row: 1.5 * side * s.stretch_v / 100.0,
        side_x: s.cell / 2,
        leg_y: (1.5 * side).round() as i32,
        wall_half: ((side / 2.0).round() as i32).max(1),
        across: s.turn == Turn::WallsAcross,
    }
}

fn cell_xy(g: &Geo, q: i32, r: i32) -> (f64, f64) {
    let x = q as f64 * g.col + if r.rem_euclid(2) == 1 { g.col / 2.0 } else { 0.0 };
    let y = r as f64 * g.row;
    if g.across { (-y, x) } else { (x, y) }
}

/// The neighbouring cell along spoke direction k (0°, 60°, 120° in lattice terms).
fn neighbour(q: i32, r: i32, k: usize) -> (i32, i32) {
    let odd = r.rem_euclid(2) == 1;
    match k {
        0 => (q + 1, r),
        1 => (q + if odd { 1 } else { 0 }, r + 1),
        _ => (q - if odd { 0 } else { 1 }, r + 1),
    }
}

fn neighbour_back(q: i32, r: i32, k: usize) -> (i32, i32) {
    let odd = r.rem_euclid(2) == 1;
    match k {
        0 => (q - 1, r),
        1 => (q - if odd { 0 } else { 1 }, r - 1),
        _ => (q + if odd { 1 } else { 0 }, r - 1),
    }
}

/// Star cells: every other cell in every other row, staggered. Each one carries
/// three pieces, which together cover every wall of the honeycomb.
fn is_unit(q: i32, r: i32) -> bool {
    r.rem_euclid(2) == 0 && (q - r.div_euclid(2)).rem_euclid(2) == 0
}

fn in_area(s: &Settings, x: f64, y: f64) -> bool {
    let w = s.width as f64 / 2.0;
    let l = s.length as f64 / 2.0;
    match s.area {
        Area::Rectangle => x.abs() <= w && y.abs() <= l,
        Area::Circle => x.hypot(y) <= w,
        Area::Oval => (x / w).powi(2) + (y / l).powi(2) <= 1.0,
        Area::Hexagon => x.abs() <= w && x.abs() / 2.0 + y.abs() * 3f64.sqrt() / 2.0 <= w,
    }
}

fn spoke_here(s: &Settings, q: i32, r: i32) -> bool {
    match s.spokes {
        SpokeMode::None => false,
        SpokeMode::EveryCell => true,
        SpokeMode::AltRows => r.rem_euclid(2) == 0,
        SpokeMode::Custom => {
            let m = s.spoke_rows.max(1);
            let n = s.spoke_every.max(1);
            if r.rem_euclid(m) != 0 {
                return false;
            }
            let shift = if s.spoke_stagger { r.div_euclid(m) } else { 0 };
            (q - s.spoke_offset - shift).rem_euclid(n) == 0
        }
    }
}

fn norm(a: f64) -> f64 {
    let mut a = (a + 180.0).rem_euclid(360.0) - 180.0;
    if a == -180.0 {
        a = 180.0;
    }
    if a.abs() < 1e-9 {
        a = 0.0;
    }
    a
}

/// Entity rotation as a 3x3 matrix (columns = local axes in world).
pub(crate) fn ent_matrix(yaw: f64, upside: bool) -> [[f64; 3]; 3] {
    let (c, s) = (yaw.to_radians().cos(), yaw.to_radians().sin());
    if upside {
        // Rz(yaw) * Rx(180)
        [[c, s, 0.0], [s, -c, 0.0], [0.0, 0.0, -1.0]]
    } else {
        [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
    }
}

fn ent_quat(yaw: f64, upside: bool) -> Quat4f {
    let h = yaw.to_radians() / 2.0;
    if upside {
        Quat4f { x: h.cos() as f32, y: h.sin() as f32, z: 0.0, w: 0.0 }
    } else {
        Quat4f { x: 0.0, y: 0.0, z: h.sin() as f32, w: h.cos() as f32 }
    }
}

pub(crate) fn mul(m: &[[f64; 3]; 3], v: (f64, f64, f64)) -> (f64, f64, f64) {
    (
        m[0][0] * v.0 + m[0][1] * v.1 + m[0][2] * v.2,
        m[1][0] * v.0 + m[1][1] * v.1 + m[1][2] * v.2,
        m[2][0] * v.0 + m[2][1] * v.1 + m[2][2] * v.2,
    )
}
fn mul_t(m: &[[f64; 3]; 3], v: (f64, f64, f64)) -> (f64, f64, f64) {
    (
        m[0][0] * v.0 + m[1][0] * v.1 + m[2][0] * v.2,
        m[0][1] * v.0 + m[1][1] * v.1 + m[2][1] * v.2,
        m[0][2] * v.0 + m[1][2] * v.1 + m[2][2] * v.2,
    )
}

pub fn validate(s: &Settings) -> Result<(), String> {
    let even = |v: i32, what: &str| if v % 2 != 0 || v <= 0 { Err(format!("{what} must be a whole number of micro bricks")) } else { Ok(()) };
    even(s.cell, "Cell size")?;
    even(s.wall_height, "Wall height")?;
    even(s.wall_thickness, "Wall thickness")?;
    even(s.spoke_thickness, "Spoke thickness")?;
    if s.cell < 8 {
        return Err("Cell size must be at least 4 micro bricks".into());
    }
    if s.width < s.cell || s.length < s.cell {
        return Err("The area must be at least one cell wide".into());
    }
    if !(25.0..=400.0).contains(&s.stretch_h) || !(25.0..=400.0).contains(&s.stretch_v) {
        return Err("Stretch must be between 25% and 400%".into());
    }
    if s.wall_height > 2000 {
        return Err("Wall height is too tall for one brick".into());
    }
    if s.material >= MATERIALS.len() || s.intensity > 10 {
        return Err("Unknown material or intensity".into());
    }
    Ok(())
}

/// Lay out the whole pattern.
pub fn build(s: &Settings) -> Result<Built, String> {
    validate(s)?;
    if (s.stretch_h - 100.0).abs() > 1e-9 || (s.stretch_v - 100.0).abs() > 1e-9 {
        // Stretch arrives in stage 3 (angles change, so pieces must be recut).
        return Err("Stretch is not available yet (coming in stage 3)".into());
    }
    let g = geo(s);
    let upside = s.way_up == WayUp::UpsideDown;
    let turn = if s.turn == Turn::WallsAcross { 90.0 } else { 0.0 };
    let dirs = [turn, turn + 60.0, turn + 120.0];

    // Pieces in world: (direction index, centre x, y, walls?, spoke?)
    let span_q = (s.width.max(s.length) as f64 / g.col).ceil() as i32 + 3;
    let span_r = (s.width.max(s.length) as f64 / g.row).ceil() as i32 + 3;
    let mut pieces: Vec<(usize, f64, f64, bool, bool, bool, bool)> = Vec::new();
    let mut cells = 0;
    let mut units = std::collections::HashSet::new();
    for r in -span_r..=span_r {
        for q in -span_q..=span_q {
            if is_unit(q, r) {
                let (x, y) = cell_xy(&g, q, r);
                if in_area(s, x, y) {
                    units.insert((q, r));
                }
            }
        }
    }
    for r in -span_r..=span_r {
        for q in -span_q..=span_q {
            let (x, y) = cell_xy(&g, q, r);
            let unit = units.contains(&(q, r));
            // A cell counts if it is a unit or touches one (each unit covers 4 cells' worth of walls).
            let near_unit = unit
                || [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)]
                    .iter()
                    .any(|(dq, dr)| units.contains(&(q + dq, r + dr)));
            if !near_unit && !in_area(s, x, y) {
                continue;
            }
            if unit {
                cells += 4;
            }
            let has_spoke = |q: i32, r: i32| -> bool {
                let (x, y) = cell_xy(&g, q, r);
                let nu = units.contains(&(q, r))
                    || [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)]
                        .iter()
                        .any(|(dq, dr)| units.contains(&(q + dq, r + dr)));
                in_area(s, x, y) && nu && spoke_here(s, q, r)
            };
            let spoke = has_spoke(q, r);
            for k in 0..3 {
                let sp = spoke && s.spoke_dirs[k];
                if unit || sp {
                    let (fq, fr) = neighbour(q, r, k);
                    let (bq, br) = neighbour_back(q, r, k);
                    let tf = sp && has_spoke(fq, fr);
                    let tb = sp && has_spoke(bq, br);
                    pieces.push((k, x, y, unit, sp, tf, tb));
                }
            }
        }
    }
    if pieces.is_empty() {
        return Err("The area is too small for one cell".into());
    }

    // Main grid: pieces whose direction is 0° (only when walls are upright).
    let main_k: Option<usize> = if s.turn == Turn::WallsUpright { Some(0) } else { None };
    let mut main: Vec<B> = Vec::new();
    let walls_t = piece_walls(s, &g);
    let mut main_move: f64 = 0.0;
    let piece_bricks = |walls: bool, spoke: bool, tf: bool, tb: bool| -> Vec<B> {
        let mut v = Vec::new();
        if walls {
            v.extend(walls_t.iter().copied());
        }
        if spoke {
            v.push(spoke_brick(s, &g, tf, tb));
        }
        v
    };
    for &(k, x, y, w, sp, tf, tb) in &pieces {
        if Some(k) != main_k {
            continue;
        }
        let (cx, cy) = (x.round() as i32, y.round() as i32);
        main_move = main_move.max((x - cx as f64).hypot(y - cy as f64));
        for mut b in piece_bricks(w, sp, tf, tb) {
            if upside {
                b = flip_x(b);
            }
            b.pos = (b.pos.0 + cx, b.pos.1 + cy, b.pos.2);
            main.push(b);
        }
    }

    // Base-layer footprint of everything, for finding free bearing spots.
    let mut taken: Vec<((f64, f64), (f64, f64))> = Vec::new();
    let base_band = |b: &B| -> bool {
        let h = half(b);
        let z0 = b.pos.2 - h.2;
        let z1 = b.pos.2 + h.2;
        if upside { z0 < 0 && z1 > -2 } else { z0 < 2 && z1 > 0 }
    };
    for b in &main {
        if base_band(b) {
            let h = half(b);
            taken.push((((b.pos.0 - h.0) as f64, (b.pos.1 - h.1) as f64), ((b.pos.0 + h.0) as f64, (b.pos.1 + h.1) as f64)));
        }
    }
    for &(k, x, y, w, sp, tf, tb) in &pieces {
        if Some(k) == main_k {
            continue;
        }
        let m = ent_matrix(dirs[k], upside);
        for b in piece_bricks(w, sp, tf, tb) {
            let h = half(&b);
            let zc = if upside { -(b.pos.2 as f64) } else { b.pos.2 as f64 };
            let (z0, z1) = (zc - h.2 as f64, zc + h.2 as f64);
            let in_band = if upside { z0 < 0.0 && z1 > -2.0 } else { z0 < 2.0 && z1 > 0.0 };
            if !in_band {
                continue;
            }
            let c = mul(&m, (b.pos.0 as f64, b.pos.1 as f64, 0.0));
            let e0 = m[0][0].abs() * h.0 as f64 + m[0][1].abs() * h.1 as f64 + 0.7;
            let e1 = m[1][0].abs() * h.0 as f64 + m[1][1].abs() * h.1 as f64 + 0.7;
            taken.push(((x + c.0 - e0, y + c.1 - e1), (x + c.0 + e0, y + c.1 + e1)));
        }
    }

    // Centre of the layout, to put bearings near the middle.
    let n = pieces.len() as f64;
    let mx = pieces.iter().map(|p| p.1).sum::<f64>() / n;
    let my = pieces.iter().map(|p| p.2).sum::<f64>() / n;

    let ent_ks: Vec<usize> = (0..3).filter(|k| Some(*k) != main_k).collect();
    let mut entities = Vec::new();
    let mut max_move = main_move;
    let mut bearings: Vec<(i32, i32)> = Vec::new();
    for &k in &ent_ks {
        let yaw = norm(dirs[k]);
        let m = ent_matrix(yaw, upside);
        let mine: Vec<&(usize, f64, f64, bool, bool, bool, bool)> = pieces.iter().filter(|p| p.0 == k).collect();
        if mine.is_empty() {
            continue;
        }
        // Find a free spot for the bearing, spiralling out from the middle.
        let mut found = None;
        'search: for rad in 0..200i32 {
            for dx in -rad..=rad {
                for dy in -rad..=rad {
                    if dx.abs() != rad && dy.abs() != rad {
                        continue;
                    }
                    let (bx, by) = (mx.round() as i32 + dx, my.round() as i32 + dy);
                    let lo = ((bx - 1) as f64, (by - 1) as f64);
                    let hi = ((bx + 1) as f64, (by + 1) as f64);
                    let clash = taken.iter().any(|(a, b)| hi.0 > a.0 && lo.0 < b.0 && hi.1 > a.1 && lo.1 < b.1)
                        || bearings.iter().any(|(ox, oy)| (ox - bx).abs() < 3 && (oy - by).abs() < 3);
                    if !clash {
                        found = Some((bx, by));
                        break 'search;
                    }
                }
            }
        }
        let (bx, by) = found.ok_or("No free spot for a bearing")?;
        bearings.push((bx, by));
        let bz = if upside { -1 } else { 1 };
        let origin = (bx, by, 0);
        let mut bricks = Vec::new();
        let mut mv: f64 = 0.0;
        for &&(_, x, y, w, sp, tf, tb) in &mine {
            let t = mul_t(&m, (x - origin.0 as f64, y - origin.1 as f64, 0.0));
            let (tx, ty) = (t.0.round() as i32, t.1.round() as i32);
            mv = mv.max((t.0 - tx as f64).hypot(t.1 - ty as f64));
            for mut b in piece_bricks(w, sp, tf, tb) {
                b.pos = (b.pos.0 + tx, b.pos.1 + ty, b.pos.2);
                bricks.push(b);
            }
        }
        max_move = max_move.max(mv);
        let angle = if upside { norm(yaw - 180.0) } else { norm(-yaw) };
        entities.push(Grid { yaw: Some(yaw), bricks, origin, bearing: (bx, by, bz), angle, max_move: mv });
    }
    let angles = (0..3).map(|k| norm(dirs[k]).rem_euclid(180.0)).collect();
    Ok(Built { main, entities, cells, max_move, angles })
}

/// World-space bounding box of everything (entities as turned boxes).
pub fn world_bounds(built: &Built, upside: bool) -> ((f64, f64, f64), (f64, f64, f64)) {
    let mut lo = (f64::MAX, f64::MAX, f64::MAX);
    let mut hi = (f64::MIN, f64::MIN, f64::MIN);
    let mut add = |c: (f64, f64, f64), e: (f64, f64, f64)| {
        lo = (lo.0.min(c.0 - e.0), lo.1.min(c.1 - e.1), lo.2.min(c.2 - e.2));
        hi = (hi.0.max(c.0 + e.0), hi.1.max(c.1 + e.1), hi.2.max(c.2 + e.2));
    };
    for b in &built.main {
        let h = half(b);
        add((b.pos.0 as f64, b.pos.1 as f64, b.pos.2 as f64), (h.0 as f64, h.1 as f64, h.2 as f64));
    }
    for g in &built.entities {
        add((g.bearing.0 as f64, g.bearing.1 as f64, g.bearing.2 as f64), (1.0, 1.0, 1.0));
        let m = ent_matrix(g.yaw.unwrap(), upside);
        for b in &g.bricks {
            let h = half(b);
            let c = mul(&m, (b.pos.0 as f64, b.pos.1 as f64, b.pos.2 as f64));
            let e = (
                m[0][0].abs() * h.0 as f64 + m[0][1].abs() * h.1 as f64 + m[0][2].abs() * h.2 as f64,
                m[1][0].abs() * h.0 as f64 + m[1][1].abs() * h.1 as f64 + m[1][2].abs() * h.2 as f64,
                m[2][0].abs() * h.0 as f64 + m[2][1].abs() * h.1 as f64 + m[2][2].abs() * h.2 as f64,
            );
            add((c.0 + g.origin.0 as f64, c.1 + g.origin.1 as f64, c.2 + g.origin.2 as f64), e);
        }
    }
    (lo, hi)
}

fn to_brick(s: &Settings, b: &B) -> Brick {
    let dir = [Direction::XPositive, Direction::XNegative, Direction::YPositive, Direction::YNegative, Direction::ZPositive, Direction::ZNegative][(b.ori >> 2) as usize];
    let rot = [Rotation::Deg0, Rotation::Deg90, Rotation::Deg180, Rotation::Deg270][(b.ori & 3) as usize];
    Brick {
        asset: BrickType::Procedural {
            asset: brdb::BString::str(if b.wedge { MICRO_WEDGE } else { MICRO_BRICK }),
            size: BrickSize { x: b.size.0 as u16, y: b.size.1 as u16, z: b.size.2 as u16 },
        },
        position: (b.pos.0, b.pos.1, b.pos.2).into(),
        direction: dir,
        rotation: rot,
        color: (s.color[0], s.color[1], s.color[2]).into(),
        material: brdb::BString::from(MATERIALS[s.material].1.to_string()),
        material_intensity: s.intensity,
        ..Default::default()
    }
}

/// Build the prefab world: main grid, entities, bearings and joints, centred.
pub fn to_world(s: &Settings, built: &Built) -> World {
    let upside = s.way_up == WayUp::UpsideDown;
    let (lo, hi) = world_bounds(built, upside);
    // Centre on whole units, like the game's own copies.
    let c = (((lo.0 + hi.0) / 2.0).round() as i32, ((lo.1 + hi.1) / 2.0).round() as i32, ((lo.2 + hi.2) / 2.0).round() as i32);
    let mut w = World::new();
    w.register_all_components();
    for b in &built.main {
        let mut br = to_brick(s, b);
        br.position = Position::new(b.pos.0 - c.0, b.pos.1 - c.1, b.pos.2 - c.2);
        w.add_brick(br);
    }
    for g in &built.entities {
        let (bearing, bearing_id) = Brick {
            asset: BrickType::Basic(brdb::BString::str(BEARING)),
            position: Position::new(g.bearing.0 - c.0, g.bearing.1 - c.1, g.bearing.2 - c.2),
            direction: if upside { Direction::ZNegative } else { Direction::ZPositive },
            rotation: Rotation::Deg0,
            color: (s.color[0], s.color[1], s.color[2]).into(),
            material: brdb::BString::from(MATERIALS[s.material].1.to_string()),
            material_intensity: s.intensity,
            ..Default::default()
        }
        .with_component(LiteralComponent::new("Component_Internal_RigidBearing").with_data([
            ("TargetAngle", Box::new(g.angle as f32) as Box<dyn AsBrdbValue>),
            ("bAnglesArePercentages", Box::new(false)),
            ("bReversed", Box::new(false)),
            ("CurrentAngle", Box::new(g.angle as f32)),
            ("InterpMode", Box::new(2u8)),
            ("SmoothTime", Box::new(0.25f32)),
            ("TopSpeed", Box::new(0.0f32)),
        ]))
        .with_id_split();
        w.add_brick(bearing);
        let id = Brick::next_id();
        w.add_brick_grid(
            Entity {
                id: Some(id),
                frozen: true,
                location: Vector3f {
                    x: (g.origin.0 - c.0) as f32,
                    y: (g.origin.1 - c.1) as f32,
                    z: (g.origin.2 - c.2) as f32,
                },
                rotation: ent_quat(g.yaw.unwrap(), upside),
                ..Default::default()
            },
            g.bricks.iter().map(|b| to_brick(s, b)),
        );
        w.add_joint(bearing_id, id, Vector3f::default(), Quat4f { x: 0.0, y: 0.0, z: 0.0, w: 1.0 });
    }
    w.meta.bundle.name = s.name.clone();
    w.meta.bundle.description = format!("Made with BRZ Pattern Generator v{VERSION}");
    w.meta.bundle.game_version = "CL16507".into();
    w.make_prefab();
    // Prefab size from everything, entities included (make_prefab only sees the main grid).
    // Centre 0 with whole-number half sizes, like the game's own copies.
    let hx = (lo.0 - c.0 as f64).abs().max(hi.0 - c.0 as f64).ceil() as i32;
    let hy = (lo.1 - c.1 as f64).abs().max(hi.1 - c.1 as f64).ceil() as i32;
    let hz = (lo.2 - c.2 as f64).abs().max(hi.2 - c.2 as f64).ceil() as i32;
    let lo_i = Position::new(-hx, -hy, -hz);
    let hi_i = Position::new(hx, hy, hz);
    let mut pj = brdb::PrefabJson::from_bounds(lo_i, hi_i);
    if let Some(old) = &w.meta.prefab {
        pj.added_global_grid_offset = old.added_global_grid_offset.clone();
    }
    w.meta.prefab = Some(pj);
    w
}

pub fn to_brz_bytes(s: &Settings) -> Result<(Built, Vec<u8>), String> {
    let built = build(s)?;
    let w = to_world(s, &built);
    let bytes = w.to_brz_vec().map_err(|e| e.to_string())?;
    Ok((built, bytes))
}

pub fn write_brz(s: &Settings, path: &std::path::Path) -> Result<Built, String> {
    let (built, bytes) = to_brz_bytes(s)?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())?;
    Ok(built)
}

pub mod check;
