//! Final touches after a pattern is laid out.
//!
//! In plain English:
//! 1. Each entity is told which colour it uses (main walls, entity 1, entity 2, pillars).
//! 2. If "main walls on their own entity" is ticked, the main-grid walls move onto a
//!    0° entity with its own bearing, so they can run through pillars like the others.
//! 3. Bricks on one grid can't overlap. When walls are so thick that they would,
//!    the overlapping ones move to an extra entity at the same angle. Its bearing is
//!    placed so that its bricks land within a tiny fraction of a unit of where they
//!    would have been, so the walls still line up.

use crate::check::boxes_overlap;
use crate::*;

/// Move world-space main-grid bricks onto a 0° entity whose origin is `o`.
fn main_to_entity(bricks: Vec<B>, o: (i32, i32), upside: bool) -> Vec<B> {
    bricks
        .into_iter()
        .map(|mut b| {
            b.pos = (b.pos.0 - o.0, b.pos.1 - o.1, b.pos.2);
            if upside { flip_x(b) } else { b }
        })
        .collect()
}

/// Find a bearing spot near `base` for an entity at `yaw`. Returns the spot and the
/// whole-unit shift of the entity's bricks, preferring spots that keep bricks in place.
fn find_spot(built: &Built, base: (i32, i32), yaw: f64, upside: bool) -> Option<((i32, i32), (i32, i32), f64)> {
    let m = ent_matrix(yaw, upside);
    let band = |b: &B| {
        let h = half(b);
        let (z0, z1) = (b.pos.2 - h.2, b.pos.2 + h.2);
        if upside { z0 < 0 && z1 > -2 } else { z0 < 2 && z1 > 0 }
    };
    let main: Vec<&B> = built.main.iter().filter(|b| band(b)).collect();
    let bearings: Vec<(i32, i32)> = built.entities.iter().map(|g| (g.bearing.0, g.bearing.1)).collect();
    let mut cands: Vec<(u8, f64, i32, i32, (i32, i32), f64)> = Vec::new();
    const R: i32 = 200;
    for dx in -R..=R {
        for dy in -R..=R {
            let t = mul_t(&m, (dx as f64, dy as f64, 0.0));
            let l = (t.0.round() as i32, t.1.round() as i32);
            let res = (t.0 - l.0 as f64).hypot(t.1 - l.1 as f64);
            let tier = if res <= 0.1 { 0 } else if res <= 0.25 { 1 } else { 2 };
            cands.push((tier, ((dx * dx + dy * dy) as f64).sqrt(), dx, dy, l, res));
        }
    }
    cands.sort_by(|a, b| (a.0, a.1).partial_cmp(&(b.0, b.1)).unwrap());
    for (_, _, dx, dy, l, res) in cands {
        let (bx, by) = (base.0 + dx, base.1 + dy);
        if bearings.iter().any(|&(x, y)| (x - bx).abs() < 3 && (y - by).abs() < 3) {
            continue;
        }
        let bear = B { pillar: false, kind: Kind::Micro, size: (1, 1, 1), pos: (bx, by, if upside { -1 } else { 1 }), ori: 16 };
        if main.iter().any(|b| boxes_overlap(b, &bear)) {
            continue;
        }
        return Some(((bx, by), l, res));
    }
    None
}

/// Share bricks out so none overlap within a group. Bricks marked `fixed` stay in group 0.
fn layer_groups(bricks: &[B], fixed: &[bool]) -> Vec<usize> {
    let n = bricks.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| bricks[i].pos.0 - half(&bricks[i]).0);
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for oi in 0..n {
        let i = order[oi];
        let xi = bricks[i].pos.0 + half(&bricks[i]).0;
        for &j in &order[oi + 1..] {
            if bricks[j].pos.0 - half(&bricks[j]).0 >= xi {
                break;
            }
            if boxes_overlap(&bricks[i], &bricks[j]) {
                adj[i].push(j);
                adj[j].push(i);
            }
        }
    }
    let mut group = vec![usize::MAX; n];
    let mut seq: Vec<usize> = (0..n).collect();
    // fixed bricks first, then the most crowded
    seq.sort_by_key(|&i| (!fixed[i], std::cmp::Reverse(adj[i].len()), i));
    for i in seq {
        if adj[i].is_empty() {
            group[i] = 0;
            continue;
        }
        let used: std::collections::HashSet<usize> = adj[i].iter().map(|&j| group[j]).filter(|&g| g != usize::MAX).collect();
        let mut c = 0;
        while used.contains(&c) {
            c += 1;
        }
        group[i] = c;
    }
    group
}

