use crate::{
    persistence::AppDataPaths,
    theme::{
        visuals::parse_hex_color, Theme, ThemeColors, ThemeStore, ThemeVariant, VimModeColors,
    },
};

#[derive(Default)]
pub struct ThemePanel {
    open: bool,
    filter: ThemeFilter,
    builder: CustomThemeBuilder,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ThemeFilter {
    Dark,
    Light,
    #[default]
    All,
}

#[derive(Clone, Debug)]
pub enum ThemePanelAction {
    Preview(usize),
    ClearPreview,
    Apply(usize),
    Imported(String),
    ImportFailed(String),
    Saved(String),
    SaveFailed(String),
}

#[derive(Clone, Debug)]
struct CustomThemeBuilder {
    open: bool,
    name: String,
    author: String,
    variant: ThemeVariant,
    colors: [egui::Color32; 12],
    modes: [egui::Color32; 5],
}

impl Default for CustomThemeBuilder {
    fn default() -> Self {
        let fallback = Theme::fallback_dark();
        Self {
            open: false,
            name: "Custom Theme".to_string(),
            author: "NeoNote User".to_string(),
            variant: ThemeVariant::Dark,
            colors: theme_colors_to_array(&fallback),
            modes: mode_colors_to_array(&fallback),
        }
    }
}

impl ThemePanel {
    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        paths: &AppDataPaths,
        themes: &mut ThemeStore,
    ) -> Option<ThemePanelAction> {
        if !self.open {
            return None;
        }

        let mut action = None;
        let mut hovered_any = false;

        egui::Window::new("Themes")
            .open(&mut self.open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.filter, ThemeFilter::Dark, "Dark");
                    ui.selectable_value(&mut self.filter, ThemeFilter::Light, "Light");
                    ui.selectable_value(&mut self.filter, ThemeFilter::All, "All");
                });
                ui.separator();

                let variant = match self.filter {
                    ThemeFilter::Dark => Some(ThemeVariant::Dark),
                    ThemeFilter::Light => Some(ThemeVariant::Light),
                    ThemeFilter::All => None,
                };

                egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                    for (index, theme) in themes.filtered(variant) {
                        let response = ui.group(|ui| {
                            ui.horizontal(|ui| {
                                let active = if themes.is_active(index) { "✓ " } else { "" };
                                ui.strong(format!("{active}{}", theme.name));
                                ui.label(format!("by {}", theme.author));
                            });
                            ui.horizontal(|ui| {
                                for color in swatches(theme) {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(18.0, 18.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(rect, 2.0, color);
                                }
                            });
                            if ui.button("Apply").clicked() {
                                action = Some(ThemePanelAction::Apply(index));
                            }
                        });
                        if response.response.hovered() {
                            hovered_any = true;
                            action = Some(ThemePanelAction::Preview(index));
                        }
                    }
                });

                if !hovered_any && action.is_none() {
                    action = Some(ThemePanelAction::ClearPreview);
                }

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("+ Import Theme").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Theme JSON", &["json"])
                            .pick_file()
                        {
                            match themes.import_theme(&path, paths) {
                                Ok(theme) => {
                                    action = Some(ThemePanelAction::Imported(theme.name));
                                }
                                Err(error) => action = Some(ThemePanelAction::ImportFailed(error)),
                            }
                        }
                    }
                    if ui.button("+ Create Custom Theme").clicked() {
                        self.builder.open = true;
                    }
                });

                if self.builder.open {
                    ui.separator();
                    ui.collapsing("Custom Theme", |ui| {
                        ui.text_edit_singleline(&mut self.builder.name);
                        ui.text_edit_singleline(&mut self.builder.author);
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut self.builder.variant,
                                ThemeVariant::Dark,
                                "Dark",
                            );
                            ui.selectable_value(
                                &mut self.builder.variant,
                                ThemeVariant::Light,
                                "Light",
                            );
                        });
                        color_row(ui, "Colors", &mut self.builder.colors);
                        color_row(ui, "Modes", &mut self.builder.modes);
                        if ui.button("Save").clicked() {
                            let theme = self.builder.to_theme();
                            match themes.save_user_theme(&theme, paths) {
                                Ok(path) => {
                                    action = Some(ThemePanelAction::Saved(path.display().to_string()))
                                }
                                Err(error) => action = Some(ThemePanelAction::SaveFailed(error)),
                            }
                        }
                    });
                }
            });

        action
    }
}

impl CustomThemeBuilder {
    fn to_theme(&self) -> Theme {
        Theme {
            name: self.name.clone(),
            variant: self.variant,
            author: self.author.clone(),
            colors: ThemeColors {
                background: to_hex(self.colors[0]),
                background_alt: to_hex(self.colors[1]),
                surface: to_hex(self.colors[2]),
                border: to_hex(self.colors[3]),
                text: to_hex(self.colors[4]),
                text_muted: to_hex(self.colors[5]),
                accent_primary: to_hex(self.colors[6]),
                accent_secondary: to_hex(self.colors[7]),
                success: to_hex(self.colors[8]),
                warning: to_hex(self.colors[9]),
                error: to_hex(self.colors[10]),
                cursor: to_hex(self.colors[11]),
            },
            vim_modes: VimModeColors {
                normal: to_hex(self.modes[0]),
                insert: to_hex(self.modes[1]),
                visual: to_hex(self.modes[2]),
                command: to_hex(self.modes[3]),
                replace: to_hex(self.modes[4]),
            },
        }
    }
}

fn color_row(ui: &mut egui::Ui, label: &str, colors: &mut [egui::Color32]) {
    ui.label(label);
    ui.horizontal_wrapped(|ui| {
        for color in colors {
            ui.color_edit_button_srgba(color);
        }
    });
}

fn swatches(theme: &Theme) -> Vec<egui::Color32> {
    [
        &theme.colors.background,
        &theme.colors.surface,
        &theme.colors.accent_primary,
        &theme.colors.accent_secondary,
        &theme.colors.success,
        &theme.colors.error,
    ]
    .iter()
    .filter_map(|color| parse_hex_color(color))
    .collect()
}

fn theme_colors_to_array(theme: &Theme) -> [egui::Color32; 12] {
    [
        &theme.colors.background,
        &theme.colors.background_alt,
        &theme.colors.surface,
        &theme.colors.border,
        &theme.colors.text,
        &theme.colors.text_muted,
        &theme.colors.accent_primary,
        &theme.colors.accent_secondary,
        &theme.colors.success,
        &theme.colors.warning,
        &theme.colors.error,
        &theme.colors.cursor,
    ]
    .map(|color| parse_hex_color(color).unwrap_or(egui::Color32::WHITE))
}

fn mode_colors_to_array(theme: &Theme) -> [egui::Color32; 5] {
    [
        &theme.vim_modes.normal,
        &theme.vim_modes.insert,
        &theme.vim_modes.visual,
        &theme.vim_modes.command,
        &theme.vim_modes.replace,
    ]
    .map(|color| parse_hex_color(color).unwrap_or(egui::Color32::WHITE))
}

fn to_hex(color: egui::Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
}
