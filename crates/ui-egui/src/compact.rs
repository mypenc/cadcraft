//! Phone and tablet layout.
//!
//! The desktop window is built for a mouse and a big screen: a menu bar, a tool bar, docked
//! palettes on both sides, a tiny status bar. On a phone or pad that leaves the drawing a thin
//! strip. In the compact layouts (modelled on the mobile DWG viewers) the chrome is rebuilt for
//! fingers and the drawing gets the whole screen:
//!
//! * **top bar**: close (X), the drawing name with `*` when it has unsaved changes, Undo, Redo,
//!   Save and the menu button (the whole menu tree);
//! * **tool rail** down the right edge: Select, Sheets, Properties, Layers, View, Draw, Modify,
//!   Annotate, Insert, Tools, Constrain, each an icon with a label. A category opens a flyout
//!   beside the rail (commands as a grid, or the Layers / Properties palettes); choosing a
//!   command closes it so the drawing is clear for the prompts;
//! * **bottom dock** (thin, fixed, hidden with the status bar setting): Esc, Enter and the
//!   drafting toggles; the command line and coordinates stay in the drawing area.
//!
//! The mode follows the window width unless View ▸ Layout forces one. The floating touch bar
//! ([`crate::touch`]) is off by default because the dock carries its keys.

use egui::{Align, Layout, Margin, RichText, Sense, Vec2};
use serde_json::json;

use crate::theme::Tokens;
use crate::{CadApp, chrome, icons, menus};

/// Widest window (points) still laid out as a phone.
pub const PHONE_MAX_WIDTH: f32 = 600.0;

/// The user's choice in View ▸ Layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pref {
    /// Phone or tablet by window width on a touch device, desktop otherwise.
    #[default]
    Auto,
    Desktop,
    Tablet,
    Phone,
}

impl Pref {
    pub fn as_str(self) -> &'static str {
        match self {
            Pref::Auto => "auto",
            Pref::Desktop => "desktop",
            Pref::Tablet => "tablet",
            Pref::Phone => "phone",
        }
    }

    pub fn parse(s: &str) -> Option<Pref> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Some(Pref::Auto),
            "desktop" => Some(Pref::Desktop),
            "tablet" | "pad" => Some(Pref::Tablet),
            "phone" => Some(Pref::Phone),
            _ => None,
        }
    }
}

/// The layout in use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Desktop,
    Tablet,
    Phone,
}

/// Which layout a window of `width` points gets.
pub fn mode_for(pref: Pref, width: f32, touch: bool) -> Mode {
    match pref {
        Pref::Desktop => Mode::Desktop,
        Pref::Tablet => Mode::Tablet,
        Pref::Phone => Mode::Phone,
        Pref::Auto if !touch => Mode::Desktop,
        Pref::Auto if width < PHONE_MAX_WIDTH => Mode::Phone,
        Pref::Auto => Mode::Tablet,
    }
}

/// A rail category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cat {
    Select,
    Sheets,
    Properties,
    Layers,
    View,
    Draw,
    Modify,
    Annotate,
    Insert,
    Tools,
    Constrain,
}

impl Cat {
    const ALL: [Cat; 11] =
        [Cat::Select, Cat::Sheets, Cat::Properties, Cat::Layers, Cat::View, Cat::Draw, Cat::Modify, Cat::Annotate, Cat::Insert, Cat::Tools, Cat::Constrain];

