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
        .filter_map(DetectedEvent::from_game_event)
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
    fn from_game_event(event: &GameEvent) -> Option<Self> {
        match event.event_name.as_str() {
            "ChampionKill" => Some(Self::ChampionKilled {
                event_id: event.event_id,
                event_time: event.event_time,
                killer_name: event.killer_name.clone(),
                victim_name: event.victim_name.clone(),
                assisters: event.assisters.clone(),
            }),
            "TurretKilled" => Some(Self::TowerDestroyed {
                event_id: event.event_id,
                event_time: event.event_time,
                killer_name: event.killer_name.clone(),
                turret_killed: event.turret_killed.clone(),
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

        assert_eq!(events.len(), 5);
        assert_eq!(
            events[0],
            DetectedEvent::ChampionKilled {
                event_id: Some(1),
                event_time: Some(120.5),
                killer_name: Some("Ahri".to_string()),
                victim_name: Some("Jinx".to_string()),
                assisters: vec!["Lee Sin".to_string()],
            }
        );
        assert!(matches!(events[1], DetectedEvent::TowerDestroyed { .. }));
        assert!(matches!(events[2], DetectedEvent::DragonTaken { .. }));
        assert!(matches!(events[3], DetectedEvent::BaronTaken { .. }));
        assert!(matches!(events[4], DetectedEvent::RiftHeraldTaken { .. }));
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
