use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use image::{imageops::FilterType, RgbaImage};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::obs_vision_adapter::Frame;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisibleObjective {
    pub objective_type: ObjectiveType,
    pub x: f32,
    pub y: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum ObjectiveType {
    Turret,
    Dragon,
    Baron,
    Herald,
}

impl ObjectiveType {
    pub fn file_prefix(self) -> &'static str {
        match self {
            Self::Turret => "turret",
            Self::Dragon => "dragon",
            Self::Baron => "baron",
            Self::Herald => "herald",
        }
    }
}

impl FromStr for ObjectiveType {
    type Err = ObjectiveTemplateError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "turret" => Ok(Self::Turret),
            "dragon" => Ok(Self::Dragon),
            "baron" => Ok(Self::Baron),
            "herald" => Ok(Self::Herald),
            _ => Err(ObjectiveTemplateError::InvalidObjectiveType {
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub objective_type: ObjectiveType,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub source_path: Option<PathBuf>,
}

impl Template {
    pub fn from_frame(objective_type: ObjectiveType, frame: Frame) -> Self {
        Self {
            objective_type,
            width: frame.width,
            height: frame.height,
            rgba: frame.rgba,
            source_path: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TemplateStore {
    templates: Vec<Template>,
}

impl TemplateStore {
    pub fn load_from_dir(path: impl AsRef<Path>) -> Result<Self, ObjectiveTemplateError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }

        let mut templates = Vec::new();
        for entry in fs::read_dir(path).map_err(|source| ObjectiveTemplateError::Io { source })? {
            let entry = entry.map_err(|source| ObjectiveTemplateError::Io { source })?;
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|value| value.to_str()) != Some("png") {
                continue;
            }

            let Some(objective_type) = objective_type_from_path(&path)? else {
                continue;
            };

            let image = image::open(&path)
                .map_err(|source| ObjectiveTemplateError::Image { source })?
                .to_rgba8();
            templates.push(Template {
                objective_type,
                width: image.width(),
                height: image.height(),
                rgba: image.into_raw(),
                source_path: Some(path),
            });
        }

        Ok(Self { templates })
    }

    pub fn from_templates(templates: Vec<Template>) -> Self {
        Self { templates }
    }

    pub fn templates(&self) -> &[Template] {
        &self.templates
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectiveDetectorConfig {
    pub confidence_threshold: f32,
    pub scan_step: usize,
    pub dedup_distance_ratio: f32,
    pub nms_iou_threshold: f32,
    pub min_effective_pixel_ratio: f32,
    pub min_template_energy: f32,
    pub scale_factors: &'static [f32],
}

impl Default for ObjectiveDetectorConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.90,
            scan_step: 3,
            dedup_distance_ratio: 0.04,
            nms_iou_threshold: 0.3,
            min_effective_pixel_ratio: 0.25,
            min_template_energy: 0.001,
            scale_factors: &[0.9, 1.0, 1.1],
        }
    }
}

#[derive(Debug, Clone)]
pub struct MinimapObjectiveDetector {
    templates: TemplateStore,
    config: ObjectiveDetectorConfig,
}

impl MinimapObjectiveDetector {
    pub fn new(templates: TemplateStore, config: ObjectiveDetectorConfig) -> Self {
        Self { templates, config }
    }

    pub fn with_default_config(templates: TemplateStore) -> Self {
        Self::new(templates, ObjectiveDetectorConfig::default())
    }

    pub fn detect(&self, frame: &Frame) -> Vec<VisibleObjective> {
        if frame.width == 0 || frame.height == 0 || self.templates.is_empty() {
            return Vec::new();
        }

        let mut matches = Vec::new();
        let mut debug = MatchDebug::default();
        for template in self.templates.templates() {
            for scaled_template in scaled_templates(template, self.config.scale_factors) {
                if scaled_template.width == 0
                    || scaled_template.height == 0
                    || scaled_template.width > frame.width
                    || scaled_template.height > frame.height
                {
                    continue;
                }

                scan_template(frame, &scaled_template, &self.config, &mut matches, &mut debug);
            }
        }

        debug.matches_after_threshold = matches.len();
        let objects = nms_objectives(
            matches,
            self.config.dedup_distance_ratio,
            self.config.nms_iou_threshold,
        );
        debug.matches_after_nms = objects.len();
        print_match_debug(&debug);

        objects
    }
}

#[derive(Debug, Error)]
pub enum ObjectiveTemplateError {
    #[error("invalid objective type: {value}")]
    InvalidObjectiveType { value: String },

