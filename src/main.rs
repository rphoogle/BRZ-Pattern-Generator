//! BRZ Pattern Generator: the window. All the pattern-making lives in the
//! library (src/lib.rs); this file only draws the controls and the preview,
//! and saves / copies the result.
#![cfg_attr(windows, windows_subsystem = "windows")]

use eframe::egui::{self, Color32, CornerRadius, Pos2, RichText, Stroke, Vec2};
use patterngen::*;
use std::sync::mpsc;

// ---- colours (same as the Circle Generator) ----
const BG: Color32 = Color32::from_rgb(0x16, 0x23, 0x3a);
const CARD: Color32 = Color32::from_rgb(0x46, 0x5a, 0x80);
const FIELD: Color32 = Color32::from_rgb(0x1d, 0x2a, 0x43);
const ACCENT: Color32 = Color32::from_rgb(0x3d, 0x8b, 0xfd);
const GREEN: Color32 = Color32::from_rgb(0x3d, 0xbb, 0x6b);
const PREVIEW: Color32 = Color32::from_rgb(0x0d, 0x16, 0x26);
const SOFT: Color32 = Color32::from_rgb(0xc5, 0xd0, 0xe4);
const NOTE: Color32 = Color32::from_rgb(0x9f, 0xb6, 0xdc);
const WARN: Color32 = Color32::from_rgb(0xff, 0xc8, 0x57);
const ERR: Color32 = Color32::from_rgb(0xff, 0x7b, 0x7b);
const DIM: Color32 = Color32::from_rgb(0x2b, 0x3a, 0x57);
/// Wall colours: main grid, entity 1, 2, 3.
const GRID_COL: [Color32; 4] = [
    Color32::from_rgb(0xdf, 0xe4, 0xef),
    Color32::from_rgb(0x7b, 0x9b, 0xe6),
    Color32::from_rgb(0xf2, 0xa7, 0xb7),
    Color32::from_rgb(0x8f, 0xd1, 0x9e),
];
const SPOKE_COL: [Color32; 4] = [
    Color32::from_rgb(0x9a, 0xa6, 0xbd),
    Color32::from_rgb(0x4e, 0x6c, 0xb8),
    Color32::from_rgb(0xc9, 0x70, 0x8a),
    Color32::from_rgb(0x4f, 0x9a, 0x62),
];
const PILLAR_COL: Color32 = Color32::from_rgb(0xff, 0xff, 0xff);
const BEARING_COL: Color32 = Color32::from_rgb(0xff, 0xc8, 0x57);

#[derive(Clone, Copy, PartialEq)]
enum LogKind { Info, Ok, Warn, Err }

#[derive(Clone, Copy, PartialEq)]
enum Units { Studs, Micro }

struct Result_ {
    id: u64,
    out: Result<(Built, Vec<Foot>), String>,
}

struct App {
    s: Settings,
    units: Units,
    hex: String,
    file_name: String,
    name_edited: bool,
    log: Vec<(LogKind, String)>,
    always_on_top: bool,
    // background building
    key: String,
    next_id: u64,
    waiting: Option<u64>,
    changed_at: f64,
    tx: mpsc::Sender<Result_>,
    rx: mpsc::Receiver<Result_>,
    built: Option<Result<(Built, Vec<Foot>), String>>,
    built_settings: Settings,
    // preview view
    zoom: f32,
    pan: Vec2,
    // presets
    preset_name: String,
    presets: Vec<String>,
    preset_pick: usize,
    confirm_replace: Option<std::path::PathBuf>,
    cards_h: f32,
}

impl App {
    fn new(cc: &eframe::CreationContext) -> Self {
        style_ui(&cc.egui_ctx);
        let (tx, rx) = mpsc::channel();
        let s = Settings::default();
        let mut a = App {
            hex: hex_of(s.color),
            s: s.clone(),
            units: Units::Studs,
            file_name: String::new(),
            name_edited: false,
            log: vec![(LogKind::Info, "Ready. Choose a pattern, then Save or Copy.".into())],
            always_on_top: false,
            key: String::new(),
            next_id: 0,
            waiting: None,
            changed_at: 0.0,
            tx,
            rx,
            built: None,
            built_settings: s,
            zoom: 1.0,
            pan: Vec2::ZERO,
            preset_name: String::new(),
            presets: Vec::new(),
            preset_pick: 0,
            confirm_replace: None,
            cards_h: 190.0,
        };
        a.refresh_presets();
        a
    }

    fn log(&mut self, k: LogKind, m: impl Into<String>) {
        self.log.push((k, m.into()));
        if self.log.len() > 200 {
            self.log.remove(0);
        }
    }

    fn auto_name(&self) -> String {
        let s = &self.s;
        let style = if s.style == WallStyle::MicroWedge { "Wedge" } else { "Brick" };
        let way = if s.way_up == WayUp::UpsideDown { "_UpsideDown" } else { "" };
        let posts = if s.corner_posts > 0 { if s.posts_round { "_RoundPillars" } else { "_SquarePillars" } } else { "" };
        format!("Honeycomb_{}_{}{}{}", fmt_num(s.cell as f64 / 10.0), style, posts, way).replace('.', "p")
    }

    /// Start a background build when the settings have changed (after a short pause).
    fn rebuild_if_needed(&mut self, ctx: &egui::Context) {
        while let Ok(r) = self.rx.try_recv() {
            if Some(r.id) == self.waiting {
                self.waiting = None;
                if let Ok((b, _)) = &r.out {
                    if b.max_move > 0.0 {
                        // nothing to log every time; the stats show it
                    }
                }
                self.built = Some(r.out);
            }
        }
        let key = self.s.to_text();
        let now = ctx.input(|i| i.time);
        if key != self.key {
            self.key = key;
            self.changed_at = now;
            self.next_id += 1;
            self.waiting = Some(self.next_id);
            if !self.name_edited {
                self.file_name = self.auto_name();
            }
            let id = self.next_id;
            let s = self.s.clone();
            self.built_settings = s.clone();
            let tx = self.tx.clone();
            let ctx2 = ctx.clone();
            std::thread::spawn(move || {
                let out = build(&s).map(|b| {
                    let f = footprints(&s, &b);
                    (b, f)
                });
                let _ = tx.send(Result_ { id, out });
                ctx2.request_repaint();
            });
        }
    }