    fn label(self) -> &'static str {
        match self {
            Cat::Select => "Select",
            Cat::Sheets => "Sheets",
            Cat::Properties => "Properties",
            Cat::Layers => "Layers",
            Cat::View => "View",
            Cat::Draw => "Draw",
            Cat::Modify => "Modify",
            Cat::Annotate => "Annotate",
            Cat::Insert => "Insert",
            Cat::Tools => "Tools",
            Cat::Constrain => "Constrain",
        }
    }

    fn icon(self) -> icons::Icon {
        use icons::Icon;
        match self {
            Cat::Select => Icon::QuickProps,
            Cat::Sheets => Icon::PageSetup,
            Cat::Properties => Icon::Properties,
            Cat::Layers => Icon::Layers,
            Cat::View => Icon::ZoomExtents,
            Cat::Draw => Icon::Line,
            Cat::Modify => Icon::Move,
            Cat::Annotate => Icon::Text,
            Cat::Insert => Icon::Insert,
            Cat::Tools => Icon::Measure,
            Cat::Constrain => Icon::Constraint,
        }
    }

    /// The Tool Sets groups a category shows.
    fn groups(self) -> &'static [&'static str] {
        match self {
            Cat::Draw => &["Draw", "Hatch"],
            Cat::Modify => &["Modify"],
            Cat::Annotate => &["Text", "Dimension", "Leader", "Table"],
            Cat::Insert => &["Block"],
            Cat::Constrain => &["Parametric"],
            _ => &[],
        }
    }
}

/// Compact layout state (not saved except the preference).
#[derive(Debug, Default)]
pub struct State {
    pub pref: Pref,
    /// The rail category whose flyout is open.
    pub rail: Option<Cat>,
}

/// Height of a touch row.
const ROW: f32 = 44.0;

/// Draw the whole window in the compact layout.
pub fn draw(app: &mut CadApp, ui: &mut egui::Ui, mode: Mode) {
    let t = Tokens::get();
    top_bar(app, ui);
    if app.ui.show_status_bar {
        dock(app, ui);
    }
    let has_doc = !app.session.docs.is_empty() && !app.ui.start_tab;
    if has_doc {
        rail(app, ui, mode);
        if let Some(cat) = app.compact.rail.filter(|c| *c != Cat::Select) {
            flyout(app, ui, mode, cat);
        }
    }
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.canvas)).show(ui, |ui| {
        if has_doc {
            ui.painter().rect_filled(ui.max_rect(), 0.0, t.canvas);
            crate::canvas::show(app, ui);
        } else {
            chrome::start_page(app, ui);
        }
    });
}

fn top_bar(app: &mut CadApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    egui::Panel::top("cc_compact_top").exact_size(ROW).frame(egui::Frame::NONE.fill(t.chrome).inner_margin(Margin::symmetric(6, 3))).show(ui, |ui| {
        let mut clicked: Option<String> = None;
        let mut command: Option<&'static str> = None;
        let has_doc = !app.session.docs.is_empty() && !app.ui.start_tab;
        ui.horizontal_centered(|ui| {
            if has_doc && icons::button(ui, icons::Icon::Close, 30.0, "Close", false).clicked() {
                command = Some("close");
            }
            let title = match app.session.state() {
                Ok(st) if has_doc => format!("[{}{}]", st.title, if st.is_dirty() { "*" } else { "" }),
                _ => "CADCraft".to_string(),
            };
            ui.add(egui::Label::new(RichText::new(title).strong()).truncate());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let tree = menus::tree(app);
                ui.menu_button(RichText::new("\u{2630}").size(22.0), |ui| {
                    egui::ScrollArea::vertical().max_height(ui.ctx().content_rect().height() * 0.7).show(ui, |ui| {
                        for (name, entries) in &tree {
                            if entries.is_empty() {
                                continue;
                            }
                            ui.menu_button(crate::i18n::menu(name), |ui| {
                                for e in entries {
                                    menus::entry_ui(ui, e, &mut clicked);
                                }
                            });
                        }
                    });
                });
                if has_doc {
                    if icons::button(ui, icons::Icon::Save, 30.0, "Save", false).clicked() {
                        command = Some("qsave");
                    }
                    if icons::button(ui, icons::Icon::Redo, 30.0, "Redo", false).clicked() {
                        command = Some("redo");
                    }
                    if icons::button(ui, icons::Icon::Undo, 30.0, "Undo", false).clicked() {
                        command = Some("u");
                    }
                }
            });
        });
        if let Some(id) = clicked {
            menus::activate(app, &id);
        }
        if let Some(c) = command {
            app.start(c);
        }
    });
}

