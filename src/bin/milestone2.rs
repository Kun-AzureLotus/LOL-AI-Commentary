use lol_ai_commentator::{
    llm::LlmClient,
    riot_live_client::{RiotLiveClient, RiotLiveClientConfig},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let riot_client = RiotLiveClient::new(RiotLiveClientConfig::default())?;
    let all_game_data = riot_client.get_all_game_data().await?;
    let structured_json = serde_json::to_string_pretty(&all_game_data)?;

    let llm_client = LlmClient::from_env()?;
    let commentary = llm_client.generate_commentary(&structured_json).await?;

    println!("{commentary}");

    Ok(())
}
