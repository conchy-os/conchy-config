//! Внешний вид для xdg-desktop-portal (`org.freedesktop.appearance`): тёмная/светлая тема,
//! акцентный цвет, контраст — из `theme.kdl` шелла и необязательного `appearance.kdl`.
//!
//! Модуль не зависит от gpui: его использует и демон портала. Формулу «тёмная или светлая»
//! нужно держать в синхроне с `Base16Palette::appearance()` в
//! `conchy-shell/crates/ui/src/theme/base16.rs` (светлота HSL у `base00` меньше 0.5 → тёмная).
//!
//! Явные оверрайды живут в отдельном `appearance.kdl`, а не в `theme.kdl`: шелл переписывает
//! `theme.kdl` целиком при каждом сохранении темы и стёр бы неизвестные ему блоки.
//!
//! ```kdl
//! appearance {
//!     color-scheme "dark"      // dark | light | none
//!     accent "#3584e4"         // любой CSS-цвет из тем (hex, rgb(), hsl(), oklch())
//!     contrast "high"          // normal | high
//!     reduced-motion true
//! }
//! ```

use kdl::{KdlDocument, KdlValue};

/// Значение `org.freedesktop.appearance` `color-scheme`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    NoPreference,
    Dark,
    Light,
}

impl ColorScheme {
    /// Код для xdg-desktop-portal: 0 нет предпочтения, 1 тёмная, 2 светлая.
    pub fn portal_value(self) -> u32 {
        match self {
            Self::NoPreference => 0,
            Self::Dark => 1,
            Self::Light => 2,
        }
    }

    /// Значение gsettings `org.gnome.desktop.interface color-scheme` для старых GTK/Flatpak.
    pub fn gnome_value(self) -> &'static str {
        match self {
            Self::NoPreference => "default",
            Self::Dark => "prefer-dark",
            Self::Light => "prefer-light",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            "none" | "default" | "no-preference" => Some(Self::NoPreference),
            _ => None,
        }
    }
}

/// Итог разбора.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Appearance {
    pub color_scheme: ColorScheme,
    /// Акцент в sRGB, каждый канал `0.0..=1.0`.
    pub accent: [f64; 3],
    /// `0` — обычный, `1` — высокий.
    pub contrast: u32,
    /// `0` — нет предпочтения, `1` — уменьшить анимацию.
    pub reduced_motion: u32,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            color_scheme: ColorScheme::NoPreference,
            // GNOME blue, как у xdg-desktop-portal-gnome по умолчанию.
            accent: [0.208, 0.518, 0.894],
            contrast: 0,
            reduced_motion: 0,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppearanceError {
    #[error("не удалось разобрать KDL: {0}")]
    Kdl(String),
}

/// Разбирает `theme.kdl` (+ необязательный `appearance.kdl`). Битый файл — `Err`: вызывающий
/// оставляет прежнее значение, а не откатывается к умолчанию.
pub fn appearance_from_theme_kdl(
    theme: &str,
    overrides: Option<&str>,
) -> Result<Appearance, AppearanceError> {
    let theme_doc: KdlDocument = theme
        .parse()
        .map_err(|e: kdl::KdlError| AppearanceError::Kdl(e.to_string()))?;
    let over_doc: Option<KdlDocument> = match overrides {
        Some(t) => Some(
            t.parse()
                .map_err(|e: kdl::KdlError| AppearanceError::Kdl(e.to_string()))?,
        ),
        None => None,
    };

    let mut out = Appearance::default();

    // Тёмная/светлая: светлота base16.base00 (явный оверрайд ниже). Ключ `variant` в theme.kdl
    // не поддерживается: `StoredTheme` шелла — `deny_unknown_fields`, шелл не прочитал бы файл.
    let base16 = theme_doc.get("base16").and_then(|n| n.children());
    if let Some(l) = base16
        .and_then(|b| str_arg(b, "base00"))
        .and_then(|s| parse_color(&s))
        .map(hsl_lightness)
    {
        out.color_scheme = if l < 0.5 {
            ColorScheme::Dark
        } else {
            ColorScheme::Light
        };
    }

    // Акцент: colors.accent → base16.base0d.
    let accent = theme_doc
        .get("colors")
        .and_then(|n| n.children())
        .and_then(|c| str_arg(c, "accent"))
        .and_then(|s| parse_color(&s))
        .or_else(|| {
            base16
                .and_then(|b| str_arg(b, "base0d"))
                .and_then(|s| parse_color(&s))
        });
    if let Some(c) = accent {
        out.accent = [c[0], c[1], c[2]];
    }

    if let Some(block) = over_doc
        .as_ref()
        .and_then(|d| d.get("appearance"))
        .and_then(|n| n.children())
    {
        if let Some(s) = str_arg(block, "color-scheme").and_then(|s| ColorScheme::parse(&s)) {
            out.color_scheme = s;
        }
        if let Some(c) = str_arg(block, "accent").and_then(|s| parse_color(&s)) {
            out.accent = [c[0], c[1], c[2]];
        }
        match str_arg(block, "contrast").as_deref() {
            Some("high") => out.contrast = 1,
            Some("normal") => out.contrast = 0,
            _ => {}
        }
        if let Some(KdlValue::Bool(b)) = block.get_arg("reduced-motion") {
            out.reduced_motion = u32::from(*b);
        }
    }
    Ok(out)
}

