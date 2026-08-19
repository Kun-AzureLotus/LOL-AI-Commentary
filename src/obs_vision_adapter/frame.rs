use std::time::{SystemTime, UNIX_EPOCH};

use super::ObsVisionError;

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub timestamp: SystemTime,
    pub rgba: Vec<u8>,
}

impl Frame {
    pub fn timestamp_millis(&self) -> u128 {
        self.timestamp
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default()
    }

    pub fn crop(&self, rect: RelativeRect) -> Result<Self, ObsVisionError> {
        let rect = rect.to_pixel_rect(self.width, self.height)?;
        let expected_len = (self.width * self.height * 4) as usize;
        if self.rgba.len() != expected_len {
            return Err(ObsVisionError::InvalidRoi {
                message: format!(
                    "frame RGBA buffer length {} did not match expected {}",
                    self.rgba.len(),
                    expected_len
                ),
            });
        }

        let mut rgba = Vec::with_capacity((rect.width * rect.height * 4) as usize);
        let row_bytes = (rect.width * 4) as usize;

        for row in 0..rect.height {
            let source_offset =
                (((rect.y + row) * self.width + rect.x) * 4) as usize;
            let source_row = &self.rgba[source_offset..source_offset + row_bytes];
            rgba.extend_from_slice(source_row);
        }

        Ok(Self {
            width: rect.width,
            height: rect.height,
            timestamp: self.timestamp,
            rgba,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelativeRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl RelativeRect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn to_pixel_rect(self, frame_width: u32, frame_height: u32) -> Result<PixelRect, ObsVisionError> {
        if frame_width == 0 || frame_height == 0 {
            return Err(ObsVisionError::InvalidRoi {
                message: "frame dimensions must be non-zero".to_string(),
            });
        }

        if self.x < 0.0
            || self.y < 0.0
            || self.width <= 0.0
            || self.height <= 0.0
            || self.x + self.width > 1.0
            || self.y + self.height > 1.0
        {
            return Err(ObsVisionError::InvalidRoi {
                message: format!("relative rectangle out of bounds: {self:?}"),
            });
        }

        let x = (self.x * frame_width as f32).round() as u32;
        let y = (self.y * frame_height as f32).round() as u32;
        let mut width = (self.width * frame_width as f32).round() as u32;
        let mut height = (self.height * frame_height as f32).round() as u32;

        width = width.max(1).min(frame_width - x);
        height = height.max(1).min(frame_height - y);

        Ok(PixelRect {
            x,
            y,
            width,
            height,
        })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Region {
    MainGame,
    Minimap,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoiConfig {
    pub main_game: RelativeRect,
    pub minimap: RelativeRect,
}

impl Default for RoiConfig {
    fn default() -> Self {
        Self {
            main_game: RelativeRect::new(0.0, 0.0, 1.0, 0.75),
            minimap: RelativeRect::new(0.84, 0.72, 0.16, 0.28),
        }
    }
}

impl RoiConfig {
    pub fn rect_for(&self, region: Region) -> RelativeRect {
        match region {
            Region::MainGame => self.main_game,
            Region::Minimap => self.minimap,
        }
    }

    pub fn crop(&self, frame: &Frame, region: Region) -> Result<Frame, ObsVisionError> {
        frame.crop(self.rect_for(region))
    }

    pub fn crop_all(&self, frame: &Frame) -> Result<RoiFrames, ObsVisionError> {
        Ok(RoiFrames {
            main_game: self.crop(frame, Region::MainGame)?,
            minimap: self.crop(frame, Region::Minimap)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoiFrames {
    pub main_game: Frame,
    pub minimap: Frame,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct PixelRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;

    #[test]
    fn crops_relative_region_from_rgba_frame() {
        let frame = test_frame_4x4();
        let cropped = frame
            .crop(RelativeRect::new(0.25, 0.25, 0.5, 0.5))
            .expect("crop should succeed");

        assert_eq!(cropped.width, 2);
        assert_eq!(cropped.height, 2);
        assert_eq!(cropped.rgba.len(), 2 * 2 * 4);
        assert_eq!(cropped.rgba[0], pixel_value(1, 1));
        assert_eq!(cropped.rgba[4], pixel_value(2, 1));
        assert_eq!(cropped.rgba[8], pixel_value(1, 2));
        assert_eq!(cropped.rgba[12], pixel_value(2, 2));
    }

    #[test]
    fn rejects_out_of_bounds_roi() {
        let frame = test_frame_4x4();
        let result = frame.crop(RelativeRect::new(0.9, 0.9, 0.2, 0.2));

        assert!(matches!(result, Err(ObsVisionError::InvalidRoi { .. })));
    }

    #[test]
    fn default_roi_config_crops_both_regions() {
        let frame = Frame {
            width: 2560,
            height: 1440,
            timestamp: SystemTime::UNIX_EPOCH,
            rgba: vec![0; 2560 * 1440 * 4],
        };

        let roi_frames = RoiConfig::default().crop_all(&frame).expect("crop all");

        assert!(roi_frames.main_game.width > 0);
        assert!(roi_frames.main_game.height > 0);
        assert!(roi_frames.minimap.width > 0);
        assert!(roi_frames.minimap.height > 0);
    }

    fn test_frame_4x4() -> Frame {
        let mut rgba = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                rgba.extend_from_slice(&[pixel_value(x, y), 0, 0, 255]);
            }
        }

        Frame {
            width: 4,
            height: 4,
            timestamp: SystemTime::UNIX_EPOCH,
            rgba,
        }
    }

    fn pixel_value(x: u32, y: u32) -> u8 {
        (y * 4 + x) as u8
    }
}
