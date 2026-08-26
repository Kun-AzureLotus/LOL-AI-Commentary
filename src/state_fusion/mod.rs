use serde::{Deserialize, Serialize};

use crate::{
    event_engine::DetectedEvent,
    game_state::GameState,
    visibility_filter::{
        LegalVisibleActivityCluster, LegalVisibleMarker, VisibilityFilterOutput,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnifiedMatchState {
    pub game_state: GameState,
    pub confirmed_events: Vec<DetectedEvent>,
    pub visible_markers: Vec<LegalVisibleMarker>,
    pub visible_activity: Vec<LegalVisibleActivityCluster>,
}

pub fn fuse_state(
    game_state: GameState,
    confirmed_events: Vec<DetectedEvent>,
    visibility: VisibilityFilterOutput,
) -> UnifiedMatchState {
    UnifiedMatchState {
        game_state,
        confirmed_events,
        visible_markers: visibility.markers,
        visible_activity: visibility.clusters,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::{
        event_engine::DetectedEvent,
        game_state::{
            AliveChampions, DragonSoul, GameState, GoldAdvantage, ObjectiveControl,
            TeamFightStatus, TeamObjectiveCount, TimedTeamBuff,
        },
        visibility_filter::{
            LegalVisibleActivityCluster, LegalVisibleMarker, VisibilityFilterOutput, VisualSource,
        },
    };

    use super::*;

    #[test]
    fn preserves_game_state() {
        let game_state = base_game_state();
        let unified = fuse_state(game_state.clone(), Vec::new(), empty_visibility());

        assert_eq!(unified.game_state, game_state);
    }

    #[test]
    fn preserves_detected_events() {
        let event = DetectedEvent::ChampionKilled {
            event_id: Some(1),
            event_time: Some(100.0),
            killer_name: Some("Ahri".to_string()),
            victim_name: Some("Jinx".to_string()),
            assisters: vec!["Lee Sin".to_string()],
            killer_is_ally: false,
            victim_is_ally: false,
            victim_is_local_player: false,
        };

        let unified = fuse_state(base_game_state(), vec![event.clone()], empty_visibility());

        assert_eq!(unified.confirmed_events, vec![event]);
    }

    #[test]
    fn preserves_legal_visible_activity_clusters() {
        let cluster = legal_cluster();
        let visibility = VisibilityFilterOutput {
            markers: Vec::new(),
            clusters: vec![cluster.clone()],
        };

        let unified = fuse_state(base_game_state(), Vec::new(), visibility);

        assert_eq!(unified.visible_activity, vec![cluster]);
    }

    #[test]
    fn preserves_markers_without_champion_identity_fields() {
        let marker = LegalVisibleMarker {
            x: 0.25,
            y: 0.75,
            confidence: 0.9,
            source: VisualSource::VisualCurrentFrame,
        };
        let visibility = VisibilityFilterOutput {
            markers: vec![marker.clone()],
            clusters: Vec::new(),
        };

        let unified = fuse_state(base_game_state(), Vec::new(), visibility);
        let marker_json = serde_json::to_value(&unified.visible_markers[0]).expect("marker json");
        let keys = marker_json
            .as_object()
            .expect("marker object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();

        assert_eq!(unified.visible_markers, vec![marker]);
        assert_eq!(keys, vec!["confidence", "source", "x", "y"]);
    }

    #[test]
    fn empty_visual_input_does_not_generate_enemy_positions() {
        let unified = fuse_state(base_game_state(), Vec::new(), empty_visibility());
        let json = serde_json::to_value(&unified).expect("unified json");

        assert!(unified.visible_markers.is_empty());
        assert!(unified.visible_activity.is_empty());
        assert!(!contains_key_recursive(&json, "enemy_position"));
        assert!(!contains_key_recursive(&json, "champion_position"));
    }

    #[test]
    fn empty_events_still_generate_unified_match_state() {
        let unified = fuse_state(base_game_state(), Vec::new(), empty_visibility());

        assert!(unified.confirmed_events.is_empty());
        assert_eq!(unified.game_state.gold_advantage.difference_order_minus_chaos, 0);
    }

    #[test]
    fn output_contains_only_current_inputs_without_history() {
        let cluster = legal_cluster();
        let visibility = VisibilityFilterOutput {
            markers: Vec::new(),
            clusters: vec![cluster.clone()],
        };

        let unified = fuse_state(base_game_state(), Vec::new(), visibility);

        assert_eq!(unified.visible_activity, vec![cluster]);
        assert_eq!(unified.visible_activity.len(), 1);
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

    fn empty_visibility() -> VisibilityFilterOutput {
        VisibilityFilterOutput {
            markers: Vec::new(),
            clusters: Vec::new(),
        }
    }

    fn legal_cluster() -> LegalVisibleActivityCluster {
        LegalVisibleActivityCluster {
            x: 0.5,
            y: 0.5,
            radius: 0.05,
            marker_count: 3,
            confidence: 0.8,
            source: VisualSource::VisualCurrentFrame,
        }
    }

    fn contains_key_recursive(value: &Value, key: &str) -> bool {
        match value {
            Value::Object(object) => object
                .iter()
                .any(|(current_key, current_value)| {
                    current_key == key || contains_key_recursive(current_value, key)
                }),
            Value::Array(values) => values
                .iter()
                .any(|value| contains_key_recursive(value, key)),
            _ => false,
        }
    }
}
