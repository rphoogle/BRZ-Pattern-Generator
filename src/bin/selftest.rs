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
                                            let p = check::check(&s, &b);
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
                                            let pr = check::check(&s, &b);
                                            if !pr.is_empty() {
                                                bad += 1;
                                                if bad < 400 { println!("PROBLEM {:?} {:?} {:?} {:?} {:?} {:?} cell {} top {} posts {}: {}", p, style, spokes, area, way_up, turn, cell, top, posts, pr.join("; ")); }
                                            }
                                        }
                                        Err(e) if e.starts_with("Walls are too thick") => skipped += 1,
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
    println!("({skipped} skipped: walls too thick for the cell, as expected)");
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
