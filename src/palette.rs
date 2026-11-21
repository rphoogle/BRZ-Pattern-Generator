//! Brickadia colour palettes: the game's .bp preset files of type "ColorPalette"
//! (kept in AppData\Local\Brickadia\Saved\Presets\ColorPalettes).
//!
//! In plain English: a palette is a list of named groups (columns), each holding
//! colours. The file stores colours as "linear" values, which is how the game does
//! its lighting maths; the hex codes you see on screen are "sRGB". This module
//! reads and writes palettes and converts between the two, using the same formula
//! as the community palette tool (palette.brickadia.dev), so colours match exactly.
//! It's written to be shared by the other tools in the BRZ suite.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct PaletteGroup {
    pub name: String,
    /// Colours as you see them (sRGB).
    pub colors: Vec<[u8; 3]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    /// The file name without ".bp".
    pub name: String,
    pub description: String,
    pub groups: Vec<PaletteGroup>,
}

/// Linear value (as stored in the file) to the on-screen value.
pub fn linear_to_srgb(c: u8) -> u8 {
    let x = c as f64 / 255.0;
    let s = if x > 0.0031308 { 1.055 * x.powf(1.0 / 2.4) - 0.055 } else { x * 12.92 };
    (s * 255.0).round().clamp(0.0, 255.0) as u8
}

/// On-screen value to the linear value the file stores.
pub fn srgb_to_linear(c: u8) -> u8 {
    let x = c as f64 / 255.0;
    let l = if x > 0.04045 { (x / 1.055 + 0.0521327).powf(2.4) } else { x / 12.92 };
    (l * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Read a palette from its text.
pub fn parse(name: &str, text: &str) -> Result<Palette, String> {
    let v: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| format!("Not a palette file ({e})"))?;
    if v.get("type").and_then(|t| t.as_str()) != Some("ColorPalette") {
        return Err("This .bp file isn't a colour palette".into());
    }
    let data = v.get("data").ok_or("Palette file has no data")?;
    let description = data.get("description").and_then(|d| d.as_str()).unwrap_or("").trim().to_string();
    let mut groups = Vec::new();
    for g in data.get("groups").and_then(|g| g.as_array()).ok_or("Palette file has no groups")? {
        let gname = g.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
        let mut colors = Vec::new();
        for c in g.get("colors").and_then(|c| c.as_array()).map(|a| a.as_slice()).unwrap_or(&[]) {
            let ch = |k: &str| c.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0).round().clamp(0.0, 255.0) as u8;
            colors.push([linear_to_srgb(ch("r")), linear_to_srgb(ch("g")), linear_to_srgb(ch("b"))]);
        }
        groups.push(PaletteGroup { name: gname, colors });
    }
    Ok(Palette { name: name.to_string(), description, groups })
}

/// Read a palette file.
pub fn load(path: &Path) -> Result<Palette, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    parse(&name, &text)
}

/// Write a palette in the game's format (for the suite's palette maker later).
pub fn to_text(p: &Palette) -> String {
    let groups: Vec<serde_json::Value> = p
        .groups
        .iter()
        .map(|g| {
            let colors: Vec<serde_json::Value> = g
                .colors
                .iter()
                .map(|c| serde_json::json!({"b": srgb_to_linear(c[2]), "g": srgb_to_linear(c[1]), "r": srgb_to_linear(c[0]), "a": 255}))
                .collect();
            serde_json::json!({"colors": colors, "name": g.name})
        })
        .collect();
    let v = serde_json::json!({
        "formatVersion": "1",
        "presetVersion": "1",
        "type": "ColorPalette",
        "data": {"groups": groups, "description": p.description},
    });
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

/// Every palette in a folder, sorted by name.
pub fn list(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map(|x| x.eq_ignore_ascii_case("bp")).unwrap_or(false))
                .map(|p| (p.file_stem().unwrap().to_string_lossy().to_string(), p))
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    v
}
