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
pub const POLE: &str = "PB_DefaultPole";

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
/// Which pattern to draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pattern { Honeycomb, Triangles, Stars, Cubes, Octagons, Diamonds }

impl Pattern {
    pub const ALL: [Pattern; 6] = [Pattern::Honeycomb, Pattern::Triangles, Pattern::Stars, Pattern::Cubes, Pattern::Octagons, Pattern::Diamonds];
    pub fn label(self) -> &'static str {
        match self {
            Pattern::Honeycomb => "Honeycomb",
            Pattern::Triangles => "Triangles",
            Pattern::Stars => "Star of David",
            Pattern::Cubes => "Cube illusion",
            Pattern::Octagons => "Octagons and squares",
            Pattern::Diamonds => "Diamonds",
        }
    }
    /// Short name for file names.
    pub fn short(self) -> &'static str {
        match self {
            Pattern::Honeycomb => "Honeycomb",
            Pattern::Triangles => "Triangles",
            Pattern::Stars => "Stars",
            Pattern::Cubes => "Cubes",
            Pattern::Octagons => "Octagons",
            Pattern::Diamonds => "Diamonds",
        }
    }
}

/// Every setting, in brick units (2 units = 1 micro brick).
#[derive(Clone, Debug)]
pub struct Settings {
    pub name: String,
    pub pattern: Pattern,
    /// Cell size, flat side to flat side.
    pub cell: i32,
    /// Row spacing as a percentage of a perfect honeycomb (stretch V); 100 = perfect.
    pub stretch_v: f64,
    /// Column spacing percentage (stretch H).
    pub stretch_h: f64,
    pub turn: Turn,
    pub style: WallStyle,
    pub wall_height: i32,
    /// Micro brick style.
    pub wall_thickness: i32,
    /// Micro wedge style: thickness of the wedge pair (each wedge is half), multiple of 4 units.
    pub wedge_thickness: i32,
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
    /// Corner posts: width in units (0 = off). Walls are shortened to end at the posts.
    pub corner_posts: i32,
    /// Round pillars (pole bricks) instead of square ones.
    pub posts_round: bool,
    /// Entity walls keep their full length and run through the pillars.
    pub walls_through: bool,
    /// Pillars taller (+) or shorter (-) than the walls, in units.
    pub posts_extra: i32,
    /// Try many bearing spots and keep the one whose rounding makes corners meet best.
    pub best_bearings: bool,
    pub color: [u8; 3],
    pub material: usize,
    pub intensity: u8,
    /// One colour for everything (the three fields above). Off: `paints` per part.
    pub one_paint: bool,
    pub paints: [Paint; 5],
    /// Pillars stick out equally above and below the walls (else all extra height goes on top).
    pub posts_centred: bool,
    /// What to save / copy.
    pub include_main: bool,
    pub include_entities: bool,
    pub include_bearings: bool,
    /// Bearings invisible and without collision (they still hold their entity).
    pub hide_bearings: bool,
    /// Put the main-grid walls on their own 0° entity too (so they can run through pillars).
    pub main_on_entity: bool,
}

