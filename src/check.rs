//! Self-check: catches overlapping bricks, bad sizes, bearings in the way, and
//! files that don't read back. Runs on every GitHub build.

use crate::*;

pub(crate) fn boxes_overlap(a: &B, b: &B) -> bool {
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
        // Bearing must not clash with main-grid bricks.
        let bear = B { pillar: false, kind: Kind::Micro, size: (1, 1, 1), pos: g.bearing, ori: 16 };
        if built.main.iter().any(|b| boxes_overlap(b, &bear)) {
            p.push(format!("entity {i}: bearing overlaps a main-grid brick"));
        }
        // Entity origin sits on the bearing's mounting face.
        let face = if upside { g.bearing.2 + 1 } else { g.bearing.2 - 1 };
        if g.origin != (g.bearing.0, g.bearing.1, face) {
            p.push(format!("entity {i}: origin is not on the bearing face"));
        }
        // (Entity bricks may pass through a bearing: entities can overlap other grids.)
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

/// Things worth knowing that don't stop a save.
pub fn warnings(s: &Settings, built: &Built) -> Vec<String> {
    let mut w = Vec::new();
    let far = built.entities.iter().any(|g| g.bricks.iter().any(|b| [b.pos.0, b.pos.1].iter().any(|v| v.abs() > 1000)));
    if far {
        w.push("Some entity bricks are over 100 studs from their bearing. This is fine; just noting it.".into());
    }
    if built.layers > 0 {
        w.push(format!("Walls this thick overlap, so {} extra entit{} at the same angles hold the overlapping parts.", built.layers, if built.layers == 1 { "y" } else { "ies" }));
    }
    if s.include_entities && !s.include_bearings {
        w.push("Entities without bearings will paste frozen and won't be held at their angle if unfrozen.".into());
    }
    w
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
