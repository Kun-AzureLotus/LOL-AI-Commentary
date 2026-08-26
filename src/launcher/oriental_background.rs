use std::sync::OnceLock;

use image::{imageops::FilterType, RgbImage};

use crate::launcher::theme::Palette;

const DARK_PNG: &[u8] = include_bytes!("embedded/dark_landscape.png");
const PAPER_PNG: &[u8] = include_bytes!("embedded/paper_landscape.png");

/// Blend the painting toward the theme background so UI text stays readable.
const WASH_TOWARD_BG: u8 = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverCrop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Cover/crop the source painting into `dst`, keeping the right (bamboo/sun)
/// and a little extra sky rather than stretching.
pub fn cover_crop_src_rect(src_w: u32, src_h: u32, dst_w: i32, dst_h: i32) -> CoverCrop {
    let dst_w = dst_w.max(1) as f32;
    let dst_h = dst_h.max(1) as f32;
    let src_w_f = src_w.max(1) as f32;
    let src_h_f = src_h.max(1) as f32;
    let scale = (dst_w / src_w_f).max(dst_h / src_h_f).max(0.0001);
    let view_w = (dst_w / scale).min(src_w_f).max(1.0);
    let view_h = (dst_h / scale).min(src_h_f).max(1.0);
    let x = (src_w_f - view_w).max(0.0);
    let y = ((src_h_f - view_h) * 0.28).max(0.0);
    let mut crop = CoverCrop {
        x: x.round() as u32,
        y: y.round() as u32,
        width: view_w.round() as u32,
        height: view_h.round() as u32,
    };
    crop.width = crop.width.max(1).min(src_w);
    crop.height = crop.height.max(1).min(src_h);
    if crop.x + crop.width > src_w {
        crop.x = src_w - crop.width;
    }
    if crop.y + crop.height > src_h {
        crop.y = src_h - crop.height;
    }
    crop
}

fn decode_png(bytes: &[u8]) -> Option<RgbImage> {
    image::load_from_memory(bytes).ok().map(|image| image.to_rgb8())
}

fn source_image(ink: bool) -> Option<&'static RgbImage> {
    static DARK: OnceLock<Option<RgbImage>> = OnceLock::new();
    static PAPER: OnceLock<Option<RgbImage>> = OnceLock::new();
    if ink {
        DARK.get_or_init(|| decode_png(DARK_PNG)).as_ref()
    } else {
        PAPER.get_or_init(|| decode_png(PAPER_PNG)).as_ref()
    }
}

fn mix_u8(src: u8, bg: u8, toward_bg: u8) -> u8 {
    let toward = u16::from(toward_bg.min(100));
    ((u16::from(src) * (100 - toward) + u16::from(bg) * toward) / 100) as u8
}

fn scale_to_bgra(src: &RgbImage, dst_w: i32, dst_h: i32, bg: (u8, u8, u8)) -> Option<Vec<u8>> {
    let dst_w = dst_w.max(1) as u32;
    let dst_h = dst_h.max(1) as u32;
    let crop = cover_crop_src_rect(src.width(), src.height(), dst_w as i32, dst_h as i32);
    let cropped = image::imageops::crop_imm(src, crop.x, crop.y, crop.width, crop.height).to_image();
    let scaled = image::imageops::resize(&cropped, dst_w, dst_h, FilterType::Triangle);
    let width = scaled.width() as usize;
    let height = scaled.height() as usize;
    let mut bgra = vec![0u8; width.checked_mul(height)?.checked_mul(4)?];
    for y in 0..height {
        let src_y = height - 1 - y;
        for x in 0..width {
            let pixel = scaled.get_pixel(x as u32, src_y as u32).0;
            let index = (y * width + x) * 4;
            bgra[index] = mix_u8(pixel[2], bg.2, WASH_TOWARD_BG);
            bgra[index + 1] = mix_u8(pixel[1], bg.1, WASH_TOWARD_BG);
            bgra[index + 2] = mix_u8(pixel[0], bg.0, WASH_TOWARD_BG);
        }
    }
    Some(bgra)
}

#[cfg(windows)]
pub unsafe fn paint_landscape(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    content: windows::Win32::Foundation::RECT,
    palette: Palette,
) {
    win::paint_landscape(hdc, content, palette);
}

#[cfg(windows)]
mod win {
    use super::{scale_to_bgra, source_image};
    use crate::launcher::theme::Palette;
    use std::sync::Mutex;
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{
        IntersectClipRect, RestoreDC, SaveDC, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER,
        DIB_RGB_COLORS, HDC, SRCCOPY, BI_RGB,
    };