    // ---------------------------------------------------------------- output

    fn checked_bytes(&mut self) -> Option<(Vec<u8>, usize, usize)> {
        let mut s = self.s.clone();
        s.name = self.clean_name();
        let built = match build(&s) {
            Ok(b) => b,
            Err(e) => {
                self.log(LogKind::Err, e);
                return None;
            }
        };
        let probs = check::check(&s, &built);
        if !probs.is_empty() {
            self.log(LogKind::Err, format!("Self-check failed ({}). Nothing was saved. Please send these settings to Claude.", probs.join("; ")));
            return None;
        }
        let n = built.brick_count();
        let e = built.entities.len();
        match to_world(&s, &built).to_brz_vec() {
            Ok(b) => Some((b, n, e)),
            Err(err) => {
                self.log(LogKind::Err, format!("Could not create the prefab: {err}"));
                None
            }
        }
    }

    fn clean_name(&self) -> String {
        let n: String = self.file_name.trim().chars().map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c }).collect();
        let n = n.trim_end_matches(".brz").to_string();
        if n.is_empty() { "Pattern".into() } else { n }
    }

    fn prefab_dir() -> Option<std::path::PathBuf> {
        dirs::data_local_dir().map(|d| d.join("Brickadia").join("Saved").join("Prefabs"))
    }

    fn save(&mut self, replace: bool) {
        let Some(dir) = Self::prefab_dir() else {
            self.log(LogKind::Err, "Could not find your AppData\\Local folder.");
            return;
        };
        let path = dir.join(format!("{}.brz", self.clean_name()));
        if path.exists() && !replace {
            self.confirm_replace = Some(path);
            return;
        }
        let Some((bytes, n, e)) = self.checked_bytes() else { return };
        if let Err(err) = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, bytes)) {
            self.log(LogKind::Err, format!("Could not save: {err}"));
            return;
        }
        self.log(LogKind::Ok, format!("Saved {n} bricks and {e} entities to {} (checked: no overlaps).", path.display()));
    }

    fn copy(&mut self) {
        let Some((bytes, n, e)) = self.checked_bytes() else { return };
        let dir = std::env::temp_dir().join("BRZPatternGenerator");
        let path = dir.join(format!("{}.brz", self.clean_name()));
        if let Err(err) = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, bytes)) {
            self.log(LogKind::Err, format!("Could not prepare the copy: {err}"));
            return;
        }
        match copy_file_to_clipboard(&path) {
            Ok(()) => self.log(LogKind::Ok, format!("Copied {n} bricks and {e} entities. In game, paste with Ctrl+V.")),
            Err(err) => self.log(LogKind::Err, err),
        }
    }

    // ---------------------------------------------------------------- presets

    fn preset_dir() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|d| d.join("BRZPatternGenerator").join("Presets"))
    }

    fn refresh_presets(&mut self) {
        self.presets.clear();
        if let Some(dir) = Self::preset_dir() {
            if let Ok(rd) = std::fs::read_dir(dir) {
                for e in rd.flatten() {
                    let p = e.path();
                    if p.extension().and_then(|x| x.to_str()) == Some("txt") {
                        if let Some(n) = p.file_stem().and_then(|x| x.to_str()) {
                            self.presets.push(n.to_string());
                        }
                    }
                }
            }
        }
        self.presets.sort();
        if self.preset_pick >= self.presets.len() {
            self.preset_pick = 0;
        }
    }

    fn save_preset(&mut self) {
        let name: String = self.preset_name.trim().chars().map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c }).collect();
        if name.is_empty() {
            self.log(LogKind::Warn, "Type a name for the preset first.");
            return;
        }
        let Some(dir) = Self::preset_dir() else { return };
        let text = format!("units={}\n{}", if self.units == Units::Studs { "studs" } else { "micro" }, self.s.to_text());
        match std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(dir.join(format!("{name}.txt")), text)) {
            Ok(()) => {
                self.log(LogKind::Ok, format!("Preset \"{name}\" saved."));
                self.refresh_presets();
                if let Some(i) = self.presets.iter().position(|p| *p == name) {
                    self.preset_pick = i;
                }
            }
            Err(e) => self.log(LogKind::Err, format!("Could not save the preset: {e}")),
        }
    }

    fn load_preset(&mut self) {
        let Some(name) = self.presets.get(self.preset_pick).cloned() else { return };
        let Some(dir) = Self::preset_dir() else { return };
        match std::fs::read_to_string(dir.join(format!("{name}.txt"))) {
            Ok(t) => {
                self.s = Settings::from_text(&t);
                self.units = if t.contains("units=micro") { Units::Micro } else { Units::Studs };
                self.hex = hex_of(self.s.color);
                self.preset_name = name.clone();
                self.log(LogKind::Ok, format!("Preset \"{name}\" loaded."));
            }
            Err(e) => self.log(LogKind::Err, format!("Could not load the preset: {e}")),
        }
    }

    fn delete_preset(&mut self) {
        let Some(name) = self.presets.get(self.preset_pick).cloned() else { return };
        if let Some(dir) = Self::preset_dir() {
            let _ = std::fs::remove_file(dir.join(format!("{name}.txt")));
            self.log(LogKind::Info, format!("Preset \"{name}\" deleted."));
        }
        self.refresh_presets();
    }
}