impl Settings {
    /// Paint for a part (0 main grid, 1-2 entities, 3 pillars, 4 bearings).
    pub fn paint(&self, part: usize) -> Paint {
        if self.one_paint {
            Paint { color: self.color, material: self.material, intensity: self.intensity }
        } else {
            self.paints[part.min(4)]
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            name: "Honeycomb".into(),
            pattern: Pattern::Honeycomb,
            cell: 20,
            stretch_v: 100.0,
            stretch_h: 100.0,
            turn: Turn::WallsUpright,
            style: WallStyle::MicroWedge,
            wall_height: 30,
            wall_thickness: 2,
            wedge_thickness: 4,
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
            corner_posts: 0,
            posts_round: true,
            walls_through: true,
            posts_extra: 0,
            best_bearings: true,
            color: [0xFF, 0xFF, 0xFF],
            material: 0,
            intensity: 0,
            one_paint: true,
            paints: [Paint { color: [0xFF, 0xFF, 0xFF], material: 0, intensity: 0 }; 5],
            posts_centred: true,
            include_main: true,
            include_entities: true,
            include_bearings: true,
            hide_bearings: true,
            main_on_entity: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind { Micro, Wedge, Pole }

/// Colour, material and intensity for one part of the build.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paint {
    pub color: [u8; 3],
    pub material: usize,
    pub intensity: u8,
}

/// The parts that can have their own paint.
pub const PARTS: [&str; 5] = ["Main grid", "Entity 1", "Entity 2", "Pillars", "Bearings"];

/// One brick before it is placed in a grid: size, local position, orientation byte.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct B {
    pub kind: Kind,
    /// Part of a corner pillar (for its own colour).
    pub pillar: bool,
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
    /// This entity holds pillars (not walls).
    pub pillars: bool,
    /// Which paint it uses: 0 main grid walls, 1 entity 1, 2 entity 2, 3 pillars.
    pub part: usize,
}

#[derive(Clone, Debug)]
pub struct Built {
    pub main: Vec<B>,
    pub entities: Vec<Grid>,
    pub cells: usize,
    pub max_move: f64,
    pub angles: Vec<f64>,
    /// How far wall ends miss at corners (units): average and worst.
    pub corner_mean: f64,
    pub corner_worst: f64,
    /// Pillars were too wide for the main grid: how many entities hold them (0 = on the main grid).
    pub posts_entity: usize,
    /// Extra entities added so thick walls can overlap (0 = none needed).
    pub layers: usize,
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

/// Quarter turn (anticlockwise from above) about the vertical axis. Worked out
/// from the user's two piece layouts: the facing direction turns, the rotation
/// number stays (box-shaped bricks just swap their sideways sizes).
fn rot90(b: B) -> B {
    let dir = b.ori >> 2;
    let rot = b.ori & 3;
    let ori = match dir {
        0 => (2 << 2) | rot,
        2 => (1 << 2) | rot,
        1 => (3 << 2) | rot,
        3 => rot,
        _ => (dir << 2) | ((rot + 1) & 3),
    };
    B { pos: (-b.pos.1, b.pos.0, b.pos.2), ori, ..b }
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
            let hw = (s.wedge_thickness / 4).max(1);
            let (a, b) = if plus { ((cx + hw, 8u8), (cx - hw, 11u8)) } else { ((cx - hw, 12u8), (cx + hw, 15u8)) };
            out.push(B { pillar: false, kind: Kind::Wedge, size: (hh, hw, lh), pos: (a.0, cy, zc), ori: a.1 });
            out.push(B { pillar: false, kind: Kind::Wedge, size: (hw, hh, lh), pos: (b.0, cy, zc), ori: b.1 });
        }
        WallStyle::MicroBrick => {
            out.push(B { pillar: false, kind: Kind::Micro, size: (s.wall_thickness / 2, lh, hh), pos: (cx, cy, zc), ori: 16 });
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
fn piece_walls(s: &Settings, g: &Geo, lh: i32) -> Vec<B> {
    let mut v = Vec::new();
    let zb = wall_base(s);
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
    B { pillar: false, kind: Kind::Micro, size: (t, total / 2, t), pos: (cx, 0, z), ori: 19 }
}

// ---------- honeycomb geometry ----------

struct Geo {
    /// Exact wall length of a perfect honeycomb.
    side: f64,
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
        side,
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
    if s.posts_extra % 2 != 0 || s.wall_height + s.posts_extra < 2 {
        return Err("Pillar height must be a whole number of micro bricks and at least 1 tall".into());
    }
    if s.corner_posts < 0 || s.corner_posts % 2 != 0 || s.corner_posts > 2000 {
        return Err("Pillar width must be a whole number of micro bricks".into());
    }
    if s.wedge_thickness < 4 || s.wedge_thickness % 4 != 0 {
        return Err("Wedge wall thickness must be a whole number of wedge pairs (2, 4, 6 ... micro)".into());
    }
    let thick = if s.style == WallStyle::MicroWedge { s.wedge_thickness } else { s.wall_thickness };
    if thick > 2000 {
        return Err("Walls are too thick for one brick".into());
    }
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
        // Stretch arrives in v0.3.1 (angles change, so pieces must be recut).
        return Err("Stretch is not available yet (coming in v0.3.1)".into());
    }
    let built = match s.pattern {
        Pattern::Honeycomb => build_honeycomb(s)?,
        _ => patterns::build_faces(s)?,
    };
    layers::finish(s, built)
}

/// The honeycomb (its own code, as tested in game).
fn build_honeycomb(s: &Settings) -> Result<Built, String> {
    let g = geo(s);
    let upside = s.way_up == WayUp::UpsideDown;
    let turn: f64 = if s.turn == Turn::WallsAcross { 90.0 } else { 0.0 };
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

    // Main grid: pieces whose direction is 0° (or 90° when the pattern is turned).
    let main_k: Option<usize> = Some(0);
    let across = s.turn == Turn::WallsAcross;
    let to_main = |mut b: B| -> B {
        if upside {
            b = flip_x(b);
        }
        if across {
            b = rot90(b);
        }
        b
    };

    // Ideal corners: which pieces' walls meet at each corner.
    let ends_local = |g: &Geo| -> Vec<(f64, f64)> {
        let (sx, ly, hs) = (s.cell as f64 / 2.0, 1.5 * g.side, g.side / 2.0);
        vec![(sx, hs), (sx, -hs), (-sx, hs), (-sx, -hs), (0.0, ly + hs), (0.0, ly - hs), (0.0, -ly + hs), (0.0, -ly - hs)]
    };
    let mut corners: std::collections::HashMap<(i64, i64), Vec<usize>> = std::collections::HashMap::new();
    for (i, p) in pieces.iter().enumerate() {
        if !p.3 {
            continue;
        }
        let th = dirs[p.0].to_radians();
        let (c, sn) = (th.cos(), th.sin());
        for (lx, ly) in ends_local(&g) {
            let (wx, wy) = (p.1 + c * lx - sn * ly, p.2 + sn * lx + c * ly);
            corners.entry(((wx * 20.0).round() as i64, (wy * 20.0).round() as i64)).or_default().push(i);
        }
    }

    // Pillars: on the main grid while they fit; otherwise on their own entity.
    let ph = s.corner_posts / 2;
    let full_half = g.wall_half;
    let short_half = full_half - ph;
    let mut posts_entity = false;
    let mut posts: Vec<(i32, i32)> = Vec::new();
    let main_post_spots = |half: i32| -> Vec<(i32, i32)> {
        let mut v = Vec::new();
        for &(k, x, y, w, ..) in &pieces {
            if Some(k) != main_k || !w {
                continue;
            }
            let (cx, cy) = (x.round() as i32, y.round() as i32);
            for (wx, wy) in [(g.side_x, 0), (-g.side_x, 0), (0, g.leg_y), (0, -g.leg_y)] {
                for sgn in [1, -1] {
                    let (lx, ly) = (wx, wy + sgn * (half + ph));
                    let (lx, ly) = if upside { (lx, -ly) } else { (lx, ly) };
                    let (lx, ly) = if across { (-ly, lx) } else { (lx, ly) };
                    v.push((cx + lx, cy + ly));
                }
            }
        }
        v
    };
    let ideal_corner_spots = |near: &Vec<(i32, i32)>| -> Vec<(i32, i32)> {
        let mut v = Vec::new();
        for key in corners.keys() {
            let (vx, vy) = ((key.0 as f64 / 20.0).round() as i32, (key.1 as f64 / 20.0).round() as i32);
            if !near.iter().any(|(px, py)| (px - vx).abs() <= 2 * ph + 1 && (py - vy).abs() <= 2 * ph + 1) {
                v.push((vx, vy));
            }
        }
        v
    };
    if s.corner_posts > 0 {
        let fits = short_half >= 1 && {
            let mut p = main_post_spots(short_half);
            let extra = ideal_corner_spots(&p);
            p.extend(extra);
            p.sort();
            p.dedup();
            let mut ok = true;
            'outer: for i in 0..p.len() {
                for j in i + 1..p.len() {
                    if p[j].0 - p[i].0 >= 2 * ph {
                        break;
                    }
                    if (p[j].1 - p[i].1).abs() < 2 * ph {
                        ok = false;
                        break 'outer;
                    }
                }
            }
            if ok {
                posts = p;
            }
            ok
        };
        if !fits {
            posts_entity = true;
            posts = ideal_corner_spots(&Vec::new());
            posts.sort();
            posts.dedup();
        }
    }
    // Pillars that would overlap must go on different entities. Share them out
    // with as few entities as possible: always place the most-crowded pillar next
    // (the "DSatur" method).
    let mut post_group: Vec<usize> = Vec::new();
    let mut post_groups = 0usize;
    if posts_entity {
        let n = posts.len();
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by_key(|&i| posts[i]);
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for oi in 0..n {
            let i = order[oi];
            for &j in &order[oi + 1..] {
                if posts[j].0 - posts[i].0 >= 2 * ph {
                    break;
                }
                if (posts[j].1 - posts[i].1).abs() < 2 * ph {
                    adj[i].push(j);
                    adj[j].push(i);
                }
            }
        }
        post_group = vec![usize::MAX; n];
        let mut seen: Vec<std::collections::HashSet<usize>> = vec![std::collections::HashSet::new(); n];
        for _ in 0..n {
            let mut best = usize::MAX;
            for i in 0..n {
                if post_group[i] != usize::MAX {
                    continue;
                }
                if best == usize::MAX || (seen[i].len(), adj[i].len()) > (seen[best].len(), adj[best].len()) {
                    best = i;
                }
            }
            let mut c = 0;
            while seen[best].contains(&c) {
                c += 1;
            }
            post_group[best] = c;
            post_groups = post_groups.max(c + 1);
            for &j in &adj[best] {
                seen[j].insert(c);
            }
        }
    }
    let stop_half = short_half.max(1);
    // Main-grid walls stop at pillars on the main grid, unless they move to their own entity and run through.
    let main_half = if s.corner_posts > 0 && (!s.walls_through || (!posts_entity && !s.main_on_entity)) { stop_half } else { full_half };
    let ent_half = if s.corner_posts > 0 && !s.walls_through { stop_half } else { full_half };

