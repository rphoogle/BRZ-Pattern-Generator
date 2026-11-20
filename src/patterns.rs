//! The other patterns (triangles, stars, cubes, octagons and squares, diamonds).
//!
//! In plain English: each pattern is drawn as a set of flat shapes ("faces"),
//! like tiles on a floor. Every edge of every tile becomes a wall. Walls are
//! sorted by the way they point: walls that point the same way (or at right
//! angles, if they never touch) share one brick grid. The grid whose walls run
//! straight across the build is the normal brick grid; every other grid is an
//! entity turned to that angle and held by a hidden Micro Rigid Bearing.
//! Walls that carry straight on in a line are joined into one long brick.
//! The honeycomb keeps its own, already tested, code in lib.rs.

use crate::*;
use std::collections::HashMap;

/// One tile of the pattern: its corners in order, its place in the lattice
/// (used for spoke spacing) and its middle.
#[derive(Clone, Debug)]
pub struct Face {
    pub pts: Vec<(f64, f64)>,
    pub idx: (i32, i32),
    pub c: (f64, f64),
}

fn face(pts: Vec<(f64, f64)>, idx: (i32, i32)) -> Face {
    let n = pts.len() as f64;
    let c = (pts.iter().map(|p| p.0).sum::<f64>() / n, pts.iter().map(|p| p.1).sum::<f64>() / n);
    Face { pts, idx, c }
}

/// The smallest gap between two parallel walls of the same grid, as a share
/// of the cell size (walls thicker than this would run into each other).
pub fn min_gap(p: Pattern) -> f64 {
    match p {
        Pattern::Octagons => 2f64.sqrt() - 1.0,
        Pattern::Stars | Pattern::Cubes => 0.5,
        _ => 1.0,
    }
}

/// What "cell size" measures for each pattern (shown in the window).
pub fn cell_meaning(p: Pattern) -> &'static str {
    match p {
        Pattern::Honeycomb => "flat side to flat side of a hexagon",
        Pattern::Triangles => "height of a triangle (flat side to point)",
        Pattern::Stars => "flat side to flat side of a hexagon",
        Pattern::Cubes => "flat side to flat side of a cube (hexagon)",
        Pattern::Octagons => "flat side to flat side of an octagon",
        Pattern::Diamonds => "gap between parallel walls (diamond width across flats)",
    }
}

/// Grid angles: (main grid label, entity angles in order). Matches `build_faces`.
pub fn pattern_angles(p: Pattern, turned: bool) -> (String, Vec<f64>) {
    let t = if turned { 90.0 } else { 0.0 };
    let n = |a: f64| (a + t).rem_euclid(180.0);
    match p {
        Pattern::Honeycomb | Pattern::Triangles | Pattern::Stars => {
            if turned { ("90°".into(), vec![150.0, 30.0]) } else { ("0°".into(), vec![60.0, 120.0]) }
        }
        Pattern::Cubes => {
            let mut e = vec![n(30.0), n(150.0)];
            e.sort_by(|a, b| a.partial_cmp(b).unwrap());
            (if turned { "0°" } else { "90°" }.into(), e)
        }
        Pattern::Octagons => ("0° and 90°".into(), vec![45.0, 135.0]),
        Pattern::Diamonds => {
            let mut e = vec![n(60.0), n(120.0)];
            e.sort_by(|a, b| a.partial_cmp(b).unwrap());
            ("no walls".into(), e)
        }
    }
}

