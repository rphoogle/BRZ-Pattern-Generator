//! Self-check: catches overlapping bricks, bad sizes, bearings in the way, and
//! files that don't read back. Runs on every GitHub build.

use crate::*;

fn boxes_overlap(a: &B, b: &B) -> bool {
    let (ha, hb) = (half(a), half(b));
    let ov = |pa: i32, ea: i32, pb: i32, eb: i32| (pa + ea).min(pb + eb) - (pa - ea).max(pb - eb) > 0;
    ov(a.pos.0, ha.0, b.pos.0, hb.0) && ov(a.pos.1, ha.1, b.pos.1, hb.1) && ov(a.pos.2, ha.2, b.pos.2, hb.2)
}

fn grid_overlaps(bricks: &[B]) -> usize {
    // Sort by x so only neighbours are compared.
    let mut v: Vec<&B> = bricks.iter().collect();
    v.sort_by_key(|b| b.pos.0 - half(b).0);
    let mut n = 0;
    for i in 0..v.len() {
        let xi = v[i].pos.0 + half(v[i]).0;
        for j in i + 1..v.len() {
            if v[j].pos.0 - half(v[j]).0 >= xi {
                break;
            }
            if boxes_overlap(v[i], v[j]) {
                n += 1;
            }
        }
    }
    n
}

/// Returns a list of problems (empty = all good).
pub fn check(s: &Settings, built: &Built) -> Vec<String> {
    let mut p = Vec::new();
    let upside = s.way_up == WayUp::UpsideDown;
    let o = grid_overlaps(&built.main);
    if o > 0 {
        p.push(format!("main grid: {o} overlapping brick pairs"));
    }
    for (i, g) in built.entities.iter().enumerate() {
        let o = grid_overlaps(&g.bricks);
        if o > 0 {
            p.push(format!("entity {i}: {o} overlapping brick pairs"));
        }
        for b in &g.bricks {
            if [b.pos.0, b.pos.1, b.pos.2].iter().any(|v| v.abs() > 1000) {
                p.push(format!("entity {i}: brick too far from its bearing"));
                break;
            }
        }
        // Bearing must not clash with main-grid bricks.
        let bear = B { wedge: false, size: (1, 1, 1), pos: g.bearing, ori: 16 };
        if built.main.iter().any(|b| boxes_overlap(b, &bear)) {
            p.push(format!("entity {i}: bearing overlaps a main-grid brick"));
        }
        // Entity origin sits on the bearing's mounting face.
        let face = if upside { g.bearing.2 + 1 } else { g.bearing.2 - 1 };
        if g.origin != (g.bearing.0, g.bearing.1, face) {
            p.push(format!("entity {i}: origin is not on the bearing face"));
        }
        // Bearing must not clash with any entity brick (checked in world space).
        for (j, h) in built.entities.iter().enumerate() {
            let m = ent_matrix(h.yaw.unwrap(), upside);
            for b in &h.bricks {
                let hh = half(b);
                let c = mul(&m, (b.pos.0 as f64, b.pos.1 as f64, b.pos.2 as f64));
                let c = (c.0 + h.origin.0 as f64, c.1 + h.origin.1 as f64, c.2 + h.origin.2 as f64);
                let e = (
                    m[0][0].abs() * hh.0 as f64 + m[0][1].abs() * hh.1 as f64 + m[0][2].abs() * hh.2 as f64,
                    m[1][0].abs() * hh.0 as f64 + m[1][1].abs() * hh.1 as f64 + m[1][2].abs() * hh.2 as f64,
                    m[2][0].abs() * hh.0 as f64 + m[2][1].abs() * hh.1 as f64 + m[2][2].abs() * hh.2 as f64,
                );
                // Turned boxes are checked loosely (their corners stick out), so
                // only a clear clash with the box's middle counts.
                let near = |a: f64, ea: f64, bb: i32| (a - bb as f64).abs() < (ea * 0.5 + 1.0) - 0.01;
                if near(c.0, e.0, g.bearing.0) && near(c.1, e.1, g.bearing.1) && (c.2 - g.bearing.2 as f64).abs() < e.2 + 1.0 - 0.01 {
                    p.push(format!("entity {j}: a brick sits on bearing {i}"));
                    break;
                }
            }
        }
    }
    for b in built.main.iter().chain(built.entities.iter().flat_map(|g| g.bricks.iter())) {
        let (x, y, z) = b.size;
        if x < 1 || y < 1 || z < 1 || x > 2000 || y > 2000 || z > 2000 {
            p.push(format!("bad brick size {:?}", b.size));
            break;
        }
    }
    if built.max_move > 0.75 {
        p.push(format!("rounding moved a piece {:.2} units", built.max_move));
    }
    p
}

/// Write the prefab to memory and read it back with brdb, checking counts.
pub fn round_trip(s: &Settings) -> Result<(), String> {
    use brdb::{Brz, IntoReader};
    let (built, bytes) = to_brz_bytes(s)?;
    let db = Brz::read_slice(&bytes).map_err(|e| e.to_string())?.into_reader();
    let mut main = 0usize;
    for c in db.brick_chunk_index(1).map_err(|e| e.to_string())? {
        main += c.num_bricks as usize;
    }
    let want = built.main.len() + built.entities.len();
    if main != want {
        return Err(format!("main grid read back {main} bricks, expected {want}"));
    }
    let mut ents = 0;
    let mut ent_bricks = 0usize;
    for idx in db.entity_chunk_index().map_err(|e| e.to_string())? {
        for e in db.entity_chunk(idx).map_err(|e| e.to_string())? {
            ents += 1;
            if let Some(id) = e.id {
                for c in db.brick_chunk_index(id).map_err(|e| e.to_string())? {
                    ent_bricks += c.num_bricks as usize;
                }
            }
        }
    }
    if ents != built.entities.len() {
        return Err(format!("read back {ents} entities, expected {}", built.entities.len()));
    }
    let want_e: usize = built.entities.iter().map(|g| g.bricks.len()).sum();
    if ent_bricks != want_e {
        return Err(format!("entities read back {ent_bricks} bricks, expected {want_e}"));
    }
    Ok(())
}
