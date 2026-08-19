use std::{fs, path::Path};

use image::ColorType;
use lol_ai_commentator::{
    minimap_objective_detector::{
        MinimapObjectiveDetector, ObjectiveType, TemplateStore, VisibleObjective,
    },
    obs_vision_adapter::{Frame, ObsVisionClient, ObsVisionConfig, Region, RoiConfig},
};

const TEMPLATE_DIR: &str = "samples/templates";
const OUTPUT_DIR: &str = "samples";
const DEBUG_OUTPUT: &str = "samples/obs_objectives_debug.png";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let templates = TemplateStore::load_from_dir(TEMPLATE_DIR)?;
    if templates.is_empty() {
        eprintln!("No templates found in {TEMPLATE_DIR}");
        eprintln!("Create templates with `cargo run --bin minimap_template_capture -- ...` first.");
        return Ok(());
    }

    let config = ObsVisionConfig::from_env()?;
    let mut client = ObsVisionClient::connect(config).await?;
    let frame = client.next_frame().await?;
    let minimap = RoiConfig::default().crop(&frame, Region::Minimap)?;
    let detector = MinimapObjectiveDetector::with_default_config(templates);
    let objectives = detector.detect(&minimap);

    println!("visible objectives: {}", objectives.len());
    for objective in &objectives {
        println!(
            "type: {:?}, x: {:.3}, y: {:.3}, confidence: {:.3}",
            objective.objective_type, objective.x, objective.y, objective.confidence
        );
    }

    fs::create_dir_all(OUTPUT_DIR)?;
    let debug_frame = draw_debug_objectives(&minimap, &objectives);
    save_frame_png(&debug_frame, DEBUG_OUTPUT)?;
    println!("debug output: {DEBUG_OUTPUT}");

    Ok(())
}

fn draw_debug_objectives(frame: &Frame, objectives: &[VisibleObjective]) -> Frame {
    let mut debug_frame = frame.clone();

    for objective in objectives {
        let x = (objective.x * frame.width as f32).round() as i32;
        let y = (objective.y * frame.height as f32).round() as i32;
        draw_box(&mut debug_frame, x, y, marker_color(objective.objective_type));
    }

    debug_frame
}

fn draw_box(frame: &mut Frame, center_x: i32, center_y: i32, rgba: [u8; 4]) {
    let radius = 5;
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

fn marker_color(objective_type: ObjectiveType) -> [u8; 4] {
    match objective_type {
        ObjectiveType::Turret => [255, 255, 0, 255],
        ObjectiveType::Dragon => [255, 128, 0, 255],
        ObjectiveType::Baron => [180, 80, 255, 255],
        ObjectiveType::Herald => [80, 220, 255, 255],
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