#[cfg(windows)]
fn copy_file_to_clipboard(path: &std::path::Path) -> Result<(), String> {
    let p = path.to_string_lossy().to_string();
    clipboard_win::raw::open().map_err(|e| format!("Could not open the clipboard: {e}"))?;
    let set = clipboard_win::raw::set_file_list(&[p]).map_err(|e| format!("Could not copy: {e}"));
    let _ = clipboard_win::raw::close();
    set
}

#[cfg(not(windows))]
fn copy_file_to_clipboard(path: &std::path::Path) -> Result<(), String> {
    Err(format!("Copy only works on Windows. The file is at {}", path.display()))
}

fn hex_of(c: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

fn fmt_num(x: f64) -> String {
    let s = format!("{:.2}", x);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn style_ui(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("dv".into(), std::sync::Arc::new(egui::FontData::from_static(include_bytes!("../assets/DejaVuSans.ttf"))));
    fonts.font_data.insert("dvb".into(), std::sync::Arc::new(egui::FontData::from_static(include_bytes!("../assets/DejaVuSans-Bold.ttf"))));
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0, "dv".into());
    fonts.families.insert(egui::FontFamily::Name("bold".into()), vec!["dvb".into(), "dv".into()]);
    ctx.set_fonts(fonts);
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = BG;
    v.extreme_bg_color = FIELD;
    v.selection.bg_fill = ACCENT;
    v.widgets.inactive.weak_bg_fill = DIM;
    v.widgets.inactive.bg_fill = DIM;
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x36, 0x49, 0x6c);
    v.widgets.active.weak_bg_fill = ACCENT;
    v.slider_trailing_fill = true;
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.item_spacing = Vec2::new(8.0, 6.0);
        s.spacing.button_padding = Vec2::new(8.0, 4.0);
        s.spacing.slider_width = 110.0;
        s.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(13.5));
        s.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(13.5));
    });
}

fn bold(t: impl Into<String>, size: f32) -> RichText {
    RichText::new(t).family(egui::FontFamily::Name("bold".into())).size(size)
}

fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let w = ui.available_width() - 24.0;
    egui::Frame::new()
        .fill(CARD)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(12, 9))
        .show(ui, |ui| {
            ui.set_width(w.max(50.0));
            add(ui)
        })
        .inner
}

fn label(ui: &mut egui::Ui, t: &str, w: f32) {
    ui.allocate_ui_with_layout(Vec2::new(w, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.set_min_width(w);
        ui.label(bold(t, 13.0).color(Color32::WHITE));
    });
}

fn note(ui: &mut egui::Ui, indent: f32, t: impl Into<String>) {
    ui.horizontal(|ui| {
        ui.add_space(indent);
        ui.label(RichText::new(t).size(11.5).color(NOTE));
    });
}

/// Segmented buttons. Returns true if changed.
fn seg<T: PartialEq + Copy>(ui: &mut egui::Ui, v: &mut T, opts: &[(T, &str)]) -> bool {
    let mut changed = false;
    egui::Frame::new().fill(DIM).corner_radius(CornerRadius::same(8)).inner_margin(egui::Margin::same(3)).show(ui, |ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        ui.horizontal_wrapped(|ui| {
            for (val, text) in opts {
                let on = *v == *val;
                let b = egui::Button::new(bold(*text, 12.5).color(if on { Color32::WHITE } else { SOFT }))
                    .fill(if on { ACCENT } else { Color32::TRANSPARENT })
                    .corner_radius(CornerRadius::same(6));
                if ui.add(b).clicked() && !on {
                    *v = *val;
                    changed = true;
                }
            }
        });
    });
    changed
}

/// Up / Down arrows change a focused number box by `step`, Shift + arrow by 10 steps.
fn arrow_keys(ui: &mut egui::Ui, v: &mut f64, step: f64, min: f64, max: f64) {
    let id = ui.next_auto_id();
    if ui.memory(|m| m.has_focus(id)) {
        let (shift, up, dn) = ui.input_mut(|i| {
            (
                i.modifiers.shift,
                i.count_and_consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) as i32,
                i.count_and_consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) as i32,
            )
        });
        let d = (up - dn) as f64 * step * if shift { 10.0 } else { 1.0 };
        if d != 0.0 {
            *v = (*v + d).clamp(min, max);
            ui.data_mut(|data| data.remove::<String>(id));
        }
    }
}

/// A size in micro bricks shown in the chosen units: slider + typed box + note.
/// `v` is in micro bricks. Returns true if changed.
#[allow(clippy::too_many_arguments)]
fn size_row(ui: &mut egui::Ui, name: &str, lw: f32, v: &mut i32, min: i32, slider_max: i32, type_max: i32, units: Units, note_txt: &str, enabled: bool) -> bool {
    size_row_v(ui, name, lw, v, min, slider_max, type_max, units, note_txt, enabled, false)
}

