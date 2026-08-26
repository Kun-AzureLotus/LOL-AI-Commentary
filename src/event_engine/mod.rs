use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::riot_live_client::{AllGameData, GameEvent};

pub fn detect_events(previous: &AllGameData, current: &AllGameData) -> Vec<DetectedEvent> {
    let previous_keys = previous
        .events
        .events
        .iter()
        .map(EventKey::from)
        .collect::<HashSet<_>>();

    current
        .events
        .events
        .iter()
        .filter(|event| !previous_keys.contains(&EventKey::from(*event)))
        .filter_map(|event| DetectedEvent::from_game_event(event, current))
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum DetectedEvent {
    ChampionKilled {
        event_id: Option<u32>,
        event_time: Option<f64>,
        killer_name: Option<String>,
        victim_name: Option<String>,
        assisters: Vec<String>,
        #[serde(default)]
        killer_is_ally: bool,
        #[serde(default)]
        victim_is_ally: bool,
        #[serde(default)]
        victim_is_local_player: bool,
    },
    TowerDestroyed {
        event_id: Option<u32>,
        event_time: Option<f64>,
        killer_name: Option<String>,
        turret_killed: Option<String>,
        assisters: Vec<String>,
    },
    DragonTaken {
        event_id: Option<u32>,
        event_time: Option<f64>,
        killer_name: Option<String>,
        dragon_type: Option<String>,
        stolen: Option<String>,
    },
    BaronTaken {
        event_id: Option<u32>,
        event_time: Option<f64>,
        killer_name: Option<String>,
        stolen: Option<String>,
    },
    RiftHeraldTaken {
        event_id: Option<u32>,
        event_time: Option<f64>,
        killer_name: Option<String>,
        stolen: Option<String>,
    },
}

impl DetectedEvent {
    pub fn is_friendly_kill(&self) -> bool {
        matches!(
            self,
            Self::ChampionKilled {
                killer_is_ally: true,
                ..
            }
        )
    }

    pub fn is_ally_death(&self) -> bool {
        matches!(
            self,
            Self::ChampionKilled {
                victim_is_ally: true,
                victim_is_local_player: false,
                ..
            }
        )
    }

    pub fn is_local_player_death(&self) -> bool {
        matches!(
            self,
            Self::ChampionKilled {
                victim_is_local_player: true,
                ..
            }
        )
    }

    pub fn involves_player_team(&self) -> bool {
        self.is_friendly_kill() || self.is_ally_death() || self.is_local_player_death()
    }

    pub fn is_high_value_tower(&self) -> bool {
        match self {
            Self::TowerDestroyed { turret_killed, .. } => turret_is_high_value(turret_killed),
            _ => false,
        }
    }

    fn from_game_event(event: &GameEvent, all_game_data: &AllGameData) -> Option<Self> {
        match event.event_name.as_str() {
            "ChampionKill" => {
                let (killer_is_ally, victim_is_ally, victim_is_local_player) =
                    kill_sides(all_game_data, event.killer_name.as_deref(), event.victim_name.as_deref());
                Some(Self::ChampionKilled {
                    event_id: event.event_id,
                    event_time: event.event_time,
                    killer_name: event.killer_name.clone(),
                    victim_name: event.victim_name.clone(),
                    assisters: event.assisters.clone(),
                    killer_is_ally,
                    victim_is_ally,
                    victim_is_local_player,
                })
            }
            "TurretKilled" => Some(Self::TowerDestroyed {
                event_id: event.event_id,
                event_time: event.event_time,
                killer_name: event.killer_name.clone(),
                turret_killed: event.turret_killed.clone(),
                assisters: event.assisters.clone(),
            }),
            "InhibKilled" => Some(Self::TowerDestroyed {
                event_id: event.event_id,
                event_time: event.event_time,
                killer_name: event.killer_name.clone(),
                turret_killed: event
                    .inhib_killed
                    .clone()
                    .or_else(|| Some("Inhibitor".to_string())),
                assisters: event.assisters.clone(),
            }),
            "DragonKill" => Some(Self::DragonTaken {
                event_id: event.event_id,
                event_time: event.event_time,
                killer_name: event.killer_name.clone(),
                dragon_type: event.dragon_type.clone(),
                stolen: event.stolen.clone(),
            }),
            "BaronKill" => Some(Self::BaronTaken {
                event_id: event.event_id,
                event_time: event.event_time,
                killer_name: event.killer_name.clone(),
                stolen: event.stolen.clone(),
            }),
            "HeraldKill" => Some(Self::RiftHeraldTaken {
                event_id: event.event_id,
                event_time: event.event_time,
                killer_name: event.killer_name.clone(),
                stolen: event.stolen.clone(),
            }),
            _ => None,
        }
    }
}

fn kill_sides(
    all_game_data: &AllGameData,
    killer_name: Option<&str>,
    victim_name: Option<&str>,
) -> (bool, bool, bool) {
    let local_team = local_player_team(all_game_data);
    let killer_is_ally = same_known_team(local_team, player_team_by_name(all_game_data, killer_name));
    let victim_team = player_team_by_name(all_game_data, victim_name);
    let victim_is_local_player = is_local_player(all_game_data, victim_name);
    let victim_is_ally = victim_is_local_player || same_known_team(local_team, victim_team);
    (killer_is_ally, victim_is_ally, victim_is_local_player)
}

fn local_player_team(all_game_data: &AllGameData) -> Option<TeamRef> {
    local_player(all_game_data).and_then(player_team_ref)
}

fn local_player(all_game_data: &AllGameData) -> Option<&crate::riot_live_client::Player> {
    let active = &all_game_data.active_player;
    all_game_data.all_players.iter().find(|player| {
        names_match(active.summoner_name.as_deref(), player.summoner_name.as_deref())
            || names_match(active.riot_id.as_deref(), player.riot_id.as_deref())
            || names_match(active.summoner_name.as_deref(), player.riot_id.as_deref())
            || names_match(active.riot_id.as_deref(), player.summoner_name.as_deref())
    })
}

fn is_local_player(all_game_data: &AllGameData, name: Option<&str>) -> bool {
    let Some(name) = name else {
        return false;
    };
    let active = &all_game_data.active_player;
    if names_match(Some(name), active.summoner_name.as_deref())
        || names_match(Some(name), active.riot_id.as_deref())
    {
        return true;
    }
    local_player(all_game_data).is_some_and(|player| {
        names_match(Some(name), player.summoner_name.as_deref())
            || names_match(Some(name), player.riot_id.as_deref())
            || names_match(Some(name), player.champion_name.as_deref())
    })
}

fn player_team_by_name(all_game_data: &AllGameData, name: Option<&str>) -> Option<TeamRef> {
    let name = name?;
    let normalized = normalize_name(name);
    all_game_data.all_players.iter().find_map(|player| {
        let matches_player = names_match(Some(name), player.summoner_name.as_deref())
            || riot_id_matches(player.riot_id.as_deref(), &normalized)
            || names_match(Some(name), player.champion_name.as_deref());
        matches_player.then(|| player_team_ref(player)).flatten()
    })
}

fn player_team_ref(player: &crate::riot_live_client::Player) -> Option<TeamRef> {
    match player
        .team
        .as_deref()
        .map(|team| team.trim().to_ascii_uppercase())
        .as_deref()
    {
        Some("ORDER") => Some(TeamRef::Order),
        Some("CHAOS") => Some(TeamRef::Chaos),
        _ => None,
    }
}

fn same_known_team(left: Option<TeamRef>, right: Option<TeamRef>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if left == right)
}

