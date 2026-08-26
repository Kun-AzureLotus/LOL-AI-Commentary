use serde::{Deserialize, Serialize};

use crate::{
    event_engine::DetectedEvent,
    game_state::{GameState, TeamFightStatus},
    state_fusion::UnifiedMatchState,
    visibility_filter::LegalVisibleActivityCluster,
};

const LOW_GOLD_SWING_THRESHOLD: i32 = 1_000;
const MEDIUM_GOLD_SWING_THRESHOLD: i32 = 2_000;
const POWER_SPIKE_GOLD_THRESHOLD: i32 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NarrativeEngineConfig {
    pub visual_cluster_marker_threshold: u32,
    pub visual_cluster_confidence: f32,
    pub tight_visual_cluster_marker_threshold: u32,
    pub tight_visual_cluster_radius: f32,
    pub tight_visual_cluster_confidence: f32,
}

impl Default for NarrativeEngineConfig {
    fn default() -> Self {
        Self {
            visual_cluster_marker_threshold: 3,
            visual_cluster_confidence: 0.75,
            tight_visual_cluster_marker_threshold: 2,
            tight_visual_cluster_radius: 0.055,
            tight_visual_cluster_confidence: 0.85,
        }
    }
}

pub fn build_narrative_intent(game_state: &GameState) -> NarrativeIntent {
    if game_state.elder_buff.holder.is_some() {
        return NarrativeIntent::new(true, Priority::High, Emotion::Epic, Topic::Objective);
    }

    if game_state.dragon_soul.holder.is_some() {
        return NarrativeIntent::new(true, Priority::High, Emotion::Epic, Topic::Objective);
    }

    if game_state.baron_buff.holder.is_some() {
        return NarrativeIntent::new(true, Priority::High, Emotion::Excited, Topic::Objective);
    }

    match game_state.team_fight_status {
        TeamFightStatus::RecentKills { champion_kills } if champion_kills >= 3 => {
            return NarrativeIntent::new(true, Priority::High, Emotion::Epic, Topic::TeamFight);
        }
        TeamFightStatus::RecentKills { champion_kills } if champion_kills >= 2 => {
            return NarrativeIntent::new(true, Priority::High, Emotion::Excited, Topic::TeamFight);
        }
        TeamFightStatus::RecentKills { champion_kills } if champion_kills == 1 => {
            return NarrativeIntent::new(true, Priority::Medium, Emotion::Excited, Topic::Kill);
        }
        _ => {}
    }

    let gold_difference = game_state
        .gold_advantage
        .difference_order_minus_chaos
        .abs();

    if gold_difference >= POWER_SPIKE_GOLD_THRESHOLD {
        return NarrativeIntent::new(true, Priority::High, Emotion::Excited, Topic::PowerSpike);
    }

    if gold_difference >= MEDIUM_GOLD_SWING_THRESHOLD {
        return NarrativeIntent::new(true, Priority::Medium, Emotion::Calm, Topic::GoldSwing);
    }

    if gold_difference >= LOW_GOLD_SWING_THRESHOLD {
        return NarrativeIntent::new(true, Priority::Low, Emotion::Calm, Topic::GoldSwing);
    }

    NarrativeIntent::new(false, Priority::Low, Emotion::Calm, Topic::None)
}

pub fn build_narrative_intent_from_unified_state(
    state: &UnifiedMatchState,
) -> NarrativeIntent {
    evaluate_narrative_from_unified_state(state).intent
}

pub fn evaluate_narrative_from_unified_state(state: &UnifiedMatchState) -> NarrativeEvaluation {
    evaluate_narrative_from_unified_state_with_config(state, NarrativeEngineConfig::default())
}

pub fn build_narrative_intent_from_unified_state_with_config(
    state: &UnifiedMatchState,
    config: NarrativeEngineConfig,
) -> NarrativeIntent {
    evaluate_narrative_from_unified_state_with_config(state, config).intent
}

pub fn evaluate_narrative_from_unified_state_with_config(
    state: &UnifiedMatchState,
    config: NarrativeEngineConfig,
) -> NarrativeEvaluation {
    let visual_warning_gate_passed =
        visual_warning_gate_passed_with_config(&state.visible_activity, config);

    let intent = if let Some(intent) = confirmed_event_intent(&state.confirmed_events) {
        intent
    } else if let Some(intent) = visual_warning_intent(&state.visible_activity, config) {
        intent
    } else if !state.visible_activity.is_empty() {
        visual_silence_intent()
    } else {
        silence_intent()
    };

    NarrativeEvaluation {
        intent,
        confirmed_event_count: state.confirmed_events.len(),
        visible_activity_count: state.visible_activity.len(),
        visual_warning_gate_passed,
    }
}