/// All the tiles that cover the area (before trimming to the area's shape).
pub fn faces(s: &Settings) -> Vec<Face> {
    let f = s.cell as f64;
    let r3 = 3f64.sqrt();
    let reach = (s.width.max(s.length) as f64) * 0.75 + 3.0 * f;
    let mut v: Vec<Face> = Vec::new();
    match s.pattern {
        Pattern::Honeycomb => {}
        Pattern::Triangles => {
            // Triangle height = cell, side a. Lattice points P(i, j).
            let h = f;
            let a = 2.0 * h / r3;
            let p = |i: i32, j: i32| (i as f64 * a + j as f64 * a / 2.0, j as f64 * h);
            let nj = (reach / h).ceil() as i32 + 1;
            let ni = (reach / a).ceil() as i32 + 1;
            for j in -nj..=nj {
                for i in (-ni - nj)..=(ni + nj) {
                    v.push(face(vec![p(i, j), p(i + 1, j), p(i, j + 1)], (2 * i + j, j)));
                    v.push(face(vec![p(i + 1, j), p(i + 1, j + 1), p(i, j + 1)], (2 * i + j + 1, j)));
                }
            }
        }
        Pattern::Stars => {
            // Hexagons (across flats = cell) on a triangular lattice of spacing a,
            // with small triangles between: every line runs straight, making stars.
            let a = 2.0 * f / r3;
            let p = |i: i32, j: i32| (i as f64 * a + j as f64 * a / 2.0, j as f64 * a * r3 / 2.0);
            let nj = (reach / (a * r3 / 2.0)).ceil() as i32 + 1;
            let ni = (reach / a).ceil() as i32 + 1;
            let mid = |p: (f64, f64), q: (f64, f64)| ((p.0 + q.0) / 2.0, (p.1 + q.1) / 2.0);
            for j in -nj..=nj {
                for i in (-ni - nj)..=(ni + nj) {
                    let c = p(i, j);
                    let hex: Vec<(f64, f64)> = (0..6)
                        .map(|k| {
                            let t = (k as f64 * 60.0).to_radians();
                            (c.0 + a / 2.0 * t.cos(), c.1 + a / 2.0 * t.sin())
                        })
                        .collect();
                    v.push(face(hex, (i + j.div_euclid(2), j)));
                    let (p0, p1, p2, p3) = (p(i, j), p(i + 1, j), p(i, j + 1), p(i + 1, j + 1));
                    v.push(face(vec![mid(p0, p1), mid(p1, p2), mid(p2, p0)], (i + j.div_euclid(2), j)));
                    v.push(face(vec![mid(p1, p3), mid(p3, p2), mid(p2, p1)], (i + j.div_euclid(2), j)));
                }
            }
        }
        Pattern::Cubes => {
            // The honeycomb lattice, each hexagon split into three diamonds.
            let side = f / r3;
            let row = 1.5 * side;
            let nr = (reach / row).ceil() as i32 + 1;
            let nq = (reach / f).ceil() as i32 + 1;
            for r in -nr..=nr {
                for q in -nq..=nq {
                    let cx = q as f64 * f + if r.rem_euclid(2) == 1 { f / 2.0 } else { 0.0 };
                    let cy = r as f64 * row;
                    let vtx = |deg: f64| (cx + side * deg.to_radians().cos(), cy + side * deg.to_radians().sin());
                    let c = (cx, cy);
                    v.push(face(vec![c, vtx(30.0), vtx(90.0), vtx(150.0)], (q, r)));
                    v.push(face(vec![c, vtx(150.0), vtx(210.0), vtx(270.0)], (q, r)));
                    v.push(face(vec![c, vtx(270.0), vtx(330.0), vtx(30.0)], (q, r)));
                }
            }
        }
        Pattern::Octagons => {
            let a = f * (2f64.sqrt() - 1.0);
            let rr = f / 2.0 / (22.5f64).to_radians().cos();
            let n = (reach / f).ceil() as i32 + 1;
            for j in -n..=n {
                for i in -n..=n {
                    let (cx, cy) = (i as f64 * f, j as f64 * f);
                    let oct: Vec<(f64, f64)> = (0..8)
                        .map(|k| {
                            let t = (22.5 + 45.0 * k as f64).to_radians();
                            (cx + rr * t.cos(), cy + rr * t.sin())
                        })
                        .collect();
                    v.push(face(oct, (i, j)));
                    let (sx, sy, d) = (cx + f / 2.0, cy + f / 2.0, a / 2f64.sqrt());
                    v.push(face(vec![(sx + d, sy), (sx, sy + d), (sx - d, sy), (sx, sy - d)], (i, j)));
                }
            }
        }
        Pattern::Diamonds => {
            // Walls at 60° and 120°, parallel walls `cell` apart: tall diamonds.
            let e = f / (60f64).to_radians().sin();
            let u = (e * 0.5, e * r3 / 2.0);
            let w = (-e * 0.5, e * r3 / 2.0);
            let n = (reach / (e / 2.0)).ceil() as i32 + 2;
            for j in -n..=n {
                for i in -n..=n {
                    let p = (i as f64 * u.0 + j as f64 * w.0, i as f64 * u.1 + j as f64 * w.1 - e * r3 / 2.0);
                    let pts = vec![p, (p.0 + u.0, p.1 + u.1), (p.0 + u.0 + w.0, p.1 + u.1 + w.1), (p.0 + w.0, p.1 + w.1)];
                    v.push(face(pts, (i - j, i + j)));
                }
            }
        }
    }
    if s.turn == Turn::WallsAcross {
        for fc in &mut v {
            for p in &mut fc.pts {
                *p = (-p.1, p.0);
            }
            fc.c = (-fc.c.1, fc.c.0);
        }
    }
    v
}

/// A straight run of wall or spoke in a grid's own (local) units.
#[derive(Clone, Copy, Debug)]
struct Run {
    along_x: bool,
    v: i32,
    a: i32,
    b: i32,
}

/// Cut `[lo, hi]` out of a run, keeping pieces of even length (at least 2).
fn cut(r: Run, lo: i32, hi: i32) -> Vec<Run> {
    let mut out = Vec::new();
    if hi <= r.a || lo >= r.b {
        out.push(r);
        return out;
    }
    let mut left = Run { b: lo.min(r.b), ..r };
    if (left.b - left.a) % 2 != 0 {
        left.b -= 1;
    }
    if left.b - left.a >= 2 {
        out.push(left);
    }
    let mut right = Run { a: hi.max(r.a), ..r };
    if (right.b - right.a) % 2 != 0 {
        right.a += 1;
    }
    if right.b - right.a >= 2 {
        out.push(right);
    }
    out
}

