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
        ("U01_Rotation_90_Two_Entities", Settings { turn: Turn::WallsAcross, ..d.clone() }),
        ("U02_Wedge_Thickness_4M", Settings { wedge_thickness: 8, ..d.clone() }),
        ("U03_Brick_Walls_2M", Settings { style: WallStyle::MicroBrick, wall_thickness: 4, ..d.clone() }),
        ("U04_Pillars_Round_2M_Centred_Taller", Settings { corner_posts: 4, posts_extra: 4, ..d.clone() }),
        ("U05_Pillars_Wide_Own_Entity", Settings { corner_posts: 12, ..d.clone() }),
        ("U06_Pillars_Wider_Than_Cells", Settings { corner_posts: 24, posts_round: false, ..d.clone() }),
        ("U07_Colours_Each_Part", Settings { one_paint: false, paints, corner_posts: 4, ..d.clone() }),
        ("U08_Entities_Only_No_Bearings", Settings { include_main: false, include_bearings: false, ..d.clone() }),
        ("U09_Main_Grid_Only", Settings { include_entities: false, include_bearings: false, ..d.clone() }),
        ("U10_Rotation_90_Upside_Down_Pillars", Settings { turn: Turn::WallsAcross, way_up: WayUp::UpsideDown, corner_posts: 4, ..d.clone() }),
    ];
    std::fs::create_dir_all("testpack").unwrap();
    for (name, mut s) in tests {
        s.name = name.into();
        let path = std::path::PathBuf::from(format!("testpack/{name}.brz"));
        match write_brz(&s, &path) {
            Ok(b) => {
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
