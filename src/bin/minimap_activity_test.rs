use std::{fs, path::Path};

use image::ColorType;
use lol_ai_commentator::{
    minimap_vision_detector::{
        MinimapVisionDetector, VisibleActivityCluster, VisibleActivityClusterer, VisibleMarker,
    },
    obs_vision_adapter::{Frame, ObsVisionClient, ObsVisionConfig, Region, RoiConfig},
};

const OUTPUT_DIR: &str = "samples";
const DEBUG_OUTPUT: &str = "samples/obs_minimap_activity_debug.png";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ObsVisionConfig::from_env()?;
    let mut client = ObsVisionClient::connect(config).await?;
    let frame = client.next_frame().await?;
    let minimap = RoiConfig::default().crop(&frame, Region::Minimap)?;
    let detector = MinimapVisionDetector::default();
    let markers = detector.detect_markers(&minimap);
    let clusterer = VisibleActivityClusterer::default();
    let clusters = clusterer.cluster(&markers);

    println!("marker_count: {}", markers.len());
    println!("cluster_count: {}", clusters.len());
    for cluster in &clusters {
        println!(
            "x: {:.3}, y: {:.3}, radius: {:.3}, marker_count: {}, confidence: {:.3}",
            cluster.x, cluster.y, cluster.radius, cluster.marker_count, cluster.confidence
        );
    }

    fs::create_dir_all(OUTPUT_DIR)?;
    let debug_frame = draw_debug_activity(&minimap, &markers, &clusters);
    save_frame_png(&debug_frame, DEBUG_OUTPUT)?;
    println!("debug output: {DEBUG_OUTPUT}");

    Ok(())
}

fn draw_debug_activity(
    frame: &Frame,
    markers: &[VisibleMarker],
    clusters: &[VisibleActivityCluster],
) -> Frame {
    let mut debug_frame = frame.clone();

    for marker in markers {
        let x = (marker.x * frame.width as f32).round() as i32;
        let y = (marker.y * frame.height as f32).round() as i32;
        draw_cross(&mut debug_frame, x, y, [255, 255, 0, 255]);
    }

    for cluster in clusters {
        let x = (cluster.x * frame.width as f32).round() as i32;
        let y = (cluster.y * frame.height as f32).round() as i32;
        let radius = (cluster.radius * frame.width.max(frame.height) as f32).round() as i32;
        draw_box(&mut debug_frame, x, y, radius.max(4), [255, 0, 255, 255]);
    }

    debug_frame
}

fn draw_cross(frame: &mut Frame, x: i32, y: i32, rgba: [u8; 4]) {
    for offset in -3..=3 {
        set_pixel_if_in_bounds(frame, x + offset, y, rgba);
        set_pixel_if_in_bounds(frame, x, y + offset, rgba);
    }
}

fn draw_box(frame: &mut Frame, center_x: i32, center_y: i32, radius: i32, rgba: [u8; 4]) {
    for offset in -radius..=radius {
        set_pixel_if_in_bounds(frame, center_x + offset, center_y - radius, rgba);
        set_pixel_if_in_bounds(frame, center_x + offset, center_y + radius, rgba);
        set_pixel_if_in_bounds(frame, center_x - radius, center_y + offset, rgba);
        set_pixel_if_in_bounds(frame, center_x + radius, center_y + offset, rgba);
    }
}

fn set_pixel_if_in_bounds(frame: &mut Frame, x: i32, y: i32, rgba: [u8; 4]) {
    if x < 0 || y < 0 || x >= frame.width as i32 || y >= frame.height as i32 {
        return;
    }

    let index = ((y as u32 * frame.width + x as u32) * 4) as usize;
    frame.rgba[index..index + 4].copy_from_slice(&rgba);
}

fn save_frame_png(frame: &Frame, path: impl AsRef<Path>) -> Result<(), image::ImageError> {
    image::save_buffer(
        path,
        &frame.rgba,
        frame.width,
        frame.height,
        ColorType::Rgba8,
    )
}