pub fn finish(s: &Settings, mut b: Built) -> Result<Built, String> {
    let upside = s.way_up == WayUp::UpsideDown;
    let mut n = 0;
    for g in &mut b.entities {
        g.part = if g.pillars { 3 } else { n += 1; n.min(2) };
    }
    let mid = |v: &[B]| -> (i32, i32) {
        if v.is_empty() {
            return (0, 0);
        }
        let k = v.len() as i64;
        ((v.iter().map(|b| b.pos.0 as i64).sum::<i64>() / k) as i32, (v.iter().map(|b| b.pos.1 as i64).sum::<i64>() / k) as i32)
    };
    let make = |yaw: f64, o: (i32, i32), bricks: Vec<B>, part: usize, mv: f64| Grid {
        yaw: Some(yaw),
        bricks,
        origin: (o.0, o.1, 0),
        bearing: (o.0, o.1, if upside { -1 } else { 1 }),
        angle: if upside { norm(yaw - 180.0) } else { norm(-yaw) },
        max_move: mv,
        pillars: false,
        part,
    };

    let mut layers = 0;
    if s.main_on_entity {
        // Main walls go to their own 0° entity; any overlaps are sorted out with the other entities below.
        let (walls, keep): (Vec<B>, Vec<B>) = std::mem::take(&mut b.main).into_iter().partition(|x| !x.pillar);
        b.main = keep;
        if !walls.is_empty() {
            let (o, _, _) = find_spot(&b, mid(&walls), 0.0, upside).ok_or("No free spot for the main walls' bearing")?;
            let g = make(0.0, o, main_to_entity(walls, o, upside), 0, 0.0);
            b.entities.insert(0, g);
        }
    } else {
        // Overlapping main-grid walls (pillars stay put) move to extra 0° entities.
        let fixed: Vec<bool> = b.main.iter().map(|x| x.pillar).collect();
        let groups = layer_groups(&b.main, &fixed);
        let ng = groups.iter().max().map(|g| g + 1).unwrap_or(1);
        let mut main_layers: Vec<Vec<B>> = vec![Vec::new(); ng];
        for (br, g) in std::mem::take(&mut b.main).into_iter().zip(groups) {
            main_layers[g].push(br);
        }
        b.main = main_layers.remove(0);
        for extra in main_layers {
            let (o, _, _) = find_spot(&b, mid(&extra), 0.0, upside).ok_or("No free spot for a bearing")?;
            let g = make(0.0, o, main_to_entity(extra, o, upside), 0, 0.0);
            b.entities.push(g);
            layers += 1;
        }
    }

    // Overlapping walls on each entity.
    let count = b.entities.len();
    for gi in 0..count {
        if b.entities[gi].pillars {
            continue;
        }
        let fixed = vec![false; b.entities[gi].bricks.len()];
        let groups = layer_groups(&b.entities[gi].bricks, &fixed);
        let ng = groups.iter().max().map(|g| g + 1).unwrap_or(1);
        if ng <= 1 {
            continue;
        }
        let mut parts: Vec<Vec<B>> = vec![Vec::new(); ng];
        for (br, g) in std::mem::take(&mut b.entities[gi].bricks).into_iter().zip(groups) {
            parts[g].push(br);
        }
        b.entities[gi].bricks = parts.remove(0);
        let (yaw, origin, part) = (b.entities[gi].yaw.unwrap(), b.entities[gi].origin, b.entities[gi].part);
        for extra in parts {
            let (o, l, res) = find_spot(&b, (origin.0, origin.1), yaw, upside).ok_or("No free spot for a bearing")?;
            let bricks = extra.into_iter().map(|mut x| { x.pos = (x.pos.0 - l.0, x.pos.1 - l.1, x.pos.2); x }).collect();
            b.entities.push(make(yaw, o, bricks, part, res));
            b.max_move = b.max_move.max(res);
            layers += 1;
        }
    }
    b.layers = layers;
    Ok(b)
}
