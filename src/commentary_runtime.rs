use std::{
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
    sync::Arc,
    time::Duration,
};

use crate::{
    commentary_generator::{generate_commentary, generate_commentary_with_config},
    commentary_policy::{apply_policy_to_intent, CommentaryPolicy, CommentaryPolicyInput},
    event_engine::{detect_events, DetectedEvent},
    game_state::build_game_state,
    launcher::{style_tts_emotion, AiConnectionHint, CommentaryStyle, ObsConnectionHint},
    llm::LlmConfig,
    minimap_vision_detector::{MinimapVisionDetector, VisibleActivityClusterer},
    narrative_engine::{evaluate_narrative_from_unified_state, NarrativeEvaluation, NarrativeIntent},
    obs_vision_adapter::{ObsVisionClient, ObsVisionConfig, Region, RoiConfig},
    prompt_builder::{build_prompt_with_style_and_language, PromptOutputLanguage},
    riot_live_client::{AllGameData, RiotLiveClient, RiotLiveClientConfig},
    state_fusion::{fuse_state, UnifiedMatchState},
    tts::{TtsConfig, TtsEngine, TtsPlayback, TtsPlaybackClass},
    visibility_filter::{VisibilityFilter, VisibilityFilterOutput},
};

const POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub struct CommentaryRuntimeConfig {
    pub tts: TtsConfig,
    pub style: CommentaryStyle,
    pub style_instruction: Option<String>,
    pub output_language: PromptOutputLanguage,
    pub llm: Option<LlmConfig>,
    pub obs_hint: Arc<AtomicU8>,
    pub ai_hint: Arc<AtomicU8>,
}