    #[error("failed to read template directory")]
    Io {
        #[source]
        source: std::io::Error,
    },

    #[error("failed to decode template image")]
    Image {
        #[source]
        source: image::ImageError,
    },
}

#[derive(Debug, Clone)]
struct RawObjectiveMatch {
    objective_type: ObjectiveType,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    confidence: f32,
    normalized_x: f32,
    normalized_y: f32,
}

#[derive(Debug, Default)]
struct MatchDebug {
    raw_matches: usize,
    matches_after_threshold: usize,
    matches_after_nms: usize,
    best_confidence: f32,
    best_position: Option<(u32, u32)>,
}

fn print_match_debug(debug: &MatchDebug) {
    println!("[ObjectiveTemplateDebug]");
    println!("best confidence: {:.4}", debug.best_confidence);
    match debug.best_position {
        Some((x, y)) => println!("best match position: ({x}, {y})"),
        None => println!("best match position: none"),
    }
    println!("number of raw matches: {}", debug.raw_matches);
    println!("number after threshold: {}", debug.matches_after_threshold);
    println!("number after NMS: {}", debug.matches_after_nms);
}

fn objective_type_from_path(path: &Path) -> Result<Option<ObjectiveType>, ObjectiveTemplateError> {
    let Some(file_stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return Ok(None);
    };

    let Some(prefix) = file_stem.split('_').next() else {
        return Ok(None);
    };

    ObjectiveType::from_str(prefix).map(Some)
}

fn scaled_templates(template: &Template, scale_factors: &[f32]) -> Vec<Template> {
    scale_factors
        .iter()
        .filter_map(|scale| {
            let width = ((template.width as f32) * scale).round().max(1.0) as u32;
            let height = ((template.height as f32) * scale).round().max(1.0) as u32;
            resize_template(template, width, height)
        })
        .collect()
}

fn resize_template(template: &Template, width: u32, height: u32) -> Option<Template> {
    if width == template.width && height == template.height {
        return Some(template.clone());
    }

    let image = RgbaImage::from_raw(template.width, template.height, template.rgba.clone())?;
    let resized = image::imageops::resize(&image, width, height, FilterType::Triangle);
    Some(Template {
        objective_type: template.objective_type,
        width,
        height,
        rgba: resized.into_raw(),
        source_path: template.source_path.clone(),
    })
}

fn scan_template(
    frame: &Frame,
    template: &Template,
    config: &ObjectiveDetectorConfig,
    matches: &mut Vec<RawObjectiveMatch>,
    debug: &mut MatchDebug,
) {
    let max_y = frame.height - template.height;
    let max_x = frame.width - template.width;

    for y in (0..=max_y).step_by(config.scan_step.max(1)) {
        for x in (0..=max_x).step_by(config.scan_step.max(1)) {
            debug.raw_matches += 1;
            let Some(confidence) = template_confidence(frame, template, x, y, config) else {
                continue;
            };
            if confidence > debug.best_confidence {
                debug.best_confidence = confidence;
                debug.best_position = Some((x, y));
            }

            if confidence < config.confidence_threshold {
                continue;
            }

            matches.push(RawObjectiveMatch {
                objective_type: template.objective_type,
                x,
                y,
                width: template.width,
                height: template.height,
                normalized_x: ((x as f32) + template.width as f32 / 2.0) / frame.width as f32,
                normalized_y: ((y as f32) + template.height as f32 / 2.0) / frame.height as f32,
                confidence,
            });
        }
    }
}

fn template_confidence(
    frame: &Frame,
    template: &Template,
    x: u32,
    y: u32,
    config: &ObjectiveDetectorConfig,
) -> Option<f32> {
    let mut valid_pixels = 0usize;
    let mut template_sum = [0.0; 3];
    let mut frame_sum = [0.0; 3];

    for ty in 0..template.height {
        for tx in 0..template.width {
            let template_index = ((ty * template.width + tx) * 4) as usize;
            let frame_index = (((y + ty) * frame.width + (x + tx)) * 4) as usize;
            let alpha = template.rgba[template_index + 3] as f32 / 255.0;
            if alpha < 0.2 {
                continue;
            }

            for channel in 0..3 {
                template_sum[channel] += template.rgba[template_index + channel] as f32 / 255.0;
                frame_sum[channel] += frame.rgba[frame_index + channel] as f32 / 255.0;
            }
            valid_pixels += 1;
        }
    }

    let total_pixels = (template.width * template.height) as usize;
    if valid_pixels == 0
        || valid_pixels as f32 / (total_pixels as f32) < config.min_effective_pixel_ratio
    {
        return None;
    }

    let mut template_mean = [0.0; 3];
    let mut frame_mean = [0.0; 3];
    for channel in 0..3 {
        template_mean[channel] = template_sum[channel] / valid_pixels as f32;
        frame_mean[channel] = frame_sum[channel] / valid_pixels as f32;
    }

    let mut numerator = 0.0;
    let mut template_energy = 0.0;
    let mut frame_energy = 0.0;

    for ty in 0..template.height {
        for tx in 0..template.width {
            let template_index = ((ty * template.width + tx) * 4) as usize;
            let frame_index = (((y + ty) * frame.width + (x + tx)) * 4) as usize;
            let alpha = template.rgba[template_index + 3] as f32 / 255.0;
            if alpha < 0.2 {
                continue;
            }

            for channel in 0..3 {
                let template_value =
                    template.rgba[template_index + channel] as f32 / 255.0 - template_mean[channel];
                let frame_value =
                    frame.rgba[frame_index + channel] as f32 / 255.0 - frame_mean[channel];
                numerator += template_value * frame_value * alpha;
                template_energy += template_value * template_value * alpha;
                frame_energy += frame_value * frame_value * alpha;
            }
        }
    }

    if template_energy < config.min_template_energy || frame_energy <= f32::EPSILON {
        return None;
    }

    let correlation = numerator / (template_energy.sqrt() * frame_energy.sqrt());
    Some(correlation.clamp(0.0, 1.0))
}

fn nms_objectives(
    mut objectives: Vec<RawObjectiveMatch>,
    dedup_distance_ratio: f32,
    nms_iou_threshold: f32,
) -> Vec<VisibleObjective> {
    objectives.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));

    let mut deduped = Vec::new();
    for objective in objectives {
        let duplicate = deduped.iter().any(|kept: &RawObjectiveMatch| {
            kept.objective_type == objective.objective_type
                && (normalized_distance(kept, &objective) < dedup_distance_ratio
                    || intersection_over_union(kept, &objective) >= nms_iou_threshold)
        });

        if !duplicate {
            deduped.push(objective);
        }
    }

    deduped
        .into_iter()
        .map(|objective| VisibleObjective {
            objective_type: objective.objective_type,
            x: objective.normalized_x,
            y: objective.normalized_y,
            confidence: objective.confidence,
        })
        .collect()
}

