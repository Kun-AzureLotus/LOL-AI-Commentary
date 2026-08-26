use std::time::{Duration, Instant};

use crate::{
    event_engine::DetectedEvent,
    game_state::{GameState, TeamFightStatus},
    narrative_engine::{Emotion, NarrativeIntent, NarrativeMode, Priority, Topic},
};

const COMMENTARY_COOLDOWN: Duration = Duration::from_secs(5);
const MULTI_KILL_THRESHOLD: usize = 2;

#[derive(Debug, Clone, Copy)]
pub struct CommentaryPolicyInput<'a> {
    pub narrative_intent: &'a NarrativeIntent,
    pub confirmed_events: &'a [DetectedEvent],
    pub game_state: &'a GameState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentaryPolicyDecision {
    pub should_commentary: bool,
    pub priority: Priority,
    pub emotion: Emotion,
    pub mode: NarrativeMode,
    pub topic: Topic,
}

#[derive(Debug, Default)]
pub struct CommentaryPolicy {
    last_topic: Option<Topic>,
    last_commentary_at: Option<Instant>,
}

impl CommentaryPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn evaluate(&self, input: CommentaryPolicyInput<'_>) -> CommentaryPolicyDecision {
        let mut decision = decide_commentary_policy(input);
        if decision.should_commentary && self.should_suppress_duplicate(&decision) {
            decision.should_commentary = false;
        }
        decision
    }

    pub fn is_in_cooldown(&self, decision: &CommentaryPolicyDecision) -> bool {
        if can_bypass_cooldown(decision) {
            return false;
        }

        self.last_commentary_at
            .is_some_and(|last_commentary_at| last_commentary_at.elapsed() < COMMENTARY_COOLDOWN)
    }

    pub fn note_emitted(&mut self, decision: &CommentaryPolicyDecision) {
        if !decision.should_commentary {
            return;
        }

        self.last_topic = Some(decision.topic);
        self.last_commentary_at = Some(Instant::now());
    }

    fn should_suppress_duplicate(&self, decision: &CommentaryPolicyDecision) -> bool {
        if can_bypass_cooldown(decision) {
            return false;
        }

        let same_topic = self.last_topic == Some(decision.topic);
        same_topic && self.is_in_cooldown(decision)
    }
}

pub fn decide_commentary_policy(input: CommentaryPolicyInput<'_>) -> CommentaryPolicyDecision {
    if !input.confirmed_events.is_empty() {
        return decide_confirmed_event(input);
    }

    if visual_warning_gate_passed(input.narrative_intent) {
        return decide_visual_warning(input.narrative_intent);
    }

    silence_decision(input.narrative_intent)
}

pub fn apply_policy_to_intent(
    intent: &mut NarrativeIntent,
    decision: &CommentaryPolicyDecision,
) {
    intent.need_commentary = decision.should_commentary;
    intent.priority = decision.priority;
    intent.emotion = decision.emotion;
    intent.mode = decision.mode;
    intent.topic = decision.topic;
}