/// Join touching straight pieces into runs, then round them to whole units.
fn make_runs(segs: &[((f64, f64), (f64, f64))]) -> Vec<Run> {
    let mut groups: HashMap<(bool, i64), Vec<(f64, f64, f64)>> = HashMap::new();
    for &(p, q) in segs {
        let along_x = (q.0 - p.0).abs() >= (q.1 - p.1).abs();
        let (u0, u1, v) = if along_x { (p.0.min(q.0), p.0.max(q.0), (p.1 + q.1) / 2.0) } else { (p.1.min(q.1), p.1.max(q.1), (p.0 + q.0) / 2.0) };
        groups.entry((along_x, (v * 1000.0).round() as i64)).or_default().push((u0, u1, v));
    }
    let mut keys: Vec<_> = groups.keys().copied().collect();
    keys.sort();
    let mut runs = Vec::new();
    for key in keys {
        let mut list = groups.remove(&key).unwrap();
        list.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut merged: Vec<(f64, f64, f64)> = Vec::new();
        for it in list {
            if let Some(last) = merged.last_mut() {
                if it.0 <= last.1 + 1e-6 {
                    last.1 = last.1.max(it.1);
                    continue;
                }
            }
            merged.push(it);
        }
        for (u0, u1, v) in merged {
            let (mut a, mut b) = (u0.round() as i32, u1.round() as i32);
            if (b - a) % 2 != 0 {
                // Fix the end that moves least.
                let opts = [(a - 1, b), (a + 1, b), (a, b - 1), (a, b + 1)];
                let best = opts
                    .iter()
                    .filter(|(x, y)| y - x >= 2)
                    .min_by(|x, y| {
                        let ex = (x.0 as f64 - u0).abs().max((x.1 as f64 - u1).abs());
                        let ey = (y.0 as f64 - u0).abs().max((y.1 as f64 - u1).abs());
                        ex.partial_cmp(&ey).unwrap()
                    })
                    .copied()
                    .unwrap_or((a, a + 2));
                a = best.0;
                b = best.1;
            }
            if b - a < 2 {
                b = a + 2;
            }
            // Long runs are split into bricks of at most MAX_RUN units.
            let mut start = a;
            while b - start > MAX_RUN {
                let pieces = ((b - start) as f64 / MAX_RUN as f64).ceil() as i32;
                let len = (((b - start) / pieces) / 2 * 2).max(2);
                runs.push(Run { along_x: key.0, v: v.round() as i32, a: start, b: start + len });
                start += len;
            }
            runs.push(Run { along_x: key.0, v: v.round() as i32, a: start, b });
        }
    }
    runs
}

/// Longest single brick used for a run (units).
const MAX_RUN: i32 = 4000;