fn normalized_distance(a: &RawObjectiveMatch, b: &RawObjectiveMatch) -> f32 {
    let dx = a.normalized_x - b.normalized_x;
    let dy = a.normalized_y - b.normalized_y;
    (dx * dx + dy * dy).sqrt()
}

fn intersection_over_union(a: &RawObjectiveMatch, b: &RawObjectiveMatch) -> f32 {
    let a_right = a.x + a.width;
    let a_bottom = a.y + a.height;
    let b_right = b.x + b.width;
    let b_bottom = b.y + b.height;

    let intersection_left = a.x.max(b.x);
    let intersection_top = a.y.max(b.y);
    let intersection_right = a_right.min(b_right);
    let intersection_bottom = a_bottom.min(b_bottom);

    if intersection_right <= intersection_left || intersection_bottom <= intersection_top {
        return 0.0;
    }

    let intersection_area =
        (intersection_right - intersection_left) * (intersection_bottom - intersection_top);
    let a_area = a.width * a.height;
    let b_area = b.width * b.height;
    let union_area = a_area + b_area - intersection_area;

    if union_area == 0 {
        0.0
    } else {
        intersection_area as f32 / union_area as f32
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use image::ColorType;

    use super::*;

    #[test]
    fn detects_exact_template_match() {
        let mut frame = blank_frame(24, 24);
        paint_pattern(&mut frame, 10, 12);
        let template = crop_frame(&frame, 10, 12, 4, 4, ObjectiveType::Turret);
        let detector = MinimapObjectiveDetector::new(
            TemplateStore::from_templates(vec![template]),
            ObjectiveDetectorConfig {
                confidence_threshold: 0.95,
                scan_step: 1,
                dedup_distance_ratio: 0.05,
                scale_factors: &[1.0],
                ..ObjectiveDetectorConfig::default()
            },
        );

        let objectives = detector.detect(&frame);

        assert_eq!(objectives.len(), 1);
        assert_eq!(objectives[0].objective_type, ObjectiveType::Turret);
        assert!(objectives[0].confidence >= 0.95);
    }

    #[test]
    fn rejects_low_confidence_match() {
        let mut frame = blank_frame(24, 24);
        paint_square(&mut frame, 10, 12, [20, 20, 20, 255]);
        let mut template_frame = blank_frame(4, 4);
        paint_pattern(&mut template_frame, 0, 0);
        let detector = MinimapObjectiveDetector::new(
            TemplateStore::from_templates(vec![Template::from_frame(
                ObjectiveType::Turret,
                template_frame,
            )]),
            ObjectiveDetectorConfig {
                confidence_threshold: 0.95,
                scan_step: 1,
                dedup_distance_ratio: 0.05,
                scale_factors: &[1.0],
                ..ObjectiveDetectorConfig::default()
            },
        );

        let objectives = detector.detect(&frame);

        assert!(objectives.is_empty());
    }

    #[test]
    fn deduplicates_nearby_template_matches() {
        let mut frame = blank_frame(24, 24);
        paint_pattern(&mut frame, 10, 10);
        paint_pattern(&mut frame, 11, 10);
        let mut template = blank_frame(4, 4);
        paint_pattern(&mut template, 0, 0);
        let detector = MinimapObjectiveDetector::new(
            TemplateStore::from_templates(vec![Template::from_frame(
                ObjectiveType::Turret,
                template,
            )]),
            ObjectiveDetectorConfig {
                confidence_threshold: 0.95,
                scan_step: 1,
                dedup_distance_ratio: 0.2,
                scale_factors: &[1.0],
                ..ObjectiveDetectorConfig::default()
            },
        );

        let objectives = detector.detect(&frame);

        assert_eq!(objectives.len(), 1);
    }

    #[test]
    fn detects_dragon_baron_and_herald_templates() {
        let mut frame = blank_frame(48, 48);
        paint_pattern_with_palette(&mut frame, 8, 8, dragon_palette());
        paint_pattern_with_palette(&mut frame, 24, 10, baron_palette());
        paint_pattern_with_palette(&mut frame, 16, 30, herald_palette());
        let templates = vec![
            crop_frame(&frame, 8, 8, 4, 4, ObjectiveType::Dragon),
            crop_frame(&frame, 24, 10, 4, 4, ObjectiveType::Baron),
            crop_frame(&frame, 16, 30, 4, 4, ObjectiveType::Herald),
        ];
        let detector = MinimapObjectiveDetector::new(
            TemplateStore::from_templates(templates),
            ObjectiveDetectorConfig {
                confidence_threshold: 0.95,
                scan_step: 1,
                dedup_distance_ratio: 0.05,
                scale_factors: &[1.0],
                ..ObjectiveDetectorConfig::default()
            },
        );

        let objectives = detector.detect(&frame);

        assert_eq!(objectives.len(), 3);
        assert!(objectives
            .iter()
            .any(|objective| objective.objective_type == ObjectiveType::Dragon));
        assert!(objectives
            .iter()
            .any(|objective| objective.objective_type == ObjectiveType::Baron));
        assert!(objectives
            .iter()
            .any(|objective| objective.objective_type == ObjectiveType::Herald));
    }

    #[test]
    fn loads_objective_templates_from_filename_prefixes() {
        let template_dir = std::env::temp_dir().join(format!(
            "lol_ai_commentator_templates_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time after epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&template_dir).expect("create temp template dir");
        save_test_template(&template_dir.join("dragon_01.png"), [200, 80, 40, 255]);
        save_test_template(&template_dir.join("baron_01.png"), [140, 60, 220, 255]);
        save_test_template(&template_dir.join("herald_01.png"), [60, 180, 220, 255]);
        save_test_template(&template_dir.join("turret_01.png"), [220, 220, 80, 255]);

        let store = TemplateStore::load_from_dir(&template_dir).expect("load templates");
        let loaded_types = store
            .templates()
            .iter()
            .map(|template| template.objective_type)
            .collect::<Vec<_>>();

        assert!(loaded_types.contains(&ObjectiveType::Dragon));
        assert!(loaded_types.contains(&ObjectiveType::Baron));
        assert!(loaded_types.contains(&ObjectiveType::Herald));
        assert!(loaded_types.contains(&ObjectiveType::Turret));

        let _ = fs::remove_dir_all(&template_dir);
    }

    fn blank_frame(width: u32, height: u32) -> Frame {
        Frame {
            width,
            height,
            timestamp: SystemTime::UNIX_EPOCH,
            rgba: vec![0; (width * height * 4) as usize],
        }
    }

    fn paint_square(frame: &mut Frame, x: u32, y: u32, rgba: [u8; 4]) {
        for yy in y..y + 4 {
            for xx in x..x + 4 {
                set_pixel(frame, xx, yy, rgba);
            }
        }
    }

    fn paint_pattern(frame: &mut Frame, x: u32, y: u32) {
        paint_pattern_with_palette(
            frame,
            x,
            y,
            [
                [240, 220, 80, 255],
                [190, 150, 40, 255],
                [80, 70, 30, 255],
                [220, 200, 70, 255],
            ],
        );
    }

    fn paint_pattern_with_palette(frame: &mut Frame, x: u32, y: u32, pattern: [[u8; 4]; 4]) {
        for yy in 0..4 {
            for xx in 0..4 {
                let value = pattern[((xx + yy) % pattern.len() as u32) as usize];
                set_pixel(frame, x + xx, y + yy, value);
            }
        }
    }

    fn dragon_palette() -> [[u8; 4]; 4] {
        [
            [240, 120, 40, 255],
            [200, 70, 30, 255],
            [90, 40, 20, 255],
            [230, 160, 80, 255],
        ]
    }

    fn baron_palette() -> [[u8; 4]; 4] {
        [
            [180, 90, 240, 255],
            [110, 40, 180, 255],
            [45, 25, 90, 255],
            [210, 130, 255, 255],
        ]
    }

    fn herald_palette() -> [[u8; 4]; 4] {
        [
            [70, 210, 240, 255],
            [35, 130, 190, 255],
            [20, 60, 90, 255],
            [120, 230, 255, 255],
        ]
    }

    fn save_test_template(path: &std::path::Path, rgba: [u8; 4]) {
        let pixels = [
            [240, 220, 80, 255],
            rgba,
            [40, 40, 40, 255],
            rgba,
        ];
        let mut data = Vec::new();

        for pixel in pixels {
            data.extend_from_slice(&pixel);
        }

        image::save_buffer(path, &data, 2, 2, ColorType::Rgba8).expect("save test template");
    }

    fn set_pixel(frame: &mut Frame, x: u32, y: u32, rgba: [u8; 4]) {
        let index = ((y * frame.width + x) * 4) as usize;
        frame.rgba[index..index + 4].copy_from_slice(&rgba);
    }

    fn crop_frame(
        frame: &Frame,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        objective_type: ObjectiveType,
    ) -> Template {
        let mut rgba = Vec::new();
        for yy in y..y + height {
            for xx in x..x + width {
                let index = ((yy * frame.width + xx) * 4) as usize;
                rgba.extend_from_slice(&frame.rgba[index..index + 4]);
            }
        }

        Template {
            objective_type,
            width,
            height,
            rgba,
            source_path: None,
        }
    }
}
