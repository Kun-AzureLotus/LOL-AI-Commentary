use lol_ai_commentator::{
    minimap_vision_detector::{MinimapVisionDetector, VisibleActivityClusterer},
    obs_vision_adapter::{ObsVisionClient, ObsVisionConfig, Region, RoiConfig},
    visibility_filter::VisibilityFilter,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ObsVisionConfig::from_env()?;
    let mut client = ObsVisionClient::connect(config).await?;
    let frame = client.next_frame().await?;
    let minimap = RoiConfig::default().crop(&frame, Region::Minimap)?;

    let detector = MinimapVisionDetector::default();
    let raw_markers = detector.detect_markers(&minimap);
    let clusterer = VisibleActivityClusterer::default();
    let raw_clusters = clusterer.cluster(&raw_markers);
    let legal = VisibilityFilter::default().filter(&raw_markers, &raw_clusters);

    println!("raw markers: {}", raw_markers.len());
    println!("legal markers: {}", legal.markers.len());
    println!("raw clusters: {}", raw_clusters.len());
    println!("legal clusters: {}", legal.clusters.len());

    Ok(())
}
