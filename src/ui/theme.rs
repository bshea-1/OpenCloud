#![allow(dead_code)]

use egui::{Color32, Stroke, Style, Theme, Visuals};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudioThemeId {
    Light,
    Obsidian,
    Slate,
    Midnight,
}

impl StudioThemeId {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().trim() {
            "obsidian" | "dark" => StudioThemeId::Obsidian,
            "slate" => StudioThemeId::Slate,
            "midnight" => StudioThemeId::Midnight,
            _ => StudioThemeId::Light,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            StudioThemeId::Light => "light",
            StudioThemeId::Obsidian => "obsidian",
            StudioThemeId::Slate => "slate",
            StudioThemeId::Midnight => "midnight",
        }
    }
}

/// Dynamic Theme Palette representing all interface surface and accent colors
#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    pub is_dark: bool,
    pub canvas_bg: Color32,
    pub panel_bg: Color32,
    pub card_bg: Color32,
    pub card_hover: Color32,
    pub border: Color32,
    pub border_active: Color32,
    pub text_primary: Color32,
    pub text_muted: Color32,
    pub accent_blue: Color32,
    pub success_green: Color32,
    pub update_amber: Color32,
    pub banner_bg: Color32,
    pub banner_border: Color32,
    pub banner_text: Color32,
    pub sidebar_active: Color32,
    pub sidebar_hover: Color32,
    pub search_bg: Color32,
    pub button_secondary_bg: Color32,
    pub button_secondary_hover: Color32,
    pub category_pill_bg: Color32,
    pub popup_bg: Color32,
}

impl ThemePalette {
    pub fn from_theme(theme_name: &str) -> Self {
        match StudioThemeId::from_str(theme_name) {
            StudioThemeId::Light => Self::light(),
            StudioThemeId::Obsidian => Self::obsidian(),
            StudioThemeId::Slate => Self::slate(),
            StudioThemeId::Midnight => Self::midnight(),
        }
    }

    pub fn light() -> Self {
        Self {
            is_dark: false,
            canvas_bg: Color32::from_rgb(244, 244, 246),      // #f4f4f6 (Subtle studio canvas)
            panel_bg: Color32::from_rgb(255, 255, 255),       // #ffffff (Clean white header & sidebar)
            card_bg: Color32::from_rgb(255, 255, 255),        // #ffffff (Card surface)
            card_hover: Color32::from_rgb(250, 250, 252),     // #fafafc
            border: Color32::from_rgb(228, 228, 232),         // #e4e4e8 (Subtle divider border)
            border_active: Color32::from_rgb(180, 182, 190),  // #b4b6be
            text_primary: Color32::from_rgb(26, 26, 30),      // #1a1a1e (Deep black)
            text_muted: Color32::from_rgb(110, 110, 118),     // #6e6e76 (Muted description)
            accent_blue: Color32::from_rgb(20, 115, 230),    // #1473e6 (Signature Action Blue)
            success_green: Color32::from_rgb(38, 162, 105),  // #26a269 (Active / Installed)
            update_amber: Color32::from_rgb(230, 134, 25),   // #e68619 (Update Available)
            banner_bg: Color32::from_rgb(28, 29, 36),        // #1c1d24 (Dark Hero Card)
            banner_border: Color32::from_rgb(45, 46, 56),
            banner_text: Color32::WHITE,
            sidebar_active: Color32::from_rgb(235, 235, 239), // #ebebef (Active Pill Fill)
            sidebar_hover: Color32::from_rgb(244, 244, 247),
            search_bg: Color32::from_rgb(240, 240, 243),      // #f0f0f3 (Search Input Pill)
            button_secondary_bg: Color32::from_rgb(244, 244, 246),
            button_secondary_hover: Color32::from_rgb(235, 235, 239),
            category_pill_bg: Color32::from_rgb(240, 240, 243),
            popup_bg: Color32::from_rgb(255, 255, 255),
        }
    }

