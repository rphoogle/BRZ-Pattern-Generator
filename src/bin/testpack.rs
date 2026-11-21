//! Writes the in-game test prefabs for this version into ./testpack
use patterngen::*;

fn main() {
    let d = Settings::default();
    let mut paints = d.paints;
    paints[0].color = [0xDF, 0xE4, 0xEF];
    paints[1].color = [0x7B, 0x9B, 0xE6];
    paints[2].color = [0xF2, 0xA7, 0xB7];
    paints[3].color = [0xFF, 0xC8, 0x57];
    paints[3].material = 4;
    paints[3].intensity = 8;
    let euro = |c: [u8; 3], m: usize| Paint { color: c, material: m, intensity: if m == 4 { 4 } else { 0 } };
    let tests: Vec<(&str, Settings)> = vec![
        ("W01_Hidden_Bearings", d.clone()),
        ("W02_Visible_Bearings", Settings { hide_bearings: false, ..d.clone() }),
        ("W03_Honeycomb_Main_Walls_On_Entity_Pillars", Settings { main_on_entity: true, corner_posts: 4, ..d.clone() }),
        ("W04_Octagons_Main_Walls_On_Entity_Pillars", Settings { pattern: Pattern::Octagons, main_on_entity: true, corner_posts: 4, spokes: SpokeMode::None, ..d.clone() }),
        ("W05_Honeycomb_Thick_Brick_Walls_8M", Settings { style: WallStyle::MicroBrick, wall_thickness: 16, spokes: SpokeMode::None, ..d.clone() }),
        ("W06_Triangles_Thick_Wedge_Walls_12M", Settings { pattern: Pattern::Triangles, wedge_thickness: 24, ..d.clone() }),
        ("W07_Octagons_Thick_Brick_Walls_8M", Settings { pattern: Pattern::Octagons, style: WallStyle::MicroBrick, wall_thickness: 16, spokes: SpokeMode::None, ..d.clone() }),
        ("W08_Max_Area_Far_Bricks", Settings { pattern: Pattern::Triangles, width: 4000, length: 4000, cell: 40, ..d.clone() }),
        ("W09_Eurotown_Palette_Colours", Settings { pattern: Pattern::Stars, one_paint: false, paints: [euro([166, 64, 53], 0), euro([139, 125, 105], 0), euro([28, 71, 255], 0), euro([198, 195, 168], 4), euro([255, 255, 255], 0)], corner_posts: 4, ..d.clone() }),
    ];
    std::fs::create_dir_all("testpack").unwrap();
    for (name, mut s) in tests {
        s.name = name.into();
        let path = std::path::PathBuf::from(format!("testpack/{name}.brz"));
        match write_brz(&s, &path) {
            Ok(b) => {
                // A top-down picture to compare with the game.
                let f = footprints(&s, &b);
                let (lo, hi) = world_bounds(&b, s.way_up == WayUp::UpsideDown);
                let sc = 800.0 / (hi.0 - lo.0).max(hi.1 - lo.1);
                let cols = ["#dfe4ef", "#7b9be6", "#f2a7b7", "#9be6a0", "#e6d27b"];
                let mut svg = String::from("<svg xmlns='http://www.w3.org/2000/svg' width='840' height='840' style='background:#0d1626'>");
                for ft in &f {
                    let col = match ft.kind { FootKind::Bearing => "#ff4040", FootKind::Pillar => "#ffc857", _ => cols[ft.grid.min(4)] };
                    let op = if ft.kind == FootKind::Spoke { 0.5 } else { 0.9 };
                    let pts: Vec<String> = ft.pts.iter().map(|(x, y)| format!("{:.1},{:.1}", 20.0 + (x - lo.0) * sc, 20.0 + (hi.1 - y) * sc)).collect();
                    svg += &format!("<polygon points='{}' fill='{col}' fill-opacity='{op}'/>", pts.join(" "));
                }
                svg += "</svg>";
                std::fs::write(format!("testpack/{name}.svg"), svg).unwrap();
                let probs = check::check(&s, &b);
                println!(
                    "{name}: {} bricks, {} entities ({} for pillars), corners within {:.2}, angles {:?} {}",
                    b.brick_count(), b.entities.len(), b.posts_entity, b.corner_worst,
                    b.entities.iter().map(|g| g.angle).collect::<Vec<_>>(),
                    if probs.is_empty() { "OK".to_string() } else { probs.join("; ") }
                );
            }
            Err(e) => println!("{name}: ERROR {e}"),
        }
    }
}
