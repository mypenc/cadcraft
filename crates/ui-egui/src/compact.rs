//! Phone and tablet layout.
//!
//! The desktop window is built for a mouse and a big screen: a menu bar, a tool bar, docked
//! palettes on both sides, a tiny status bar. On a phone or pad that leaves the drawing a thin
//! strip. In the compact layouts the chrome is rebuilt for fingers:
//!
//! * **top bar**: a menu button (the whole menu tree), the drawing name, Undo / Redo and a
//!   scrolling row of the tool bar's commands at touch size;
//! * **bottom dock** (fixed, not floating): Esc, Enter, the drafting toggles (Grid, Snap, Ortho,
//!   Polar, Object Snap, Dynamic Input) and the Panels button; the command line and the
//!   coordinates stay in the drawing area as on the desktop;
//! * **panels** (Layers, Properties, Tool Sets): a side panel on a tablet, a bottom sheet on a
//!   phone, opened from the dock, never docked by default.
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Layers,
    Properties,
    ToolSets,
}

/// Compact layout state (not saved except the preference).
#[derive(Debug, Default)]
pub struct State {
    pub pref: Pref,
    /// The panels are open (a side panel on a tablet, a sheet on a phone).
    pub panels_open: bool,
    pub tab: Tab,
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
    if has_doc && mode == Mode::Tablet && app.compact.panels_open {
        egui::Panel::right("cc_compact_side").exact_size(300.0).frame(egui::Frame::NONE.fill(t.panel)).show(ui, |ui| {
            panel_tabs(app, ui);
        });
    }
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.canvas)).show(ui, |ui| {
        if has_doc {
            ui.painter().rect_filled(ui.max_rect(), 0.0, t.canvas);
            crate::canvas::show(app, ui);
        } else {
            chrome::start_page(app, ui);
        }
    });
    if has_doc && mode == Mode::Phone && app.compact.panels_open {
        sheet(app, ui.ctx());
    }
}

fn top_bar(app: &mut CadApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    egui::Panel::top("cc_compact_top").exact_size(ROW * 2.0).frame(egui::Frame::NONE.fill(t.chrome).inner_margin(Margin::symmetric(6, 3))).show(ui, |ui| {
        let mut clicked: Option<String> = None;
        let mut command: Option<&'static str> = None;
        ui.horizontal(|ui| {
            ui.set_min_height(ROW - 6.0);
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
            let title = app.session.state().map(|s| s.title.clone()).unwrap_or_else(|_| "CADCraft".into());
            ui.add(egui::Label::new(RichText::new(title).strong()).truncate());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icons::button(ui, icons::Icon::Redo, 30.0, "Redo", false).clicked() {
                    command = Some("redo");
                }
                if icons::button(ui, icons::Icon::Undo, 30.0, "Undo", false).clicked() {
                    command = Some("u");
                }
            });
        });
        // The tool bar's commands at touch size, one scrolling row.
        egui::ScrollArea::horizontal().id_salt("compact_toolbar").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
            ui.horizontal(|ui| {
                for group in chrome::TOOLBAR {
                    for (icon, cmd, tip, _) in group.iter() {
                        if icons::button(ui, *icon, 32.0, tip, false).clicked() {
                            command = Some(cmd);
                        }
                    }
                    ui.add_space(10.0);
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

/// The fixed bottom dock: Esc, Enter, the drafting toggles and the panels button.
fn dock(app: &mut CadApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    egui::Panel::bottom("cc_compact_dock").exact_size(ROW + 8.0).frame(egui::Frame::NONE.fill(t.chrome).inner_margin(Margin::symmetric(6, 4))).show(ui, |ui| {
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
        let mut panels = false;
        let size = Vec2::new(54.0, ROW - 4.0);
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
                ui.separator();
                if ui.add(egui::Button::new("Panels").selected(app.compact.panels_open).min_size(Vec2::new(64.0, size.y))).clicked() {
                    panels = true;
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
        if panels {
            app.compact.panels_open = !app.compact.panels_open;
        }
    });
}

fn panel_tabs(app: &mut CadApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        for (tab, label) in [(Tab::Layers, "Layers"), (Tab::Properties, "Properties"), (Tab::ToolSets, "Tool Sets")] {
            if ui.selectable_label(app.compact.tab == tab, crate::i18n::t(label)).clicked() {
                app.compact.tab = tab;
            }
        }
    });
    ui.separator();
    match app.compact.tab {
        Tab::Layers => {
            egui::ScrollArea::vertical().id_salt("compact-layers").auto_shrink([false, false]).show(ui, |ui| crate::palettes::layers_section(app, ui));
        }
        Tab::Properties => {
            egui::ScrollArea::vertical().id_salt("compact-properties").auto_shrink([false, false]).show(ui, |ui| {
                crate::shortcut_menus::palette(app, ui, crate::palettes::properties_section, crate::shortcut_menus::properties_entries)
            });
        }
        Tab::ToolSets => crate::palettes::toolsets(app, ui),
    }
}

/// The phone's panels: a sheet over the bottom of the drawing.
fn sheet(app: &mut CadApp, ctx: &egui::Context) {
    let screen = ctx.content_rect();
    let mut open = true;
    egui::Window::new(crate::i18n::t("Panels"))
        .id(egui::Id::new("cc_compact_sheet"))
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -(ROW + 12.0)))
        .fixed_size(Vec2::new(screen.width() - 12.0, screen.height() * 0.5))
        .show(ctx, |ui| {
            let _ = Sense::hover();
            panel_tabs(app, ui);
        });
    if !open {
        app.compact.panels_open = false;
    }
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
