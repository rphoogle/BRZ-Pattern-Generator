//! Draws top-down pictures of each pattern into ./previews (for checking shapes).
use patterngen::*;

fn main() {
    std::fs::create_dir_all("previews").unwrap();
    let args: Vec<String> = std::env::args().collect();
    let extra = args.get(1).cloned().unwrap_or_default();
    for p in Pattern::ALL {
        let mut s = Settings { pattern: p, width: if extra.contains("small") {100} else {240}, length: if extra.contains("small") {100} else {240}, area: Area::Rectangle, ..Settings::default() };
        if extra.contains("turn") { s.turn = Turn::WallsAcross; }
        if extra.contains("posts") { s.corner_posts = 4; }
        if extra.contains("nospoke") { s.spokes = SpokeMode::None; }
        if extra.contains("every") { s.spokes = SpokeMode::EveryCell; }
        if extra.contains("brick") { s.style = WallStyle::MicroBrick; s.wall_thickness = 2; }
        if extra.contains("upside") { s.way_up = WayUp::UpsideDown; }
        match build(&s) {
            Ok(b) => {
                let probs = check::check(&s, &b);
                let f = footprints(&s, &b);
                let sc = if extra.contains("small") { 7.0 } else { 3.0 };
                let (w, h) = (900, 900);
                let mut svg = format!("<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}' style='background:#0d1626'>");
                let cols = ["#dfe4ef", "#7b9be6", "#f2a7b7", "#9be6a0", "#e6d27b"];
                for ft in &f {
                    let col = match ft.kind { FootKind::Bearing => "#ff4040", FootKind::Pillar => "#ffc857", _ => cols[ft.grid.min(4)] };
                    let op = if ft.kind == FootKind::Spoke { 0.5 } else { 0.9 };
                    let pts: Vec<String> = ft.pts.iter().map(|(x, y)| format!("{:.1},{:.1}", w as f64 / 2.0 + x * sc, h as f64 / 2.0 - y * sc)).collect();
                    svg += &format!("<polygon points='{}' fill='{col}' fill-opacity='{op}'/>", pts.join(" "));
                }
                svg += "</svg>";
                std::fs::write(format!("previews/{:?}{extra}.svg", p), svg).unwrap();
                println!("{:?}: {} bricks, {} ents, corners {:.2}/{:.2} move {:.2} {}", p, b.brick_count(), b.entities.len(), b.corner_mean, b.corner_worst, b.max_move, if probs.is_empty() { "OK".into() } else { probs.join("; ") });
            }
            Err(e) => println!("{:?}: ERROR {e}", p),
        }
    }
}