/// `vertical`: heights show in plates (2 M each) when Studs is chosen.
#[allow(clippy::too_many_arguments)]
fn size_row_v(ui: &mut egui::Ui, name: &str, lw: f32, v: &mut i32, min: i32, slider_max: i32, type_max: i32, units: Units, note_txt: &str, enabled: bool, vertical: bool) -> bool {
    let before = *v;
    let scale = match (units, vertical) { (Units::Micro, _) => 1.0, (Units::Studs, false) => 5.0, (Units::Studs, true) => 2.0 };
    ui.add_enabled_ui(enabled, |ui| {
        ui.horizontal(|ui| {
            label(ui, name, lw);
            let mut sv = (*v).clamp(min, slider_max);
            if ui.add(egui::Slider::new(&mut sv, min..=slider_max).logarithmic(slider_max > 200).show_value(false)).changed() {
                *v = sv;
            }
            let mut shown = *v as f64 / scale;
            ui.allocate_ui_with_layout(Vec2::new(58.0, 22.0), egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                arrow_keys(ui, &mut shown, 1.0, min as f64 / scale, type_max as f64 / scale);
                ui.add(egui::DragValue::new(&mut shown).range(min as f64 / scale..=type_max as f64 / scale).speed(0.2).max_decimals(1));
            });
            let back = (shown * scale).round() as i32;
            if back != *v {
                *v = back.clamp(min, type_max);
            }
            let unit = match (units, vertical) { (Units::Micro, _) => "M", (Units::Studs, false) => "studs", (Units::Studs, true) => "plates" };
            ui.label(RichText::new(format!("{unit}{}", if note_txt.is_empty() { String::new() } else { format!(" ({note_txt})") })).size(12.0).color(Color32::from_rgb(0xd6, 0xde, 0xf0)));
        });
    });
    *v != before
}

/// A plain whole-number row (counts, percentages).
fn int_row(ui: &mut egui::Ui, name: &str, lw: f32, v: &mut i32, min: i32, max: i32, suffix: &str, enabled: bool) {
    ui.add_enabled_ui(enabled, |ui| {
        ui.horizontal(|ui| {
            label(ui, name, lw);
            let mut f = *v as f64;
            ui.allocate_ui_with_layout(Vec2::new(58.0, 22.0), egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                arrow_keys(ui, &mut f, 1.0, min as f64, max as f64);
                ui.add(egui::DragValue::new(&mut f).range(min as f64..=max as f64).speed(0.1).max_decimals(0));
            });
            *v = f.round() as i32;
            ui.label(RichText::new(suffix).size(12.0).color(SOFT));
        });
    });
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.rebuild_if_needed(ctx);

        egui::TopBottomPanel::top("head")
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin { left: 18, right: 18, top: 12, bottom: 8 }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(bold("BRZ Pattern Generator", 24.0).color(Color32::WHITE));
                            ui.label(RichText::new(concat!("v", env!("CARGO_PKG_VERSION"))).color(Color32::from_rgb(0x8f, 0xa2, 0xc2)));
                        });
                        ui.label(bold("Angled brick patterns on bearings, saved as Brickadia prefabs.", 12.5).color(SOFT));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.checkbox(&mut self.always_on_top, "Always on top").changed() {
                            let level = if self.always_on_top { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal };
                            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
                        }
                        ui.add_space(14.0);
                        // presets
                        if ui.button("Delete").on_hover_text("Delete the selected preset").clicked() {
                            self.delete_preset();
                        }
                        if ui.button("Load").on_hover_text("Load the selected preset").clicked() {
                            self.load_preset();
                        }
                        let pick = self.presets.get(self.preset_pick).cloned().unwrap_or_else(|| "(no presets yet)".into());
                        egui::ComboBox::from_id_salt("presets").width(150.0).selected_text(pick).show_ui(ui, |ui| {
                            for (i, n) in self.presets.clone().iter().enumerate() {
                                ui.selectable_value(&mut self.preset_pick, i, n);
                            }
                        });
                        if ui.button("Save preset").clicked() {
                            self.save_preset();
                        }
                        ui.add_sized([130.0, 22.0], egui::TextEdit::singleline(&mut self.preset_name).hint_text("preset name"));
                        ui.label(bold("Presets", 12.5).color(SOFT));
                    });
                });
            });

        egui::TopBottomPanel::bottom("log")
            .exact_height(46.0)
            .frame(egui::Frame::new().fill(Color32::BLACK).inner_margin(egui::Margin::symmetric(12, 6)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                    for (k, m) in &self.log {
                        let c = match k {
                            LogKind::Info => Color32::from_rgb(0xd7, 0xe3, 0xd7),
                            LogKind::Ok => Color32::from_rgb(0x7f, 0xe0, 0x8f),
                            LogKind::Warn => WARN,
                            LogKind::Err => ERR,
                        };
                        ui.label(RichText::new(format!("[INFO] {m}")).monospace().size(12.0).color(c));
                    }
                });
            });

        egui::SidePanel::left("controls")
            .exact_width(500.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin { left: 14, right: 6, top: 8, bottom: 10 }))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.left_controls(ui));
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin { left: 6, right: 14, top: 8, bottom: 10 }))
            .show(ctx, |ui| self.right_side(ui));

        if let Some(path) = self.confirm_replace.clone() {
            let (mut close, mut go) = (false, false);
            egui::Modal::new(egui::Id::new("replace")).show(ctx, |ui| {
                ui.label(RichText::new("A prefab with this name already exists").strong().size(16.0));
                ui.label(path.display().to_string());
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Replace it").clicked() {
                        go = true;
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                });
            });
            if go {
                self.confirm_replace = None;
                self.save(true);
            } else if close {
                self.confirm_replace = None;
                self.log(LogKind::Info, "Not saved. Change the file name to keep both.");
            }
        }
        if self.waiting.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
}

// ======================================================================= controls
impl App {
    fn unit_note(&self, micro: i32) -> String {
        // the other unit, in brackets
        if self.units == Units::Studs { format!("{micro} M") } else { format!("{} studs", fmt_num(micro as f64 / 5.0)) }
    }