/// An icon over a label, as the rail and the flyout grid draw their cells.
fn cell(ui: &mut egui::Ui, size: Vec2, icon: icons::Icon, label: &str, selected: bool) -> egui::Response {
    let t = Tokens::get();
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if selected {
        ui.painter().rect_filled(rect.shrink(2.0), 6.0, t.accent.gamma_multiply(0.35));
    } else if resp.hovered() || resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(rect.shrink(2.0), 6.0, t.control_hover.gamma_multiply(0.6));
    }
    let icon_rect = egui::Rect::from_center_size(egui::pos2(rect.center().x, rect.top() + 21.0), Vec2::splat(26.0));
    icons::paint(ui.painter(), icon_rect, icon, false);
    let text = crate::i18n::t(label).to_string();
    let galley = ui.painter().layout(text, egui::FontId::proportional(10.5), t.text, size.x - 4.0);
    let at = egui::pos2(rect.center().x - galley.size().x / 2.0, rect.top() + 38.0);
    ui.painter().galley(at, galley, t.text);
    resp
}

/// The tool rail down the right edge.
fn rail(app: &mut CadApp, ui: &mut egui::Ui, mode: Mode) {
    let t = Tokens::get();
    let width = if mode == Mode::Tablet { 84.0 } else { 70.0 };
    let mut chosen: Option<Cat> = None;
    egui::Panel::right("cc_rail").exact_size(width).frame(egui::Frame::NONE.fill(t.chrome)).show(ui, |ui| {
        egui::ScrollArea::vertical().id_salt("compact_rail").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            for cat in Cat::ALL {
                let on = app.compact.rail == Some(cat);
                if cell(ui, Vec2::new(width, 58.0), cat.icon(), cat.label(), on).clicked() {
                    chosen = Some(cat);
                }
            }
        });
    });
    if let Some(cat) = chosen {
        if cat == Cat::Select {
            // Back to selecting: cancel the running command and close the flyout.
            app.compact.rail = None;
            app.touch.key(egui::Key::Escape, egui::Modifiers::NONE);
        } else if app.compact.rail == Some(cat) {
            app.compact.rail = None;
        } else {
            app.compact.rail = Some(cat);
        }
    }
}

