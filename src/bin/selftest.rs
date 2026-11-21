//! Builds many combinations of settings and checks each one.
use patterngen::*;

fn main() {
    let mut n = 0;
    let mut bad = 0;
    for &style in &[WallStyle::MicroWedge, WallStyle::MicroBrick] {
        for &spokes in &[SpokeMode::None, SpokeMode::EveryCell, SpokeMode::AltRows, SpokeMode::Custom] {
            for &area in &[Area::Hexagon, Area::Rectangle, Area::Circle, Area::Oval] {
                for &way_up in &[WayUp::Upright, WayUp::UpsideDown] {
                    for &turn in &[Turn::WallsUpright, Turn::WallsAcross] {
                        for &cell in &[10, 20, 30, 40] {
                            for &flip in &[false, true] {
                                for &(top, posts, round, through, extra, wt) in &[(false, 0, false, true, 0, 4), (true, 0, false, true, 0, 8), (false, 2, true, true, 4, 4), (true, 4, false, false, -2, 12), (false, 4, true, true, 0, 4), (false, 16, true, true, 0, 8), (true, 40, false, false, 0, 4)] {
                                    let s = Settings {
                                        style, spokes, area, way_up, turn, cell, flip_wedges: flip, spoke_top: top, corner_posts: posts, wedge_thickness: wt.min(cell / 2 / 4 * 4).max(4), posts_round: round, walls_through: through, posts_extra: extra, best_bearings: cell == 20 && flip,
                                        width: cell * 8, length: cell * 6,
                                        ..Settings::default()
                                    };
                                    n += 1;
                                    match build(&s) {
                                        Ok(b) => {
                                            let mut p = check::check(&s, &b);
                                            if b.layers > 0 { p.push(format!("{} extra layer entities at a normal thickness", b.layers)); }
                                            if !p.is_empty() {
                                                bad += 1;
                                                if true { println!("PROBLEM {:?} {:?} {:?} {:?} {:?} cell {} flip {} top {} posts {}: {}", style, spokes, area, way_up, turn, cell, flip, top, posts, p.join("; ")); }
                                            }
                                        }
                                        Err(e) => { bad += 1; if bad < 400 { println!("ERROR {e} | {:?} {:?} {:?} {:?} {:?} cell {} top {} posts {} wt {}", style, spokes, area, way_up, turn, cell, top, posts, wt); } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // The other patterns.
    let mut skipped = 0;
    for p in Pattern::ALL.iter().skip(1) {
        for &style in &[WallStyle::MicroWedge, WallStyle::MicroBrick] {
            for &spokes in &[SpokeMode::None, SpokeMode::EveryCell, SpokeMode::AltRows, SpokeMode::Custom] {
                for &area in &[Area::Hexagon, Area::Rectangle, Area::Circle, Area::Oval] {
                    for &way_up in &[WayUp::Upright, WayUp::UpsideDown] {
                        for &turn in &[Turn::WallsUpright, Turn::WallsAcross] {
                            for &cell in &[10, 20, 40] {
                                for &(top, posts, round, through, extra, flip) in &[(false, 0, false, true, 0, false), (true, 4, true, false, 4, true), (false, 16, false, true, -2, false)] {
                                    let s = Settings {
                                        pattern: *p, style, spokes, area, way_up, turn, cell, flip_wedges: flip, spoke_top: top, corner_posts: posts, posts_round: round, walls_through: through, posts_extra: extra,
                                        wall_thickness: 2, best_bearings: cell == 20,
                                        width: cell * 8, length: cell * 6,
                                        ..Settings::default()
                                    };
                                    n += 1;
                                    match build(&s) {
                                        Ok(b) => {
                                            let mut pr = check::check(&s, &b);
                                            let thick = if style == WallStyle::MicroWedge { s.wedge_thickness } else { s.wall_thickness } as f64;
                                            let old_ok = thick <= patterns::min_gap(*p) * cell as f64 - 2.0;
                                            if old_ok && b.layers > 0 { pr.push(format!("{} extra layer entities at a normal thickness", b.layers)); }
                                            if !old_ok { skipped += 1; }
                                            if !pr.is_empty() {
                                                bad += 1;
                                                if bad < 400 { println!("PROBLEM {:?} {:?} {:?} {:?} {:?} {:?} cell {} top {} posts {}: {}", p, style, spokes, area, way_up, turn, cell, top, posts, pr.join("; ")); }
                                            }
                                        }
                                        Err(e) => { bad += 1; if bad < 400 { println!("ERROR {e} | {:?} {:?} {:?} {:?} {:?} {:?} cell {} posts {}", p, style, spokes, area, way_up, turn, cell, posts); } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    println!("({skipped} of those had walls thicker than the old limit, now held by extra entities)");
    // Very thick walls on every pattern: overlaps must all be moved to extra entities.
    for p in Pattern::ALL {
        for &style in &[WallStyle::MicroWedge, WallStyle::MicroBrick] {
            for &way_up in &[WayUp::Upright, WayUp::UpsideDown] {
                for &(cell, thick) in &[(20, 12), (20, 24), (40, 40), (10, 16)] {
                    for &(posts, moe) in &[(0, false), (4, true), (6, false)] {
                        let s = Settings { pattern: p, style, way_up, cell, wall_thickness: thick, wedge_thickness: thick / 4 * 4, corner_posts: posts, main_on_entity: moe, width: cell * 6, length: cell * 5, best_bearings: false, ..Settings::default() };
                        n += 1;
                        match build(&s) {
                            Ok(b) => {
                                let pr = check::check(&s, &b);
                                if !pr.is_empty() { bad += 1; println!("THICK PROBLEM {:?} {:?} {:?} cell {} thick {} posts {} moe {}: {}", p, style, way_up, cell, thick, posts, moe, pr.join("; ")); }
                            }
                            Err(e) => { bad += 1; println!("THICK ERROR {e} | {:?} {:?} {:?} cell {} thick {}", p, style, way_up, cell, thick); }
                        }
                    }
                }
            }
        }
    }
    // Main walls on their own entity, every pattern, with pillars running through.
    for p in Pattern::ALL {
        for &way_up in &[WayUp::Upright, WayUp::UpsideDown] {
            for &turn in &[Turn::WallsUpright, Turn::WallsAcross] {
                let s = Settings { pattern: p, way_up, turn, main_on_entity: true, corner_posts: 4, ..Settings::default() };
                n += 1;
                match build(&s) {
                    Ok(b) => {
                        let mut pr = check::check(&s, &b);
                        if b.main.iter().any(|x| !x.pillar) && p != Pattern::Diamonds { pr.push("main grid still has walls".into()); }
                        if !pr.is_empty() { bad += 1; println!("MAIN-ENTITY PROBLEM {:?} {:?} {:?}: {}", p, way_up, turn, pr.join("; ")); }
                    }
                    Err(e) => { bad += 1; println!("MAIN-ENTITY ERROR {e} | {:?} {:?} {:?}", p, way_up, turn); }
                }
            }
        }
    }
    // Palettes: read a small palette, check colours convert both ways.
    let txt = r#"{"formatVersion":"1","presetVersion":"1","type":"ColorPalette","data":{"groups":[{"colors":[{"b":255,"g":255,"r":255,"a":255},{"b":9,"g":5,"r":87,"a":255}],"name":"Test"}],"description":"x"}}"#;
    match palette::parse("t", txt) {
        Ok(pal) => {
            let back = palette::parse("t", &palette::to_text(&pal)).unwrap();
            if pal.groups[0].colors[0] != [255, 255, 255] || back != pal { bad += 1; println!("PALETTE round trip changed colours: {:?} {:?}", pal, back); }
        }
        Err(e) => { bad += 1; println!("PALETTE {e}"); }
    }
    for p in Pattern::ALL.iter().skip(1) {
        let s = Settings { pattern: *p, corner_posts: 4, ..Settings::default() };
        if let Err(e) = check::round_trip(&s) { bad += 1; println!("ROUND TRIP {:?}: {e}", p); }
    }
    // Partial saves and per-part colours must write without errors.
    for (m, e, b) in [(true, false, false), (false, true, false), (false, false, true), (false, true, true), (true, true, false)] {
        let s = Settings { include_main: m, include_entities: e, include_bearings: b, one_paint: false, corner_posts: 30, ..Settings::default() };
        if let Err(err) = to_brz_bytes(&s) { bad += 1; println!("PARTIAL {m} {e} {b}: {err}"); }
    }
    for s in [Settings::default(), Settings { way_up: WayUp::UpsideDown, ..Settings::default() }, Settings { turn: Turn::WallsAcross, corner_posts: 30, posts_extra: 4, ..Settings::default() }] {
        if let Err(e) = check::round_trip(&s) { bad += 1; println!("ROUND TRIP: {e}"); }
    }
    println!("{n} layouts built, {bad} problems");
    if bad > 0 { std::process::exit(1); }
}
