#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub bg: (u8, u8, u8),
    pub sidebar: (u8, u8, u8),
    pub caption: (u8, u8, u8),
    pub surface: (u8, u8, u8),
    pub elevated: (u8, u8, u8),
    pub hero: (u8, u8, u8),
    pub paper: (u8, u8, u8),
    pub text: (u8, u8, u8),
    pub text_muted: (u8, u8, u8),
    pub green: (u8, u8, u8),
    pub green_deep: (u8, u8, u8),
    pub nav_active: (u8, u8, u8),
    pub nav_text: (u8, u8, u8),
    pub nav_idle: (u8, u8, u8),
    pub indicator: (u8, u8, u8),
    pub border: (u8, u8, u8),
    pub vermilion: (u8, u8, u8),
    pub vermilion_hover: (u8, u8, u8),
    pub vermilion_deep: (u8, u8, u8),
    pub gold: (u8, u8, u8),
    pub ink_line: (u8, u8, u8),
    pub prompt_bg: (u8, u8, u8),
    pub close_hover: (u8, u8, u8),
    pub bamboo: (u8, u8, u8),
    pub sun: (u8, u8, u8),
    pub bamboo_opacity: u8,
    pub sun_opacity: u8,
    pub paper_ornaments: bool,
    pub ink_ornaments: bool,
}

impl Palette {
    pub fn dark() -> Self {
        Self {
            bg: (0x1E, 0x1F, 0x1C),
            sidebar: (0x20, 0x21, 0x1F),
            caption: (0x1B, 0x1C, 0x1A),
            surface: (0x2E, 0x30, 0x2C),
            elevated: (0x35, 0x37, 0x33),
            hero: (0x28, 0x2A, 0x26),
            paper: (0xE5, 0xDC, 0xC9),
            text: (0xF1, 0xE9, 0xDA),
            text_muted: (0xAA, 0xA3, 0x96),
            green: (0x77, 0x8A, 0x7E),
            green_deep: (0x59, 0x6C, 0x62),
            nav_active: (0x30, 0x36, 0x32),
            nav_text: (0xE5, 0xDC, 0xC9),
            nav_idle: (0x99, 0x96, 0x8B),
            indicator: (0x77, 0x8A, 0x7E),
            border: (0x45, 0x47, 0x42),
            vermilion: (0xB9, 0x4A, 0x35),
            vermilion_hover: (0xC7, 0x5A, 0x43),
            vermilion_deep: (0x8F, 0x39, 0x2D),
            gold: (0xBD, 0xAE, 0x91),
            ink_line: (0x22, 0x23, 0x20),
            prompt_bg: (0x20, 0x21, 0x1F),
            close_hover: (0x4A, 0x2C, 0x28),
            bamboo: (0x30, 0x37, 0x32),
            sun: (0xB8, 0x49, 0x3A),
            bamboo_opacity: 10,
            sun_opacity: 42,
            paper_ornaments: false,
            ink_ornaments: true,
        }
    }

    pub fn light() -> Self {
        Self {
            bg: (0xF3, 0xEF, 0xE6),
            sidebar: (0xE6, 0xE0, 0xD2),
            caption: (0xF3, 0xEF, 0xE6),
            surface: (0xF0, 0xE9, 0xDA),
            elevated: (0xF5, 0xEF, 0xE3),
            hero: (0xED, 0xE6, 0xD6),
            paper: (0xF5, 0xEF, 0xE3),
            text: (0x2C, 0x2A, 0x25),
            text_muted: (0x6F, 0x6A, 0x5F),
            green: (0x75, 0x8A, 0x7D),
            green_deep: (0x53, 0x66, 0x5C),
            nav_active: (0xC8, 0xD6, 0xCC),
            nav_text: (0x2C, 0x2A, 0x25),
            nav_idle: (0x6F, 0x6A, 0x5F),
            indicator: (0x75, 0x8A, 0x7D),
            border: (0xC8, 0xBF, 0xAE),
            vermilion: (0xB8, 0x4C, 0x3B),
            vermilion_hover: (0xC7, 0x5A, 0x43),
            vermilion_deep: (0x8F, 0x39, 0x2D),
            gold: (0xA8, 0x93, 0x70),
            ink_line: (0xD4, 0xCB, 0xB8),
            prompt_bg: (0xF5, 0xEF, 0xE3),
            close_hover: (0xE8, 0xC4, 0xBC),
            bamboo: (0x75, 0x8A, 0x7D),
            sun: (0xB8, 0x49, 0x3A),
            bamboo_opacity: 18,
            sun_opacity: 58,
            paper_ornaments: true,
            ink_ornaments: false,
        }
    }
}

