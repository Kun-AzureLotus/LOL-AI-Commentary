use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use image::ColorType;
use lol_ai_commentator::{
    minimap_objective_detector::ObjectiveType,
    obs_vision_adapter::{
        Frame, ObsVisionClient, ObsVisionConfig, Region, RelativeRect, RoiConfig,
    },
};

const TEMPLATE_DIR: &str = "samples/templates";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 6 {
        print_usage(&args[0]);
        return Ok(());
    }

    let objective_type = args[1].parse::<ObjectiveType>()?;
    let x = args[2].parse::<f32>()?;
    let y = args[3].parse::<f32>()?;
    let width = args[4].parse::<f32>()?;
    let height = args[5].parse::<f32>()?;
    let template_name = args.get(6).cloned();

    let config = ObsVisionConfig::from_env()?;
    let mut client = ObsVisionClient::connect(config).await?;
    let frame = client.next_frame().await?;
    let minimap = RoiConfig::default().crop(&frame, Region::Minimap)?;
    let template = minimap.crop(RelativeRect::new(x, y, width, height))?;

    fs::create_dir_all(TEMPLATE_DIR)?;
    let output_path = template_path(objective_type, template_name);
    save_frame_png(&template, &output_path)?;

    println!("template type: {:?}", objective_type);
    println!("template width: {}", template.width);
    println!("template height: {}", template.height);
    println!("saved: {}", output_path.display());

    Ok(())
}

fn template_path(objective_type: ObjectiveType, template_name: Option<String>) -> PathBuf {
    let file_name = template_name.unwrap_or_else(|| {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default();
        format!("{}_{}", objective_type.file_prefix(), timestamp)
    });

    Path::new(TEMPLATE_DIR).join(format!("{file_name}.png"))
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

fn print_usage(binary_name: &str) {
    eprintln!(
        "Usage: cargo run --bin minimap_template_capture -- <turret|dragon|baron|herald> <x> <y> <width> <height> [template_name]"
    );
    eprintln!("Coordinates are normalized within the Minimap ROI, from 0.0 to 1.0.");
    eprintln!("Example: cargo run --bin minimap_template_capture -- turret 0.42 0.31 0.06 0.06 turret_01");
    eprintln!("Binary: {binary_name}");
}