fn str_arg(doc: &KdlDocument, key: &str) -> Option<String> {
    match doc.get_arg(key)? {
        KdlValue::String(s) | KdlValue::RawString(s) => Some(s.clone()),
        _ => None,
    }
}

/// HSL-светлота цвета: `(max + min) / 2`.
fn hsl_lightness(c: [f64; 4]) -> f64 {
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    (max + min) / 2.0
}

/// CSS-цвет в формате тем (hex, `rgb()`, `hsl()`, `oklch()`, «голый» hex) → sRGBA `0..=1`.
pub fn parse_color(input: &str) -> Option<[f64; 4]> {
    let s = input.trim();
    if let Some(h) = s.strip_prefix('#') {
        return parse_hex(h);
    }
    for name in ["rgba", "rgb"] {
        if let Some(inner) = func_args(s, name) {
            return parse_rgb(&inner);
        }
    }
    for name in ["hsla", "hsl"] {
        if let Some(inner) = func_args(s, name) {
            return parse_hsl(&inner);
        }
    }
    if let Some(inner) = func_args(s, "oklch") {
        return parse_oklch(&inner);
    }
    parse_hex(s)
}

fn parse_hex(h: &str) -> Option<[f64; 4]> {
    if !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let digits: Vec<u8> = h
        .bytes()
        .map(|b| (b as char).to_digit(16).unwrap() as u8)
        .collect();
    let (r, g, b, a) = match digits.len() {
        3 | 4 => {
            let d = |i: usize| digits[i] * 17;
            (d(0), d(1), d(2), if digits.len() == 4 { d(3) } else { 255 })
        }
        6 | 8 => {
            let d = |i: usize| digits[i] * 16 + digits[i + 1];
            (d(0), d(2), d(4), if digits.len() == 8 { d(6) } else { 255 })
        }
        _ => return None,
    };
    Some([r, g, b, a].map(|v| f64::from(v) / 255.0))
}

/// Содержимое скобок `name( … )` как список токенов (запятые, пробелы и `/` — разделители).
fn func_args(s: &str, name: &str) -> Option<Vec<String>> {
    let rest = s.strip_prefix(name)?.trim_start().strip_prefix('(')?;
    let inner = rest.strip_suffix(')')?;
    Some(
        inner
            .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
            .filter(|t| !t.is_empty())
            .map(str::to_owned)
            .collect(),
    )
}

fn num(tok: &str) -> Option<f64> {
    tok.trim_end_matches("deg").parse().ok()
}

/// Число или проценты → `0..=1` относительно `full`.
fn scaled(tok: &str, full: f64) -> Option<f64> {
    match tok.strip_suffix('%') {
        Some(p) => Some(p.parse::<f64>().ok()? / 100.0),
        None => Some(tok.parse::<f64>().ok()? / full),
    }
}

fn parse_rgb(t: &[String]) -> Option<[f64; 4]> {
    if t.len() < 3 {
        return None;
    }
    let a = t.get(3).map_or(Some(1.0), |a| scaled(a, 1.0))?;
    Some([
        scaled(&t[0], 255.0)?.clamp(0.0, 1.0),
        scaled(&t[1], 255.0)?.clamp(0.0, 1.0),
        scaled(&t[2], 255.0)?.clamp(0.0, 1.0),
        a.clamp(0.0, 1.0),
    ])
}

fn parse_hsl(t: &[String]) -> Option<[f64; 4]> {
    if t.len() < 3 {
        return None;
    }
    let h = num(&t[0])?.rem_euclid(360.0);
    let s = scaled(&t[1], 100.0)?.clamp(0.0, 1.0);
    let l = scaled(&t[2], 100.0)?.clamp(0.0, 1.0);
    let a = t.get(3).map_or(Some(1.0), |a| scaled(a, 1.0))?;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Some([r + m, g + m, b + m, a.clamp(0.0, 1.0)])
}