    pub fn obsidian() -> Self {
        Self {
            is_dark: true,
            canvas_bg: Color32::from_rgb(18, 18, 20),         // #121214 (Obsidian deep canvas)
            panel_bg: Color32::from_rgb(26, 26, 30),          // #1a1a1e (Dark charcoal panel)
            card_bg: Color32::from_rgb(34, 34, 40),           // #222228 (Elevated dark card)
            card_hover: Color32::from_rgb(44, 44, 52),
            border: Color32::from_rgb(52, 52, 62),            // #34343e (Subtle dark border)
            border_active: Color32::from_rgb(85, 85, 102),
            text_primary: Color32::from_rgb(242, 242, 247),   // Crisp white/light grey
            text_muted: Color32::from_rgb(160, 160, 175),     // Medium grey
            accent_blue: Color32::from_rgb(37, 130, 242),     // Vibrant blue
            success_green: Color32::from_rgb(46, 184, 114),
            update_amber: Color32::from_rgb(245, 158, 11),
            banner_bg: Color32::from_rgb(12, 12, 15),
            banner_border: Color32::from_rgb(45, 45, 55),
            banner_text: Color32::WHITE,
            sidebar_active: Color32::from_rgb(44, 44, 54),
            sidebar_hover: Color32::from_rgb(34, 34, 42),
            search_bg: Color32::from_rgb(34, 34, 40),
            button_secondary_bg: Color32::from_rgb(42, 42, 52),
            button_secondary_hover: Color32::from_rgb(54, 54, 66),
            category_pill_bg: Color32::from_rgb(42, 42, 52),
            popup_bg: Color32::from_rgb(28, 28, 34),
        }
    }

    pub fn slate() -> Self {
        Self {
            is_dark: true,
            canvas_bg: Color32::from_rgb(15, 23, 42),         // #0f172a (Slate-900 canvas)
            panel_bg: Color32::from_rgb(30, 41, 59),          // #1e293b (Slate-800 panel)
            card_bg: Color32::from_rgb(51, 65, 85),           // #334155 (Slate-700 card)
            card_hover: Color32::from_rgb(71, 85, 105),       // #475569
            border: Color32::from_rgb(71, 85, 105),
            border_active: Color32::from_rgb(100, 116, 139),
            text_primary: Color32::from_rgb(248, 250, 252),
            text_muted: Color32::from_rgb(148, 163, 184),
            accent_blue: Color32::from_rgb(56, 189, 248),     // Sky accent
            success_green: Color32::from_rgb(52, 211, 153),
            update_amber: Color32::from_rgb(251, 191, 36),
            banner_bg: Color32::from_rgb(10, 15, 30),
            banner_border: Color32::from_rgb(38, 52, 75),
            banner_text: Color32::WHITE,
            sidebar_active: Color32::from_rgb(51, 65, 85),
            sidebar_hover: Color32::from_rgb(38, 52, 75),
            search_bg: Color32::from_rgb(22, 32, 48),
            button_secondary_bg: Color32::from_rgb(51, 65, 85),
            button_secondary_hover: Color32::from_rgb(71, 85, 105),
            category_pill_bg: Color32::from_rgb(38, 52, 75),
            popup_bg: Color32::from_rgb(30, 41, 59),
        }
    }

    pub fn midnight() -> Self {
        Self {
            is_dark: true,
            canvas_bg: Color32::from_rgb(0, 0, 0),            // Pure OLED black
            panel_bg: Color32::from_rgb(11, 15, 25),          // #0b0f19
            card_bg: Color32::from_rgb(17, 24, 39),           // #111827
            card_hover: Color32::from_rgb(31, 41, 55),        // #1f2937
            border: Color32::from_rgb(31, 41, 55),
            border_active: Color32::from_rgb(55, 65, 81),
            text_primary: Color32::from_rgb(255, 255, 255),
            text_muted: Color32::from_rgb(156, 163, 175),
            accent_blue: Color32::from_rgb(96, 165, 250),
            success_green: Color32::from_rgb(52, 211, 153),
            update_amber: Color32::from_rgb(251, 191, 36),
            banner_bg: Color32::from_rgb(6, 9, 15),
            banner_border: Color32::from_rgb(31, 41, 55),
            banner_text: Color32::WHITE,
            sidebar_active: Color32::from_rgb(24, 34, 53),
            sidebar_hover: Color32::from_rgb(15, 22, 35),
            search_bg: Color32::from_rgb(17, 24, 39),
            button_secondary_bg: Color32::from_rgb(24, 34, 53),
            button_secondary_hover: Color32::from_rgb(31, 41, 55),
            category_pill_bg: Color32::from_rgb(24, 34, 53),
            popup_bg: Color32::from_rgb(11, 15, 25),
        }
    }
}

