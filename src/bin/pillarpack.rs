//! Pillar test pack: square vs round, walls running through.
use patterngen::*;

fn main() {
    let across = Settings { turn: Turn::WallsAcross, ..Settings::default() };
    let upright = Settings::default();
    let tests: Vec<(&str, Settings)> = vec![
        ("P1_Square_1x1_Through", Settings { corner_posts: 2, ..across.clone() }),
        ("P2_Round_Thin_Through", Settings { corner_posts: 2, posts_round: true, ..across.clone() }),
        ("P3_Round_2x2_Through", Settings { corner_posts: 4, posts_round: true, ..across.clone() }),
        ("P4_Round_Thin_Taller", Settings { corner_posts: 2, posts_round: true, posts_extra: 4, ..across.clone() }),
        ("P5_Upright_Round_Thin", Settings { corner_posts: 2, posts_round: true, ..upright.clone() }),
        ("P6_No_Pillars_A_Only", across.clone()),
    ];
    std::fs::create_dir_all("pillarpack").unwrap();
    for (name, mut s) in tests {
        s.name = name.into();
        let b = write_brz(&s, std::path::Path::new(&format!("pillarpack/{name}.brz"))).unwrap();
        let probs = check::check(&s, &b);
        println!("{name}: {} bricks, {} entities, corner miss avg {:.2} worst {:.2} {}",
            b.brick_count(), b.entities.len(), b.corner_mean, b.corner_worst,
            if probs.is_empty() { "OK".into() } else { probs.join("; ") });
    }
}