/// The flyout beside the rail.
fn flyout(app: &mut CadApp, ui: &mut egui::Ui, mode: Mode, cat: Cat) {
    let t = Tokens::get();
    let screen = ui.ctx().content_rect().width();
    let wide = matches!(cat, Cat::Layers | Cat::Properties | Cat::Sheets);
    let want = match (mode, wide) {
        (Mode::Tablet, true) => 340.0,
        (Mode::Tablet, false) => 300.0,
        (_, true) => 280.0,
        (_, false) => 240.0,
    };
    let width = want.min(screen * 0.7);
    let mut command: Option<&'static str> = None;
    egui::Panel::right("cc_flyout").exact_size(width).frame(egui::Frame::NONE.fill(t.panel).inner_margin(Margin::same(6))).show(ui, |ui| {
        ui.label(RichText::new(crate::i18n::t(cat.label())).strong());
        ui.separator();
        match cat {
            Cat::Properties => {
                egui::ScrollArea::vertical().id_salt("compact-properties").auto_shrink([false, false]).show(ui, |ui| {
                    crate::shortcut_menus::palette(app, ui, crate::palettes::properties_section, crate::shortcut_menus::properties_entries)
                });
            }
            Cat::Layers => {
                egui::ScrollArea::vertical().id_salt("compact-layers").auto_shrink([false, false]).show(ui, |ui| crate::palettes::layers_section(app, ui));
            }
            Cat::Sheets => sheets(app, ui),
            Cat::View => view_tools(app, ui, width, &mut command),
            Cat::Tools => {
                let tools: [(icons::Icon, &'static str, &str); 3] =
                    [(icons::Icon::Measure, "dist", "Distance"), (icons::Icon::Area, "area", "Area"), (icons::Icon::List, "list", "List")];
                grid(ui, width, &tools, &mut command);
            }
            _ => {
                egui::ScrollArea::vertical().id_salt(("compact-tools", cat.label())).auto_shrink([false, false]).show(ui, |ui| {
                    for (name, large, small) in crate::palettes::DRAFTING {
                        if !cat.groups().contains(name) {
                            continue;
                        }
                        let tools: Vec<(icons::Icon, &'static str, &str)> = large.iter().chain(small.iter()).copied().collect();
                        if cat.groups().len() > 1 {
                            ui.label(RichText::new(crate::i18n::t(name)).weak());
                        }
                        grid(ui, width, &tools, &mut command);
                    }
                    if cat == Cat::Insert {
                        let more: [(icons::Icon, &'static str, &str); 3] =
                            [(icons::Icon::Import, "import", "Import"), (icons::Icon::Attach, "xattach", "Attach"), (icons::Icon::Blocks, "ui.dialog.blocks", "Blocks")];
                        grid(ui, width, &more, &mut command);
                    }
                });
            }
        }
    });
    if let Some(c) = command {
        // A command closes the flyout so the drawing is clear for its prompts.
        app.compact.rail = None;
        app.start(c);
    }
}

/// Commands as a wrapping grid of icon cells.
fn grid(ui: &mut egui::Ui, width: f32, tools: &[(icons::Icon, &'static str, &str)], command: &mut Option<&'static str>) {
    let columns = ((width - 12.0) / 74.0).floor().max(1.0);
    let cell_w = (width - 12.0) / columns;
    ui.spacing_mut().item_spacing = Vec2::ZERO;
    ui.horizontal_wrapped(|ui| {
        for (icon, id, label) in tools {
            if cell(ui, Vec2::new(cell_w.floor(), 62.0), *icon, label, false).clicked() {
                *command = Some(id);
            }
        }
    });
}

/// View: zoom and pan commands, then the drafting toggles.
fn view_tools(app: &mut CadApp, ui: &mut egui::Ui, width: f32, command: &mut Option<&'static str>) {
    let tools: [(icons::Icon, &'static str, &str); 4] = [
        (icons::Icon::ZoomExtents, "zoom.extents", "Extents"),
        (icons::Icon::ZoomWindow, "zoom.window", "Window"),
        (icons::Icon::Pan, "pan", "Pan"),
        (icons::Icon::Orbit, "3dorbit", "Orbit"),
    ];
    grid(ui, width, &tools, command);
    ui.separator();
    toggles(app, ui);
}

/// Drafting toggles as selectable buttons.
fn toggles(app: &mut CadApp, ui: &mut egui::Ui) {
    let s = app.session.settings.clone();
    use cadcraft_engine::snap::mode;
    let osnap_on = s.osmode & mode::OFF == 0 && s.osmode != 0;
    let list: [(&str, bool, &str); 8] = [
        ("Grid", s.gridmode, "grid"),
        ("Snap", s.snapmode, "snap"),
        ("Ortho", s.orthomode, "ortho"),
        ("Polar", s.polarmode, "polar"),
        ("OSnap", osnap_on, "osnap"),
        ("OTrack", s.otrack, "otrack"),
        ("DYN", s.dynmode, "dynmode"),
        ("Lineweight", s.lwdisplay, "lwdisplay"),
    ];
    let mut toggle: Option<&str> = None;
    ui.horizontal_wrapped(|ui| {
        for (label, on, cmd) in list {
            if ui.add(egui::Button::new(crate::i18n::t(label)).selected(on).min_size(Vec2::new(64.0, 36.0))).clicked() {
                toggle = Some(cmd);
            }
        }
    });
    if let Some(c) = toggle {
        menus::run_transparent(app, c);
    }
}

/// Sheets: Model and the layouts, and a new layout.
fn sheets(app: &mut CadApp, ui: &mut egui::Ui) {
    let current = app.session.state().map(|s| match &s.space {
        cadcraft_doc::Space::Model => "Model".to_string(),
        cadcraft_doc::Space::Paper(n) => n.clone(),
    });
    let mut names: Vec<String> = vec!["Model".into()];
    if let Ok(st) = app.session.state() {
        let mut ls: Vec<_> = st.doc.layouts.iter().collect();
        ls.sort_by_key(|l| l.tab_order);
        names.extend(ls.iter().map(|l| l.name.clone()));
    }
    let mut switch: Option<String> = None;
    for n in &names {
        let on = current.as_deref().is_ok_and(|c| c == n);
        let label = if n == "Model" { crate::i18n::t("Model").to_string() } else { n.clone() };
        if ui.add(egui::Button::new(label).selected(on).min_size(Vec2::new(ui.available_width(), 40.0))).clicked() {
            switch = Some(n.clone());
        }
    }
    if ui.add(egui::Button::new(format!("+ {}", crate::i18n::t("New layout"))).min_size(Vec2::new(ui.available_width(), 40.0))).clicked() {
        let _ = app.run("layout.new", json!({}));
    }
    if let Some(n) = switch {
        let _ = app.run("layout.set", json!({ "name": n }));
    }
}

/// The thin fixed bottom dock: Esc, Enter and the drafting toggles.
fn dock(app: &mut CadApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    egui::Panel::bottom("cc_compact_dock").exact_size(ROW).frame(egui::Frame::NONE.fill(t.chrome).inner_margin(Margin::symmetric(6, 3))).show(ui, |ui| {
        let s = app.session.settings.clone();
        use cadcraft_engine::snap::mode;
        let osnap_on = s.osmode & mode::OFF == 0 && s.osmode != 0;
        let toggles: [(&str, bool, &str); 6] = [
            ("Grid", s.gridmode, "grid"),
            ("Snap", s.snapmode, "snap"),
            ("Ortho", s.orthomode, "ortho"),
            ("Polar", s.polarmode, "polar"),
            ("OSnap", osnap_on, "osnap"),
            ("DYN", s.dynmode, "dynmode"),
        ];
        let mut toggle: Option<&str> = None;
        let mut key: Option<egui::Key> = None;
        let size = Vec2::new(52.0, ROW - 8.0);
        egui::ScrollArea::horizontal().id_salt("compact_dock").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new("Esc").min_size(size)).clicked() {
                    key = Some(egui::Key::Escape);
                }
                if ui.add(egui::Button::new("Enter").min_size(size)).clicked() {
                    key = Some(egui::Key::Enter);
                }
                ui.separator();
                for (label, on, cmd) in toggles {
                    if ui.add(egui::Button::new(label).selected(on).min_size(size)).clicked() {
                        toggle = Some(cmd);
                    }
                }
            });
        });
        if let Some(c) = toggle {
            menus::run_transparent(app, c);
        }
        if let Some(k) = key {
            app.touch.key(k, egui::Modifiers::NONE);
            ui.ctx().request_repaint();
        }
    });
}

/// View ▸ Layout: show or change the layout choice.
pub fn layout_json(app: &CadApp, mode: Mode) -> serde_json::Value {
    json!({ "layout": app.compact.pref.as_str(), "shown": match mode { Mode::Desktop => "desktop", Mode::Tablet => "tablet", Mode::Phone => "phone" } })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_devices_pick_phone_or_tablet_by_width_and_desktops_stay_desktop() {
        assert_eq!(mode_for(Pref::Auto, 400.0, true), Mode::Phone);
        assert_eq!(mode_for(Pref::Auto, 900.0, true), Mode::Tablet);
        assert_eq!(mode_for(Pref::Auto, 1400.0, true), Mode::Tablet);
        assert_eq!(mode_for(Pref::Auto, 400.0, false), Mode::Desktop);
        assert_eq!(mode_for(Pref::Phone, 2000.0, false), Mode::Phone);
        assert_eq!(mode_for(Pref::Desktop, 300.0, true), Mode::Desktop);
    }

    #[test]
    fn the_preference_round_trips_and_junk_is_refused() {
        for p in [Pref::Auto, Pref::Desktop, Pref::Tablet, Pref::Phone] {
            assert_eq!(Pref::parse(p.as_str()), Some(p));
        }
        assert_eq!(Pref::parse("pad"), Some(Pref::Tablet));
        assert_eq!(Pref::parse("watch"), None);
    }
}