fn decide_confirmed_event(input: CommentaryPolicyInput<'_>) -> CommentaryPolicyDecision {
    let champion_kills = input
        .confirmed_events
        .iter()
        .filter(|event| matches!(event, DetectedEvent::ChampionKilled { .. }))
        .count();
    let recent_kills = match input.game_state.team_fight_status {
        TeamFightStatus::RecentKills { champion_kills } => champion_kills as usize,
        TeamFightStatus::Quiet => 0,
    };
    let high_value_fight = champion_kills.max(recent_kills) >= MULTI_KILL_THRESHOLD
        || has_existing_high_value_signal(input);

    input
        .confirmed_events
        .iter()
        .map(|event| match event {
            DetectedEvent::BaronTaken { stolen, .. } | DetectedEvent::DragonTaken { stolen, .. } => {
                decision(
                    true,
                    Priority::High,
                    Emotion::Epic,
                    NarrativeMode::ConfirmedEvent,
                    Topic::Objective,
                )
                .maybe_upgrade_stolen(stolen)
            }
            DetectedEvent::ChampionKilled { .. } if high_value_fight => decision(
                true,
                Priority::High,
                Emotion::Epic,
                NarrativeMode::ConfirmedEvent,
                Topic::TeamFight,
            ),
            DetectedEvent::ChampionKilled { .. } if event.involves_player_team() => decision(
                true,
                Priority::High,
                Emotion::Excited,
                NarrativeMode::ConfirmedEvent,
                Topic::Kill,
            ),
            DetectedEvent::ChampionKilled { .. } => decision(
                true,
                Priority::Medium,
                Emotion::Excited,
                NarrativeMode::ConfirmedEvent,
                Topic::Kill,
            ),
            DetectedEvent::RiftHeraldTaken { stolen, .. } => decision(
                true,
                Priority::Medium,
                Emotion::Excited,
                NarrativeMode::ConfirmedEvent,
                Topic::Objective,
            )
            .maybe_upgrade_stolen(stolen),
            DetectedEvent::TowerDestroyed { .. } if event.is_high_value_tower() => decision(
                true,
                Priority::High,
                Emotion::Excited,
                NarrativeMode::ConfirmedEvent,
                Topic::Objective,
            ),
            DetectedEvent::TowerDestroyed { .. } => decision(
                true,
                Priority::Medium,
                Emotion::Excited,
                NarrativeMode::ConfirmedEvent,
                Topic::Objective,
            ),
        })
        .max_by_key(decision_rank)
        .unwrap_or_else(|| silence_decision(input.narrative_intent))
}

fn decide_visual_warning(intent: &NarrativeIntent) -> CommentaryPolicyDecision {
    let emotion = if intent.emotion == Emotion::Excited {
        Emotion::Excited
    } else {
        Emotion::Calm
    };

    decision(
        true,
        Priority::Medium,
        emotion,
        NarrativeMode::VisualWarning,
        Topic::VisibleActivity,
    )
}

fn visual_warning_gate_passed(intent: &NarrativeIntent) -> bool {
    intent.need_commentary
        && intent.mode == NarrativeMode::VisualWarning
        && intent.topic == Topic::VisibleActivity
        && intent.priority != Priority::Low
}

fn silence_decision(intent: &NarrativeIntent) -> CommentaryPolicyDecision {
    CommentaryPolicyDecision {
        should_commentary: false,
        priority: intent.priority,
        emotion: intent.emotion,
        mode: intent.mode,
        topic: intent.topic,
    }
}

fn has_existing_high_value_signal(input: CommentaryPolicyInput<'_>) -> bool {
    if looks_like_ace(input.game_state) {
        return true;
    }

    input.confirmed_events.iter().any(|event| match event {
        DetectedEvent::DragonTaken { stolen, .. }
        | DetectedEvent::BaronTaken { stolen, .. }
        | DetectedEvent::RiftHeraldTaken { stolen, .. } => is_stolen(stolen),
        _ => false,
    })
}

fn looks_like_ace(game_state: &GameState) -> bool {
    let order_alive = !game_state.alive_champions.order.is_empty();
    let chaos_alive = !game_state.alive_champions.chaos.is_empty();
    order_alive != chaos_alive
}

fn is_stolen(stolen: &Option<String>) -> bool {
    stolen.as_deref().is_some_and(|value| {
        let value = value.trim();
        !value.is_empty() && !value.eq_ignore_ascii_case("false") && value != "0"
    })
}

fn can_bypass_cooldown(decision: &CommentaryPolicyDecision) -> bool {
    decision.should_commentary
        && decision.mode == NarrativeMode::ConfirmedEvent
        && (decision.priority == Priority::High || decision.emotion == Emotion::Epic)
}

fn decision(
    should_commentary: bool,
    priority: Priority,
    emotion: Emotion,
    mode: NarrativeMode,
    topic: Topic,
) -> CommentaryPolicyDecision {
    CommentaryPolicyDecision {
        should_commentary,
        priority,
        emotion,
        mode,
        topic,
    }
}