    fn left_controls(&mut self, ui: &mut egui::Ui) {
        let lw = 92.0;
        let u = self.units;
        // ---- Pattern ----
        card(ui, |ui| {
            ui.horizontal(|ui| {
                label(ui, "Pattern", lw);
                egui::ComboBox::from_id_salt("pattern").width(200.0).selected_text("Honeycomb").show_ui(ui, |ui| {
                    let _ = ui.selectable_label(true, "Honeycomb");
                    for p in ["Triangles", "Star of David", "Cube illusion", "Octagons and squares", "Diamond lattice"] {
                        ui.add_enabled(false, egui::Button::selectable(false, format!("{p} (coming in stage 3)")));
                    }
                });
            });
            ui.horizontal(|ui| {
                label(ui, "Turn", lw);
                seg(ui, &mut self.s.turn, &[(Turn::WallsUpright, "Walls upright"), (Turn::WallsAcross, "Walls across")]);
            });
            ui.horizontal(|ui| {
                label(ui, "Units", lw);
                seg(ui, &mut self.units, &[(Units::Studs, "Studs"), (Units::Micro, "Micro")]);
            });
            let mut cell = self.s.cell / 2;
            let n = self.unit_note(cell);
            if size_row(ui, "Cell size", lw, &mut cell, 4, 100, 1000, u, &n, true) {
                self.s.cell = cell * 2;
                let max_post = (self.s.cell / 4 / 2) * 2;
                if self.s.corner_posts > max_post { self.s.corner_posts = max_post.max(2) }
            }
            note(ui, lw + 8.0, "flat side to flat side");
            let mut sh = self.s.stretch_h as i32;
            let mut sv = self.s.stretch_v as i32;
            int_row(ui, "Stretch H", lw, &mut sh, 25, 400, "%  (stage 3)", false);
            int_row(ui, "Stretch V", lw, &mut sv, 25, 400, "%  (stage 3)", false);
            let ents = match &self.built {
                Some(Ok((b, _))) => b.entities.len(),
                _ => if self.s.turn == Turn::WallsAcross { 3 } else { 2 },
            };
            let angles = if self.s.turn == Turn::WallsAcross { "90° · 150° · 30°" } else { "0° · 60° · 120°" };
            note(ui, lw + 8.0, format!("Angles {angles} → {ents} entities"));
        });
        ui.add_space(6.0);

        // ---- Walls ----
        card(ui, |ui| {
            ui.horizontal(|ui| {
                label(ui, "Wall style", lw);
                seg(ui, &mut self.s.style, &[(WallStyle::MicroBrick, "Micro brick"), (WallStyle::MicroWedge, "Micro wedge")]);
            });
            let mut h = self.s.wall_height / 2;
            let n = if u == Units::Studs { format!("{h} M · {} bricks", fmt_num(h as f64 / 6.0)) } else { format!("{} bricks", fmt_num(h as f64 / 6.0)) };
            if size_row_v(ui, "Height", lw, &mut h, 1, 120, 1000, u, &n, true, true) {
                self.s.wall_height = h * 2;
            }
            let brick = self.s.style == WallStyle::MicroBrick;
            let mut t = self.s.wall_thickness / 2;
            let n = self.unit_note(t);
            if size_row(ui, "Thickness", lw, &mut t, 1, 10, 50, u, &n, brick) {
                self.s.wall_thickness = t * 2;
            }
            ui.add_enabled_ui(!brick, |ui| {
                ui.horizontal(|ui| {
                    label(ui, "Wedges", lw);
                    ui.checkbox(&mut self.s.flip_wedges, "Flip slopes");
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                label(ui, "Pillars", lw);
                let mut on = self.s.corner_posts > 0;
                if ui.checkbox(&mut on, "Pillar at every corner").changed() {
                    self.s.corner_posts = if on { 4.min((self.s.cell / 4 / 2) * 2).max(2) } else { 0 };
                    if on { self.s.posts_round = true; }
                }
            });
            let on = self.s.corner_posts > 0;
            ui.add_enabled_ui(on, |ui| {
                ui.horizontal(|ui| {
                    label(ui, "Shape", lw);
                    seg(ui, &mut self.s.posts_round, &[(true, "Round (pole)"), (false, "Square")]);
                });
                let mut pw = (self.s.corner_posts / 2).max(1);
                let maxw = (self.s.cell / 4 / 2).max(1);
                let n = self.unit_note(pw);
                if size_row(ui, "Width", lw, &mut pw, 1, maxw.max(2), maxw.max(1), u, &n, on) && on {
                    self.s.corner_posts = pw.min(maxw) * 2;
                }
                let mut ex = self.s.posts_extra / 2;
                ui.horizontal(|ui| {
                    label(ui, "Height", lw);
                    let mut f = ex as f64;
                    ui.allocate_ui_with_layout(Vec2::new(58.0, 22.0), egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                        arrow_keys(ui, &mut f, 1.0, -(self.s.wall_height as f64 / 2.0 - 1.0), 60.0);
                        ui.add(egui::DragValue::new(&mut f).range(-(self.s.wall_height as f64 / 2.0 - 1.0)..=60.0).speed(0.1).max_decimals(0));
                    });
                    ex = f.round() as i32;
                    ui.label(RichText::new("M taller (0 = level)").size(12.0).color(SOFT));
                });
                self.s.posts_extra = ex * 2;
                ui.horizontal(|ui| {
                    label(ui, "Walls", lw);
                    seg(ui, &mut self.s.walls_through, &[(true, "Run through"), (false, "Stop at pillar")]);
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                label(ui, "Colour", lw);
                if egui::color_picker::color_edit_button_srgb(ui, &mut self.s.color).changed() {
                    self.hex = hex_of(self.s.color);
                }
                let r = ui.add_sized([90.0, 22.0], egui::TextEdit::singleline(&mut self.hex));
                if r.changed() {
                    let h = self.hex.trim().trim_start_matches('#');
                    if h.len() == 6 {
                        if let Ok(v) = u32::from_str_radix(h, 16) {
                            self.s.color = [(v >> 16) as u8, (v >> 8) as u8, v as u8];
                        }
                    }
                }
            });
            ui.horizontal(|ui| {
                label(ui, "Material", lw);
                egui::ComboBox::from_id_salt("material").width(150.0).selected_text(MATERIALS[self.s.material].0).show_ui(ui, |ui| {
                    for (i, (n, _)) in MATERIALS.iter().enumerate() {
                        ui.selectable_value(&mut self.s.material, i, *n);
                    }
                });
            });
            ui.horizontal(|ui| {
                label(ui, "Intensity", lw);
                let mut i = self.s.intensity as i32;
                ui.add(egui::Slider::new(&mut i, 0..=10));
                self.s.intensity = i as u8;
                ui.label(RichText::new("glow / shine").size(12.0).color(SOFT));
            });
        });
        ui.add_space(6.0);

        // ---- Spokes ----
        card(ui, |ui| {
            ui.horizontal(|ui| {
                label(ui, "Spokes", lw);
                seg(ui, &mut self.s.spokes, &[(SpokeMode::None, "None"), (SpokeMode::EveryCell, "Every cell"), (SpokeMode::AltRows, "Alt. rows"), (SpokeMode::Custom, "Custom")]);
            });
            let on = self.s.spokes != SpokeMode::None;
            let custom = self.s.spokes == SpokeMode::Custom;
            ui.add_enabled_ui(on, |ui| {
                ui.horizontal(|ui| {
                    label(ui, "Directions", lw);
                    let names = if self.s.turn == Turn::WallsAcross { ["90°", "150°", "30°"] } else { ["0°", "60°", "120°"] };
                    for k in 0..3 {
                        ui.checkbox(&mut self.s.spoke_dirs[k], names[k]);
                    }
                });
            });
            int_row(ui, "Every", lw, &mut self.s.spoke_every, 1, 20, "cells along a row", on && custom);
            int_row(ui, "Rows", lw, &mut self.s.spoke_rows, 1, 20, "every this many rows", on && custom);
            ui.add_enabled_ui(on && custom, |ui| {
                ui.horizontal(|ui| {
                    label(ui, "Offset", lw);
                    let mut f = self.s.spoke_offset as f64;
                    ui.allocate_ui_with_layout(Vec2::new(58.0, 22.0), egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                        arrow_keys(ui, &mut f, 1.0, 0.0, 19.0);
                        ui.add(egui::DragValue::new(&mut f).range(0.0..=19.0).speed(0.1).max_decimals(0));
                    });
                    self.s.spoke_offset = f.round() as i32;
                    ui.checkbox(&mut self.s.spoke_stagger, "Stagger rows");
                });
            });
            note(ui, lw + 8.0, "2 cells, 2 rows, staggered = your original");
            ui.add_enabled_ui(on, |ui| {
                ui.horizontal(|ui| {
                    label(ui, "Length", lw);
                    let mut full = self.s.spoke_length == 0;
                    if ui.checkbox(&mut full, "Wall to wall").changed() {
                        self.s.spoke_length = if full { 0 } else { (self.s.cell - 4).max(2) / 2 * 2 };
                    }
                });
            });
            if on && self.s.spoke_length > 0 {
                let mut l = self.s.spoke_length / 2;
                let maxl = self.s.cell / 2;
                let n = self.unit_note(l);
                if size_row(ui, "", lw, &mut l, 1, maxl, maxl, u, &n, true) {
                    self.s.spoke_length = l * 2;
                }
            }
            let mut t = self.s.spoke_thickness / 2;
            let n = self.unit_note(t);
            if size_row(ui, "Thickness", lw, &mut t, 1, 10, 50, u, &n, on) {
                self.s.spoke_thickness = t * 2;
            }
            ui.add_enabled_ui(on, |ui| {
                ui.horizontal(|ui| {
                    label(ui, "Height", lw);
                    seg(ui, &mut self.s.spoke_top, &[(false, "Base"), (true, "Top")]);
                });
            });
        });
    }

    // ------------------------------------------------------------- right side

    fn right_side(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_size();
        let stats_h = 58.0;
        let out_h = 40.0;
        let tool_h = 30.0;
        let prev_h = (avail.y - tool_h - stats_h - self.cards_h - out_h - 4.0 * 8.0).max(220.0);

        // ---- toolbar ----
        ui.horizontal(|ui| {
            if ui.button("Fit").on_hover_text("Fit the whole pattern in the preview").clicked() {
                self.zoom = 1.0;
                self.pan = Vec2::ZERO;
            }
            ui.label(RichText::new("Zoom").color(SOFT));
            ui.add(egui::Slider::new(&mut self.zoom, 1.0..=12.0).logarithmic(true).show_value(false));
            ui.add_space(10.0);
            for t in ["Paint cells", "Fill", "Clear", "Undo"] {
                ui.add_enabled(false, egui::Button::new(t)).on_disabled_hover_text("Paint mode arrives in stage 3");
            }
            ui.label(RichText::new("scroll to zoom · drag to move").size(11.5).color(NOTE));
        });
        ui.add_space(6.0);

        // ---- preview ----
        egui::Frame::new().fill(PREVIEW).corner_radius(CornerRadius::same(12)).stroke(Stroke::new(1.0_f32, Color32::from_rgb(0x26, 0x35, 0x4f))).show(ui, |ui| {
            let (resp, painter) = ui.allocate_painter(Vec2::new(ui.available_width(), prev_h), egui::Sense::drag());
            let r = resp.rect;
            if resp.dragged() {
                self.pan += resp.drag_delta();
            }
            if resp.hovered() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 {
                    let old = self.zoom;
                    self.zoom = (self.zoom * (1.0 + scroll * 0.002)).clamp(1.0, 12.0);
                    // keep the point under the mouse still
                    if let Some(m) = ui.input(|i| i.pointer.hover_pos()) {
                        let c = r.center() + self.pan;
                        self.pan += (m - c) * (1.0 - self.zoom / old);
                    }
                }
            }
            painter.text(r.left_top() + Vec2::new(14.0, 12.0), egui::Align2::LEFT_TOP, "TOP-DOWN PREVIEW", egui::FontId::proportional(12.0), Color32::from_rgb(0x8f, 0xa2, 0xc2));
            let ents = match &self.built {
                Some(Ok((b, _))) => b.entities.len(),
                _ => 0,
            };
            let mut legend: Vec<(&str, Color32)> = vec![("main grid", GRID_COL[0])];
            for (i, t) in ["entity 1", "entity 2", "entity 3"].iter().enumerate().take(ents) {
                legend.push((t, GRID_COL[i + 1]));
            }
            if self.built_settings.corner_posts > 0 {
                legend.push(("pillar", PILLAR_COL));
            }
            legend.push(("bearing", BEARING_COL));
            let mut lx = r.right() - 14.0;
            for (t, c) in legend.iter().rev() {
                let g = painter.layout_no_wrap(t.to_string(), egui::FontId::proportional(11.5), Color32::from_rgb(0xa9, 0xb8, 0xd3));
                lx -= g.size().x;
                painter.galley(Pos2::new(lx, r.top() + 12.0), g.clone(), Color32::WHITE);
                lx -= 15.0;
                painter.rect_filled(egui::Rect::from_min_size(Pos2::new(lx, r.top() + 14.0), Vec2::splat(10.0)), 2.0, *c);
                lx -= 12.0;
            }
            let painter = painter.with_clip_rect(r.shrink(2.0));
            match &self.built {
                Some(Ok((_, feet))) => {
                    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
                    for f in feet {
                        for p in f.pts {
                            x0 = x0.min(p.0);
                            y0 = y0.min(p.1);
                            x1 = x1.max(p.0);
                            y1 = y1.max(p.1);
                        }
                    }
                    let area = r.shrink2(Vec2::new(20.0, 34.0));
                    let fit = (area.width() as f64 / (x1 - x0).max(1.0)).min(area.height() as f64 / (y1 - y0).max(1.0));
                    let sc = fit * self.zoom as f64;
                    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                    let c = area.center() + Vec2::new(0.0, 8.0) + self.pan;
                    let to = |p: (f64, f64)| Pos2::new(c.x + ((p.0 - cx) * sc) as f32, c.y - ((p.1 - cy) * sc) as f32);
                    for pass in 0..4 {
                        for f in feet {
                            let k = match (f.kind, pass) {
                                (FootKind::Wall, 0) | (FootKind::Spoke, 1) | (FootKind::Pillar, 2) | (FootKind::Bearing, 3) => f.kind,
                                _ => continue,
                            };
                            let col = match k {
                                FootKind::Wall => GRID_COL[f.grid.min(3)],
                                FootKind::Spoke => SPOKE_COL[f.grid.min(3)],
                                FootKind::Pillar => PILLAR_COL,
                                FootKind::Bearing => BEARING_COL,
                            };
                            let pts: Vec<Pos2> = f.pts.iter().map(|p| to(*p)).collect();
                            if k == FootKind::Pillar && self.built_settings.posts_round {
                                let mid = Pos2::new((pts[0].x + pts[2].x) / 2.0, (pts[0].y + pts[2].y) / 2.0);
                                painter.circle_filled(mid, ((pts[2].x - pts[0].x).abs() / 2.0).max(1.0), col);
                            } else if k == FootKind::Bearing {
                                let mid = Pos2::new((pts[0].x + pts[2].x) / 2.0, (pts[0].y + pts[2].y) / 2.0);
                                let rr = ((pts[2].x - pts[0].x).abs() / 2.0).max(3.5);
                                painter.circle(mid, rr, col, Stroke::new(1.0_f32, Color32::BLACK));
                            } else {
                                painter.add(egui::Shape::convex_polygon(pts, col, Stroke::NONE));
                            }
                        }
                    }
                }
                Some(Err(e)) => {
                    painter.text(r.center(), egui::Align2::CENTER_CENTER, e, egui::FontId::proportional(14.0), WARN);
                }
                None => {}
            }
            if self.waiting.is_some() {
                painter.text(r.left_bottom() + Vec2::new(14.0, -12.0), egui::Align2::LEFT_BOTTOM, "updating…", egui::FontId::proportional(12.0), NOTE);
            }
        });
        ui.add_space(8.0);

        // ---- stats ----
        let (bricks, ents, cells, move_txt) = match &self.built {
            Some(Ok((b, _))) => (b.brick_count().to_string(), b.entities.len().to_string(), b.cells.to_string(), format!("corners within {:.1} M", b.corner_worst / 2.0)),
            _ => ("–".into(), "–".into(), "–".into(), String::new()),
        };
        let size = match &self.built {
            Some(Ok((_, feet))) => {
                let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
                for f in feet {
                    for p in f.pts {
                        x0 = x0.min(p.0);
                        y0 = y0.min(p.1);
                        x1 = x1.max(p.0);
                        y1 = y1.max(p.1);
                    }
                }
                format!("{} × {}", fmt_num(((x1 - x0) / 10.0 * 2.0).round() / 2.0), fmt_num(((y1 - y0) / 10.0 * 2.0).round() / 2.0))
            }
            _ => "–".into(),
        };
        let tall = fmt_num(self.s.wall_height as f64 / 12.0);
        ui.horizontal(|ui| {
            let w = (ui.available_width() - 3.0 * 8.0) / 4.0;
            for (big, small) in [
                (bricks, "bricks in total".to_string()),
                (ents, "entities (on bearings)".to_string()),
                (cells, format!("cells · {move_txt}")),
                (size, format!("studs · {tall} bricks tall")),
            ] {
                egui::Frame::new().fill(CARD).corner_radius(CornerRadius::same(10)).inner_margin(egui::Margin::symmetric(12, 7)).show(ui, |ui| {
                    ui.set_width((w - 24.0).max(40.0));
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.label(bold(big, 19.0).color(Color32::WHITE));
                        ui.label(RichText::new(small).size(11.0).color(SOFT));
                    });
                });
            }
        });
        ui.add_space(8.0);

