//! v74 theme tokens and accessibility checks.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeTokens {
    pub surface: Rgba,
    pub surface_alt: Rgba,
    pub text: Rgba,
    pub accent: Rgba,
    pub high_contrast: bool,
}

impl ThemeTokens {
    pub fn dark() -> Self {
        Self {
            surface: Rgba::rgb(18, 22, 28),
            surface_alt: Rgba::rgb(34, 39, 48),
            text: Rgba::rgb(236, 240, 248),
            accent: Rgba::rgb(92, 199, 255),
            high_contrast: false,
        }
    }

    pub fn high_contrast() -> Self {
        Self {
            surface: Rgba::rgb(0, 0, 0),
            surface_alt: Rgba::rgb(20, 20, 20),
            text: Rgba::rgb(255, 255, 255),
            accent: Rgba::rgb(255, 215, 0),
            high_contrast: true,
        }
    }
}

fn channel_luma(c: u8) -> f64 {
    let n = f64::from(c) / 255.0;
    if n <= 0.03928 {
        n / 12.92
    } else {
        ((n + 0.055) / 1.055).powf(2.4)
    }
}

pub fn relative_luminance(c: Rgba) -> f64 {
    0.2126 * channel_luma(c.r) + 0.7152 * channel_luma(c.g) + 0.0722 * channel_luma(c.b)
}

pub fn contrast_ratio(a: Rgba, b: Rgba) -> f64 {
    let la = relative_luminance(a);
    let lb = relative_luminance(b);
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

pub fn passes_wcag_aa_text(fg: Rgba, bg: Rgba) -> bool {
    contrast_ratio(fg, bg) >= 4.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_contrast_theme_passes_text_contrast() {
        let t = ThemeTokens::high_contrast();
        assert!(passes_wcag_aa_text(t.text, t.surface));
    }

    #[test]
    fn dark_theme_has_stable_tokens() {
        let t = ThemeTokens::dark();
        assert_eq!(t.surface, Rgba::rgb(18, 22, 28));
        assert_eq!(t.accent, Rgba::rgb(92, 199, 255));
        assert!(contrast_ratio(t.text, t.surface) > 10.0);
    }
}