fn names_match(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => {
            let left = normalize_name(left);
            let right = normalize_name(right);
            !left.is_empty()
                && (left == right
                    || left
                        .split_once('#')
                        .is_some_and(|(game_name, _)| game_name == right)
                    || right
                        .split_once('#')
                        .is_some_and(|(game_name, _)| game_name == left))
        }
        _ => false,
    }
}

fn riot_id_matches(candidate: Option<&str>, normalized_name: &str) -> bool {
    let Some(candidate) = candidate else {
        return false;
    };
    let normalized_candidate = normalize_name(candidate);
    normalized_candidate == *normalized_name
        || normalized_candidate
            .split_once('#')
            .is_some_and(|(game_name, _)| game_name == normalized_name)
}

fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn turret_is_high_value(turret_killed: &Option<String>) -> bool {
    let Some(name) = turret_killed.as_deref() else {
        return false;
    };
    let name = name.to_ascii_uppercase();
    if name.contains("INHIB") || name.contains("BARRACKS") {
        return true;
    }
    const HIGH_TOKENS: [&str; 6] = ["_C_01", "_C_02", "_C_03", "_C_04", "_L_01", "_L_02"];
    HIGH_TOKENS.iter().any(|token| name.contains(token))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TeamRef {
    Order,
    Chaos,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum EventKey {
    Id(u32),
    Fallback {
        event_name: String,
        event_time_bits: Option<u64>,
    },
}

impl From<&GameEvent> for EventKey {
    fn from(event: &GameEvent) -> Self {
        if let Some(event_id) = event.event_id {
            Self::Id(event_id)
        } else {
            Self::Fallback {
                event_name: event.event_name.clone(),
                event_time_bits: event.event_time.map(f64::to_bits),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    #[test]
    fn detects_only_supported_new_events() {
        let previous = snapshot(vec![json!({
            "EventID": 0,
            "EventName": "GameStart",
            "EventTime": 0.0
        })]);
        let current = snapshot(vec![
            json!({
                "EventID": 0,
                "EventName": "GameStart",
                "EventTime": 0.0
            }),
            json!({
                "EventID": 1,
                "EventName": "ChampionKill",
                "EventTime": 120.5,
                "KillerName": "Ahri",
                "VictimName": "Jinx",
                "Assisters": ["Lee Sin"]
            }),
            json!({
                "EventID": 2,
                "EventName": "TurretKilled",
                "EventTime": 260.0,
                "KillerName": "Caitlyn",
                "TurretKilled": "Turret_T1_L_03_A"
            }),
            json!({
                "EventID": 3,
                "EventName": "DragonKill",
                "EventTime": 360.0,
                "KillerName": "Lee Sin",
                "DragonType": "Fire",
                "Stolen": "False"
            }),
            json!({
                "EventID": 4,
                "EventName": "BaronKill",
                "EventTime": 1260.0,
                "KillerName": "Ahri",
                "Stolen": "False"
            }),
            json!({
                "EventID": 5,
                "EventName": "HeraldKill",
                "EventTime": 520.0,
                "KillerName": "Lee Sin",
                "Stolen": "True"
            }),
            json!({
                "EventID": 6,
                "EventName": "InhibKilled",
                "EventTime": 1500.0
            }),
        ]);

        let events = detect_events(&previous, &current);

        assert_eq!(events.len(), 6);
        assert_eq!(
            events[0],
            DetectedEvent::ChampionKilled {
                event_id: Some(1),
                event_time: Some(120.5),
                killer_name: Some("Ahri".to_string()),
                victim_name: Some("Jinx".to_string()),
                assisters: vec!["Lee Sin".to_string()],
                killer_is_ally: false,
                victim_is_ally: false,
                victim_is_local_player: false,
            }
        );
        assert!(matches!(events[1], DetectedEvent::TowerDestroyed { .. }));
        assert!(matches!(events[2], DetectedEvent::DragonTaken { .. }));
        assert!(matches!(events[3], DetectedEvent::BaronTaken { .. }));
        assert!(matches!(events[4], DetectedEvent::RiftHeraldTaken { .. }));
        assert!(matches!(events[5], DetectedEvent::TowerDestroyed { .. }));
        assert!(events[5].is_high_value_tower());
    }

    #[test]
    fn ignores_events_already_present_in_previous_snapshot() {
        let kill = json!({
            "EventID": 10,
            "EventName": "ChampionKill",
            "EventTime": 100.0,
            "KillerName": "Ahri",
            "VictimName": "Jinx"
        });
        let previous = snapshot(vec![kill.clone()]);
        let current = snapshot(vec![kill]);

        let events = detect_events(&previous, &current);

        assert!(events.is_empty());
    }

    #[test]
    fn falls_back_to_event_name_and_time_when_event_id_is_missing() {
        let previous = snapshot(vec![json!({
            "EventName": "DragonKill",
            "EventTime": 400.0,
            "KillerName": "Lee Sin"
        })]);
        let current = snapshot(vec![
            json!({
                "EventName": "DragonKill",
                "EventTime": 400.0,
                "KillerName": "Lee Sin"
            }),
            json!({
                "EventName": "DragonKill",
                "EventTime": 500.0,
                "KillerName": "Vi"
            }),
        ]);

        let events = detect_events(&previous, &current);

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DetectedEvent::DragonTaken { .. }));
    }

    #[test]
    fn annotates_friendly_kill_from_local_player_team() {
        let events = detect_kill("Ahri", "Jinx");
        assert_eq!(events.len(), 1);
        assert!(events[0].is_friendly_kill());
        assert!(!events[0].is_ally_death());
        assert!(!events[0].is_local_player_death());
    }

    #[test]
    fn annotates_ally_death() {
        let events = detect_kill("Jinx", "Leona");
        assert_eq!(events.len(), 1);
        assert!(events[0].is_ally_death());
        assert!(!events[0].is_friendly_kill());
        assert!(!events[0].is_local_player_death());
    }

    #[test]
    fn annotates_local_player_death() {
        let events = detect_kill("Jinx", "Ahri");
        assert_eq!(events.len(), 1);
        assert!(events[0].is_local_player_death());
        assert!(events[0].involves_player_team());
        assert!(!events[0].is_ally_death());
    }

    #[test]
    fn outer_tower_is_not_high_value() {
        let event = DetectedEvent::TowerDestroyed {
            event_id: Some(2),
            event_time: Some(260.0),
            killer_name: Some("Caitlyn".to_string()),
            turret_killed: Some("Turret_T1_L_03_A".to_string()),
            assisters: Vec::new(),
        };
        assert!(!event.is_high_value_tower());
    }

    #[test]
    fn inner_inhib_and_nexus_towers_are_high_value() {
        for turret in ["Turret_T1_L_02_A", "Turret_T1_L_01_A", "Turret_T1_C_01_A", "Barracks_T2_L1"] {
            let event = DetectedEvent::TowerDestroyed {
                event_id: Some(2),
                event_time: Some(260.0),
                killer_name: Some("Caitlyn".to_string()),
                turret_killed: Some(turret.to_string()),
                assisters: Vec::new(),
            };
            assert!(event.is_high_value_tower(), "{turret}");
        }
    }

    fn detect_kill(killer: &str, victim: &str) -> Vec<DetectedEvent> {
        let previous = team_snapshot(vec![json!({
            "EventID": 0,
            "EventName": "GameStart",
            "EventTime": 0.0
        })]);
        let current = team_snapshot(vec![
            json!({
                "EventID": 0,
                "EventName": "GameStart",
                "EventTime": 0.0
            }),
            json!({
                "EventID": 1,
                "EventName": "ChampionKill",
                "EventTime": 120.5,
                "KillerName": killer,
                "VictimName": victim,
                "Assisters": []
            }),
        ]);
        detect_events(&previous, &current)
    }

    fn team_snapshot(events: Vec<Value>) -> AllGameData {
        serde_json::from_value(json!({
            "activePlayer": {
                "summonerName": "Ahri"
            },
            "allPlayers": [
                {
                    "summonerName": "Ahri",
                    "championName": "Ahri",
                    "team": "ORDER"
                },
                {
                    "summonerName": "Leona",
                    "championName": "Leona",
                    "team": "ORDER"
                },
                {
                    "summonerName": "Jinx",
                    "championName": "Jinx",
                    "team": "CHAOS"
                }
            ],
            "events": {
                "Events": events
            },
            "gameData": {}
        }))
        .expect("valid allgamedata team snapshot")
    }

    fn snapshot(events: Vec<Value>) -> AllGameData {
        serde_json::from_value(json!({
            "activePlayer": {},
            "allPlayers": [],
            "events": {
                "Events": events
            },
            "gameData": {}
        }))
        .expect("valid allgamedata test snapshot")
    }
}
