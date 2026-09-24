use ditto_core::config::Config;
use gpui_kit::gpui::{rgba, Rgba};

#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub bg_base: Rgba,
    pub bg_surface: Rgba,
    pub border_card: Rgba,
    pub text_primary: Rgba,
    pub text_secondary: Rgba,
    pub accent_primary: Rgba,
    pub accent_hover: Rgba,
}

pub fn parse_hex_color(hex: &str, alpha: f32) -> Rgba {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            let val = ((r as u32) << 24)
                | ((g as u32) << 16)
                | ((b as u32) << 8)
                | ((alpha * 255.0) as u32);
            return rgba(val);
        }
    } else if hex.len() == 8 {
        if let (Ok(r), Ok(g), Ok(b), Ok(a)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
            u8::from_str_radix(&hex[6..8], 16),
        ) {
            let val = ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32);
            return rgba(val);
        }
    }
    rgba(0x0c0c0eff)
}

impl ThemeColors {
    pub fn from_config(config: &Config) -> Self {
        let opacity_val = (config.opacity.clamp(20, 100) as f32) / 100.0;

        match config.theme.as_str() {
            "rose" => Self {
                bg_base: parse_hex_color("#0c0c0e", 1.0),
                bg_surface: parse_hex_color("#141419", opacity_val),
                border_card: rgba(0xffffff14), // rgba(255, 255, 255, 0.08)
                text_primary: parse_hex_color("#f3f4f6", 1.0),
                text_secondary: parse_hex_color("#9ca3af", 1.0),
                accent_primary: parse_hex_color("#f43f5e", 1.0),
                accent_hover: parse_hex_color("#e11d48", 1.0),
            },
            "cyan" => Self {
                bg_base: parse_hex_color("#0c0c0e", 1.0),
                bg_surface: parse_hex_color("#141419", opacity_val),
                border_card: rgba(0xffffff14),
                text_primary: parse_hex_color("#f3f4f6", 1.0),
                text_secondary: parse_hex_color("#9ca3af", 1.0),
                accent_primary: parse_hex_color("#06b6d4", 1.0),
                accent_hover: parse_hex_color("#0891b2", 1.0),
            },
            "emerald" => Self {
                bg_base: parse_hex_color("#0c0c0e", 1.0),
                bg_surface: parse_hex_color("#141419", opacity_val),
                border_card: rgba(0xffffff14),
                text_primary: parse_hex_color("#f3f4f6", 1.0),
                text_secondary: parse_hex_color("#9ca3af", 1.0),
                accent_primary: parse_hex_color("#10b981", 1.0),
                accent_hover: parse_hex_color("#059669", 1.0),
            },
            "amber" => Self {
                bg_base: parse_hex_color("#0c0c0e", 1.0),
                bg_surface: parse_hex_color("#141419", opacity_val),
                border_card: rgba(0xffffff14),
                text_primary: parse_hex_color("#f3f4f6", 1.0),
                text_secondary: parse_hex_color("#9ca3af", 1.0),
                accent_primary: parse_hex_color("#f59e0b", 1.0),
                accent_hover: parse_hex_color("#d97706", 1.0),
            },
            "light-pure" => Self {
                bg_base: parse_hex_color("#f9fafb", 1.0),
                bg_surface: parse_hex_color("#ffffff", 0.85),
                border_card: rgba(0x00000014),
                text_primary: parse_hex_color("#111827", 1.0),
                text_secondary: parse_hex_color("#4b5563", 1.0),
                accent_primary: parse_hex_color("#eab308", 1.0),
                accent_hover: parse_hex_color("#ca8a04", 1.0),
            },
            "light-nordic" => Self {
                bg_base: parse_hex_color("#f3f4f6", 1.0),
                bg_surface: parse_hex_color("#f3f4f6", 0.85),
                border_card: rgba(0x0000000f),
                text_primary: parse_hex_color("#1f2937", 1.0),
                text_secondary: parse_hex_color("#4b5563", 1.0),
                accent_primary: parse_hex_color("#3b82f6", 1.0),
                accent_hover: parse_hex_color("#2563eb", 1.0),
            },
            "custom" => Self {
                bg_base: parse_hex_color("#0c0c0e", 1.0),
                bg_surface: parse_hex_color("#141419", opacity_val),
                border_card: rgba(0xffffff14),
                text_primary: parse_hex_color(&config.custom_primary, 1.0),
                text_secondary: parse_hex_color(&config.custom_secondary, 1.0),
                accent_primary: parse_hex_color(&config.custom_accent, 1.0),
                accent_hover: parse_hex_color(&config.custom_accent, 1.0),
            },
            _ /* dark (default) */ => Self {
                bg_base: parse_hex_color("#0c0c0e", 1.0),
                bg_surface: parse_hex_color("#141419", opacity_val),
                border_card: rgba(0xffffff14),
                text_primary: parse_hex_color("#f3f4f6", 1.0),
                text_secondary: parse_hex_color("#9ca3af", 1.0),
                accent_primary: parse_hex_color("#eab308", 1.0),
                accent_hover: parse_hex_color("#ca8a04", 1.0),
            },
        }
    }
}
