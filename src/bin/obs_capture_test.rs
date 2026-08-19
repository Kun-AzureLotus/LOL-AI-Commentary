use lol_ai_commentator::obs_vision_adapter::{ObsVisionClient, ObsVisionConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ObsVisionConfig::from_env()?;
    let mut client = ObsVisionClient::connect(config).await?;
    let frame = client.next_frame().await?;

    println!("frame width: {}", frame.width);
    println!("frame height: {}", frame.height);
    println!("timestamp: {}", frame.timestamp_millis());

    Ok(())
}
