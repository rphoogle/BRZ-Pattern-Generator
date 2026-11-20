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
    let tests: Vec<(&str, Settings)> = vec![
        ("V01_Triangles", Settings { pattern: Pattern::Triangles, ..d.clone() }),
        ("V02_Star_Of_David_Spokes", Settings { pattern: Pattern::Stars, ..d.clone() }),
        ("V03_Cube_Illusion", Settings { pattern: Pattern::Cubes, spokes: SpokeMode::None, ..d.clone() }),
        ("V04_Octagons_Squares_Spokes_Every_Cell", Settings { pattern: Pattern::Octagons, spokes: SpokeMode::EveryCell, ..d.clone() }),
        ("V05_Diamonds", Settings { pattern: Pattern::Diamonds, ..d.clone() }),
        ("V06_Triangles_Brick_Walls_90", Settings { pattern: Pattern::Triangles, style: WallStyle::MicroBrick, wall_thickness: 4, turn: Turn::WallsAcross, ..d.clone() }),
        ("V07_Stars_Upside_Down_Pillars", Settings { pattern: Pattern::Stars, way_up: WayUp::UpsideDown, corner_posts: 4, posts_extra: 4, ..d.clone() }),
        ("V08_Octagons_90_Upside_Down_Square_Pillars", Settings { pattern: Pattern::Octagons, turn: Turn::WallsAcross, way_up: WayUp::UpsideDown, corner_posts: 4, posts_round: false, cell: 30, ..d.clone() }),
        ("V09_Cubes_Colours_Each_Part", Settings { pattern: Pattern::Cubes, one_paint: false, paints, corner_posts: 4, spokes: SpokeMode::None, ..d.clone() }),
        ("V10_Diamonds_Wide_Pillars", Settings { pattern: Pattern::Diamonds, corner_posts: 16, ..d.clone() }),
        ("V11_Honeycomb_Unchanged", d.clone()),
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
