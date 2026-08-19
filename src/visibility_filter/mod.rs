use serde::{Deserialize, Serialize};

use crate::minimap_vision_detector::{VisibleActivityCluster, VisibleMarker};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegalVisibleMarker {
    pub x: f32,
    pub y: f32,
    pub confidence: f32,
    pub source: VisualSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LegalVisibleActivityCluster {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub marker_count: u32,
    pub confidence: f32,
    pub source: VisualSource,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum VisualSource {
    VisualCurrentFrame,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisibilityFilterOutput {
    pub markers: Vec<LegalVisibleMarker>,
    pub clusters: Vec<LegalVisibleActivityCluster>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisibilityFilterConfig {
    pub min_marker_confidence: f32,
    pub min_cluster_confidence: f32,
    pub min_cluster_markers: u32,
    pub max_cluster_radius: f32,
}

impl Default for VisibilityFilterConfig {
    fn default() -> Self {
        Self {
            min_marker_confidence: 0.65,
            min_cluster_confidence: 0.65,
            min_cluster_markers: 2,
            max_cluster_radius: 0.20,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VisibilityFilter {
    config: VisibilityFilterConfig,
}

impl Default for VisibilityFilter {
    fn default() -> Self {
        Self {
            config: VisibilityFilterConfig::default(),
        }
    }
}

impl VisibilityFilter {
    pub fn new(config: VisibilityFilterConfig) -> Self {
        Self { config }
    }

    pub fn filter(
        &self,
        markers: &[VisibleMarker],
        clusters: &[VisibleActivityCluster],
    ) -> VisibilityFilterOutput {
        VisibilityFilterOutput {
            markers: markers
                .iter()
                .filter_map(|marker| self.filter_marker(marker))
                .collect(),
            clusters: clusters
                .iter()
                .filter_map(|cluster| self.filter_cluster(cluster))
                .collect(),
        }
    }

    fn filter_marker(&self, marker: &VisibleMarker) -> Option<LegalVisibleMarker> {
        if !valid_unit_interval(marker.x)
            || !valid_unit_interval(marker.y)
            || !valid_unit_interval(marker.confidence)
            || marker.confidence < self.config.min_marker_confidence
        {
            return None;
        }

        Some(LegalVisibleMarker {
            x: marker.x,
            y: marker.y,
            confidence: marker.confidence,
            source: VisualSource::VisualCurrentFrame,
        })
    }

    fn filter_cluster(
        &self,
        cluster: &VisibleActivityCluster,
    ) -> Option<LegalVisibleActivityCluster> {
        if !valid_unit_interval(cluster.x)
            || !valid_unit_interval(cluster.y)
            || !valid_unit_interval(cluster.confidence)
            || !valid_radius(cluster.radius, self.config.max_cluster_radius)
            || cluster.confidence < self.config.min_cluster_confidence
            || cluster.marker_count < self.config.min_cluster_markers
        {
            return None;
        }

        Some(LegalVisibleActivityCluster {
            x: cluster.x,
            y: cluster.y,
            radius: cluster.radius,
            marker_count: cluster.marker_count,
            confidence: cluster.confidence,
            source: VisualSource::VisualCurrentFrame,
        })
    }
}

fn valid_unit_interval(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

fn valid_radius(radius: f32, max_cluster_radius: f32) -> bool {
    radius.is_finite() && radius >= 0.0 && radius <= max_cluster_radius
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_valid_marker() {
        let output = VisibilityFilter::default().filter(&[marker(0.5, 0.5, 0.9)], &[]);

        assert_eq!(output.markers.len(), 1);
        assert_eq!(output.markers[0].source, VisualSource::VisualCurrentFrame);
    }

    #[test]
    fn drops_low_confidence_marker() {
        let output = VisibilityFilter::default().filter(&[marker(0.5, 0.5, 0.3)], &[]);

        assert!(output.markers.is_empty());
    }

    #[test]
    fn drops_marker_with_out_of_range_coordinates() {
        let output = VisibilityFilter::default().filter(&[marker(1.2, 0.5, 0.9)], &[]);

        assert!(output.markers.is_empty());
    }

    #[test]
    fn drops_marker_with_nan_or_infinite_values() {
        let output = VisibilityFilter::default().filter(
            &[
                marker(f32::NAN, 0.5, 0.9),
                marker(0.5, f32::INFINITY, 0.9),
                marker(0.5, 0.5, f32::NAN),
            ],
            &[],
        );

        assert!(output.markers.is_empty());
    }

    #[test]
    fn keeps_valid_cluster() {
        let output = VisibilityFilter::default().filter(&[], &[cluster(0.5, 0.5, 0.05, 3, 0.8)]);

        assert_eq!(output.clusters.len(), 1);
        assert_eq!(output.clusters[0].source, VisualSource::VisualCurrentFrame);
    }

    #[test]
    fn drops_cluster_with_too_few_markers() {
        let output = VisibilityFilter::default().filter(&[], &[cluster(0.5, 0.5, 0.05, 1, 0.8)]);

        assert!(output.clusters.is_empty());
    }

    #[test]
    fn drops_low_confidence_cluster() {
        let output = VisibilityFilter::default().filter(&[], &[cluster(0.5, 0.5, 0.05, 3, 0.4)]);

        assert!(output.clusters.is_empty());
    }

    #[test]
    fn drops_cluster_with_out_of_range_radius() {
        let output = VisibilityFilter::default().filter(&[], &[cluster(0.5, 0.5, 0.4, 3, 0.8)]);

        assert!(output.clusters.is_empty());
    }

    #[test]
    fn empty_input_returns_empty_output() {
        let output = VisibilityFilter::default().filter(&[], &[]);

        assert!(output.markers.is_empty());
        assert!(output.clusters.is_empty());
    }

    fn marker(x: f32, y: f32, confidence: f32) -> VisibleMarker {
        VisibleMarker { x, y, confidence }
    }

    fn cluster(
        x: f32,
        y: f32,
        radius: f32,
        marker_count: u32,
        confidence: f32,
    ) -> VisibleActivityCluster {
        VisibleActivityCluster {
            x,
            y,
            radius,
            marker_count,
            confidence,
        }
    }
}
