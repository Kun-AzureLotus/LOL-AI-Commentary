use std::{fs, path::Path};

use image::ColorType;
use lol_ai_commentator::{
    minimap_vision_detector::{MinimapVisionDetector, VisibleObject, VisibleObjectType},
    obs_vision_adapter::{Frame, ObsVisionClient, ObsVisionConfig, Region, RoiConfig},
};

const OUTPUT_DIR: &str = "samples";
const DEBUG_OUTPUT: &str = "samples/obs_minimap_debug.png";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ObsVisionConfig::from_env()?;
    let mut client = ObsVisionClient::connect(config).await?;
    let frame = client.next_frame().await?;
    let minimap = RoiConfig::default().crop(&frame, Region::Minimap)?;
    let detector = MinimapVisionDetector::default();
    let objects = detector.detect(&minimap);

    println!("visible objects: {}", objects.len());
    for object in &objects {
        println!(
            "type: {:?}, x: {:.3}, y: {:.3}, confidence: {:.3}",
            object.object_type, object.x, object.y, object.confidence
        );
    }

    fs::create_dir_all(OUTPUT_DIR)?;
    let debug_frame = draw_debug_objects(&minimap, &objects);
    save_frame_png(&debug_frame, DEBUG_OUTPUT)?;
    println!("debug output: {DEBUG_OUTPUT}");

    Ok(())
}

fn draw_debug_objects(frame: &Frame, objects: &[VisibleObject]) -> Frame {
    let mut debug_frame = frame.clone();

    for object in objects {
        let x = (object.x * frame.width as f32).round() as i32;
        let y = (object.y * frame.height as f32).round() as i32;
        let color = marker_color(object.object_type);
        draw_cross(&mut debug_frame, x, y, color);
    }

    debug_frame
}

fn draw_cross(frame: &mut Frame, x: i32, y: i32, rgba: [u8; 4]) {
    for offset in -4..=4 {
        set_pixel_if_in_bounds(frame, x + offset, y, rgba);
        set_pixel_if_in_bounds(frame, x, y + offset, rgba);
    }
}

fn set_pixel_if_in_bounds(frame: &mut Frame, x: i32, y: i32, rgba: [u8; 4]) {
    if x < 0 || y < 0 || x >= frame.width as i32 || y >= frame.height as i32 {
        return;
    }

    let index = ((y as u32 * frame.width + x as u32) * 4) as usize;
    frame.rgba[index..index + 4].copy_from_slice(&rgba);
}

fn marker_color(object_type: VisibleObjectType) -> [u8; 4] {
    match object_type {
        VisibleObjectType::FriendlyChampion => [0, 180, 255, 255],
        VisibleObjectType::EnemyChampion => [255, 40, 40, 255],
        VisibleObjectType::Unknown => [255, 255, 0, 255],
    }
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