impl CommentaryPolicyDecision {
    fn maybe_upgrade_stolen(mut self, stolen: &Option<String>) -> Self {
        if is_stolen(stolen) {
            self.priority = Priority::High;
            self.emotion = Emotion::Epic;
        }
        self
    }
}

fn decision_rank(decision: &CommentaryPolicyDecision) -> u8 {
    match (decision.priority, decision.emotion) {
        (Priority::High, Emotion::Epic) => 5,
        (Priority::High, _) => 4,
        (Priority::Medium, Emotion::Excited) => 3,
        (Priority::Medium, _) => 2,
        (Priority::Low, _) => 1,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        event_engine::DetectedEvent,
        game_state::{
            AliveChampions, DragonSoul, GameState, GoldAdvantage, ObjectiveControl, TeamFightStatus,
            TeamObjectiveCount, TimedTeamBuff,
        },
        narrative_engine::{Emotion, NarrativeIntent, NarrativeMode, Priority, Topic},
    };

    use super::*;

    #[test]
    fn ordinary_champion_kill_is_medium_excited() {
        let events = vec![champion_kill(1)];
        let intent = confirmed_intent(Priority::Medium, Emotion::Excited, Topic::Kill);
        let game_state = base_game_state();

        let decision = decide_commentary_policy(input(&intent, &events, &game_state));

        assert!(decision.should_commentary);
        assert_eq!(decision.mode, NarrativeMode::ConfirmedEvent);
        assert_eq!(decision.priority, Priority::Medium);
        assert_eq!(decision.emotion, Emotion::Excited);
        assert_eq!(decision.topic, Topic::Kill);
    }

    #[test]
    fn consecutive_champion_kills_are_high_epic() {
        let events = vec![champion_kill(1), champion_kill(2)];
        let intent = confirmed_intent(Priority::High, Emotion::Epic, Topic::TeamFight);
        let mut game_state = base_game_state();
        game_state.team_fight_status = TeamFightStatus::RecentKills { champion_kills: 2 };

        let decision = decide_commentary_policy(input(&intent, &events, &game_state));

        assert!(decision.should_commentary);
        assert_eq!(decision.mode, NarrativeMode::ConfirmedEvent);
        assert_eq!(decision.priority, Priority::High);
        assert_eq!(decision.emotion, Emotion::Epic);
    }

    #[test]
    fn baron_taken_is_high_epic() {
        let events = vec![baron_taken(1)];
        let intent = confirmed_intent(Priority::High, Emotion::Epic, Topic::Objective);
        let game_state = base_game_state();

        let decision = decide_commentary_policy(input(&intent, &events, &game_state));

        assert_eq!(decision.priority, Priority::High);
        assert_eq!(decision.emotion, Emotion::Epic);
        assert_eq!(decision.topic, Topic::Objective);
        assert!(decision.should_commentary);
    }

    #[test]
    fn dragon_taken_is_high_epic() {
        let events = vec![dragon_taken(1)];
        let intent = confirmed_intent(Priority::High, Emotion::Epic, Topic::Objective);
        let game_state = base_game_state();

        let decision = decide_commentary_policy(input(&intent, &events, &game_state));

        assert_eq!(decision.priority, Priority::High);
        assert_eq!(decision.emotion, Emotion::Epic);
        assert_eq!(decision.topic, Topic::Objective);
        assert!(decision.should_commentary);
    }

    #[test]
    fn tower_destroyed_is_medium_excited() {
        let events = vec![tower_destroyed(1)];
        let intent = confirmed_intent(Priority::Medium, Emotion::Excited, Topic::Objective);
        let game_state = base_game_state();

        let decision = decide_commentary_policy(input(&intent, &events, &game_state));

        assert_eq!(decision.priority, Priority::Medium);
        assert_eq!(decision.emotion, Emotion::Excited);
        assert_eq!(decision.topic, Topic::Objective);
        assert!(decision.should_commentary);
    }

    #[test]
    fn visual_warning_is_medium_calm() {
        let intent = visual_intent(Emotion::Calm);
        let game_state = base_game_state();

        let decision = decide_commentary_policy(input(&intent, &[], &game_state));

        assert!(decision.should_commentary);
        assert_eq!(decision.mode, NarrativeMode::VisualWarning);
        assert_eq!(decision.priority, Priority::Medium);
        assert_eq!(decision.emotion, Emotion::Calm);
        assert_eq!(decision.topic, Topic::VisibleActivity);
    }

    #[test]
    fn high_confidence_visual_warning_is_medium_excited() {
        let intent = visual_intent(Emotion::Excited);
        let game_state = base_game_state();

        let decision = decide_commentary_policy(input(&intent, &[], &game_state));

        assert!(decision.should_commentary);
        assert_eq!(decision.mode, NarrativeMode::VisualWarning);
        assert_eq!(decision.priority, Priority::Medium);
        assert_eq!(decision.emotion, Emotion::Excited);
    }

    #[test]
    fn silence_sets_should_commentary_false() {
        let intent = NarrativeIntent {
            mode: NarrativeMode::ConfirmedEvent,
            need_commentary: false,
            priority: Priority::Low,
            emotion: Emotion::Calm,
            topic: Topic::None,
        };
        let game_state = base_game_state();

        let decision = decide_commentary_policy(input(&intent, &[], &game_state));

        assert!(!decision.should_commentary);
    }

    #[test]
    fn confirmed_event_outranks_visual_warning() {
        let events = vec![champion_kill(1)];
        let intent = visual_intent(Emotion::Excited);
        let game_state = base_game_state();

        let decision = decide_commentary_policy(input(&intent, &events, &game_state));

        assert!(decision.should_commentary);
        assert_eq!(decision.mode, NarrativeMode::ConfirmedEvent);
        assert_eq!(decision.topic, Topic::Kill);
        assert_eq!(decision.priority, Priority::Medium);
        assert_eq!(decision.emotion, Emotion::Excited);
    }

    #[test]
    fn visual_warning_is_never_high_or_epic() {
        let mut intent = visual_intent(Emotion::Epic);
        intent.priority = Priority::High;
        let game_state = base_game_state();

        let decision = decide_commentary_policy(input(&intent, &[], &game_state));

        assert_eq!(decision.mode, NarrativeMode::VisualWarning);
        assert_ne!(decision.priority, Priority::High);
        assert_ne!(decision.emotion, Emotion::Epic);
        assert_eq!(decision.priority, Priority::Medium);
    }

    #[test]
    fn visual_warning_without_gate_is_silence() {
        let intent = NarrativeIntent {
            mode: NarrativeMode::VisualWarning,
            need_commentary: false,
            priority: Priority::Medium,
            emotion: Emotion::Calm,
            topic: Topic::VisibleActivity,
        };
        let game_state = base_game_state();

        let decision = decide_commentary_policy(input(&intent, &[], &game_state));

        assert!(!decision.should_commentary);
    }

    #[test]
    fn visual_warning_cannot_bypass_cooldown() {
        let intent = visual_intent(Emotion::Calm);
        let game_state = base_game_state();
        let decision = decide_commentary_policy(input(&intent, &[], &game_state));

        assert!(!can_bypass_cooldown(&decision));
    }

    #[test]
    fn friendly_kill_is_high_priority() {
        let events = vec![player_team_kill(1, true, false, false)];
        let intent = confirmed_intent(Priority::High, Emotion::Excited, Topic::Kill);
        let game_state = base_game_state();

        let decision = decide_commentary_policy(input(&intent, &events, &game_state));

        assert!(decision.should_commentary);
        assert_eq!(decision.priority, Priority::High);
        assert_eq!(decision.emotion, Emotion::Excited);
        assert_eq!(decision.topic, Topic::Kill);
        assert!(can_bypass_cooldown(&decision));
    }

    #[test]
    fn ally_death_is_high_priority() {
        let events = vec![player_team_kill(1, false, true, false)];
        let intent = confirmed_intent(Priority::High, Emotion::Excited, Topic::Kill);
        let game_state = base_game_state();

        let decision = decide_commentary_policy(input(&intent, &events, &game_state));

        assert!(decision.should_commentary);
        assert_eq!(decision.priority, Priority::High);
        assert!(events[0].is_ally_death());
    }

    #[test]
    fn local_player_death_is_high_priority() {
        let events = vec![player_team_kill(1, false, true, true)];
        let intent = confirmed_intent(Priority::High, Emotion::Excited, Topic::Kill);
        let game_state = base_game_state();

        let decision = decide_commentary_policy(input(&intent, &events, &game_state));

        assert!(decision.should_commentary);
        assert_eq!(decision.priority, Priority::High);
        assert!(events[0].is_local_player_death());
        assert!(can_bypass_cooldown(&decision));
    }

    #[test]
    fn high_value_tower_is_high_priority() {
        let events = vec![high_tower(1)];
        let intent = confirmed_intent(Priority::High, Emotion::Excited, Topic::Objective);
        let game_state = base_game_state();

        let decision = decide_commentary_policy(input(&intent, &events, &game_state));

        assert_eq!(decision.priority, Priority::High);
        assert_eq!(decision.topic, Topic::Objective);
        assert!(decision.should_commentary);
        assert!(can_bypass_cooldown(&decision));
    }

    #[test]
    fn high_priority_kill_bypasses_cooldown() {
        let mut policy = CommentaryPolicy::new();
        let game_state = base_game_state();
        let first_events = vec![player_team_kill(1, true, false, false)];
        let first_intent = confirmed_intent(Priority::High, Emotion::Excited, Topic::Kill);
        let first = policy.evaluate(input(&first_intent, &first_events, &game_state));
        policy.note_emitted(&first);

        let second_events = vec![player_team_kill(2, false, true, false)];
        let second_intent = confirmed_intent(Priority::High, Emotion::Excited, Topic::Kill);
        let second = policy.evaluate(input(&second_intent, &second_events, &game_state));

        assert!(second.should_commentary);
        assert!(!policy.is_in_cooldown(&second));
    }

    #[test]
    fn medium_events_still_respect_cooldown() {
        let mut policy = CommentaryPolicy::new();
        let game_state = base_game_state();
        let first_events = vec![champion_kill(1)];
        let first_intent = confirmed_intent(Priority::Medium, Emotion::Excited, Topic::Kill);
        let first = policy.evaluate(input(&first_intent, &first_events, &game_state));
        policy.note_emitted(&first);

        let visual = visual_intent(Emotion::Calm);
        let second = policy.evaluate(input(&visual, &[], &game_state));
        assert!(policy.is_in_cooldown(&second));
    }

    fn input<'a>(
        intent: &'a NarrativeIntent,
        events: &'a [DetectedEvent],
        game_state: &'a GameState,
    ) -> CommentaryPolicyInput<'a> {
        CommentaryPolicyInput {
            narrative_intent: intent,
            confirmed_events: events,
            game_state,
        }
    }

    fn confirmed_intent(
        priority: Priority,
        emotion: Emotion,
        topic: Topic,
    ) -> NarrativeIntent {
        NarrativeIntent {
            mode: NarrativeMode::ConfirmedEvent,
            need_commentary: true,
            priority,
            emotion,
            topic,
        }
    }

    fn visual_intent(emotion: Emotion) -> NarrativeIntent {
        NarrativeIntent {
            mode: NarrativeMode::VisualWarning,
            need_commentary: true,
            priority: Priority::Medium,
            emotion,
            topic: Topic::VisibleActivity,
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

    fn dragon_taken(event_id: u32) -> DetectedEvent {
        DetectedEvent::DragonTaken {
            event_id: Some(event_id),
            event_time: Some(100.0),
            killer_name: Some("Ahri".to_string()),
            dragon_type: Some("Fire".to_string()),
            stolen: None,
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

    fn base_game_state() -> GameState {
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
}
