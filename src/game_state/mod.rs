use serde::{Deserialize, Serialize};

use crate::{
    event_engine::DetectedEvent,
    riot_live_client::{AllGameData, GameEvent, Player},
};

const BARON_BUFF_DURATION_SECONDS: f64 = 180.0;
const ELDER_BUFF_DURATION_SECONDS: f64 = 150.0;
const DRAGON_SOUL_KILL_COUNT: u32 = 4;

pub fn build_game_state(
    all_game_data: &AllGameData,
    detected_events: &[DetectedEvent],
) -> GameState {
    let game_time = all_game_data.game_data.game_time.unwrap_or_default();

    GameState {
        gold_advantage: build_gold_advantage(all_game_data),
        objective_control: build_objective_control(all_game_data),
        team_fight_status: build_team_fight_status(detected_events),
        alive_champions: build_alive_champions(all_game_data),
        baron_buff: build_baron_buff(all_game_data, game_time),
        dragon_soul: build_dragon_soul(all_game_data),
        elder_buff: build_elder_buff(all_game_data, game_time),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameState {
    pub gold_advantage: GoldAdvantage,
    pub objective_control: ObjectiveControl,
    pub team_fight_status: TeamFightStatus,
    pub alive_champions: AliveChampions,
    pub baron_buff: TimedTeamBuff,
    pub dragon_soul: DragonSoul,
    pub elder_buff: TimedTeamBuff,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GoldAdvantage {
    pub order_visible_item_gold: u32,
    pub chaos_visible_item_gold: u32,
    pub difference_order_minus_chaos: i32,
    pub leading_team: Option<Team>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectiveControl {
    pub dragons_taken: TeamObjectiveCount,
    pub barons_taken: TeamObjectiveCount,
    pub rift_heralds_taken: TeamObjectiveCount,
    pub towers_destroyed: TeamObjectiveCount,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TeamObjectiveCount {
    pub order: u32,
    pub chaos: u32,
    pub unknown: u32,
}

impl TeamObjectiveCount {
    fn increment(&mut self, team: Team) {
        match team {
            Team::Order => self.order += 1,
            Team::Chaos => self.chaos += 1,
            Team::Unknown => self.unknown += 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status")]
pub enum TeamFightStatus {
    Quiet,
    RecentKills { champion_kills: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AliveChampions {
    pub order: Vec<AliveChampion>,
    pub chaos: Vec<AliveChampion>,
    pub unknown: Vec<AliveChampion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AliveChampion {
    pub riot_id: Option<String>,
    pub summoner_name: Option<String>,
    pub champion_name: Option<String>,
    pub team: Team,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimedTeamBuff {
    pub holder: Option<Team>,
    pub remaining_seconds: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DragonSoul {
    pub holder: Option<Team>,
    pub dragon_type: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub enum Team {
    Order,
    Chaos,
    Unknown,
}

fn build_gold_advantage(all_game_data: &AllGameData) -> GoldAdvantage {
    let mut order_visible_item_gold = 0;
    let mut chaos_visible_item_gold = 0;

    for player in &all_game_data.all_players {
        let player_item_gold = player
            .items
            .iter()
            .map(|item| item.price.unwrap_or_default() * item.count.unwrap_or(1))
            .sum::<u32>();

        match player_team(player) {
            Team::Order => order_visible_item_gold += player_item_gold,
            Team::Chaos => chaos_visible_item_gold += player_item_gold,
            Team::Unknown => {}
        }
    }

    let difference_order_minus_chaos = order_visible_item_gold as i32 - chaos_visible_item_gold as i32;
    let leading_team = if difference_order_minus_chaos > 0 {
        Some(Team::Order)
    } else if difference_order_minus_chaos < 0 {
        Some(Team::Chaos)
    } else {
        None
    };

    GoldAdvantage {
        order_visible_item_gold,
        chaos_visible_item_gold,
        difference_order_minus_chaos,
        leading_team,
    }
}

fn build_objective_control(all_game_data: &AllGameData) -> ObjectiveControl {
    let mut objective_control = ObjectiveControl {
        dragons_taken: TeamObjectiveCount {
            order: 0,
            chaos: 0,
            unknown: 0,
        },
        barons_taken: TeamObjectiveCount {
            order: 0,
            chaos: 0,
            unknown: 0,
        },
        rift_heralds_taken: TeamObjectiveCount {
            order: 0,
            chaos: 0,
            unknown: 0,
        },
        towers_destroyed: TeamObjectiveCount {
            order: 0,
            chaos: 0,
            unknown: 0,
        },
    };

    for event in &all_game_data.events.events {
        let team = event_team(all_game_data, event);

        match event.event_name.as_str() {
            "DragonKill" => objective_control.dragons_taken.increment(team),
            "BaronKill" => objective_control.barons_taken.increment(team),
            "HeraldKill" => objective_control.rift_heralds_taken.increment(team),
            "TurretKilled" => objective_control.towers_destroyed.increment(team),
            _ => {}
        }
    }

    objective_control
}

fn build_team_fight_status(detected_events: &[DetectedEvent]) -> TeamFightStatus {
    let champion_kills = detected_events
        .iter()
        .filter(|event| matches!(event, DetectedEvent::ChampionKilled { .. }))
        .count() as u32;

    if champion_kills > 0 {
        TeamFightStatus::RecentKills { champion_kills }
    } else {
        TeamFightStatus::Quiet
    }
}

fn build_alive_champions(all_game_data: &AllGameData) -> AliveChampions {
    let mut alive_champions = AliveChampions {
        order: Vec::new(),
        chaos: Vec::new(),
        unknown: Vec::new(),
    };

    for player in &all_game_data.all_players {
        if player.is_dead != Some(false) {
            continue;
        }

        let team = player_team(player);
        let alive_champion = AliveChampion {
            riot_id: player.riot_id.clone(),
            summoner_name: player.summoner_name.clone(),
            champion_name: player.champion_name.clone(),
            team,
        };

        match team {
            Team::Order => alive_champions.order.push(alive_champion),
            Team::Chaos => alive_champions.chaos.push(alive_champion),
            Team::Unknown => alive_champions.unknown.push(alive_champion),
        }
    }

    alive_champions
}

fn build_baron_buff(all_game_data: &AllGameData, game_time: f64) -> TimedTeamBuff {
    build_latest_timed_buff(
        all_game_data,
        game_time,
        "BaronKill",
        BARON_BUFF_DURATION_SECONDS,
        |_| true,
    )
}

fn build_elder_buff(all_game_data: &AllGameData, game_time: f64) -> TimedTeamBuff {
    build_latest_timed_buff(
        all_game_data,
        game_time,
        "DragonKill",
        ELDER_BUFF_DURATION_SECONDS,
        |event| event.dragon_type.as_deref().is_some_and(is_elder_dragon),
    )
}

fn build_latest_timed_buff(
    all_game_data: &AllGameData,
    game_time: f64,
    event_name: &str,
    duration_seconds: f64,
    predicate: impl Fn(&GameEvent) -> bool,
) -> TimedTeamBuff {
    all_game_data
        .events
        .events
        .iter()
        .filter(|event| event.event_name == event_name && predicate(event))
        .filter_map(|event| {
            let event_time = event.event_time?;
            let remaining_seconds = duration_seconds - (game_time - event_time);
            (remaining_seconds > 0.0).then(|| TimedTeamBuff {
                holder: Some(event_team(all_game_data, event)),
                remaining_seconds: Some(remaining_seconds),
            })
        })
        .last()
        .unwrap_or(TimedTeamBuff {
            holder: None,
            remaining_seconds: None,
        })
}

fn build_dragon_soul(all_game_data: &AllGameData) -> DragonSoul {
    let mut order_dragon_kills = 0;
    let mut chaos_dragon_kills = 0;
    let mut order_last_dragon_type = None;
    let mut chaos_last_dragon_type = None;

    for event in &all_game_data.events.events {
        if event.event_name != "DragonKill" {
            continue;
        }

        if event.dragon_type.as_deref().is_some_and(is_elder_dragon) {
            continue;
        }

        match event_team(all_game_data, event) {
            Team::Order => {
                order_dragon_kills += 1;
                order_last_dragon_type = event.dragon_type.clone();
            }
            Team::Chaos => {
                chaos_dragon_kills += 1;
                chaos_last_dragon_type = event.dragon_type.clone();
            }
            Team::Unknown => {}
        }
    }

    if order_dragon_kills >= DRAGON_SOUL_KILL_COUNT {
        DragonSoul {
            holder: Some(Team::Order),
            dragon_type: order_last_dragon_type,
        }
    } else if chaos_dragon_kills >= DRAGON_SOUL_KILL_COUNT {
        DragonSoul {
            holder: Some(Team::Chaos),
            dragon_type: chaos_last_dragon_type,
        }
    } else {
        DragonSoul {
            holder: None,
            dragon_type: None,
        }
    }
}

fn event_team(all_game_data: &AllGameData, event: &GameEvent) -> Team {
    event
        .killer_name
        .as_deref()
        .and_then(|name| find_player_team_by_name(all_game_data, name))
        .unwrap_or(Team::Unknown)
}

fn find_player_team_by_name(all_game_data: &AllGameData, name: &str) -> Option<Team> {
    let normalized_name = normalize_name(name);

    all_game_data.all_players.iter().find_map(|player| {
        let matches_player = player
            .summoner_name
            .as_deref()
            .is_some_and(|candidate| normalize_name(candidate) == normalized_name)
            || player
                .riot_id
                .as_deref()
                .is_some_and(|candidate| riot_id_matches(candidate, &normalized_name))
            || player
                .champion_name
                .as_deref()
                .is_some_and(|candidate| normalize_name(candidate) == normalized_name);

        matches_player.then(|| player_team(player))
    })
}

fn riot_id_matches(candidate: &str, normalized_name: &str) -> bool {
    let normalized_candidate = normalize_name(candidate);

    normalized_candidate == normalized_name
        || normalized_candidate
            .split_once('#')
            .is_some_and(|(game_name, _)| game_name == normalized_name)
}

fn player_team(player: &Player) -> Team {
    match player
        .team
        .as_deref()
        .map(|team| team.trim().to_ascii_uppercase())
    {
        Some(team) if team == "ORDER" => Team::Order,
        Some(team) if team == "CHAOS" => Team::Chaos,
        _ => Team::Unknown,
    }
}

fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn is_elder_dragon(dragon_type: &str) -> bool {
    dragon_type.eq_ignore_ascii_case("elder")
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;

    #[test]
    fn builds_gold_advantage_and_alive_champions() {
        let all_game_data = snapshot(
            600.0,
            vec![
                player("Order Carry", "Ahri", "ORDER", false, vec![(1000, 1)]),
                player("Order Dead", "Garen", "ORDER", true, vec![(500, 1)]),
                player("Chaos Carry", "Jinx", "CHAOS", false, vec![(700, 1)]),
            ],
            vec![],
        );

        let state = build_game_state(&all_game_data, &[]);

        assert_eq!(state.gold_advantage.order_visible_item_gold, 1500);
        assert_eq!(state.gold_advantage.chaos_visible_item_gold, 700);
        assert_eq!(state.gold_advantage.difference_order_minus_chaos, 800);
        assert_eq!(state.gold_advantage.leading_team, Some(Team::Order));
        assert_eq!(state.alive_champions.order.len(), 1);
        assert_eq!(state.alive_champions.chaos.len(), 1);
        assert_eq!(state.alive_champions.unknown.len(), 0);
    }

    #[test]
    fn builds_objectives_buffs_and_dragon_soul() {
        let all_game_data = snapshot(
            1_000.0,
            vec![
                player("Order Jungle", "Lee Sin", "ORDER", false, vec![]),
                player("Chaos Jungle", "Vi", "CHAOS", false, vec![]),
            ],
            vec![
                dragon(1, 100.0, "Order Jungle", "Fire"),
                dragon(2, 200.0, "Order Jungle", "Fire"),
                dragon(3, 300.0, "Order Jungle", "Fire"),
                dragon(4, 400.0, "Order Jungle", "Fire"),
                json!({
                    "EventID": 5,
                    "EventName": "BaronKill",
                    "EventTime": 900.0,
                    "KillerName": "Chaos Jungle"
                }),
                dragon(6, 950.0, "Order Jungle", "Elder"),
                json!({
                    "EventID": 7,
                    "EventName": "HeraldKill",
                    "EventTime": 500.0,
                    "KillerName": "Chaos Jungle"
                }),
                json!({
                    "EventID": 8,
                    "EventName": "TurretKilled",
                    "EventTime": 650.0,
                    "KillerName": "Order Jungle",
                    "TurretKilled": "Turret_T2_C_03_A"
                }),
            ],
        );

        let state = build_game_state(&all_game_data, &[]);

        assert_eq!(state.objective_control.dragons_taken.order, 5);
        assert_eq!(state.objective_control.barons_taken.chaos, 1);
        assert_eq!(state.objective_control.rift_heralds_taken.chaos, 1);
        assert_eq!(state.objective_control.towers_destroyed.order, 1);
        assert_eq!(state.dragon_soul.holder, Some(Team::Order));
        assert_eq!(state.dragon_soul.dragon_type.as_deref(), Some("Fire"));
        assert_eq!(state.baron_buff.holder, Some(Team::Chaos));
        assert_eq!(state.baron_buff.remaining_seconds, Some(80.0));
        assert_eq!(state.elder_buff.holder, Some(Team::Order));
        assert_eq!(state.elder_buff.remaining_seconds, Some(100.0));
    }

    #[test]
    fn uses_detected_events_for_team_fight_status() {
        let all_game_data = snapshot(300.0, vec![], vec![]);
        let detected_events = vec![
            DetectedEvent::ChampionKilled {
                event_id: Some(1),
                event_time: Some(100.0),
                killer_name: Some("Ahri".to_string()),
                victim_name: Some("Jinx".to_string()),
                assisters: vec![],
            },
            DetectedEvent::TowerDestroyed {
                event_id: Some(2),
                event_time: Some(150.0),
                killer_name: Some("Ahri".to_string()),
                turret_killed: Some("Turret_T2_C_03_A".to_string()),
                assisters: vec![],
            },
        ];

        let state = build_game_state(&all_game_data, &detected_events);

        assert_eq!(
            state.team_fight_status,
            TeamFightStatus::RecentKills { champion_kills: 1 }
        );
    }

    fn snapshot(game_time: f64, players: Vec<Value>, events: Vec<Value>) -> AllGameData {
        serde_json::from_value(json!({
            "activePlayer": {},
            "allPlayers": players,
            "events": {
                "Events": events
            },
            "gameData": {
                "gameTime": game_time
            }
        }))
        .expect("valid allgamedata test snapshot")
    }

    fn player(
        summoner_name: &str,
        champion_name: &str,
        team: &str,
        is_dead: bool,
        items: Vec<(u32, u32)>,
    ) -> Value {
        json!({
            "summonerName": summoner_name,
            "riotId": format!("{summoner_name}#TEST"),
            "championName": champion_name,
            "team": team,
            "isDead": is_dead,
            "items": items
                .into_iter()
                .enumerate()
                .map(|(slot, (price, count))| {
                    json!({
                        "slot": slot,
                        "price": price,
                        "count": count
                    })
                })
                .collect::<Vec<_>>()
        })
    }

    fn dragon(event_id: u32, event_time: f64, killer_name: &str, dragon_type: &str) -> Value {
        json!({
            "EventID": event_id,
            "EventName": "DragonKill",
            "EventTime": event_time,
            "KillerName": killer_name,
            "DragonType": dragon_type
        })
    }
}