    let mut main: Vec<B> = Vec::new();
    let walls_main = piece_walls(s, &g, main_half);
    let walls_ent = piece_walls(s, &g, ent_half);
    let mut main_move: f64 = 0.0;
    let piece_bricks = |walls: bool, spoke: bool, tf: bool, tb: bool, on_main: bool| -> Vec<B> {
        let mut v = Vec::new();
        if walls {
            v.extend(if on_main { walls_main.iter() } else { walls_ent.iter() }.copied());
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
        for b in piece_bricks(w, sp, tf, tb, true) {
            let mut b = to_main(b);
            b.pos = (b.pos.0 + cx, b.pos.1 + cy, b.pos.2);
            main.push(b);
        }
    }

    let post_hh = s.wall_height / 2 + s.posts_extra / 2;
    // Centred: the pillar's middle lines up with the walls' middle, so any extra
    // height sticks out equally above and below. Otherwise it all goes on top.
    let post_zc = if s.posts_centred { wall_base(s) + s.wall_height / 2 } else { wall_base(s) + post_hh };
    let post_kind = if s.posts_round { Kind::Pole } else { Kind::Micro };
    if s.corner_posts > 0 && !posts_entity {
        for &(px, py) in &posts {
            let (z, ori) = if upside { (-post_zc, 20u8) } else { (post_zc, 16u8) };
            main.push(B { pillar: true, kind: post_kind, size: (ph, ph, post_hh), pos: (px, py, z), ori });
        }
    }

    // Base-layer footprint of everything, for finding free bearing spots.
    let mut taken: Vec<((f64, f64), (f64, f64))> = Vec::new();
    // The first `hard` entries are main-grid bricks (a bearing may never overlap
    // those); the rest are entity bricks, which a bearing may sit inside if it must.
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
    let hard = taken.len();

    for &(k, x, y, w, sp, tf, tb) in &pieces {
        if Some(k) == main_k {
            continue;
        }
        let m = ent_matrix(dirs[k], upside);
        for b in piece_bricks(w, sp, tf, tb, false) {
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

    if posts_entity && post_zc - post_hh < 2 {
        for &(px, py) in &posts {
            taken.push((((px - ph) as f64 - 0.1, (py - ph) as f64 - 0.1), ((px + ph) as f64 + 0.1, (py + ph) as f64 + 0.1)));
        }
    }

    // Centre of the layout, to put bearings near the middle.
    let n = pieces.len() as f64;
    let mx = pieces.iter().map(|p| p.1).sum::<f64>() / n;
    let my = pieces.iter().map(|p| p.2).sum::<f64>() / n;

    let ent_ks: Vec<usize> = (0..3).filter(|k| Some(*k) != main_k && pieces.iter().any(|p| p.0 == *k)).collect();

    // Rounding error of each piece (world units). Main-grid pieces are fixed.
    let mut err: Vec<(f64, f64)> = vec![(0.0, 0.0); pieces.len()];
    for (i, p) in pieces.iter().enumerate() {
        if Some(p.0) == main_k {
            err[i] = (p.1.round() - p.1, p.2.round() - p.2);
        }
    }
    let ent_err = |k: usize, o: (i32, i32), err: &mut Vec<(f64, f64)>| {
        let m = ent_matrix(norm(dirs[k]), upside);
        for (i, p) in pieces.iter().enumerate() {
            if p.0 != k {
                continue;
            }
            let t = mul_t(&m, (p.1 - o.0 as f64, p.2 - o.1 as f64, 0.0));
            let d = (t.0.round() - t.0, t.1.round() - t.1, 0.0);
            let e = mul(&m, d);
            err[i] = (e.0, e.1);
        }
    };
    // How badly corners miss: sum of squared spread, and the worst single corner.
    let corner_cost = |err: &Vec<(f64, f64)>| -> (f64, f64) {
        let (mut sum, mut worst) = (0.0f64, 0.0f64);
        for v in corners.values() {
            if v.len() < 2 {
                continue;
            }
            let n = v.len() as f64;
            let mx = v.iter().map(|i| err[*i].0).sum::<f64>() / n;
            let my = v.iter().map(|i| err[*i].1).sum::<f64>() / n;
            for i in v {
                let d2 = (err[*i].0 - mx).powi(2) + (err[*i].1 - my).powi(2);
                sum += d2;
                worst = worst.max(d2.sqrt());
            }
        }
        (sum, worst)
    };

    // Free bearing spots, nearest the middle first.
    let mut use_soft = true;
    let free_with = |soft: bool, bx: i32, by: i32, others: &[(i32, i32)]| -> bool {
        let lo = ((bx - 1) as f64, (by - 1) as f64);
        let hi = ((bx + 1) as f64, (by + 1) as f64);
        let list = if soft { &taken[..] } else { &taken[..hard] };
        !list.iter().any(|(a, b)| hi.0 > a.0 && lo.0 < b.0 && hi.1 > a.1 && lo.1 < b.1)
            && !others.iter().any(|(ox, oy)| (ox - bx).abs() < 3 && (oy - by).abs() < 3)
    };
    // Fewer candidate spots on very large patterns, so building stays quick.
    let need = ent_ks.len() + post_groups;
    let limit = (if s.best_bearings { (3_000_000 / corners.len().max(1)).clamp(150, 2500) } else { 50 }).max(need * 20);
    let find_spots = |soft: bool| -> Vec<(i32, i32)> {
        let mut spots: Vec<(i32, i32)> = Vec::new();
        'search: for rad in 0..200i32 {
            for dx in -rad..=rad {
                for dy in -rad..=rad {
                    if dx.abs() != rad && dy.abs() != rad {
                        continue;
                    }
                    let (bx, by) = (mx.round() as i32 + dx, my.round() as i32 + dy);
                    if free_with(soft, bx, by, &[]) {
                        spots.push((bx, by));
                        if spots.len() >= limit {
                            break 'search;
                        }
                    }
                }
            }
        }
        spots
    };
    let mut spots = find_spots(true);
    if spots.len() < need * 10 {
        // Nowhere clear: let bearings sit inside entity bricks (never main-grid ones).
        use_soft = false;
        spots = find_spots(false);
    }
    let free = |bx: i32, by: i32, others: &[(i32, i32)]| free_with(use_soft, bx, by, others);
    if spots.is_empty() {
        return Err("No free spot for a bearing".into());
    }
    // Start with the nearest free spots, then (if asked) improve one entity at a time.
    let mut chosen: Vec<(i32, i32)> = Vec::new();
    for _ in 0..ent_ks.len() + post_groups {
        let pick = *spots.iter().find(|p| free(p.0, p.1, &chosen)).ok_or("No free spot for a bearing")?;
        chosen.push(pick);
    }
    for (j, &k) in ent_ks.iter().enumerate() {
        ent_err(k, chosen[j], &mut err);
    }
    if s.best_bearings {
        for _pass in 0..2 {
            for (j, &k) in ent_ks.iter().enumerate() {
                let others: Vec<(i32, i32)> = chosen.iter().enumerate().filter(|(i, _)| *i != j).map(|(_, p)| *p).collect();
                let mut best = (corner_cost(&err).0, chosen[j]);
                for &sp in &spots {
                    if !free(sp.0, sp.1, &others) {
                        continue;
                    }
                    ent_err(k, sp, &mut err);
                    let c = corner_cost(&err).0;
                    if c < best.0 - 1e-9 {
                        best = (c, sp);
                    }
                }
                chosen[j] = best.1;
                ent_err(k, chosen[j], &mut err);
            }
        }
    }
    let (_, worst_corner) = corner_cost(&err);
    let mean_corner = {
        let (mut t, mut n) = (0.0, 0.0);
        for v in corners.values() {
            if v.len() < 2 {
                continue;
            }
            let k = v.len() as f64;
            let mx = v.iter().map(|i| err[*i].0).sum::<f64>() / k;
            let my = v.iter().map(|i| err[*i].1).sum::<f64>() / k;
            let spread = v.iter().map(|i| (err[*i].0 - mx).hypot(err[*i].1 - my)).fold(0.0, f64::max);
            t += spread;
            n += 1.0;
        }
        if n > 0.0 { t / n } else { 0.0 }
    };

    let mut entities = Vec::new();
    let mut max_move = main_move;
    for (j, &k) in ent_ks.iter().enumerate() {
        let yaw = norm(dirs[k]);
        let m = ent_matrix(yaw, upside);
        let (bx, by) = chosen[j];
        let bz = if upside { -1 } else { 1 };
        let origin = (bx, by, 0);
        let mut bricks = Vec::new();
        let mut mv: f64 = 0.0;
        for &(pk, x, y, w, sp, tf, tb) in &pieces {
            if pk != k {
                continue;
            }
            let t = mul_t(&m, (x - origin.0 as f64, y - origin.1 as f64, 0.0));
            let (tx, ty) = (t.0.round() as i32, t.1.round() as i32);
            mv = mv.max((t.0 - tx as f64).hypot(t.1 - ty as f64));
            for mut b in piece_bricks(w, sp, tf, tb, false) {
                b.pos = (b.pos.0 + tx, b.pos.1 + ty, b.pos.2);
                bricks.push(b);
            }
        }
        max_move = max_move.max(mv);
        let angle = if upside { norm(yaw - 180.0) } else { norm(-yaw) };
        entities.push(Grid { yaw: Some(yaw), bricks, origin, bearing: (bx, by, bz), angle, max_move: mv, pillars: false, part: 0 });
    }
    for gi in 0..post_groups {
        // Pillars on their own entity at 0°: whole-unit positions, so no rounding.
        let (bx, by) = chosen[ent_ks.len() + gi];
        let origin = (bx, by, 0);
        let bricks = posts
            .iter()
            .zip(post_group.iter())
            .filter(|(_, g)| **g == gi)
            .map(|(&(px, py), _)| {
                let ly = if upside { -(py - by) } else { py - by };
                B { pillar: true, kind: post_kind, size: (ph, ph, post_hh), pos: (px - bx, ly, post_zc), ori: 16 }
            })
            .collect();
        let angle = if upside { 180.0 } else { 0.0 };
        entities.push(Grid { yaw: Some(0.0), bricks, origin, bearing: (bx, by, if upside { -1 } else { 1 }), angle, max_move: 0.0, pillars: true, part: 3 });
    }
    let angles = (0..3).map(|k| norm(dirs[k]).rem_euclid(180.0)).collect();
    Ok(Built { main, entities, cells, max_move, angles, corner_mean: mean_corner, corner_worst: worst_corner, posts_entity: post_groups, layers: 0 })
}

/// World-space bounding box of everything (entities as turned boxes).
pub fn world_bounds(built: &Built, upside: bool) -> ((f64, f64, f64), (f64, f64, f64)) {
    world_bounds_parts(built, upside, true, true, true)
}

/// Bounding box of only the chosen parts.
pub fn world_bounds_parts(built: &Built, upside: bool, main: bool, ents: bool, bears: bool) -> ((f64, f64, f64), (f64, f64, f64)) {
    let mut lo = (f64::MAX, f64::MAX, f64::MAX);
    let mut hi = (f64::MIN, f64::MIN, f64::MIN);
    let mut add = |c: (f64, f64, f64), e: (f64, f64, f64)| {
        lo = (lo.0.min(c.0 - e.0), lo.1.min(c.1 - e.1), lo.2.min(c.2 - e.2));
        hi = (hi.0.max(c.0 + e.0), hi.1.max(c.1 + e.1), hi.2.max(c.2 + e.2));
    };
    for b in built.main.iter().filter(|_| main) {
        let h = half(b);
        add((b.pos.0 as f64, b.pos.1 as f64, b.pos.2 as f64), (h.0 as f64, h.1 as f64, h.2 as f64));
    }
    for g in &built.entities {
        if bears {
            add((g.bearing.0 as f64, g.bearing.1 as f64, g.bearing.2 as f64), (1.0, 1.0, 1.0));
        }
        if !ents {
            continue;
        }
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

fn to_brick(p: Paint, b: &B) -> Brick {
    let dir = [Direction::XPositive, Direction::XNegative, Direction::YPositive, Direction::YNegative, Direction::ZPositive, Direction::ZNegative][(b.ori >> 2) as usize];
    let rot = [Rotation::Deg0, Rotation::Deg90, Rotation::Deg180, Rotation::Deg270][(b.ori & 3) as usize];
    Brick {
        asset: BrickType::Procedural {
            asset: brdb::BString::str(match b.kind { Kind::Wedge => MICRO_WEDGE, Kind::Pole => POLE, Kind::Micro => MICRO_BRICK }),
            size: BrickSize { x: b.size.0 as u16, y: b.size.1 as u16, z: b.size.2 as u16 },
        },
        position: (b.pos.0, b.pos.1, b.pos.2).into(),
        direction: dir,
        rotation: rot,
        color: (p.color[0], p.color[1], p.color[2]).into(),
        material: brdb::BString::from(MATERIALS[p.material].1.to_string()),
        material_intensity: p.intensity,
        ..Default::default()
    }
}

/// Which paint each entity uses: wall entities take Entity 1, Entity 2 in
/// order; pillar entities take Pillars.
pub fn entity_parts(built: &Built) -> Vec<usize> {
    built.entities.iter().map(|g| g.part).collect()
}

/// Build the prefab world: main grid, entities, bearings and joints, centred.
/// Only the parts ticked in the settings are included.
pub fn to_world(s: &Settings, built: &Built) -> World {
    let upside = s.way_up == WayUp::UpsideDown;
    let (inc_m, inc_e, inc_b) = (s.include_main, s.include_entities, s.include_bearings);
    let (lo, hi) = world_bounds_parts(built, upside, inc_m, inc_e, inc_b);
    // Centre on whole units, like the game's own copies.
    let c = (((lo.0 + hi.0) / 2.0).round() as i32, ((lo.1 + hi.1) / 2.0).round() as i32, ((lo.2 + hi.2) / 2.0).round() as i32);
    let mut w = World::new();
    w.register_all_components();
    if inc_m {
        for b in &built.main {
            let mut br = to_brick(s.paint(if b.pillar { 3 } else { 0 }), b);
            br.position = Position::new(b.pos.0 - c.0, b.pos.1 - c.1, b.pos.2 - c.2);
            w.add_brick(br);
        }
    }
    let parts = entity_parts(built);
    let bp = s.paint(4);
    for (gi, g) in built.entities.iter().enumerate() {
        let mut bearing_id = None;
        if inc_b {
            let (bearing, id) = Brick {
                asset: BrickType::Basic(brdb::BString::str(BEARING)),
                position: Position::new(g.bearing.0 - c.0, g.bearing.1 - c.1, g.bearing.2 - c.2),
                direction: if upside { Direction::ZNegative } else { Direction::ZPositive },
                rotation: Rotation::Deg0,
                color: (bp.color[0], bp.color[1], bp.color[2]).into(),
                material: brdb::BString::from(MATERIALS[bp.material].1.to_string()),
                material_intensity: bp.intensity,
                visible: !s.hide_bearings,
                collision: if s.hide_bearings {
                    brdb::Collision { player: false, player1: None, player2: None, player3: None, weapon: false, interact: false, tool: false, physics: false }
                } else {
                    Default::default()
                },
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
            bearing_id = Some(id);
        }
        if !inc_e {
            continue;
        }
        let paint = s.paint(parts[gi]);
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
            g.bricks.iter().map(|b| to_brick(paint, b)),
        );
        if let Some(bid) = bearing_id {
            w.add_joint(bid, id, Vector3f::default(), Quat4f { x: 0.0, y: 0.0, z: 0.0, w: 1.0 });
        }
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
    if !(s.include_main || s.include_entities || s.include_bearings) {
        return Err("Nothing to save: tick at least one of Main grid, Entities, Bearings".into());
    }
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
pub mod patterns;
pub mod layers;
pub mod palette;

// ---------- preview footprints ----------

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FootKind { Wall, Spoke, Pillar, Bearing }

/// One brick seen from above: four corners (world units), which grid it is
/// on (0 = main grid, 1.. = entities in order) and what it is.
#[derive(Clone, Debug)]
pub struct Foot {
    pub pts: [(f64, f64); 4],
    pub grid: usize,
    pub kind: FootKind,
}

fn foot_kind(b: &B) -> FootKind {
    match b.kind {
        Kind::Wedge => FootKind::Wall,
        Kind::Pole => FootKind::Pillar,
        Kind::Micro => {
            let h = half(b);
            // spokes are flat strips; pillars are square posts; the rest are walls
            if h.0 == h.1 && h.2 > h.0 { FootKind::Pillar } else if h.2 <= h.0.min(h.1) { FootKind::Spoke } else { FootKind::Wall }
        }
    }
}

/// Footprints of everything, for drawing the top-down preview.
pub fn footprints(s: &Settings, built: &Built) -> Vec<Foot> {
    let upside = s.way_up == WayUp::UpsideDown;
    let mut v = Vec::new();
    let rect = |cx: f64, cy: f64, hx: f64, hy: f64| [(cx - hx, cy - hy), (cx + hx, cy - hy), (cx + hx, cy + hy), (cx - hx, cy + hy)];
    for b in &built.main {
        let h = half(b);
        v.push(Foot { pts: rect(b.pos.0 as f64, b.pos.1 as f64, h.0 as f64, h.1 as f64), grid: 0, kind: foot_kind(b) });
    }
    for (i, g) in built.entities.iter().enumerate() {
        let m = ent_matrix(g.yaw.unwrap(), upside);
        for b in &g.bricks {
            let h = half(b);
            let mut pts = [(0.0, 0.0); 4];
            for (j, (sx, sy)) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].iter().enumerate() {
                let p = mul(&m, (b.pos.0 as f64 + sx * h.0 as f64, b.pos.1 as f64 + sy * h.1 as f64, 0.0));
                pts[j] = (p.0 + g.origin.0 as f64, p.1 + g.origin.1 as f64);
            }
            let _ = i;
            v.push(Foot { pts, grid: g.part, kind: foot_kind(b) });
        }
        v.push(Foot { pts: rect(g.bearing.0 as f64, g.bearing.1 as f64, 1.0, 1.0), grid: 0, kind: FootKind::Bearing });
    }
    v
}

/// How high each kind of brick reaches (bottom, top in units, world z):
/// walls, spokes, pillars, bearings. Used for the side view next to the preview.
pub fn height_profile(s: &Settings, built: &Built) -> Vec<(FootKind, i32, i32)> {
    let upside = s.way_up == WayUp::UpsideDown;
    let mut out: Vec<(FootKind, i32, i32)> = Vec::new();
    let mut add = |k: FootKind, z0: i32, z1: i32| {
        if let Some(e) = out.iter_mut().find(|e| e.0 == k) {
            e.1 = e.1.min(z0);
            e.2 = e.2.max(z1);
        } else {
            out.push((k, z0, z1));
        }
    };
    for b in &built.main {
        let h = half(b);
        add(foot_kind(b), b.pos.2 - h.2, b.pos.2 + h.2);
    }
    for g in &built.entities {
        for b in &g.bricks {
            let h = half(b);
            let (z0, z1) = (b.pos.2 - h.2, b.pos.2 + h.2);
            if upside { add(foot_kind(b), -z1, -z0) } else { add(foot_kind(b), z0, z1) }
        }
        add(FootKind::Bearing, g.bearing.2 - 1, g.bearing.2 + 1);
    }
    let order = |k: FootKind| match k { FootKind::Wall => 0, FootKind::Spoke => 1, FootKind::Pillar => 2, FootKind::Bearing => 3 };
    out.sort_by_key(|e| order(e.0));
    out
}

// ---------- settings as text (presets) ----------

impl Settings {
    pub fn to_text(&self) -> String {
        let e = |x: &dyn std::fmt::Debug| format!("{x:?}");
        let lines = [
            ("pattern", e(&self.pattern)),
            ("hide_bearings", self.hide_bearings.to_string()),
            ("main_on_entity", self.main_on_entity.to_string()),
            ("cell", self.cell.to_string()),
            ("stretch_h", self.stretch_h.to_string()),
            ("stretch_v", self.stretch_v.to_string()),
            ("turn", e(&self.turn)),
            ("style", e(&self.style)),
            ("wall_height", self.wall_height.to_string()),
            ("wall_thickness", self.wall_thickness.to_string()),
            ("wedge_thickness", self.wedge_thickness.to_string()),
            ("flip_wedges", self.flip_wedges.to_string()),
            ("spokes", e(&self.spokes)),
            ("spoke_dirs", format!("{},{},{}", self.spoke_dirs[0], self.spoke_dirs[1], self.spoke_dirs[2])),
            ("spoke_every", self.spoke_every.to_string()),
            ("spoke_rows", self.spoke_rows.to_string()),
            ("spoke_offset", self.spoke_offset.to_string()),
            ("spoke_stagger", self.spoke_stagger.to_string()),
            ("spoke_length", self.spoke_length.to_string()),
            ("spoke_thickness", self.spoke_thickness.to_string()),
            ("spoke_top", self.spoke_top.to_string()),
            ("area", e(&self.area)),
            ("width", self.width.to_string()),
            ("length", self.length.to_string()),
            ("way_up", e(&self.way_up)),
            ("corner_posts", self.corner_posts.to_string()),
            ("posts_round", self.posts_round.to_string()),
            ("walls_through", self.walls_through.to_string()),
            ("posts_extra", self.posts_extra.to_string()),
            ("best_bearings", self.best_bearings.to_string()),
            ("color", format!("{},{},{}", self.color[0], self.color[1], self.color[2])),
            ("material", self.material.to_string()),
            ("intensity", self.intensity.to_string()),
            ("one_paint", self.one_paint.to_string()),
            ("paints", self.paints.iter().map(|p| format!("{},{},{},{},{}", p.color[0], p.color[1], p.color[2], p.material, p.intensity)).collect::<Vec<_>>().join(";")),
            ("posts_centred", self.posts_centred.to_string()),
            ("include", format!("{},{},{}", self.include_main, self.include_entities, self.include_bearings)),
        ];
        lines.iter().map(|(k, v)| format!("{k}={v}\n")).collect()
    }

    /// Reads text written by `to_text`; unknown or broken lines are skipped.
    pub fn from_text(t: &str) -> Settings {
        let mut s = Settings::default();
        for line in t.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let v = v.trim();
            let i = || v.parse::<i32>().ok();
            let f = || v.parse::<f64>().ok();
            let b = || v.parse::<bool>().ok();
            match k.trim() {
                "pattern" => if let Some(p) = Pattern::ALL.iter().find(|p| format!("{p:?}") == v) { s.pattern = *p },
                "cell" => if let Some(x) = i() { s.cell = x },
                "stretch_h" => if let Some(x) = f() { s.stretch_h = x },
                "stretch_v" => if let Some(x) = f() { s.stretch_v = x },
                "turn" => s.turn = if v == "WallsAcross" { Turn::WallsAcross } else { Turn::WallsUpright },
                "style" => s.style = if v == "MicroBrick" { WallStyle::MicroBrick } else { WallStyle::MicroWedge },
                "wall_height" => if let Some(x) = i() { s.wall_height = x },
                "wall_thickness" => if let Some(x) = i() { s.wall_thickness = x },
                "wedge_thickness" => if let Some(x) = i() { s.wedge_thickness = x },
                "flip_wedges" => if let Some(x) = b() { s.flip_wedges = x },
                "spokes" => s.spokes = match v { "None" => SpokeMode::None, "EveryCell" => SpokeMode::EveryCell, "AltRows" => SpokeMode::AltRows, _ => SpokeMode::Custom },
                "spoke_dirs" => {
                    let p: Vec<bool> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                    if p.len() == 3 { s.spoke_dirs = [p[0], p[1], p[2]] }
                }
                "spoke_every" => if let Some(x) = i() { s.spoke_every = x },
                "spoke_rows" => if let Some(x) = i() { s.spoke_rows = x },
                "spoke_offset" => if let Some(x) = i() { s.spoke_offset = x },
                "spoke_stagger" => if let Some(x) = b() { s.spoke_stagger = x },
                "spoke_length" => if let Some(x) = i() { s.spoke_length = x },
                "spoke_thickness" => if let Some(x) = i() { s.spoke_thickness = x },
                "spoke_top" => if let Some(x) = b() { s.spoke_top = x },
                "area" => s.area = match v { "Rectangle" => Area::Rectangle, "Circle" => Area::Circle, "Oval" => Area::Oval, _ => Area::Hexagon },
                "width" => if let Some(x) = i() { s.width = x },
                "length" => if let Some(x) = i() { s.length = x },
                "way_up" => s.way_up = if v == "UpsideDown" { WayUp::UpsideDown } else { WayUp::Upright },
                "corner_posts" => if let Some(x) = i() { s.corner_posts = x },
                "posts_round" => if let Some(x) = b() { s.posts_round = x },
                "walls_through" => if let Some(x) = b() { s.walls_through = x },
                "hide_bearings" => if let Some(x) = b() { s.hide_bearings = x },
                "main_on_entity" => if let Some(x) = b() { s.main_on_entity = x },
                "posts_extra" => if let Some(x) = i() { s.posts_extra = x },
                "best_bearings" => if let Some(x) = b() { s.best_bearings = x },
                "color" => {
                    let p: Vec<u8> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                    if p.len() == 3 { s.color = [p[0], p[1], p[2]] }
                }
                "material" => if let Some(x) = v.parse::<usize>().ok() { s.material = x.min(MATERIALS.len() - 1) },
                "intensity" => if let Some(x) = v.parse::<u8>().ok() { s.intensity = x.min(10) },
                "one_paint" => if let Some(x) = b() { s.one_paint = x },
                "posts_centred" => if let Some(x) = b() { s.posts_centred = x },
                "paints" => {
                    for (i, part) in v.split(';').enumerate().take(5) {
                        let n: Vec<usize> = part.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                        if n.len() == 5 {
                            s.paints[i] = Paint { color: [n[0].min(255) as u8, n[1].min(255) as u8, n[2].min(255) as u8], material: n[3].min(MATERIALS.len() - 1), intensity: n[4].min(10) as u8 };
                        }
                    }
                }
                "include" => {
                    let p: Vec<bool> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
                    if p.len() == 3 { s.include_main = p[0]; s.include_entities = p[1]; s.include_bearings = p[2]; }
                }
                _ => {}
            }
        }
        s
    }
}