pub async fn run_commentary_pipeline<E>(
    config: CommentaryRuntimeConfig,
    tts: TtsPlayback<E>,
    stop: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    E: TtsEngine + Clone + 'static,
{
    let riot_client = RiotLiveClient::new(RiotLiveClientConfig::default())?;
    let obs_config = match ObsVisionConfig::from_env() {
        Ok(config) => Some(config),
        Err(error) => {
            eprintln!("[OBS Config Error] {error:?}");
            set_hint(&config.obs_hint, ObsConnectionHint::Unavailable.as_u8());
            None
        }
    };
    let mut obs_client = None;
    let previous_all_game_data = wait_for_initial_snapshot(&riot_client, &stop).await;
    let Some(mut previous_all_game_data) = previous_all_game_data else {
        tts.stop();
        return Ok(());
    };
    let mut commentary_policy = CommentaryPolicy::new();

    while !stop.load(Ordering::SeqCst) {
        interruptible_sleep(&stop, POLL_INTERVAL).await;
        if stop.load(Ordering::SeqCst) {
            break;
        }

        let current_all_game_data = match riot_client.get_all_game_data().await {
            Ok(snapshot) => snapshot,
            Err(error) => {
                eprintln!("[Riot Live Client Error] {error:?}");
                continue;
            }
        };

        let detected_events = detect_events(&previous_all_game_data, &current_all_game_data);
        let game_state = build_game_state(&current_all_game_data, &detected_events);
        let visibility = match collect_visual_visibility(
            &mut obs_client,
            obs_config.as_ref(),
            &config.obs_hint,
        )
        .await
        {
            Ok(visibility) => visibility,
            Err(error) => {
                eprintln!("[OBS Vision Error] {error:?}");
                set_hint(&config.obs_hint, ObsConnectionHint::Unavailable.as_u8());
                obs_client = None;
                empty_visibility()
            }
        };
        let unified_state = fuse_state(game_state, detected_events, visibility);
        let evaluation = evaluate_narrative_from_unified_state(&unified_state);

        print_narrative_round(&unified_state, &evaluation);

        let decision = commentary_policy.evaluate(CommentaryPolicyInput {
            narrative_intent: &evaluation.intent,
            confirmed_events: &unified_state.confirmed_events,
            game_state: &unified_state.game_state,
        });

        if decision.should_commentary {
            if commentary_policy.is_in_cooldown(&decision) {
                println!("[Cooldown]");
                previous_all_game_data = current_all_game_data;
                continue;
            }

            let mut narrative_intent = evaluation.intent.clone();
            apply_policy_to_intent(&mut narrative_intent, &decision);
            let latest_event = unified_state.confirmed_events.last();
            let prompt = build_prompt_with_style_and_language(
                &unified_state.game_state,
                &narrative_intent,
                latest_event,
                config.style_instruction.as_deref(),
                config.output_language,
            );

            let commentary_result = if let Some(llm) = &config.llm {
                generate_commentary_with_config(llm, &prompt).await
            } else {
                generate_commentary(&prompt).await
            };
            match commentary_result {
                Ok(commentary) => {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    print_commentary(&commentary);
                    set_hint(&config.ai_hint, AiConnectionHint::Connected.as_u8());
                    commentary_policy.note_emitted(&decision);
                    let class = TtsPlaybackClass::from_commentary(
                        decision.mode,
                        decision.priority,
                        decision.emotion,
                    );
                    let tts_emotion = style_tts_emotion(config.style, decision.mode, class);
                    tts.enqueue(&commentary, class, tts_emotion);
                    tts.start_drain_if_needed();
                }
                Err(error) => {
                    set_hint(&config.ai_hint, AiConnectionHint::Unavailable.as_u8());
                    eprintln!("[LLM Error] {error:?}");
                }
            }
        } else {
            println!("[No Commentary]");
        }

        previous_all_game_data = current_all_game_data;
    }

    tts.stop();
    Ok(())
}

async fn collect_visual_visibility(
    obs_client: &mut Option<ObsVisionClient>,
    obs_config: Option<&ObsVisionConfig>,
    obs_hint: &Arc<AtomicU8>,
) -> Result<VisibilityFilterOutput, Box<dyn std::error::Error + Send + Sync>> {
    let Some(obs_config) = obs_config else {
        set_hint(obs_hint, ObsConnectionHint::Unavailable.as_u8());
        return Ok(empty_visibility());
    };

    if obs_client.is_none() {
        *obs_client = Some(ObsVisionClient::connect(obs_config.clone()).await?);
    }

    let client = obs_client.as_mut().expect("OBS client should be connected");
    let frame = client.next_frame().await?;
    let minimap = RoiConfig::default().crop(&frame, Region::Minimap)?;
    let markers = MinimapVisionDetector::default().detect_markers(&minimap);
    let clusters = VisibleActivityClusterer::default().cluster(&markers);
    set_hint(obs_hint, ObsConnectionHint::Connected.as_u8());

    Ok(VisibilityFilter::default().filter(&markers, &clusters))
}

fn set_hint(slot: &Arc<AtomicU8>, value: u8) {
    slot.store(value, Ordering::SeqCst);
}

async fn interruptible_sleep(stop: &AtomicBool, duration: Duration) {
    let mut remaining = duration;
    let step = Duration::from_millis(50);
    while remaining > Duration::ZERO {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        let slice = remaining.min(step);
        tokio::time::sleep(slice).await;
        remaining = remaining.saturating_sub(slice);
    }
}

fn empty_visibility() -> VisibilityFilterOutput {
    VisibilityFilterOutput {
        markers: Vec::new(),
        clusters: Vec::new(),
    }
}

async fn wait_for_initial_snapshot(
    riot_client: &RiotLiveClient,
    stop: &AtomicBool,
) -> Option<AllGameData> {
    loop {
        if stop.load(Ordering::SeqCst) {
            return None;
        }
        match riot_client.get_all_game_data().await {
            Ok(snapshot) => return Some(snapshot),
            Err(error) => {
                eprintln!("[Riot Live Client Error] {error:?}");
                interruptible_sleep(stop, POLL_INTERVAL).await;
            }
        }
    }
}

fn print_narrative_round(unified_state: &UnifiedMatchState, evaluation: &NarrativeEvaluation) {
    print_narrative_input(unified_state);
    print_narrative_input_debug(evaluation);
    print_narrative_intent(&evaluation.intent);
}

fn print_narrative_input(unified_state: &UnifiedMatchState) {
    println!("[NarrativeInput]");
    if !unified_state.confirmed_events.is_empty() {
        print_events(&unified_state.confirmed_events);
    }
    if !unified_state.visible_activity.is_empty() {
        print_visible_activity(unified_state);
    }
    if unified_state.confirmed_events.is_empty() && unified_state.visible_activity.is_empty() {
        println!("[No Event]");
    }
}

fn print_narrative_input_debug(evaluation: &NarrativeEvaluation) {
    println!("[NarrativeInputDebug]");
    println!("confirmed_event_count: {}", evaluation.confirmed_event_count);
    println!("visible_activity_count: {}", evaluation.visible_activity_count);
    println!(
        "visual_warning_gate_passed: {}",
        evaluation.visual_warning_gate_passed
    );
    println!("selected_mode: {:?}", evaluation.intent.mode);
    println!("selected_priority: {:?}", evaluation.intent.priority);
    println!("selected_topic: {:?}", evaluation.intent.topic);
    println!();
}

fn print_events(events: &[DetectedEvent]) {
    println!("ConfirmedEvent:");
    for event in events {
        match event {
            DetectedEvent::ChampionKilled {
                killer_name,
                victim_name,
                assisters,
                killer_is_ally,
                victim_is_ally,
                victim_is_local_player,
                ..
            } => {
                println!(
                    "ChampionKilled: {} killed {} (killer_is_ally={}, victim_is_ally={}, victim_is_local_player={})",
                    display_optional(killer_name),
                    display_optional(victim_name),
                    killer_is_ally,
                    victim_is_ally,
                    victim_is_local_player
                );
                if !assisters.is_empty() {
                    println!("Assists: {}", assisters.join(", "));
                }
            }
            DetectedEvent::TowerDestroyed { turret_killed, .. } => {
                println!("TowerDestroyed: {}", display_optional(turret_killed));
            }
            DetectedEvent::DragonTaken {
                killer_name,
                dragon_type,
                ..
            } => {
                println!(
                    "DragonTaken: {} took {} dragon",
                    display_optional(killer_name),
                    display_optional(dragon_type)
                );
            }
            DetectedEvent::BaronTaken { killer_name, .. } => {
                println!("BaronTaken: {} took Baron", display_optional(killer_name));
            }
            DetectedEvent::RiftHeraldTaken { killer_name, .. } => {
                println!(
                    "RiftHeraldTaken: {} took Rift Herald",
                    display_optional(killer_name)
                );
            }
        }
    }
    println!();
}

fn print_visible_activity(unified_state: &UnifiedMatchState) {
    println!("VisibleActivity:");
    for cluster in &unified_state.visible_activity {
        println!(
            "VisibleActivityCluster: x={:.3}, y={:.3}, radius={:.3}, markers={}, confidence={:.3}",
            cluster.x, cluster.y, cluster.radius, cluster.marker_count, cluster.confidence
        );
    }
    println!();
}

fn print_narrative_intent(narrative_intent: &NarrativeIntent) {
    println!("[NarrativeIntent]");
    println!("Mode: {:?}", narrative_intent.mode);
    println!("NeedCommentary: {}", narrative_intent.need_commentary);
    println!("Priority: {:?}", narrative_intent.priority);
    println!("Emotion: {:?}", narrative_intent.emotion);
    println!("Topic: {:?}", narrative_intent.topic);
    println!();
}

fn print_commentary(commentary: &str) {
    println!("[Commentary]");
    println!("{commentary}");
    println!();
}

fn display_optional(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("Unknown")
}