        // ---- area / border / plate cards (wrap when narrow) ----
        let top = ui.cursor().top();
        let width = ui.available_width();
        let cols = if width > 900.0 { 3 } else if width > 560.0 { 2 } else { 1 };
        let cw = (width - (cols as f32 - 1.0) * 8.0) / cols as f32;
        let mut idx = 0;
        let mut row_items: Vec<u8> = vec![0, 1, 2];
        while !row_items.is_empty() {
            let take: Vec<u8> = row_items.drain(..cols.min(row_items.len())).collect();
            ui.horizontal_top(|ui| {
                for which in take {
                    ui.allocate_ui_with_layout(Vec2::new(cw, 10.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.set_width(cw);
                        match which {
                            0 => self.area_card(ui),
                            1 => self.border_card(ui),
                            _ => self.plate_card(ui),
                        }
                    });
                    idx += 1;
                }
            });
            ui.add_space(6.0);
        }
        let _ = idx;
        self.cards_h = (ui.cursor().top() - top).max(80.0);

        // ---- output ----
        let ok = matches!(self.built, Some(Ok(_))) && self.waiting.is_none();
        ui.horizontal(|ui| {
            let bw = 160.0;
            let r = ui.add_sized([ui.available_width() - 2.0 * bw - 16.0, 32.0], egui::TextEdit::singleline(&mut self.file_name).vertical_align(egui::Align::Center));
            if r.changed() {
                self.name_edited = true;
            }
            let save = egui::Button::new(bold("Save to Prefabs", 14.0).color(Color32::WHITE)).fill(GREEN).corner_radius(CornerRadius::same(9));
            if ui.add_enabled(ok, save).on_hover_text("Saves to %LOCALAPPDATA%\\Brickadia\\Saved\\Prefabs").clicked() {
                self.save(false);
            }
            let copy = egui::Button::new(bold("Copy to clipboard", 14.0).color(Color32::WHITE)).fill(ACCENT).corner_radius(CornerRadius::same(9));
            if ui.add_enabled(ok, copy).on_hover_text("Then paste in game with Ctrl+V").clicked() {
                self.copy();
            }
        });
        if self.name_edited && ui.small_button("↺ Use automatic name").clicked() {
            self.name_edited = false;
            self.file_name = self.auto_name();
        }
    }

    fn area_card(&mut self, ui: &mut egui::Ui) {
        let lw = 62.0;
        let u = self.units;
        card(ui, |ui| {
            ui.horizontal(|ui| {
                label(ui, "Area", lw);
                seg(ui, &mut self.s.area, &[(Area::Hexagon, "Hexagon"), (Area::Rectangle, "Rect."), (Area::Circle, "Circle"), (Area::Oval, "Oval")]);
            });
            let linked = matches!(self.s.area, Area::Hexagon | Area::Circle);
            let mut w = self.s.width / 2;
            let n = self.unit_note(w);
            if size_row(ui, "Width", lw, &mut w, 10, 2000, 20000, u, &n, true) {
                self.s.width = w * 2;
                if linked {
                    self.s.length = self.s.width;
                }
            }
            let mut l = if linked { self.s.width / 2 } else { self.s.length / 2 };
            let n = self.unit_note(l);
            if size_row(ui, "Length", lw, &mut l, 10, 2000, 20000, u, &n, !linked) {
                self.s.length = l * 2;
            }
            note(ui, 0.0, if linked { "🔗 width and length linked for this shape" } else { "Paint mode arrives in stage 3" });
        });
    }

    fn border_card(&mut self, ui: &mut egui::Ui) {
        let lw = 62.0;
        card(ui, |ui| {
            ui.add_enabled_ui(false, |ui| {
                let mut b = 0;
                ui.horizontal(|ui| {
                    label(ui, "Border", lw);
                    seg(ui, &mut b, &[(0, "Off"), (1, "Square"), (2, "Rect."), (3, "Circle"), (4, "Oval")]);
                });
                let mut e = 0;
                ui.horizontal(|ui| {
                    label(ui, "Edges", lw);
                    seg(ui, &mut e, &[(0, "Cut clean"), (1, "Whole cells")]);
                });
                ui.horizontal(|ui| {
                    label(ui, "Frame", lw);
                    let mut f = false;
                    ui.checkbox(&mut f, "on");
                });
            });
            note(ui, 0.0, "Borders arrive in stage 4");
        });
    }

    fn plate_card(&mut self, ui: &mut egui::Ui) {
        let lw = 62.0;
        card(ui, |ui| {
            ui.add_enabled_ui(false, |ui| {
                let mut p = 0;
                ui.horizontal(|ui| {
                    label(ui, "Plate", lw);
                    seg(ui, &mut p, &[(0, "None"), (1, "Full"), (2, "Follow shape")]);
                });
            });
            ui.horizontal(|ui| {
                label(ui, "Way up", lw);
                seg(ui, &mut self.s.way_up, &[(WayUp::Upright, "Upright"), (WayUp::UpsideDown, "Upside down")]);
            });
            note(ui, 0.0, "Plate arrives in stage 4");
        });
    }
}

fn main() -> eframe::Result {
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("BRZ Pattern Generator")
            .with_inner_size([1400.0, 920.0])
            .with_min_inner_size([1000.0, 700.0]),
        persist_window: true,
        ..Default::default()
    };
    eframe::run_native("BRZ Pattern Generator", opts, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
