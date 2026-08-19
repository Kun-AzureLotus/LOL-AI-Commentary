use lol_ai_commentator::{
    event_engine::detect_events,
    game_state::build_game_state,
    minimap_vision_detector::{MinimapVisionDetector, VisibleActivityClusterer},
    obs_vision_adapter::{ObsVisionClient, ObsVisionConfig, Region, RoiConfig},
    riot_live_client::{RiotLiveClient, RiotLiveClientConfig},
    state_fusion::fuse_state,
    visibility_filter::VisibilityFilter,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let riot_client = RiotLiveClient::new(RiotLiveClientConfig::default())?;
    let previous_all_game_data = riot_client.get_all_game_data().await?;
    let current_all_game_data = riot_client.get_all_game_data().await?;
    let confirmed_events = detect_events(&previous_all_game_data, &current_all_game_data);
    let game_state = build_game_state(&current_all_game_data, &confirmed_events);

    let obs_config = ObsVisionConfig::from_env()?;
    let mut obs_client = ObsVisionClient::connect(obs_config).await?;
    let frame = obs_client.next_frame().await?;
    let minimap = RoiConfig::default().crop(&frame, Region::Minimap)?;
    let markers = MinimapVisionDetector::default().detect_markers(&minimap);
    let clusters = VisibleActivityClusterer::default().cluster(&markers);
    let visibility = VisibilityFilter::default().filter(&markers, &clusters);

    let unified = fuse_state(game_state, confirmed_events, visibility);

    println!("GameState summary");
    println!(
        "gold difference order_minus_chaos: {}",
        unified.game_state.gold_advantage.difference_order_minus_chaos
    );
    println!("confirmed event count: {}", unified.confirmed_events.len());
    println!(
        "visible activity cluster count: {}",
        unified.visible_activity.len()
    );

    Ok(())
}