    struct FrameCache {
        ink: bool,
        width: i32,
        height: i32,
        bgra: Vec<u8>,
    }

    static FRAME: Mutex<Option<FrameCache>> = Mutex::new(None);

    pub unsafe fn paint_landscape(hdc: HDC, content: RECT, palette: Palette) {
        let width = content.right - content.left;
        let height = content.bottom - content.top;
        if width < 260 || height < 200 {
            return;
        }
        let ink = palette.ink_ornaments;
        let Some(source) = source_image(ink) else {
            return;
        };
        let mut cache = match FRAME.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let needs_rebuild = cache.as_ref().map_or(true, |frame| {
            frame.ink != ink || frame.width != width || frame.height != height
        });
        if needs_rebuild {
            let Some(bgra) = scale_to_bgra(source, width, height, palette.bg) else {
                return;
            };
            *cache = Some(FrameCache {
                ink,
                width,
                height,
                bgra,
            });
        }
        let Some(frame) = cache.as_ref() else {
            return;
        };
        let clip = SaveDC(hdc);
        IntersectClipRect(
            hdc,
            content.left,
            content.top,
            content.right,
            content.bottom,
        );
        let mut info = BITMAPINFO::default();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let _ = StretchDIBits(
            hdc,
            content.left,
            content.top,
            width,
            height,
            0,
            0,
            width,
            height,
            Some(frame.bgra.as_ptr().cast()),
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
        let _ = RestoreDC(hdc, clip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launcher::theme::mix_rgb;

    #[test]
    fn embedded_pngs_are_present_and_decodable() {
        assert!(DARK_PNG.len() > 50_000);
        assert!(PAPER_PNG.len() > 50_000);
        let dark = decode_png(DARK_PNG).expect("dark landscape png");
        let paper = decode_png(PAPER_PNG).expect("paper landscape png");
        assert!(dark.width() >= 1280);
        assert!(dark.height() >= 720);
        assert!(paper.width() >= 1280);
        assert!(paper.height() >= 720);
        assert!(dark.width() > dark.height());
        assert!(paper.width() > paper.height());
    }

    #[test]
    fn cover_crop_keeps_the_right_side_on_tall_windows() {
        let crop = cover_crop_src_rect(1920, 1080, 700, 720);
        assert!(crop.width > 0 && crop.height > 0);
        assert!(crop.x + crop.width <= 1920);
        assert!(crop.y + crop.height <= 1080);
        assert!(crop.x > 1920 / 4);
    }

    #[test]
    fn cover_crop_scales_without_stretching_source_aspect() {
        let compact = cover_crop_src_rect(1920, 1080, 800, 600);
        let wide = cover_crop_src_rect(1920, 1080, 1600, 1200);
        let compact_aspect = compact.width as f32 / compact.height as f32;
        let wide_aspect = wide.width as f32 / wide.height as f32;
        assert!((compact_aspect - wide_aspect).abs() < 0.05);
    }

    #[test]
    fn wash_keeps_painting_visible_behind_ui() {
        let mixed = mix_u8(0xB8, 0x1E, WASH_TOWARD_BG);
        assert_ne!(mixed, 0x1E);
        assert_ne!(mixed, 0xB8);
        let dark = mix_rgb((0x30, 0x37, 0x32), (0x1E, 0x1F, 0x1C), WASH_TOWARD_BG);
        assert_ne!(dark, (0x1E, 0x1F, 0x1C));
    }
}

#[cfg(all(test, windows))]
mod paint_tests {
    use super::*;
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, SelectObject,
    };

    #[test]
    fn paint_embedded_landscape_does_not_panic() {
        unsafe {
            let hdc = CreateCompatibleDC(None);
            let bmp = CreateCompatibleBitmap(hdc, 800, 600);
            let old = SelectObject(hdc, bmp);
            paint_landscape(
                hdc,
                RECT {
                    left: 0,
                    top: 0,
                    right: 800,
                    bottom: 600,
                },
                crate::launcher::theme::Palette::dark(),
            );
            paint_landscape(
                hdc,
                RECT {
                    left: 0,
                    top: 0,
                    right: 800,
                    bottom: 600,
                },
                crate::launcher::theme::Palette::light(),
            );
            SelectObject(hdc, old);
            let _ = DeleteObject(bmp);
            let _ = DeleteDC(hdc);
        }
    }
}
