//! Corner-join test pack (A / B / C) based on T11 (walls across).
use patterngen::*;

fn main() {
    let base = Settings { turn: Turn::WallsAcross, ..Settings::default() };
    let tests: Vec<(&str, Settings)> = vec![
        ("J0_Before_T11", Settings { best_bearings: false, ..base.clone() }),
        ("J1_A_Best_Bearing_Spots", base.clone()),
        ("J2_B_Posts_2x2_Micro", Settings { corner_posts: 4, ..base.clone() }),
        ("J3_C_Posts_1x1_Micro_Shorter_Walls", Settings { corner_posts: 2, ..base.clone() }),
    ];
    std::fs::create_dir_all("joinpack").unwrap();
    for (name, mut s) in tests {
        s.name = name.into();
        let b = write_brz(&s, std::path::Path::new(&format!("joinpack/{name}.brz"))).unwrap();
        let probs = check::check(&s, &b);
        println!("{name}: {} bricks, {} entities, corner miss avg {:.2} worst {:.2}, bearings {:?} {}",
            b.brick_count(), b.entities.len(), b.corner_mean, b.corner_worst,
            b.entities.iter().map(|g| g.bearing).collect::<Vec<_>>(),
            if probs.is_empty() { "OK".into() } else { probs.join("; ") });
    }
}