fn parse_oklch(t: &[String]) -> Option<[f64; 4]> {
    if t.len() < 3 {
        return None;
    }
    let l = scaled(&t[0], 1.0)?;
    let c = num(&t[1])?;
    let h = num(&t[2])?.to_radians();
    let a = t.get(3).map_or(Some(1.0), |a| scaled(a, 1.0))?;
    let (la, lb) = (c * h.cos(), c * h.sin());
    let l_ = (l + 0.396_337_777_4 * la + 0.215_803_757_3 * lb).powi(3);
    let m_ = (l - 0.105_561_345_8 * la - 0.063_854_172_8 * lb).powi(3);
    let s_ = (l - 0.089_484_177_5 * la - 1.291_485_548_0 * lb).powi(3);
    let lin = [
        4.076_741_662_1 * l_ - 3.307_711_591_3 * m_ + 0.230_969_929_2 * s_,
        -1.268_438_004_6 * l_ + 2.609_757_401_1 * m_ - 0.341_319_396_5 * s_,
        -0.004_196_086_3 * l_ - 0.703_418_614_7 * m_ + 1.707_614_701_0 * s_,
    ];
    let enc = |v: f64| {
        let v = v.clamp(0.0, 1.0);
        if v <= 0.003_130_8 {
            12.92 * v
        } else {
            1.055 * v.powf(1.0 / 2.4) - 0.055
        }
    };
    Some([enc(lin[0]), enc(lin[1]), enc(lin[2]), a.clamp(0.0, 1.0)])
}

#[cfg(test)]
mod tests {
    use super::*;

    const DARK: &str = r##"
name "Nord"
base16 {
    base00 "#2e3440ff"
    base0d "#81a1c1ff"
}
"##;
    const LIGHT: &str = r##"
name "Paper"
base16 {
    base00 "#f5f5f5ff"
    base0d "#1e66f5ff"
}
"##;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn dark_base16() {
        let a = appearance_from_theme_kdl(DARK, None).unwrap();
        assert_eq!(a.color_scheme, ColorScheme::Dark);
        assert_eq!(a.color_scheme.portal_value(), 1);
        assert_eq!(a.color_scheme.gnome_value(), "prefer-dark");
        assert!(close(a.accent[0], 0x81 as f64 / 255.0));
        assert!(close(a.accent[2], 0xc1 as f64 / 255.0));
    }

    #[test]
    fn light_base16() {
        let a = appearance_from_theme_kdl(LIGHT, None).unwrap();
        assert_eq!(a.color_scheme, ColorScheme::Light);
        assert_eq!(a.color_scheme.portal_value(), 2);
    }

    #[test]
    fn colors_accent_wins_over_base0d() {
        let t = format!("{DARK}\ncolors {{\n accent \"#ff0000\"\n}}\n");
        let a = appearance_from_theme_kdl(&t, None).unwrap();
        assert_eq!(a.accent, [1.0, 0.0, 0.0]);
    }

    #[test]
    fn explicit_override_wins() {
        let o = "appearance {\n color-scheme \"light\"\n accent \"#00ff00\"\n contrast \"high\"\n reduced-motion true\n}\n";
        let a = appearance_from_theme_kdl(DARK, Some(o)).unwrap();
        assert_eq!(a.color_scheme, ColorScheme::Light);
        assert_eq!(a.accent, [0.0, 1.0, 0.0]);
        assert_eq!(a.contrast, 1);
        assert_eq!(a.reduced_motion, 1);
    }

    #[test]
    fn broken_file_is_error_not_default() {
        assert!(appearance_from_theme_kdl("base16 {{{ ", None).is_err());
        assert!(appearance_from_theme_kdl(DARK, Some("appearance {")).is_err());
    }

    #[test]
    fn empty_theme_is_no_preference() {
        let a = appearance_from_theme_kdl("", None).unwrap();
        assert_eq!(a, Appearance::default());
    }

    #[test]
    fn color_forms() {
        assert_eq!(parse_color("#fff"), Some([1.0, 1.0, 1.0, 1.0]));
        assert_eq!(parse_color("ff0000"), Some([1.0, 0.0, 0.0, 1.0]));
        let c = parse_color("rgb(255 0 0 / 50%)").unwrap();
        assert!(close(c[3], 0.5));
        let c = parse_color("hsl(120 100% 50%)").unwrap();
        assert!(close(c[1], 1.0) && close(c[0], 0.0));
        // oklch(0.628 0.258 29.23) ≈ sRGB красный.
        let c = parse_color("oklch(0.628 0.2577 29.23)").unwrap();
        assert!(close(c[0], 1.0) && c[1] < 0.05);
        assert_eq!(parse_color("nonsense"), None);
    }
}