pub fn visual_warning_gate_passed(clusters: &[LegalVisibleActivityCluster]) -> bool {
    visual_warning_gate_passed_with_config(clusters, NarrativeEngineConfig::default())
}

pub fn should_generate_commentary(intent: &NarrativeIntent) -> bool {
    if !intent.need_commentary || intent.topic == Topic::None {
        return false;
    }

    match intent.mode {
        NarrativeMode::ConfirmedEvent => true,
        NarrativeMode::VisualWarning => {
            intent.need_commentary
                && intent.priority == Priority::Medium
                && intent.topic == Topic::VisibleActivity
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NarrativeEvaluation {
    pub intent: NarrativeIntent,
    pub confirmed_event_count: usize,
    pub visible_activity_count: usize,
    pub visual_warning_gate_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct NarrativeIntent {
    #[serde(rename = "Mode")]
    pub mode: NarrativeMode,

    #[serde(rename = "NeedCommentary")]
    pub need_commentary: bool,

    #[serde(rename = "Priority")]
    pub priority: Priority,

    #[serde(rename = "Emotion")]
    pub emotion: Emotion,

    #[serde(rename = "Topic")]
    pub topic: Topic,
}

impl NarrativeIntent {
    fn new(need_commentary: bool, priority: Priority, emotion: Emotion, topic: Topic) -> Self {
        Self::new_with_mode(
            need_commentary,
            priority,
            emotion,
            topic,
            NarrativeMode::ConfirmedEvent,
        )
    }

    fn new_with_mode(
        need_commentary: bool,
        priority: Priority,
        emotion: Emotion,
        topic: Topic,
        mode: NarrativeMode,
    ) -> Self {
        Self {
            mode,
            need_commentary,
            priority,
            emotion,
            topic,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum NarrativeMode {
    VisualWarning,
    ConfirmedEvent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum Emotion {
    Calm,
    Excited,
    Epic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum Topic {
    TeamFight,
    Objective,
    Kill,
    GoldSwing,
    PowerSpike,
    VisibleActivity,
    None,
}

fn confirmed_event_intent(events: &[DetectedEvent]) -> Option<NarrativeIntent> {
    if events.is_empty() {
        return None;
    }

    let champion_kills = events
        .iter()
        .filter(|event| matches!(event, DetectedEvent::ChampionKilled { .. }))
        .count();

    events
        .iter()
        .map(|event| match event {
            DetectedEvent::BaronTaken { .. } | DetectedEvent::DragonTaken { .. } => {
                NarrativeIntent::new_with_mode(
                    true,
                    Priority::High,
                    Emotion::Epic,
                    Topic::Objective,
                    NarrativeMode::ConfirmedEvent,
                )
            }
            DetectedEvent::ChampionKilled { .. } if champion_kills >= 3 => {
                NarrativeIntent::new_with_mode(
                    true,
                    Priority::High,
                    Emotion::Epic,
                    Topic::TeamFight,
                    NarrativeMode::ConfirmedEvent,
                )
            }
            DetectedEvent::ChampionKilled { .. } if event.involves_player_team() => {
                NarrativeIntent::new_with_mode(
                    true,
                    Priority::High,
                    Emotion::Excited,
                    Topic::Kill,
                    NarrativeMode::ConfirmedEvent,
                )
            }
            DetectedEvent::ChampionKilled { .. } => NarrativeIntent::new_with_mode(
                true,
                Priority::Medium,
                Emotion::Excited,
                Topic::Kill,
                NarrativeMode::ConfirmedEvent,
            ),
            DetectedEvent::TowerDestroyed { .. } if event.is_high_value_tower() => {
                NarrativeIntent::new_with_mode(
                    true,
                    Priority::High,
                    Emotion::Excited,
                    Topic::Objective,
                    NarrativeMode::ConfirmedEvent,
                )
            }
            DetectedEvent::TowerDestroyed { .. } | DetectedEvent::RiftHeraldTaken { .. } => {
                NarrativeIntent::new_with_mode(
                    true,
                    Priority::Medium,
                    Emotion::Excited,
                    Topic::Objective,
                    NarrativeMode::ConfirmedEvent,
                )
            }
        })
        .max_by_key(intent_rank)
}

fn cluster_meets_visual_warning_threshold(
    cluster: &LegalVisibleActivityCluster,
    config: NarrativeEngineConfig,
) -> bool {
    let condition_a = cluster.marker_count >= config.visual_cluster_marker_threshold
        && cluster.confidence >= config.visual_cluster_confidence;
    let condition_b = cluster.marker_count >= config.tight_visual_cluster_marker_threshold
        && cluster.radius <= config.tight_visual_cluster_radius
        && cluster.confidence >= config.tight_visual_cluster_confidence;
    condition_a || condition_b
}

fn visual_warning_gate_passed_with_config(
    clusters: &[LegalVisibleActivityCluster],
    config: NarrativeEngineConfig,
) -> bool {
    clusters
        .iter()
        .any(|cluster| cluster_meets_visual_warning_threshold(cluster, config))
}

fn visual_warning_intent(
    clusters: &[LegalVisibleActivityCluster],
    config: NarrativeEngineConfig,
) -> Option<NarrativeIntent> {
    clusters
        .iter()
        .filter(|cluster| cluster_meets_visual_warning_threshold(cluster, config))
        .max_by(|a, b| {
            a.confidence
                .partial_cmp(&b.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|cluster| {
            let emotion = if cluster.confidence >= config.tight_visual_cluster_confidence {
                Emotion::Excited
            } else {
                Emotion::Calm
            };

            NarrativeIntent::new_with_mode(
                true,
                Priority::Medium,
                emotion,
                Topic::VisibleActivity,
                NarrativeMode::VisualWarning,
            )
        })
}

fn visual_silence_intent() -> NarrativeIntent {
    NarrativeIntent::new_with_mode(
        false,
        Priority::Medium,
        Emotion::Calm,
        Topic::VisibleActivity,
        NarrativeMode::VisualWarning,
    )
}

fn silence_intent() -> NarrativeIntent {
    NarrativeIntent::new(false, Priority::Low, Emotion::Calm, Topic::None)
}

fn intent_rank(intent: &NarrativeIntent) -> u8 {
    match (intent.priority, intent.emotion, intent.topic) {
        (Priority::High, Emotion::Epic, _) => 5,
        (Priority::High, _, _) => 4,
        (Priority::Medium, Emotion::Excited, _) => 3,
        (Priority::Medium, _, _) => 2,
        (Priority::Low, _, _) => 1,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        event_engine::DetectedEvent,
        game_state::{
            AliveChampions, DragonSoul, GameState, GoldAdvantage, ObjectiveControl, Team,
            TeamFightStatus, TeamObjectiveCount, TimedTeamBuff,
        },
        state_fusion::UnifiedMatchState,
        visibility_filter::{
            LegalVisibleActivityCluster, LegalVisibleMarker, VisualSource,
        },
    };

    use super::*;

    #[test]
    fn returns_none_when_no_signal_is_worth_commentary() {
        let state = base_state();

        let intent = build_narrative_intent(&state);

        assert_eq!(
            intent,
            NarrativeIntent {
                mode: NarrativeMode::ConfirmedEvent,
                need_commentary: false,
                priority: Priority::Low,
                emotion: Emotion::Calm,
                topic: Topic::None,
            }
        );
    }

    #[test]
    fn objective_buffs_have_high_priority() {
        let mut state = base_state();
        state.baron_buff = TimedTeamBuff {
            holder: Some(Team::Order),
            remaining_seconds: Some(100.0),
        };

        let intent = build_narrative_intent(&state);

        assert_eq!(
            intent,
            NarrativeIntent {
                mode: NarrativeMode::ConfirmedEvent,
                need_commentary: true,
                priority: Priority::High,
                emotion: Emotion::Excited,
                topic: Topic::Objective,
            }
        );
    }

    #[test]
    fn elder_buff_is_epic_objective() {
        let mut state = base_state();
        state.elder_buff = TimedTeamBuff {
            holder: Some(Team::Chaos),
            remaining_seconds: Some(80.0),
        };

        let intent = build_narrative_intent(&state);

        assert_eq!(
            intent,
            NarrativeIntent {
                mode: NarrativeMode::ConfirmedEvent,
                need_commentary: true,
                priority: Priority::High,
                emotion: Emotion::Epic,
                topic: Topic::Objective,
            }
        );
    }

    #[test]
    fn single_recent_kill_is_kill_topic() {
        let mut state = base_state();
        state.team_fight_status = TeamFightStatus::RecentKills { champion_kills: 1 };

        let intent = build_narrative_intent(&state);

        assert_eq!(
            intent,
            NarrativeIntent {
                mode: NarrativeMode::ConfirmedEvent,
                need_commentary: true,
                priority: Priority::Medium,
                emotion: Emotion::Excited,
                topic: Topic::Kill,
            }
        );
    }

    #[test]
    fn multiple_recent_kills_are_team_fight_topic() {
        let mut state = base_state();
        state.team_fight_status = TeamFightStatus::RecentKills { champion_kills: 3 };

        let intent = build_narrative_intent(&state);

        assert_eq!(
            intent,
            NarrativeIntent {
                mode: NarrativeMode::ConfirmedEvent,
                need_commentary: true,
                priority: Priority::High,
                emotion: Emotion::Epic,
                topic: Topic::TeamFight,
            }
        );
    }

    #[test]
    fn moderate_gold_difference_is_gold_swing() {
        let mut state = base_state();
        state.gold_advantage.difference_order_minus_chaos = 2_500;
        state.gold_advantage.leading_team = Some(Team::Order);

        let intent = build_narrative_intent(&state);

        assert_eq!(
            intent,
            NarrativeIntent {
                mode: NarrativeMode::ConfirmedEvent,
                need_commentary: true,
                priority: Priority::Medium,
                emotion: Emotion::Calm,
                topic: Topic::GoldSwing,
            }
        );
    }

    #[test]
    fn large_gold_difference_is_power_spike() {
        let mut state = base_state();
        state.gold_advantage.difference_order_minus_chaos = -6_000;
        state.gold_advantage.leading_team = Some(Team::Chaos);

        let intent = build_narrative_intent(&state);

        assert_eq!(
            intent,
            NarrativeIntent {
                mode: NarrativeMode::ConfirmedEvent,
                need_commentary: true,
                priority: Priority::High,
                emotion: Emotion::Excited,
                topic: Topic::PowerSpike,
            }
        );
    }

    #[test]
    fn confirmed_event_and_visible_activity_prefers_confirmed_event() {
        let state = unified_state(
            vec![champion_kill(1)],
            vec![legal_cluster(0.5, 0.5, 0.04, 4, 0.95)],
        );

        let evaluation = evaluate_narrative_from_unified_state(&state);

        assert_eq!(evaluation.intent.mode, NarrativeMode::ConfirmedEvent);
        assert_eq!(evaluation.intent.topic, Topic::Kill);
        assert_eq!(evaluation.confirmed_event_count, 1);
        assert_eq!(evaluation.visible_activity_count, 1);
        assert!(should_generate_commentary(&evaluation.intent));
    }

    #[test]
    fn only_confirmed_event_uses_confirmed_event_mode() {
        let state = unified_state(vec![tower_destroyed(1)], Vec::new());

        let intent = build_narrative_intent_from_unified_state(&state);

        assert_eq!(intent.mode, NarrativeMode::ConfirmedEvent);
        assert_eq!(intent.priority, Priority::Medium);
        assert_eq!(intent.topic, Topic::Objective);
    }

    #[test]
    fn friendly_kill_is_high_priority_confirmed_event() {
        let state = unified_state(vec![player_team_kill(1, true, false, false)], Vec::new());
        let intent = build_narrative_intent_from_unified_state(&state);
        assert_eq!(intent.mode, NarrativeMode::ConfirmedEvent);
        assert_eq!(intent.priority, Priority::High);
        assert_eq!(intent.topic, Topic::Kill);
        assert!(intent.need_commentary);
    }

    #[test]
    fn nexus_tower_is_high_priority_objective() {
        let state = unified_state(vec![high_tower(1)], Vec::new());
        let intent = build_narrative_intent_from_unified_state(&state);
        assert_eq!(intent.mode, NarrativeMode::ConfirmedEvent);
        assert_eq!(intent.priority, Priority::High);
        assert_eq!(intent.topic, Topic::Objective);
    }

    #[test]
    fn two_markers_confidence_0_77_small_radius_is_silence() {
        let state = unified_state(
            Vec::new(),
            vec![legal_cluster(0.5, 0.5, 0.018, 2, 0.771)],
        );

        let evaluation = evaluate_narrative_from_unified_state(&state);

        assert_gated_visual_silence(&evaluation);
    }

    #[test]
    fn three_markers_confidence_0_734_is_silence() {
        let state = unified_state(
            Vec::new(),
            vec![legal_cluster(0.5, 0.5, 0.066, 3, 0.734)],
        );

        let evaluation = evaluate_narrative_from_unified_state(&state);

        assert_gated_visual_silence(&evaluation);
    }

    #[test]
    fn three_markers_confidence_0_750_is_visual_warning_medium() {
        let state = unified_state(
            Vec::new(),
            vec![legal_cluster(0.5, 0.5, 0.066, 3, 0.750)],
        );

        let evaluation = evaluate_narrative_from_unified_state(&state);

        assert_eq!(evaluation.intent.mode, NarrativeMode::VisualWarning);
        assert_eq!(evaluation.intent.priority, Priority::Medium);
        assert_eq!(evaluation.intent.topic, Topic::VisibleActivity);
        assert!(evaluation.intent.need_commentary);
        assert!(evaluation.visual_warning_gate_passed);
        assert!(should_generate_commentary(&evaluation.intent));
    }

    #[test]
    fn two_markers_radius_0_054_confidence_0_850_is_visual_warning() {
        let state = unified_state(
            Vec::new(),
            vec![legal_cluster(0.5, 0.5, 0.054, 2, 0.850)],
        );

        let evaluation = evaluate_narrative_from_unified_state(&state);

        assert_eq!(evaluation.intent.mode, NarrativeMode::VisualWarning);
        assert_eq!(evaluation.intent.priority, Priority::Medium);
        assert_eq!(evaluation.intent.topic, Topic::VisibleActivity);
        assert!(evaluation.intent.need_commentary);
        assert!(evaluation.visual_warning_gate_passed);
        assert!(should_generate_commentary(&evaluation.intent));
    }

    #[test]
    fn two_markers_radius_0_054_confidence_0_849_is_silence() {
        let state = unified_state(
            Vec::new(),
            vec![legal_cluster(0.5, 0.5, 0.054, 2, 0.849)],
        );

        let evaluation = evaluate_narrative_from_unified_state(&state);

        assert_gated_visual_silence(&evaluation);
    }

    #[test]
    fn three_markers_confidence_0_75_is_visual_warning_medium() {
        let state = unified_state(
            Vec::new(),
            vec![legal_cluster(0.5, 0.5, 0.06, 3, 0.75)],
        );

        let intent = build_narrative_intent_from_unified_state(&state);

        assert_eq!(intent.mode, NarrativeMode::VisualWarning);
        assert_eq!(intent.priority, Priority::Medium);
        assert_eq!(intent.topic, Topic::VisibleActivity);
        assert_eq!(intent.emotion, Emotion::Calm);
        assert!(intent.need_commentary);
        assert!(should_generate_commentary(&intent));
    }

    #[test]
    fn two_markers_radius_0_05_confidence_0_85_is_visual_warning_medium() {
        let state = unified_state(
            Vec::new(),
            vec![legal_cluster(0.5, 0.5, 0.05, 2, 0.85)],
        );

        let intent = build_narrative_intent_from_unified_state(&state);

        assert_eq!(intent.mode, NarrativeMode::VisualWarning);
        assert_eq!(intent.priority, Priority::Medium);
        assert_eq!(intent.topic, Topic::VisibleActivity);
        assert_eq!(intent.emotion, Emotion::Excited);
        assert!(intent.need_commentary);
        assert!(should_generate_commentary(&intent));
    }

    #[test]
    fn two_markers_confidence_0_84_is_silence() {
        let state = unified_state(
            Vec::new(),
            vec![legal_cluster(0.5, 0.5, 0.02, 2, 0.84)],
        );

        let evaluation = evaluate_narrative_from_unified_state(&state);

        assert_gated_visual_silence(&evaluation);
    }

    #[test]
    fn high_confidence_visible_activity_uses_visual_warning() {
        let state = unified_state(
            Vec::new(),
            vec![legal_cluster(0.5, 0.5, 0.06, 3, 0.8)],
        );

        let intent = build_narrative_intent_from_unified_state(&state);

        assert_eq!(intent.mode, NarrativeMode::VisualWarning);
        assert_eq!(intent.priority, Priority::Medium);
        assert_eq!(intent.topic, Topic::VisibleActivity);
        assert_eq!(intent.emotion, Emotion::Calm);
        assert!(intent.need_commentary);
    }

    #[test]
    fn low_confidence_visible_activity_is_silence() {
        let state = unified_state(
            Vec::new(),
            vec![legal_cluster(0.5, 0.5, 0.06, 3, 0.5)],
        );

        let evaluation = evaluate_narrative_from_unified_state(&state);

        assert_gated_visual_silence(&evaluation);
    }

    #[test]
    fn no_event_and_no_visible_activity_is_silence() {
        let state = unified_state(Vec::new(), Vec::new());

        let evaluation = evaluate_narrative_from_unified_state(&state);

        assert_empty_silence(&evaluation);
    }

    #[test]
    fn no_event_and_valid_visual_warning_uses_visual_warning() {
        let state = unified_state(
            Vec::new(),
            vec![legal_cluster(0.42, 0.58, 0.04, 3, 0.78)],
        );

        let intent = build_narrative_intent_from_unified_state(&state);

        assert_eq!(intent.mode, NarrativeMode::VisualWarning);
        assert_eq!(intent.priority, Priority::Medium);
        assert_eq!(intent.topic, Topic::VisibleActivity);
        assert!(intent.need_commentary);
        assert!(should_generate_commentary(&intent));
    }

    #[test]
    fn visual_warning_is_not_high_priority_or_epic() {
        let state = unified_state(
            Vec::new(),
            vec![legal_cluster(0.5, 0.5, 0.04, 4, 0.95)],
        );

        let intent = build_narrative_intent_from_unified_state(&state);

        assert_eq!(intent.mode, NarrativeMode::VisualWarning);
        assert_ne!(intent.priority, Priority::High);
        assert_ne!(intent.emotion, Emotion::Epic);
        assert_eq!(intent.priority, Priority::Medium);
    }

    #[test]
    fn topic_none_never_calls_commentary_generator() {
        let visual_fail = evaluate_narrative_from_unified_state(&unified_state(
            Vec::new(),
            vec![legal_cluster(0.5, 0.5, 0.018, 2, 0.771)],
        ));
        let empty = evaluate_narrative_from_unified_state(&unified_state(Vec::new(), Vec::new()));
        let forced_none = NarrativeIntent::new(true, Priority::Medium, Emotion::Calm, Topic::None);
        let invalid_visual = NarrativeIntent::new_with_mode(
            true,
            Priority::Low,
            Emotion::Calm,
            Topic::None,
            NarrativeMode::VisualWarning,
        );

        assert!(!visual_fail.intent.need_commentary);
        assert!(!should_generate_commentary(&visual_fail.intent));
        assert_eq!(empty.intent.topic, Topic::None);
        assert!(!should_generate_commentary(&empty.intent));
        assert!(!should_generate_commentary(&forced_none));
        assert!(!should_generate_commentary(&invalid_visual));
    }

    #[test]
    fn confirmed_event_can_be_high_priority_and_epic() {
        let state = unified_state(vec![baron_taken(1)], Vec::new());

        let intent = build_narrative_intent_from_unified_state(&state);

        assert_eq!(intent.mode, NarrativeMode::ConfirmedEvent);
        assert_eq!(intent.priority, Priority::High);
        assert_eq!(intent.emotion, Emotion::Epic);
    }

    #[test]
    fn multiple_confirmed_events_choose_highest_priority_event() {
        let state = unified_state(vec![tower_destroyed(1), baron_taken(2)], Vec::new());

        let intent = build_narrative_intent_from_unified_state(&state);

        assert_eq!(intent.mode, NarrativeMode::ConfirmedEvent);
        assert_eq!(intent.priority, Priority::High);
        assert_eq!(intent.emotion, Emotion::Epic);
        assert_eq!(intent.topic, Topic::Objective);
    }

    fn assert_gated_visual_silence(evaluation: &NarrativeEvaluation) {
        assert!(!evaluation.visual_warning_gate_passed);
        assert!(!evaluation.intent.need_commentary);
        assert_eq!(evaluation.intent.mode, NarrativeMode::VisualWarning);
        assert_ne!(
            (
                evaluation.intent.mode,
                evaluation.intent.priority,
                evaluation.intent.topic
            ),
            (NarrativeMode::ConfirmedEvent, Priority::Low, Topic::None)
        );
        assert!(!should_generate_commentary(&evaluation.intent));
    }

    fn assert_empty_silence(evaluation: &NarrativeEvaluation) {
        assert_eq!(evaluation.confirmed_event_count, 0);
        assert_eq!(evaluation.visible_activity_count, 0);
        assert!(!evaluation.visual_warning_gate_passed);
        assert!(!evaluation.intent.need_commentary);
        assert_eq!(evaluation.intent.topic, Topic::None);
        assert!(!should_generate_commentary(&evaluation.intent));
    }

    fn base_state() -> GameState {
        GameState {
            gold_advantage: GoldAdvantage {
                order_visible_item_gold: 0,
                chaos_visible_item_gold: 0,
                difference_order_minus_chaos: 0,
                leading_team: None,
            },
            objective_control: ObjectiveControl {
                dragons_taken: empty_count(),
                barons_taken: empty_count(),
                rift_heralds_taken: empty_count(),
                towers_destroyed: empty_count(),
            },
            team_fight_status: TeamFightStatus::Quiet,
            alive_champions: AliveChampions {
                order: Vec::new(),
                chaos: Vec::new(),
                unknown: Vec::new(),
            },
            baron_buff: TimedTeamBuff {
                holder: None,
                remaining_seconds: None,
            },
            dragon_soul: DragonSoul {
                holder: None,
                dragon_type: None,
            },
            elder_buff: TimedTeamBuff {
                holder: None,
                remaining_seconds: None,
            },
        }
    }

    fn empty_count() -> TeamObjectiveCount {
        TeamObjectiveCount {
            order: 0,
            chaos: 0,
            unknown: 0,
        }
    }

    fn unified_state(
        confirmed_events: Vec<DetectedEvent>,
        visible_activity: Vec<LegalVisibleActivityCluster>,
    ) -> UnifiedMatchState {
        UnifiedMatchState {
            game_state: base_state(),
            confirmed_events,
            visible_markers: Vec::<LegalVisibleMarker>::new(),
            visible_activity,
        }
    }

    fn legal_cluster(
        x: f32,
        y: f32,
        radius: f32,
        marker_count: u32,
        confidence: f32,
    ) -> LegalVisibleActivityCluster {
        LegalVisibleActivityCluster {
            x,
            y,
            radius,
            marker_count,
            confidence,
            source: VisualSource::VisualCurrentFrame,
        }
    }

    fn champion_kill(event_id: u32) -> DetectedEvent {
        player_team_kill(event_id, false, false, false)
    }

    fn player_team_kill(
        event_id: u32,
        killer_is_ally: bool,
        victim_is_ally: bool,
        victim_is_local_player: bool,
    ) -> DetectedEvent {
        DetectedEvent::ChampionKilled {
            event_id: Some(event_id),
            event_time: Some(100.0),
            killer_name: Some("Ahri".to_string()),
            victim_name: Some("Jinx".to_string()),
            assisters: Vec::new(),
            killer_is_ally,
            victim_is_ally,
            victim_is_local_player,
        }
    }

    fn high_tower(event_id: u32) -> DetectedEvent {
        DetectedEvent::TowerDestroyed {
            event_id: Some(event_id),
            event_time: Some(100.0),
            killer_name: Some("Ahri".to_string()),
            turret_killed: Some("Turret_T1_C_01_A".to_string()),
            assisters: Vec::new(),
        }
    }

    fn tower_destroyed(event_id: u32) -> DetectedEvent {
        DetectedEvent::TowerDestroyed {
            event_id: Some(event_id),
            event_time: Some(100.0),
            killer_name: Some("Ahri".to_string()),
            turret_killed: Some("Turret_T1_L_03_A".to_string()),
            assisters: Vec::new(),
        }
    }

    fn baron_taken(event_id: u32) -> DetectedEvent {
        DetectedEvent::BaronTaken {
            event_id: Some(event_id),
            event_time: Some(100.0),
            killer_name: Some("Ahri".to_string()),
            stolen: None,
        }
    }
}
