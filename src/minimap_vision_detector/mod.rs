use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::obs_vision_adapter::Frame;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisibleObject {
    pub object_type: VisibleObjectType,
    pub x: f32,
    pub y: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisibleMarker {
    pub x: f32,
    pub y: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisibleActivityCluster {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub marker_count: u32,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum VisibleObjectType {
    FriendlyChampion,
    EnemyChampion,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisibleActivityClusterConfig {
    pub min_marker_confidence: f32,
    pub clustering_distance: f32,
    pub tight_cluster_radius: f32,
    pub min_cluster_markers: usize,
    pub min_tight_cluster_markers: usize,
}

impl Default for VisibleActivityClusterConfig {
    fn default() -> Self {
        Self {
            min_marker_confidence: 0.65,
            clustering_distance: 0.08,
            tight_cluster_radius: 0.055,
            min_cluster_markers: 3,
            min_tight_cluster_markers: 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VisibleActivityClusterer {
    config: VisibleActivityClusterConfig,
}

impl Default for VisibleActivityClusterer {
    fn default() -> Self {
        Self {
            config: VisibleActivityClusterConfig::default(),
        }
    }
}

impl VisibleActivityClusterer {
    pub fn new(config: VisibleActivityClusterConfig) -> Self {
        Self { config }
    }

    pub fn cluster(&self, markers: &[VisibleMarker]) -> Vec<VisibleActivityCluster> {
        let markers = markers
            .iter()
            .filter(|marker| marker.confidence >= self.config.min_marker_confidence)
            .cloned()
            .collect::<Vec<_>>();
        if markers.is_empty() {
            return Vec::new();
        }

        let mut visited = vec![false; markers.len()];
        let mut clusters = Vec::new();

        for index in 0..markers.len() {
            if visited[index] {
                continue;
            }

            let member_indices = self.collect_cluster(index, &markers, &mut visited);
            if let Some(cluster) = self.build_cluster(&markers, &member_indices) {
                clusters.push(cluster);
            }
        }

        clusters
    }

    fn collect_cluster(
        &self,
        start_index: usize,
        markers: &[VisibleMarker],
        visited: &mut [bool],
    ) -> Vec<usize> {
        let mut queue = VecDeque::new();
        let mut member_indices = Vec::new();

        queue.push_back(start_index);
        visited[start_index] = true;

        while let Some(index) = queue.pop_front() {
            member_indices.push(index);

            for next_index in 0..markers.len() {
                if visited[next_index] {
                    continue;
                }

                if marker_distance(&markers[index], &markers[next_index])
                    <= self.config.clustering_distance
                {
                    visited[next_index] = true;
                    queue.push_back(next_index);
                }
            }
        }

        member_indices
    }

    fn build_cluster(
        &self,
        markers: &[VisibleMarker],
        member_indices: &[usize],
    ) -> Option<VisibleActivityCluster> {
        let marker_count = member_indices.len();
        if marker_count == 0 {
            return None;
        }

        let mut total_confidence = 0.0;
        let mut weighted_x = 0.0;
        let mut weighted_y = 0.0;
        for index in member_indices {
            let marker = &markers[*index];
            total_confidence += marker.confidence;
            weighted_x += marker.x * marker.confidence;
            weighted_y += marker.y * marker.confidence;
        }

        let x = weighted_x / total_confidence.max(f32::EPSILON);
        let y = weighted_y / total_confidence.max(f32::EPSILON);
        let radius = member_indices
            .iter()
            .map(|index| {
                let dx = markers[*index].x - x;
                let dy = markers[*index].y - y;
                (dx * dx + dy * dy).sqrt()
            })
            .fold(0.0, f32::max);

        let keep_cluster = marker_count >= self.config.min_cluster_markers
            || (marker_count >= self.config.min_tight_cluster_markers
                && radius <= self.config.tight_cluster_radius);
        if !keep_cluster {
            return None;
        }

        let average_confidence = total_confidence / marker_count as f32;
        let count_score = (marker_count as f32 / 5.0).clamp(0.0, 1.0);
        let density_score = if radius <= f32::EPSILON {
            1.0
        } else {
            (self.config.tight_cluster_radius / radius).clamp(0.0, 1.0)
        };
        let confidence =
            (average_confidence * 0.6 + density_score * 0.25 + count_score * 0.15).clamp(0.0, 1.0);

        Some(VisibleActivityCluster {
            x,
            y,
            radius,
            marker_count: marker_count as u32,
            confidence,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinimapVisionConfig {
    pub border_margin_ratio: f32,
    pub min_component_area: usize,
    pub max_component_area: usize,
    pub min_component_width: usize,
    pub max_component_width: usize,
    pub min_component_height: usize,
    pub max_component_height: usize,
    pub max_aspect_ratio: f32,
    pub min_fill_ratio: f32,
    pub min_dominant_color_ratio: f32,
    pub confidence_threshold: f32,
    pub dedup_distance_ratio: f32,
}

impl Default for MinimapVisionConfig {
    fn default() -> Self {
        Self {
            border_margin_ratio: 0.03,
            min_component_area: 12,
            max_component_area: 350,
            min_component_width: 3,
            max_component_width: 30,
            min_component_height: 3,
            max_component_height: 30,
            max_aspect_ratio: 2.2,
            min_fill_ratio: 0.28,
            min_dominant_color_ratio: 0.45,
            confidence_threshold: 0.65,
            dedup_distance_ratio: 0.035,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MinimapVisionDetector {
    config: MinimapVisionConfig,
}

impl Default for MinimapVisionDetector {
    fn default() -> Self {
        Self {
            config: MinimapVisionConfig::default(),
        }
    }
}

impl MinimapVisionDetector {
    pub fn new(config: MinimapVisionConfig) -> Self {
        Self { config }
    }

    pub fn detect_markers(&self, frame: &Frame) -> Vec<VisibleMarker> {
        if frame.width == 0 || frame.height == 0 {
            return Vec::new();
        }

        let expected_len = (frame.width * frame.height * 4) as usize;
        if frame.rgba.len() != expected_len {
            return Vec::new();
        }

        let width = frame.width as usize;
        let height = frame.height as usize;
        let margin = ((frame.width.min(frame.height) as f32) * self.config.border_margin_ratio)
            .round()
            .max(1.0) as usize;

        let mut candidates = vec![None; width * height];
        for y in margin..height.saturating_sub(margin) {
            for x in margin..width.saturating_sub(margin) {
                let index = y * width + x;
                let rgba_index = index * 4;
                let pixel = Pixel::from_rgba_slice(&frame.rgba[rgba_index..rgba_index + 4]);
                candidates[index] = classify_candidate_pixel(pixel);
            }
        }

        let mut visited = vec![false; width * height];
        let mut markers_before_dedup = Vec::new();

        for y in margin..height.saturating_sub(margin) {
            for x in margin..width.saturating_sub(margin) {
                let index = y * width + x;
                if visited[index] || candidates[index].is_none() {
                    continue;
                }

                let component = collect_component(x, y, width, height, &candidates, &mut visited);
                if let Some(marker) = self.component_to_visible_marker(component, frame.width, frame.height) {
                    markers_before_dedup.push(marker);
                }
            }
        }

        self.dedup_markers(markers_before_dedup)
    }

    pub fn detect(&self, frame: &Frame) -> Vec<VisibleObject> {
        if frame.width == 0 || frame.height == 0 {
            return Vec::new();
        }

        let expected_len = (frame.width * frame.height * 4) as usize;
        if frame.rgba.len() != expected_len {
            return Vec::new();
        }

        let width = frame.width as usize;
        let height = frame.height as usize;
        let margin = ((frame.width.min(frame.height) as f32) * self.config.border_margin_ratio)
            .round()
            .max(1.0) as usize;

        let mut candidates = vec![None; width * height];
        for y in margin..height.saturating_sub(margin) {
            for x in margin..width.saturating_sub(margin) {
                let index = y * width + x;
                let rgba_index = index * 4;
                let pixel = Pixel::from_rgba_slice(&frame.rgba[rgba_index..rgba_index + 4]);
                candidates[index] = classify_candidate_pixel(pixel);
            }
        }

        let mut visited = vec![false; width * height];
        let mut candidates_before_filter = 0;
        let mut objects_before_dedup = Vec::new();

        for y in margin..height.saturating_sub(margin) {
            for x in margin..width.saturating_sub(margin) {
                let index = y * width + x;
                if visited[index] || candidates[index].is_none() {
                    continue;
                }

                let component = collect_component(x, y, width, height, &candidates, &mut visited);
                candidates_before_filter += 1;
                if let Some(object) = self.component_to_visible_object(component, frame.width, frame.height) {
                    objects_before_dedup.push(object);
                }
            }
        }

        let candidates_after_filter = objects_before_dedup.len();
        let objects = self.dedup_objects(objects_before_dedup);
        println!("[MinimapVisionDebug]");
        println!("candidates_before_filter: {candidates_before_filter}");
        println!("candidates_after_filter: {candidates_after_filter}");
        println!("objects_after_dedup: {}", objects.len());

        objects
    }

    fn component_to_visible_marker(
        &self,
        component: Component,
        frame_width: u32,
        frame_height: u32,
    ) -> Option<VisibleMarker> {
        let width = component.max_x - component.min_x + 1;
        let height = component.max_y - component.min_y + 1;
        let long_side = width.max(height) as f32;
        let short_side = width.min(height) as f32;
        let aspect_ratio = long_side / short_side.max(1.0);
        let bbox_area = width * height;
        let fill_ratio = component.area as f32 / bbox_area.max(1) as f32;

        if component.area < self.config.min_component_area
            || component.area > self.config.max_component_area
            || width < self.config.min_component_width
            || width > self.config.max_component_width
            || height < self.config.min_component_height
            || height > self.config.max_component_height
            || aspect_ratio > self.config.max_aspect_ratio
            || fill_ratio < self.config.min_fill_ratio
        {
            return None;
        }

        let shape_score = (short_side / long_side).clamp(0.0, 1.0);
        let ideal_area = 36.0;
        let area_score = if component.area as f32 <= ideal_area {
            component.area as f32 / ideal_area
        } else {
            ideal_area / component.area as f32
        }
        .clamp(0.0, 1.0);

        let friendly_score = component.friendly_score / component.area as f32;
        let enemy_score = component.enemy_score / component.area as f32;
        let unknown_score = component.unknown_score / component.area as f32;
        let color_score = friendly_score.max(enemy_score).max(unknown_score);
        let total_color_score = friendly_score + enemy_score + unknown_score;
        let dominant_color_ratio = if total_color_score <= f32::EPSILON {
            0.0
        } else {
            color_score / total_color_score
        };
        if dominant_color_ratio < self.config.min_dominant_color_ratio {
            return None;
        }

        let confidence = (color_score * 0.45
            + area_score * 0.2
            + shape_score * 0.2
            + fill_ratio * 0.1
            + dominant_color_ratio * 0.05)
            .clamp(0.0, 1.0);

        if confidence < self.config.confidence_threshold {
            return None;
        }

        Some(VisibleMarker {
            x: (component.sum_x / component.area as f32 / frame_width as f32).clamp(0.0, 1.0),
            y: (component.sum_y / component.area as f32 / frame_height as f32).clamp(0.0, 1.0),
            confidence,
        })
    }

    fn component_to_visible_object(
        &self,
        component: Component,
        frame_width: u32,
        frame_height: u32,
    ) -> Option<VisibleObject> {
        let width = component.max_x - component.min_x + 1;
        let height = component.max_y - component.min_y + 1;
        let long_side = width.max(height) as f32;
        let short_side = width.min(height) as f32;
        let aspect_ratio = long_side / short_side.max(1.0);
        let bbox_area = width * height;
        let fill_ratio = component.area as f32 / bbox_area.max(1) as f32;

        if component.area < self.config.min_component_area
            || component.area > self.config.max_component_area
            || width < self.config.min_component_width
            || width > self.config.max_component_width
            || height < self.config.min_component_height
            || height > self.config.max_component_height
            || aspect_ratio > self.config.max_aspect_ratio
            || fill_ratio < self.config.min_fill_ratio
        {
            return None;
        }

        let shape_score = (short_side / long_side).clamp(0.0, 1.0);
        let ideal_area = 36.0;
        let area_score = if component.area as f32 <= ideal_area {
            component.area as f32 / ideal_area
        } else {
            ideal_area / component.area as f32
        }
        .clamp(0.0, 1.0);

        let friendly_score = component.friendly_score / component.area as f32;
        let enemy_score = component.enemy_score / component.area as f32;
        let unknown_score = component.unknown_score / component.area as f32;
        let total_color_score = friendly_score + enemy_score + unknown_score;
        let (object_type, color_score) = classify_component_with_ratio(
            friendly_score,
            enemy_score,
            unknown_score,
            self.config.min_dominant_color_ratio,
        )?;
        let dominant_color_ratio = if total_color_score <= f32::EPSILON {
            0.0
        } else {
            color_score / total_color_score
        };
        let confidence = (color_score * 0.45
            + area_score * 0.2
            + shape_score * 0.2
            + fill_ratio * 0.1
            + dominant_color_ratio * 0.05)
            .clamp(0.0, 1.0);

        if confidence < self.config.confidence_threshold {
            return None;
        }

        Some(VisibleObject {
            object_type,
            x: (component.sum_x / component.area as f32 / frame_width as f32).clamp(0.0, 1.0),
            y: (component.sum_y / component.area as f32 / frame_height as f32).clamp(0.0, 1.0),
            confidence,
        })
    }

    fn dedup_objects(&self, mut objects: Vec<VisibleObject>) -> Vec<VisibleObject> {
        objects.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));

        let mut deduped = Vec::new();
        for object in objects {
            let is_duplicate = deduped.iter().any(|kept: &VisibleObject| {
                normalized_distance(&object, kept) < self.config.dedup_distance_ratio
            });

            if !is_duplicate {
                deduped.push(object);
            }
        }

        deduped
    }

    fn dedup_markers(&self, mut markers: Vec<VisibleMarker>) -> Vec<VisibleMarker> {
        markers.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));

        let mut deduped = Vec::new();
        for marker in markers {
            let is_duplicate = deduped.iter().any(|kept: &VisibleMarker| {
                marker_distance(&marker, kept) < self.config.dedup_distance_ratio
            });

            if !is_duplicate {
                deduped.push(marker);
            }
        }

        deduped
    }
}

#[derive(Debug, Clone, Copy)]
struct Pixel {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl Pixel {
    fn from_rgba_slice(rgba: &[u8]) -> Self {
        Self {
            r: rgba[0] as f32 / 255.0,
            g: rgba[1] as f32 / 255.0,
            b: rgba[2] as f32 / 255.0,
            a: rgba[3] as f32 / 255.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct CandidatePixel {
    friendly_score: f32,
    enemy_score: f32,
    unknown_score: f32,
}

#[derive(Debug)]
struct Component {
    area: usize,
    min_x: usize,
    min_y: usize,
    max_x: usize,
    max_y: usize,
    sum_x: f32,
    sum_y: f32,
    friendly_score: f32,
    enemy_score: f32,
    unknown_score: f32,
}

fn classify_candidate_pixel(pixel: Pixel) -> Option<CandidatePixel> {
    if pixel.a < 0.2 {
        return None;
    }

    let max_channel = pixel.r.max(pixel.g).max(pixel.b);
    let min_channel = pixel.r.min(pixel.g).min(pixel.b);
    let brightness = max_channel;
    let saturation = if max_channel <= f32::EPSILON {
        0.0
    } else {
        (max_channel - min_channel) / max_channel
    };

    if brightness < 0.45 || saturation < 0.35 {
        return None;
    }

    let enemy_score = ((pixel.r - pixel.g.max(pixel.b)).max(0.0) * 1.8 * brightness * saturation)
        .clamp(0.0, 1.0);
    let friendly_channel = pixel.g.max(pixel.b);
    let friendly_score = ((friendly_channel - pixel.r).max(0.0) * 1.6 * brightness * saturation)
        .clamp(0.0, 1.0);
    let unknown_score = (brightness * saturation * 0.35).clamp(0.0, 1.0);

    if enemy_score >= 0.35 || friendly_score >= 0.35 || unknown_score >= 0.45 {
        Some(CandidatePixel {
            friendly_score,
            enemy_score,
            unknown_score,
        })
    } else {
        None
    }
}

fn collect_component(
    start_x: usize,
    start_y: usize,
    width: usize,
    height: usize,
    candidates: &[Option<CandidatePixel>],
    visited: &mut [bool],
) -> Component {
    let mut queue = VecDeque::new();
    let mut component = Component {
        area: 0,
        min_x: start_x,
        min_y: start_y,
        max_x: start_x,
        max_y: start_y,
        sum_x: 0.0,
        sum_y: 0.0,
        friendly_score: 0.0,
        enemy_score: 0.0,
        unknown_score: 0.0,
    };

    queue.push_back((start_x, start_y));
    visited[start_y * width + start_x] = true;

    while let Some((x, y)) = queue.pop_front() {
        let index = y * width + x;
        let Some(candidate) = candidates[index] else {
            continue;
        };

        component.area += 1;
        component.min_x = component.min_x.min(x);
        component.min_y = component.min_y.min(y);
        component.max_x = component.max_x.max(x);
        component.max_y = component.max_y.max(y);
        component.sum_x += x as f32 + 0.5;
        component.sum_y += y as f32 + 0.5;
        component.friendly_score += candidate.friendly_score;
        component.enemy_score += candidate.enemy_score;
        component.unknown_score += candidate.unknown_score;

        let x_start = x.saturating_sub(1);
        let y_start = y.saturating_sub(1);
        let x_end = (x + 1).min(width - 1);
        let y_end = (y + 1).min(height - 1);

        for next_y in y_start..=y_end {
            for next_x in x_start..=x_end {
                let next_index = next_y * width + next_x;
                if visited[next_index] || candidates[next_index].is_none() {
                    continue;
                }

                visited[next_index] = true;
                queue.push_back((next_x, next_y));
            }
        }
    }

    component
}

fn classify_component_with_ratio(
    friendly_score: f32,
    enemy_score: f32,
    unknown_score: f32,
    min_dominant_color_ratio: f32,
) -> Option<(VisibleObjectType, f32)> {
    let total = friendly_score + enemy_score + unknown_score;
    if total <= f32::EPSILON {
        return None;
    }

    let enemy_ratio = enemy_score / total;
    let friendly_ratio = friendly_score / total;
    let unknown_ratio = unknown_score / total;

    if enemy_score >= 0.4
        && enemy_ratio >= min_dominant_color_ratio
        && enemy_score - friendly_score >= 0.15
    {
        Some((VisibleObjectType::EnemyChampion, enemy_score))
    } else if friendly_score >= 0.4
        && friendly_ratio >= min_dominant_color_ratio
        && friendly_score - enemy_score >= 0.15
    {
        Some((VisibleObjectType::FriendlyChampion, friendly_score))
    } else if unknown_score >= 0.5 && unknown_ratio >= min_dominant_color_ratio {
        Some((VisibleObjectType::Unknown, unknown_score))
    } else {
        None
    }
}

fn normalized_distance(a: &VisibleObject, b: &VisibleObject) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

fn marker_distance(a: &VisibleMarker, b: &VisibleMarker) -> f32 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;

    #[test]
    fn detects_current_frame_friendly_and_enemy_candidates() {
        let mut frame = blank_frame(32, 32);
        paint_square(&mut frame, 10, 10, [40, 180, 255, 255]);
        paint_square(&mut frame, 22, 20, [255, 45, 35, 255]);

        let detector = MinimapVisionDetector::default();
        let objects = detector.detect(&frame);

        assert!(objects
            .iter()
            .any(|object| object.object_type == VisibleObjectType::FriendlyChampion));
        assert!(objects
            .iter()
            .any(|object| object.object_type == VisibleObjectType::EnemyChampion));
        assert!(objects.iter().all(|object| {
            (0.0..=1.0).contains(&object.x)
                && (0.0..=1.0).contains(&object.y)
                && (0.0..=1.0).contains(&object.confidence)
        }));
    }

    #[test]
    fn ignores_border_pixels() {
        let mut frame = blank_frame(32, 32);
        paint_square(&mut frame, 0, 0, [255, 40, 40, 255]);

        let detector = MinimapVisionDetector::default();
        let objects = detector.detect(&frame);

        assert!(objects.is_empty());
    }

    #[test]
    fn discards_tiny_noise() {
        let mut frame = blank_frame(32, 32);
        set_pixel(&mut frame, 16, 16, [255, 0, 0, 255]);

        let detector = MinimapVisionDetector::default();
        let objects = detector.detect(&frame);

        assert!(objects.is_empty());
    }

    #[test]
    fn deduplicates_nearby_candidates() {
        let mut frame = blank_frame(64, 64);
        paint_square(&mut frame, 20, 20, [255, 45, 35, 255]);
        paint_square(&mut frame, 26, 20, [255, 45, 35, 255]);
        let detector = MinimapVisionDetector::new(MinimapVisionConfig {
            dedup_distance_ratio: 0.12,
            ..MinimapVisionConfig::default()
        });

        let objects = detector.detect(&frame);

        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].object_type, VisibleObjectType::EnemyChampion);
    }

    #[test]
    fn three_nearby_markers_form_one_cluster() {
        let clusterer = VisibleActivityClusterer::default();
        let clusters = clusterer.cluster(&[
            marker(0.50, 0.50, 0.9),
            marker(0.53, 0.51, 0.8),
            marker(0.49, 0.54, 0.85),
        ]);

        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].marker_count, 3);
    }

    #[test]
    fn two_separated_marker_groups_form_two_clusters() {
        let clusterer = VisibleActivityClusterer::default();
        let clusters = clusterer.cluster(&[
            marker(0.20, 0.20, 0.9),
            marker(0.22, 0.21, 0.8),
            marker(0.19, 0.23, 0.85),
            marker(0.80, 0.80, 0.9),
            marker(0.82, 0.81, 0.8),
            marker(0.79, 0.83, 0.85),
        ]);

        assert_eq!(clusters.len(), 2);
        assert!(clusters.iter().all(|cluster| cluster.marker_count == 3));
    }

    #[test]
    fn low_confidence_markers_are_filtered_before_clustering() {
        let clusterer = VisibleActivityClusterer::default();
        let clusters = clusterer.cluster(&[
            marker(0.50, 0.50, 0.4),
            marker(0.52, 0.51, 0.5),
            marker(0.49, 0.53, 0.6),
        ]);

        assert!(clusters.is_empty());
    }

    #[test]
    fn two_far_markers_do_not_form_cluster() {
        let clusterer = VisibleActivityClusterer::default();
        let clusters = clusterer.cluster(&[
            marker(0.10, 0.10, 0.9),
            marker(0.90, 0.90, 0.9),
        ]);

        assert!(clusters.is_empty());
    }

    #[test]
    fn empty_markers_return_empty_clusters() {
        let clusterer = VisibleActivityClusterer::default();
        let clusters = clusterer.cluster(&[]);

        assert!(clusters.is_empty());
    }

    #[test]
    fn cluster_center_and_radius_are_computed_from_members() {
        let clusterer = VisibleActivityClusterer::new(VisibleActivityClusterConfig {
            clustering_distance: 0.2,
            ..VisibleActivityClusterConfig::default()
        });
        let clusters = clusterer.cluster(&[
            marker(0.40, 0.50, 1.0),
            marker(0.50, 0.50, 1.0),
            marker(0.60, 0.50, 1.0),
        ]);

        assert_eq!(clusters.len(), 1);
        assert!((clusters[0].x - 0.50).abs() < 0.001);
        assert!((clusters[0].y - 0.50).abs() < 0.001);
        assert!((clusters[0].radius - 0.10).abs() < 0.001);
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

    fn set_pixel(frame: &mut Frame, x: u32, y: u32, rgba: [u8; 4]) {
        let index = ((y * frame.width + x) * 4) as usize;
        frame.rgba[index..index + 4].copy_from_slice(&rgba);
    }

    fn marker(x: f32, y: f32, confidence: f32) -> VisibleMarker {
        VisibleMarker { x, y, confidence }
    }
}