#[allow(dead_code)]
pub fn mix_rgb(fg: (u8, u8, u8), bg: (u8, u8, u8), opacity_percent: u8) -> (u8, u8, u8) {
    let opacity = u16::from(opacity_percent.min(100));
    let mix = |fg: u8, bg: u8| {
        let fg = u16::from(fg);
        let bg = u16::from(bg);
        ((fg * opacity + bg * (100 - opacity)) / 100) as u8
    };
    (mix(fg.0, bg.0), mix(fg.1, bg.1), mix(fg.2, bg.2))
}

pub const FONT_SCALES: [i32; 4] = [90, 100, 110, 120];
pub const FONT_SCALE_DEFAULT: i32 = 100;

#[derive(Clone, Copy)]
pub struct Metrics {
    pub scale: i32,
    pub title: i32,
    pub subtitle: i32,
    pub section: i32,
    pub label: i32,
    pub input: i32,
    pub button: i32,
    pub status: i32,
    pub brand: i32,
    pub small: i32,
    pub input_h: i32,
    pub button_h: i32,
    pub start_h: i32,
    pub start_w: i32,
    pub pad: i32,
    pub label_gap: i32,
    pub field_gap: i32,
    #[allow(dead_code)]
    pub control_radius: i32,
    pub prompt_edit_h: i32,
    pub sidebar_w: i32,
    pub settings_nav_w: i32,
    pub nav_item_h: i32,
    pub title_bar_h: i32,
}

impl Metrics {
    pub fn new(scale: i32) -> Self {
        let scale = snap_font_scale(scale);
        let s = |value: i32| (value * scale) / 100;
        Self {
            scale,
            title: s(24).max(20),
            subtitle: s(14).max(12),
            section: s(16).max(14),
            label: s(13).max(12),
            input: s(15).max(14),
            button: s(15).max(14),
            status: s(15).max(14),
            brand: s(18).max(16),
            small: s(12).max(11),
            input_h: s(42).max(40),
            button_h: s(42).max(38),
            start_h: s(48).max(44),
            start_w: s(228).max(200),
            pad: s(24).max(20),
            label_gap: s(8).max(6),
            field_gap: s(16).max(12),
            control_radius: 5,
            prompt_edit_h: s(240).max(220),
            sidebar_w: s(224).max(210),
            settings_nav_w: s(176).max(160),
            nav_item_h: s(40).max(36),
            title_bar_h: s(42).max(40),
        }
    }

    pub fn window_size(self) -> (i32, i32) {
        let width = (900 + (self.scale - 100) * 4).clamp(820, 1200);
        let height = (760 + (self.scale - 100) * 3).clamp(680, 900);
        (width, height)
    }

    pub fn heading_h(self) -> i32 {
        self.section + 4
    }

    pub fn caption_btn_w(self) -> i32 {
        46
    }

    pub fn page_header_h(self) -> i32 {
        self.title_bar_h + self.pad + self.title + 8 + self.subtitle + 16
    }

    pub fn resize_border(self) -> i32 {
        8
    }
}

pub fn to_colorref(rgb: (u8, u8, u8)) -> u32 {
    u32::from(rgb.0) | (u32::from(rgb.1) << 8) | (u32::from(rgb.2) << 16)
}

pub fn snap_font_scale(scale: i32) -> i32 {
    FONT_SCALES
        .iter()
        .copied()
        .min_by_key(|preset| (preset - scale).abs())
        .unwrap_or(FONT_SCALE_DEFAULT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_theme_has_bamboo_and_sun_tokens() {
        let paper = Palette::light();
        assert!(paper.paper_ornaments);
        assert!(!paper.ink_ornaments);
        assert_eq!(paper.bg, (0xF3, 0xEF, 0xE6));
        assert_eq!(paper.bamboo, (0x75, 0x8A, 0x7D));
        assert_eq!(paper.sun, (0xB8, 0x49, 0x3A));
        assert_eq!(paper.bamboo_opacity, 18);
        assert_eq!(paper.sun_opacity, 58);
        let dark = Palette::dark();
        assert!(!dark.paper_ornaments);
        assert!(dark.ink_ornaments);
        assert_eq!(dark.bg, (0x1E, 0x1F, 0x1C));
        assert_eq!(dark.bamboo, (0x30, 0x37, 0x32));
        assert_eq!(dark.sun, (0xB8, 0x49, 0x3A));
        assert_eq!(dark.bamboo_opacity, 10);
        assert_eq!(dark.sun_opacity, 42);
    }

    #[test]
    fn mix_rgb_keeps_background_when_opacity_is_zero() {
        let fg = (0x75, 0x8A, 0x7D);
        let bg = (0xE8, 0xE0, 0xCF);
        assert_eq!(mix_rgb(fg, bg, 0), bg);
        assert_eq!(mix_rgb(fg, bg, 100), fg);
    }
}