// Default compile-time constant fallbacks for backwards compatibility
pub const COLOR_CANVAS_BG: Color32 = Color32::from_rgb(244, 244, 246);
pub const COLOR_PANEL_BG: Color32 = Color32::from_rgb(255, 255, 255);
pub const COLOR_CARD_BG: Color32 = Color32::from_rgb(255, 255, 255);
pub const COLOR_CARD_HOVER: Color32 = Color32::from_rgb(250, 250, 252);
pub const COLOR_BORDER: Color32 = Color32::from_rgb(228, 228, 232);
pub const COLOR_BORDER_ACTIVE: Color32 = Color32::from_rgb(180, 182, 190);
pub const COLOR_TEXT_PRIMARY: Color32 = Color32::from_rgb(26, 26, 30);
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(110, 110, 118);
pub const COLOR_ACCENT_BLUE: Color32 = Color32::from_rgb(20, 115, 230);
pub const COLOR_SUCCESS_GREEN: Color32 = Color32::from_rgb(38, 162, 105);
pub const COLOR_UPDATE_AMBER: Color32 = Color32::from_rgb(230, 134, 25);
pub const COLOR_BANNER_BG: Color32 = Color32::from_rgb(28, 29, 36);
pub const COLOR_SIDEBAR_ACTIVE: Color32 = Color32::from_rgb(235, 235, 239);
pub const COLOR_SEARCH_BG: Color32 = Color32::from_rgb(240, 240, 243);

pub fn apply_studio_theme(ctx: &egui::Context, palette: &ThemePalette) {
    let mut style = Style::default();
    let mut visuals = if palette.is_dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    visuals.override_text_color = Some(palette.text_primary);
    visuals.panel_fill = palette.panel_bg;
    visuals.window_fill = palette.canvas_bg;
    visuals.faint_bg_color = palette.sidebar_active;
    visuals.extreme_bg_color = palette.canvas_bg;

    // Non-interactive surfaces
    visuals.widgets.noninteractive.bg_fill = palette.card_bg;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(8);

    // Inactive buttons and controls
    visuals.widgets.inactive.bg_fill = palette.button_secondary_bg;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, palette.border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(14); // Pill buttons

    // Hovered controls
    visuals.widgets.hovered.bg_fill = palette.button_secondary_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, palette.border_active);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.text_primary);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(14);

    // Active / Clicked controls
    visuals.widgets.active.bg_fill = palette.accent_blue;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, palette.accent_blue);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(14);

    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 6.0);

    let active_theme = if palette.is_dark { Theme::Dark } else { Theme::Light };
    ctx.set_theme(active_theme);
    ctx.set_style_of(Theme::Light, Arc::new(style.clone()));
    ctx.set_style_of(Theme::Dark, Arc::new(style));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_palettes() {
        let light = ThemePalette::from_theme("light");
        assert!(!light.is_dark);
        assert_eq!(light.panel_bg, Color32::from_rgb(255, 255, 255));

        let obsidian = ThemePalette::from_theme("obsidian");
        assert!(obsidian.is_dark);
        assert_eq!(obsidian.canvas_bg, Color32::from_rgb(18, 18, 20));

        let slate = ThemePalette::from_theme("slate");
        assert!(slate.is_dark);
        assert_eq!(slate.canvas_bg, Color32::from_rgb(15, 23, 42));

        let midnight = ThemePalette::from_theme("midnight");
        assert!(midnight.is_dark);
        assert_eq!(midnight.canvas_bg, Color32::from_rgb(0, 0, 0));
    }
}
