//! Writes the stage-1 in-game test prefabs into ./testpack
use patterngen::*;

fn main() {
    let d = Settings::default();
    let tests: Vec<(&str, Settings)> = vec![
        ("T01_Default_Honeycomb", d.clone()),
        ("T02_Micro_Brick_Walls", Settings { style: WallStyle::MicroBrick, ..d.clone() }),
        ("T03_Spokes_Every_Cell", Settings { spokes: SpokeMode::EveryCell, ..d.clone() }),
        ("T04_No_Spokes", Settings { spokes: SpokeMode::None, ..d.clone() }),
        ("T05_Spokes_Alt_Rows_On_Top", Settings { spokes: SpokeMode::AltRows, spoke_top: true, ..d.clone() }),
        ("T06_Circle_30_Studs", Settings { area: Area::Circle, width: 300, length: 300, ..d.clone() }),
        ("T07_Oval_50x30_Studs", Settings { area: Area::Oval, width: 500, length: 300, ..d.clone() }),
        ("T08_Upside_Down", Settings { way_up: WayUp::UpsideDown, ..d.clone() }),
        ("T09_Flip_Wedges", Settings { flip_wedges: true, ..d.clone() }),
        ("T10_4_Stud_Cells_Brick_Walls", Settings { cell: 40, wall_height: 12, style: WallStyle::MicroBrick, wall_thickness: 4, width: 600, length: 600, ..d.clone() }),
        ("T11_Walls_Across", Settings { turn: Turn::WallsAcross, ..d.clone() }),
        ("T12_Metallic_Shiny", Settings { material: 4, intensity: 10, color: [0x7B, 0x9B, 0xE6], ..d.clone() }),
    ];
    std::fs::create_dir_all("testpack").unwrap();
    for (name, mut s) in tests {
        s.name = name.into();
        let path = std::path::PathBuf::from(format!("testpack/{name}.brz"));
        match write_brz(&s, &path) {
            Ok(b) => {
                let probs = check::check(&s, &b);
                println!(
                    "{name}: {} bricks, {} entities, {} cells, largest move {:.2}, angles {:?}, bearings {:?} {}",
                    b.brick_count(), b.entities.len(), b.cells, b.max_move,
                    b.entities.iter().map(|g| g.angle).collect::<Vec<_>>(),
                    b.entities.iter().map(|g| g.bearing).collect::<Vec<_>>(),
                    if probs.is_empty() { "OK".to_string() } else { probs.join("; ") }
                );
            }
            Err(e) => println!("{name}: ERROR {e}"),
        }
    }
}
