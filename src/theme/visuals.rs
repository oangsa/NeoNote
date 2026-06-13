use egui::{Color32, Visuals};

use super::schema::Theme;

pub fn parse_hex_color(hex: &str) -> Option<Color32> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }

    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color32::from_rgb(red, green, blue))
}

pub fn apply_to_egui_visuals(theme: &Theme) -> Visuals {
    let mut visuals = match theme.variant {
        super::schema::ThemeVariant::Dark => Visuals::dark(),
        super::schema::ThemeVariant::Light => Visuals::light(),
    };

    if let Some(color) = parse_hex_color(&theme.colors.background) {
        visuals.panel_fill = color;
        visuals.window_fill = color;
        visuals.extreme_bg_color = color;
    }
    if let Some(color) = parse_hex_color(&theme.colors.background_alt) {
        visuals.faint_bg_color = color;
    }
    if let Some(color) = parse_hex_color(&theme.colors.surface) {
        visuals.widgets.noninteractive.bg_fill = color;
        visuals.widgets.inactive.bg_fill = color;
        visuals.widgets.hovered.bg_fill = color.gamma_multiply(1.15);
        visuals.widgets.active.bg_fill = color.gamma_multiply(1.25);
    }
    if let Some(color) = parse_hex_color(&theme.colors.text) {
        visuals.override_text_color = Some(color);
    }
    if let Some(color) = parse_hex_color(&theme.colors.accent_primary) {
        visuals.selection.bg_fill = color;
        visuals.hyperlink_color = color;
    }
    if let Some(color) = parse_hex_color(&theme.colors.border) {
        visuals.window_stroke.color = color;
        visuals.widgets.noninteractive.bg_stroke.color = color;
        visuals.widgets.inactive.bg_stroke.color = color;
        visuals.widgets.hovered.bg_stroke.color = color;
    }

    visuals
}

impl Theme {
    pub fn to_visuals(&self) -> Visuals {
        apply_to_egui_visuals(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_color() {
        assert_eq!(
            parse_hex_color("#1e1e2e"),
            Some(Color32::from_rgb(30, 30, 46))
        );
    }

    #[test]
    fn rejects_invalid_hex_color() {
        assert_eq!(parse_hex_color("#12345"), None);
        assert_eq!(parse_hex_color("not-a-color"), None);
    }
}