/// Pillars that would overlap must go on different entities: share them out
/// with as few entities as possible (most-crowded pillar first, "DSatur").
fn share_pillars(posts: &[(i32, i32)], ph: i32) -> (Vec<usize>, usize) {
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
    let mut group = vec![usize::MAX; n];
    let mut groups = 0;
    let mut seen: Vec<std::collections::HashSet<usize>> = vec![Default::default(); n];
    for _ in 0..n {
        let mut best = usize::MAX;
        for i in 0..n {
            if group[i] != usize::MAX {
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
        group[best] = c;
        groups = groups.max(c + 1);
        for &j in &adj[best] {
            seen[j].insert(c);
        }
    }
    (group, groups)
}

/// Lay out one of the face-based patterns.
pub fn build_faces(s: &Settings) -> Result<Built, String> {
    let upside = s.way_up == WayUp::UpsideDown;
    let thick = if s.style == WallStyle::MicroWedge { s.wedge_thickness } else { s.wall_thickness };
    let gap = min_gap(s.pattern) * s.cell as f64;
    if thick as f64 > gap - 2.0 {
        return Err(format!("Walls are too thick for this cell size (at most {} micro for this pattern)", ((gap - 2.0) / 2.0).floor().max(0.0)));
    }

    // 1. Tiles inside the area.
    let all = faces(s);
    let kept: Vec<Face> = all.into_iter().filter(|fc| in_area(s, fc.c.0, fc.c.1)).collect();
    if kept.is_empty() {
        return Err("The area is too small for one cell".into());
    }

    // 2. Edges (shared edges once) and corners.
    let key = |p: (f64, f64)| ((p.0 * 1000.0).round() as i64, (p.1 * 1000.0).round() as i64);
    let mut edge_ix: HashMap<((i64, i64), (i64, i64)), usize> = HashMap::new();
    let mut edges: Vec<((f64, f64), (f64, f64))> = Vec::new();
    for fc in &kept {
        let n = fc.pts.len();
        for k in 0..n {
            let (p, q) = (fc.pts[k], fc.pts[(k + 1) % n]);
            let (kp, kq) = (key(p), key(q));
            let ek = if kp < kq { (kp, kq) } else { (kq, kp) };
            if !edge_ix.contains_key(&ek) {
                edge_ix.insert(ek, edges.len());
                edges.push((p, q));
            }
        }
    }
    let ang = |p: (f64, f64), q: (f64, f64)| -> f64 {
        let a = (q.1 - p.1).atan2(q.0 - p.0).to_degrees().rem_euclid(180.0);
        if (a - 180.0).abs() < 1e-6 { 0.0 } else { a }
    };
    let akey = |a: f64| -> i64 { ((a * 100.0).round() as i64).rem_euclid(18000) };
    // corners: position -> edge directions meeting there
    let mut corner_dirs: HashMap<(i64, i64), Vec<i64>> = HashMap::new();
    let mut corner_pos: HashMap<(i64, i64), (f64, f64)> = HashMap::new();
    for &(p, q) in &edges {
        let k = akey(ang(p, q));
        for pt in [p, q] {
            corner_dirs.entry(key(pt)).or_default().push(k);
            corner_pos.insert(key(pt), pt);
        }
    }

    // 3. Spokes: across each tile, between each pair of parallel opposite edges.
    struct Spoke {
        p: (f64, f64),
        q: (f64, f64),
    }
    let mut spokes: Vec<Spoke> = Vec::new();
    if s.spokes != SpokeMode::None {
        let mut fams: Vec<i64> = Vec::new();
        let mut raw: Vec<(i64, (f64, f64), (f64, f64))> = Vec::new();
        for fc in &kept {
            if !spoke_here(s, fc.idx.0, fc.idx.1) {
                continue;
            }
            let n = fc.pts.len();
            if n % 2 != 0 {
                continue;
            }
            for k in 0..n / 2 {
                let (p0, p1) = (fc.pts[k], fc.pts[k + 1]);
                let (q0, q1) = (fc.pts[k + n / 2], fc.pts[(k + n / 2 + 1) % n]);
                if akey(ang(p0, p1)) != akey(ang(q0, q1)) {
                    continue;
                }
                // Spoke runs at right angles to the edges, through the middle.
                let (ex, ey) = (p1.0 - p0.0, p1.1 - p0.1);
                let el = ex.hypot(ey);
                let (nx, ny) = (-ey / el, ex / el);
                let dist = ((q0.0 - p0.0) * nx + (q0.1 - p0.1) * ny).abs();
                let half = if s.spoke_length > 0 { (s.spoke_length as f64 / 2.0).min(dist / 2.0) } else { dist / 2.0 };
                let a = (fc.c.0 - nx * half, fc.c.1 - ny * half);
                let b = (fc.c.0 + nx * half, fc.c.1 + ny * half);
                let k2 = akey(ang(a, b));
                if !fams.contains(&k2) {
                    fams.push(k2);
                }
                raw.push((k2, a, b));
            }
        }
        fams.sort();
        for (k2, a, b) in raw {
            let fi = fams.iter().position(|x| *x == k2).unwrap_or(0);
            if s.spoke_dirs[fi % 3] {
                spokes.push(Spoke { p: a, q: b });
            }
        }
    }

    // 4. Which grid each direction goes on.
    let mut wall_classes: Vec<i64> = edges.iter().map(|e| akey(ang(e.0, e.1))).collect();
    wall_classes.sort();
    wall_classes.dedup();
    // Two directions at right angles may share a grid if no corner has both.
    let conflict = |a: i64, b: i64| corner_dirs.values().any(|d| d.contains(&a) && d.contains(&b));
    let mut grid_of: HashMap<i64, usize> = HashMap::new();
    let mut grid_yaw: Vec<f64> = Vec::new(); // index 0 = main grid (yaw 0)
    grid_yaw.push(0.0);
    for &c in &wall_classes {
        if c == 0 || c == 9000 {
            let other = if c == 0 { 9000 } else { 0 };
            if !grid_of.contains_key(&other) || !conflict(c, other) {
                grid_of.insert(c, 0);
                continue;
            }
        }
        let perp = (c + 9000).rem_euclid(18000);
        if let Some(&g) = grid_of.get(&perp) {
            if g != 0 && !conflict(c, perp) {
                grid_of.insert(c, g);
                continue;
            }
        }
        grid_of.insert(c, grid_yaw.len());
        grid_yaw.push(c as f64 / 100.0);
    }
    if grid_yaw.len() > 3 {
        return Err("This pattern needs more than two entities".into());
    }
    let spoke_grid = |k: i64| -> usize {
        if let Some(&g) = grid_of.get(&k) {
            return g;
        }
        let perp = (k + 9000).rem_euclid(18000);
        if let Some(&g) = grid_of.get(&perp) {
            return g;
        }
        if k == 0 || k == 9000 { 0 } else { 0 }
    };
    for sp in &spokes {
        let k = akey(ang(sp.p, sp.q));
        if !grid_of.contains_key(&k) && !grid_of.contains_key(&((k + 9000).rem_euclid(18000))) && k != 0 && k != 9000 {
            return Err("Spokes in this direction aren't supported for this pattern".into());
        }
    }
    let ngrids = grid_yaw.len();

    // 5. Pillars (corner posts) at corners.
    let ph = s.corner_posts / 2;
    let post_hh = s.wall_height / 2 + s.posts_extra / 2;
    let post_zc = if s.posts_centred { wall_base(s) + s.wall_height / 2 } else { wall_base(s) + post_hh };
    let post_kind = if s.posts_round { Kind::Pole } else { Kind::Micro };
    let mut corner_keys: Vec<(i64, i64)> = corner_pos.keys().copied().collect();
    corner_keys.sort();
    let ideal_posts: Vec<(i32, i32)> = corner_keys.iter().map(|k| (corner_pos[k].0.round() as i32, corner_pos[k].1.round() as i32)).collect();

    // Main-grid walls (local = world for the main grid).
    let main_wall_segs: Vec<_> = edges.iter().filter(|e| grid_of[&akey(ang(e.0, e.1))] == 0).copied().collect();
    let main_spoke_segs: Vec<_> = spokes.iter().filter(|sp| spoke_grid(akey(ang(sp.p, sp.q))) == 0).map(|sp| (sp.p, sp.q)).collect();
    let main_wall_runs = make_runs(&main_wall_segs);
    let main_spoke_runs = make_runs(&main_spoke_segs);

    let mut posts: Vec<(i32, i32)> = Vec::new();
    let mut posts_entity = false;
    if s.corner_posts > 0 {
        // Line each pillar up with the main-grid wall it sits on, so the wall
        // pieces either side come out a whole number of micro bricks long.
        let w2 = thick / 2;
        let mut p2 = ideal_posts.clone();
        for p in &mut p2 {
            for r in &main_wall_runs {
                let (pu, pv) = if r.along_x { (p.0, p.1) } else { (p.1, p.0) };
                if (pv - r.v).abs() < ph + w2 && pu + ph > r.a && pu - ph < r.b {
                    if (pu - ph - r.a).rem_euclid(2) != 0 {
                        if r.along_x { p.0 += 1 } else { p.1 += 1 }
                    }
                    break;
                }
            }
        }
        let mut sorted = p2.clone();
        sorted.sort();
        let mut ok = true;
        'o: for i in 0..sorted.len() {
            for j in i + 1..sorted.len() {
                if sorted[j].0 - sorted[i].0 >= 2 * ph {
                    break;
                }
                if (sorted[j].1 - sorted[i].1).abs() < 2 * ph {
                    ok = false;
                    break 'o;
                }
            }
        }
        if ok {
            posts = p2;
        } else {
            posts_entity = true;
            posts = ideal_posts.clone();
        }
    }
    let (post_group, post_groups) = if posts_entity { share_pillars(&posts, ph) } else { (Vec::new(), 0) };

    // 6. Turn runs into bricks.
    let zb = wall_base(s);
    let wall_bricks = |r: &Run, on_main: bool| -> Vec<B> {
        let lh = (r.b - r.a) / 2;
        let c = (r.a + r.b) / 2;
        let mut v = Vec::new();
        wall(s, 0, 0, zb, lh, !s.flip_wedges, &mut v);
        v.into_iter()
            .map(|mut b| {
                if on_main && upside {
                    b = flip_x(b);
                }
                if r.along_x {
                    b = rot90(b);
                }
                let (x, y) = if r.along_x { (c, r.v) } else { (r.v, c) };
                b.pos = (b.pos.0 + x, b.pos.1 + y, b.pos.2);
                b
            })
            .collect()
    };
    let st = (s.spoke_thickness / 2).max(1);
    let spoke_z = if s.spoke_top { zb + s.wall_height + st } else { st };
    let spoke_b = |r: &Run, on_main: bool| -> B {
        let lh = (r.b - r.a) / 2;
        let c = (r.a + r.b) / 2;
        let (x, y) = if r.along_x { (c, r.v) } else { (r.v, c) };
        let mut b = B { pillar: false, kind: Kind::Micro, size: (st, lh, st), pos: (0, 0, spoke_z), ori: if r.along_x { 19 } else { 16 } };
        if on_main && upside {
            b = flip_x(b);
        }
        b.pos = (b.pos.0 + x, b.pos.1 + y, b.pos.2);
        b
    };
    // Spokes crossing at right angles in one grid: the Y spokes stop at the X spokes.
    let uncross = |runs: Vec<Run>| -> Vec<Run> {
        let xs: Vec<Run> = runs.iter().filter(|r| r.along_x).copied().collect();
        let mut out: Vec<Run> = xs.clone();
        for r in runs.into_iter().filter(|r| !r.along_x) {
            let mut parts = vec![r];
            for x in &xs {
                if r.v + st > x.a && r.v - st < x.b {
                    parts = parts.into_iter().flat_map(|p| cut(p, x.v - st, x.v + st)).collect();
                }
            }
            out.extend(parts);
        }
        out
    };
    // Main-grid pieces stop at pillars that sit on the main grid.
    let w2 = thick / 2;
    let post_lo = post_zc - post_hh;
    let post_hi = post_zc + post_hh;
    let cut_posts = |runs: Vec<Run>, half_w: i32, z0: i32, z1: i32| -> Vec<Run> {
        if s.corner_posts == 0 || posts_entity || z1 <= post_lo || z0 >= post_hi {
            return runs;
        }
        let mut out = Vec::new();
        for r in runs {
            let mut parts = vec![r];
            for &p in &posts {
                let (pu, pv) = if r.along_x { (p.0, p.1) } else { (p.1, p.0) };
                if (pv - r.v).abs() < ph + half_w {
                    parts = parts.into_iter().flat_map(|q| cut(q, pu - ph, pu + ph)).collect();
                }
            }
            out.extend(parts);
        }
        out
    };
    let spoke_z0 = spoke_z - st;
    let spoke_z1 = spoke_z + st;
    let mut main: Vec<B> = Vec::new();
    for r in cut_posts(main_wall_runs, w2.max(1), zb, zb + s.wall_height) {
        main.extend(wall_bricks(&r, true));
    }
    for r in cut_posts(uncross(main_spoke_runs), st, spoke_z0, spoke_z1) {
        main.push(spoke_b(&r, true));
    }
    if s.corner_posts > 0 && !posts_entity {
        for &(px, py) in &posts {
            let (z, ori) = if upside { (-post_zc, 20u8) } else { (post_zc, 16u8) };
            main.push(B { pillar: true, kind: post_kind, size: (ph, ph, post_hh), pos: (px, py, z), ori });
        }
    }

    // 7. Bearing spots: never inside main-grid bricks; preferably clear of entity bricks too.
    let mut taken: Vec<((f64, f64), (f64, f64))> = Vec::new();
    for b in &main {
        let h = half(b);
        let (z0, z1) = (b.pos.2 - h.2, b.pos.2 + h.2);
        let in_band = if upside { z0 < 0 && z1 > -2 } else { z0 < 2 && z1 > 0 };
        if in_band {
            taken.push((((b.pos.0 - h.0) as f64, (b.pos.1 - h.1) as f64), ((b.pos.0 + h.0) as f64, (b.pos.1 + h.1) as f64)));
        }
    }
    let hard = taken.len();
    // Entity walls and spokes are slanted, so test the distance to each one
    // (a box around a slanted wall would block the whole cell).
    let mut segs: Vec<((f64, f64), (f64, f64), f64)> = Vec::new();
    if zb < 2 {
        for e in edges.iter().filter(|e| grid_of[&akey(ang(e.0, e.1))] != 0) {
            segs.push((e.0, e.1, w2.max(1) as f64));
        }
    }
    if !s.spoke_top {
        for sp in spokes.iter().filter(|sp| spoke_grid(akey(ang(sp.p, sp.q))) != 0) {
            segs.push((sp.p, sp.q, st as f64));
        }
    }
    if posts_entity && post_zc - post_hh < 2 {
        for &(px, py) in &posts {
            taken.push((((px - ph) as f64 - 0.1, (py - ph) as f64 - 0.1), ((px + ph) as f64 + 0.1, (py + ph) as f64 + 0.1)));
        }
    }
    let seg_dist = |x: f64, y: f64, p: (f64, f64), q: (f64, f64)| -> f64 {
        let (dx, dy) = (q.0 - p.0, q.1 - p.1);
        let l2 = dx * dx + dy * dy;
        let t = if l2 > 0.0 { (((x - p.0) * dx + (y - p.1) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
        (x - p.0 - t * dx).hypot(y - p.1 - t * dy)
    };
    let n = kept.len() as f64;
    let mx = kept.iter().map(|f| f.c.0).sum::<f64>() / n;
    let my = kept.iter().map(|f| f.c.1).sum::<f64>() / n;
    // Buckets so each test only looks at nearby bricks.
    const BK: f64 = 16.0;
    let bk = |x: f64| (x / BK).floor() as i32;
    let mut box_b: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (i, (a, b)) in taken.iter().enumerate() {
        for gx in bk(a.0)..=bk(b.0) {
            for gy in bk(a.1)..=bk(b.1) {
                box_b.entry((gx, gy)).or_default().push(i);
            }
        }
    }
    let mut seg_b: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for (i, &(p, q, hw)) in segs.iter().enumerate() {
        let m = hw + 2.2;
        for gx in bk(p.0.min(q.0) - m)..=bk(p.0.max(q.0) + m) {
            for gy in bk(p.1.min(q.1) - m)..=bk(p.1.max(q.1) + m) {
                seg_b.entry((gx, gy)).or_default().push(i);
            }
        }
    }
    let empty: Vec<usize> = Vec::new();
    let free_with = |soft: bool, bx: i32, by: i32, others: &[(i32, i32)]| -> bool {
        let lo = ((bx - 1) as f64, (by - 1) as f64);
        let hi = ((bx + 1) as f64, (by + 1) as f64);
        let cellk = (bk(bx as f64), bk(by as f64));
        let near_boxes = box_b.get(&cellk).unwrap_or(&empty);
        let hit_box = near_boxes.iter().any(|&i| {
            let (a, b) = taken[i];
            (soft || i < hard) && hi.0 > a.0 && lo.0 < b.0 && hi.1 > a.1 && lo.1 < b.1
        });
        // a bearing (2 units wide) may straddle a bucket edge: check the neighbours too
        let hit_box = hit_box || [(bx - 1, by - 1), (bx + 1, by - 1), (bx - 1, by + 1), (bx + 1, by + 1)].iter().any(|&(x, y)| {
            let k = (bk(x as f64), bk(y as f64));
            k != cellk && box_b.get(&k).unwrap_or(&empty).iter().any(|&i| {
                let (a, b) = taken[i];
                (soft || i < hard) && hi.0 > a.0 && lo.0 < b.0 && hi.1 > a.1 && lo.1 < b.1
            })
        });
        !hit_box
            && (!soft || !seg_b.get(&cellk).unwrap_or(&empty).iter().any(|&i| {
                let (p, q, hw) = segs[i];
                seg_dist(bx as f64, by as f64, p, q) < hw + 2.2
            }))
            && !others.iter().any(|(ox, oy)| (ox - bx).abs() < 3 && (oy - by).abs() < 3)
    };
    // Corners where walls from different grids meet (for lining them up).
    let mut meet: Vec<((f64, f64), Vec<usize>)> = Vec::new();
    for k in &corner_keys {
        let mut gs: Vec<usize> = corner_dirs[k].iter().map(|c| grid_of[c]).collect();
        gs.sort();
        gs.dedup();
        if gs.len() >= 2 {
            meet.push((corner_pos[k], gs));
        }
    }
    let ent_grids = ngrids - 1;
    let need = ent_grids + post_groups;
    let limit = (if s.best_bearings { (3_000_000 / meet.len().max(1)).clamp(150, 2500) } else { 50 }).max(need * 20);
    let find_spots = |soft: bool| -> Vec<(i32, i32)> {
        let mut spots = Vec::new();
        'search: for rad in 0..300i32 {
            for dx in -rad..=rad {
                for dy in -rad..=rad {
                    if dx.abs() != rad && dy.abs() != rad {
                        continue;
                    }
                    let (bx, by) = (mx.round() as i32 + dx, my.round() as i32 + dy);
                    // Keep bearings inside the pattern's area, where the walls hide them.
                    if in_area(s, bx as f64, by as f64) && free_with(soft, bx, by, &[]) {
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
    let mut use_soft = true;
    let mut spots = find_spots(true);
    if spots.len() < need * 10 {
        use_soft = false;
        spots = find_spots(false);
    }
    let free = |bx: i32, by: i32, others: &[(i32, i32)]| free_with(use_soft, bx, by, others);
    if need > 0 && spots.is_empty() {
        return Err("No free spot for a bearing".into());
    }
    let mut chosen: Vec<(i32, i32)> = Vec::new();
    for _ in 0..need {
        let pick = *spots.iter().find(|p| free(p.0, p.1, &chosen)).ok_or("No free spot for a bearing")?;
        chosen.push(pick);
    }
    // Rounding error of each corner in each grid (world units).
    let err_in = |g: usize, o: (i32, i32), p: (f64, f64)| -> (f64, f64) {
        if g == 0 {
            return (p.0.round() - p.0, p.1.round() - p.1);
        }
        let m = ent_matrix(grid_yaw[g], upside);
        let t = mul_t(&m, (p.0 - o.0 as f64, p.1 - o.1 as f64, 0.0));
        let e = mul(&m, (t.0.round() - t.0, t.1.round() - t.1, 0.0));
        (e.0, e.1)
    };
    let cost = |ch: &[(i32, i32)]| -> (f64, f64, f64) {
        let (mut sum, mut worst, mut tot) = (0.0f64, 0.0f64, 0.0f64);
        for (p, gs) in &meet {
            let es: Vec<(f64, f64)> = gs.iter().map(|&g| err_in(g, if g == 0 { (0, 0) } else { ch[g - 1] }, *p)).collect();
            let k = es.len() as f64;
            let (ex, ey) = (es.iter().map(|e| e.0).sum::<f64>() / k, es.iter().map(|e| e.1).sum::<f64>() / k);
            let mut spread: f64 = 0.0;
            for e in &es {
                let d2 = (e.0 - ex).powi(2) + (e.1 - ey).powi(2);
                sum += d2;
                spread = spread.max(d2.sqrt());
            }
            worst = worst.max(spread);
            tot += spread;
        }
        (sum, worst, if meet.is_empty() { 0.0 } else { tot / meet.len() as f64 })
    };
    if s.best_bearings && ent_grids > 0 {
        for _pass in 0..2 {
            for j in 0..ent_grids {
                let others: Vec<(i32, i32)> = chosen.iter().enumerate().filter(|(i, _)| *i != j).map(|(_, p)| *p).collect();
                let mut best = (cost(&chosen).0, chosen[j]);
                let mut trial = chosen.clone();
                for &sp in &spots {
                    if !free(sp.0, sp.1, &others) {
                        continue;
                    }
                    trial[j] = sp;
                    let c = cost(&trial).0;
                    if c < best.0 - 1e-9 {
                        best = (c, sp);
                    }
                }
                chosen[j] = best.1;
            }
        }
    }
    let (_, corner_worst, corner_mean) = cost(&chosen);

    // 8. Entity grids.
    let mut entities = Vec::new();
    let mut max_move: f64 = 0.0;
    for g in 1..ngrids {
        let yaw = grid_yaw[g];
        let m = ent_matrix(yaw, upside);
        let (bx, by) = chosen[g - 1];
        let o = (bx as f64, by as f64);
        let loc = |p: (f64, f64)| {
            let t = mul_t(&m, (p.0 - o.0, p.1 - o.1, 0.0));
            (t.0, t.1)
        };
        let wsegs: Vec<_> = edges.iter().filter(|e| grid_of[&akey(ang(e.0, e.1))] == g).map(|e| (loc(e.0), loc(e.1))).collect();
        let ssegs: Vec<_> = spokes.iter().filter(|sp| spoke_grid(akey(ang(sp.p, sp.q))) == g).map(|sp| (loc(sp.p), loc(sp.q))).collect();
        let mut wruns = make_runs(&wsegs);
        if s.corner_posts > 0 && !s.walls_through {
            // Stop entity walls at the pillars too.
            let lp: Vec<(i32, i32)> = posts.iter().map(|p| {
                let t = loc((p.0 as f64, p.1 as f64));
                (t.0.round() as i32, t.1.round() as i32)
            }).collect();
            let mut out = Vec::new();
            for r in wruns {
                let mut parts = vec![r];
                for &p in &lp {
                    let (pu, pv) = if r.along_x { (p.0, p.1) } else { (p.1, p.0) };
                    if (pv - r.v).abs() < ph + w2.max(1) {
                        parts = parts.into_iter().flat_map(|q| cut(q, pu - ph, pu + ph)).collect();
                    }
                }
                out.extend(parts);
            }
            wruns = out;
        }
        let mut bricks = Vec::new();
        for r in &wruns {
            bricks.extend(wall_bricks(r, false));
        }
        for r in uncross(make_runs(&ssegs)) {
            bricks.push(spoke_b(&r, false));
        }
        let mut mv: f64 = 0.0;
        for (p, gs) in &meet {
            if gs.contains(&g) {
                let e = err_in(g, (bx, by), *p);
                mv = mv.max(e.0.hypot(e.1));
            }
        }
        max_move = max_move.max(mv);
        let angle = if upside { norm(yaw - 180.0) } else { norm(-yaw) };
        entities.push(Grid { yaw: Some(yaw), bricks, origin: (bx, by, 0), bearing: (bx, by, if upside { -1 } else { 1 }), angle, max_move: mv, pillars: false });
    }
    for gi in 0..post_groups {
        let (bx, by) = chosen[ent_grids + gi];
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
        entities.push(Grid { yaw: Some(0.0), bricks, origin: (bx, by, 0), bearing: (bx, by, if upside { -1 } else { 1 }), angle, max_move: 0.0, pillars: true });
    }
    let angles = grid_yaw.iter().map(|a| a.rem_euclid(180.0)).collect();
    Ok(Built { main, entities, cells: kept.len(), max_move, angles, corner_mean, corner_worst, posts_entity: post_groups })
}

/// Spoke directions this pattern can have (degrees, sorted). Spoke tick box
/// k covers every direction whose place in this list, divided by 3, leaves k.
pub fn spoke_families(s: &Settings) -> Vec<f64> {
    if s.pattern == Pattern::Honeycomb {
        return if s.turn == Turn::WallsAcross { vec![90.0, 150.0, 30.0] } else { vec![0.0, 60.0, 120.0] };
    }
    let small = Settings { width: s.cell * 2, length: s.cell * 2, ..s.clone() };
    let mut fams: Vec<i64> = Vec::new();
    for fc in faces(&small) {
        let n = fc.pts.len();
        if n % 2 != 0 {
            continue;
        }
        for k in 0..n / 2 {
            let (p0, p1) = (fc.pts[k], fc.pts[k + 1]);
            let (q0, q1) = (fc.pts[k + n / 2], fc.pts[(k + n / 2 + 1) % n]);
            let a1 = (p1.1 - p0.1).atan2(p1.0 - p0.0).to_degrees().rem_euclid(180.0);
            let a2 = (q1.1 - q0.1).atan2(q1.0 - q0.0).to_degrees().rem_euclid(180.0);
            let k1 = ((a1 * 100.0).round() as i64).rem_euclid(18000);
            let k2 = ((a2 * 100.0).round() as i64).rem_euclid(18000);
            if k1 != k2 {
                continue;
            }
            let sp = (k1 + 9000).rem_euclid(18000);
            if !fams.contains(&sp) {
                fams.push(sp);
            }
        }
    }
    fams.sort();
    fams.into_iter().map(|k| k as f64 / 100.0).collect()
}
