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
                                for &(top, posts) in &[(false, 0), (true, 0), (false, 2), (true, 4)] {
                                    let s = Settings {
                                        style, spokes, area, way_up, turn, cell, flip_wedges: flip, spoke_top: top, corner_posts: posts, best_bearings: cell == 20 && flip,
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
                                        Err(e) => { bad += 1; if bad < 15 { println!("ERROR {e}"); } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    for s in [Settings::default(), Settings { way_up: WayUp::UpsideDown, ..Settings::default() }] {
        if let Err(e) = check::round_trip(&s) { bad += 1; println!("ROUND TRIP: {e}"); }
    }
    println!("{n} layouts built, {bad} problems");
    if bad > 0 { std::process::exit(1); }
}
