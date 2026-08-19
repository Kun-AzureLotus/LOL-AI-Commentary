use std::{fs, path::Path};

use image::ColorType;
use lol_ai_commentator::obs_vision_adapter::{
    Frame, ObsVisionClient, ObsVisionConfig, RoiConfig,
};

const OUTPUT_DIR: &str = "samples";
const MAIN_GAME_OUTPUT: &str = "samples/obs_main_game.png";
const MINIMAP_OUTPUT: &str = "samples/obs_minimap.png";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ObsVisionConfig::from_env()?;
    let mut client = ObsVisionClient::connect(config).await?;
    let frame = client.next_frame().await?;
    let roi_frames = RoiConfig::default().crop_all(&frame)?;

    fs::create_dir_all(OUTPUT_DIR)?;
    save_frame_png(&roi_frames.main_game, MAIN_GAME_OUTPUT)?;
    save_frame_png(&roi_frames.minimap, MINIMAP_OUTPUT)?;

    println!("original width: {}", frame.width);
    println!("original height: {}", frame.height);
    println!("main_game width: {}", roi_frames.main_game.width);
    println!("main_game height: {}", roi_frames.main_game.height);
    println!("minimap width: {}", roi_frames.minimap.width);
    println!("minimap height: {}", roi_frames.minimap.height);
    println!("main_game output: {MAIN_GAME_OUTPUT}");
    println!("minimap output: {MINIMAP_OUTPUT}");

    Ok(())
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
