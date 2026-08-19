use std::collections::HashMap;

use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AllGameData {
    #[serde(rename = "activePlayer")]
    pub active_player: ActivePlayer,

    #[serde(rename = "allPlayers")]
    pub all_players: Vec<Player>,

    pub events: EventData,

    #[serde(rename = "gameData")]
    pub game_data: GameData,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActivePlayer {
    #[serde(default)]
    pub abilities: Option<ChampionAbilities>,

    #[serde(rename = "championStats", default)]
    pub champion_stats: Option<ChampionStats>,

    #[serde(rename = "currentGold", default)]
    pub current_gold: Option<f64>,

    #[serde(rename = "fullRunes", default)]
    pub full_runes: Option<FullRunes>,

    #[serde(default)]
    pub level: Option<u32>,

    #[serde(rename = "summonerName", default)]
    pub summoner_name: Option<String>,

    #[serde(rename = "riotId", default)]
    pub riot_id: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChampionAbilities {
    #[serde(rename = "Passive", default)]
    pub passive: Option<Ability>,

    #[serde(rename = "Q", default)]
    pub q: Option<Ability>,

    #[serde(rename = "W", default)]
    pub w: Option<Ability>,

    #[serde(rename = "E", default)]
    pub e: Option<Ability>,

    #[serde(rename = "R", default)]
    pub r: Option<Ability>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ability {
    #[serde(rename = "abilityLevel", default)]
    pub ability_level: Option<u32>,

    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,

    #[serde(default)]
    pub id: Option<String>,

    #[serde(rename = "rawDescription", default)]
    pub raw_description: Option<String>,

    #[serde(rename = "rawDisplayName", default)]
    pub raw_display_name: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChampionStats {
    #[serde(rename = "abilityPower", default)]
    pub ability_power: Option<f64>,

    #[serde(default)]
    pub armor: Option<f64>,

    #[serde(rename = "attackDamage", default)]
    pub attack_damage: Option<f64>,

    #[serde(rename = "attackSpeed", default)]
    pub attack_speed: Option<f64>,

    #[serde(rename = "critChance", default)]
    pub crit_chance: Option<f64>,

    #[serde(rename = "currentHealth", default)]
    pub current_health: Option<f64>,

    #[serde(rename = "healthRegenRate", default)]
    pub health_regen_rate: Option<f64>,

    #[serde(rename = "lifeSteal", default)]
    pub life_steal: Option<f64>,

    #[serde(rename = "magicLethality", default)]
    pub magic_lethality: Option<f64>,

    #[serde(rename = "magicPenetrationFlat", default)]
    pub magic_penetration_flat: Option<f64>,

    #[serde(rename = "magicPenetrationPercent", default)]
    pub magic_penetration_percent: Option<f64>,

    #[serde(rename = "magicResist", default)]
    pub magic_resist: Option<f64>,

    #[serde(rename = "maxHealth", default)]
    pub max_health: Option<f64>,

    #[serde(rename = "moveSpeed", default)]
    pub move_speed: Option<f64>,

    #[serde(rename = "physicalLethality", default)]
    pub physical_lethality: Option<f64>,

    #[serde(rename = "resourceMax", default)]
    pub resource_max: Option<f64>,

    #[serde(rename = "resourceRegenRate", default)]
    pub resource_regen_rate: Option<f64>,

    #[serde(rename = "resourceType", default)]
    pub resource_type: Option<String>,

    #[serde(rename = "resourceValue", default)]
    pub resource_value: Option<f64>,

    #[serde(rename = "spellVamp", default)]
    pub spell_vamp: Option<f64>,

    #[serde(default)]
    pub tenacity: Option<f64>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FullRunes {
    #[serde(rename = "generalRunes", default)]
    pub general_runes: Vec<Rune>,

    #[serde(rename = "keystone", default)]
    pub keystone: Option<Rune>,

    #[serde(rename = "primaryRuneTree", default)]
    pub primary_rune_tree: Option<RuneTree>,

    #[serde(rename = "secondaryRuneTree", default)]
    pub secondary_rune_tree: Option<RuneTree>,

    #[serde(rename = "statRunes", default)]
    pub stat_runes: Vec<StatRune>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rune {
    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,

    #[serde(default)]
    pub id: Option<u32>,

    #[serde(rename = "rawDescription", default)]
    pub raw_description: Option<String>,

    #[serde(rename = "rawDisplayName", default)]
    pub raw_display_name: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuneTree {
    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,

    #[serde(default)]
    pub id: Option<u32>,

    #[serde(rename = "rawDescription", default)]
    pub raw_description: Option<String>,

    #[serde(rename = "rawDisplayName", default)]
    pub raw_display_name: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StatRune {
    #[serde(default)]
    pub id: Option<u32>,

    #[serde(rename = "rawDescription", default)]
    pub raw_description: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Player {
    #[serde(rename = "championName", default)]
    pub champion_name: Option<String>,

    #[serde(rename = "isBot", default)]
    pub is_bot: Option<bool>,

    #[serde(rename = "isDead", default)]
    pub is_dead: Option<bool>,

    #[serde(default, deserialize_with = "deserialize_items")]
    pub items: Vec<Item>,

    #[serde(default)]
    pub level: Option<u32>,

    #[serde(default)]
    pub position: Option<String>,

    #[serde(rename = "rawChampionName", default)]
    pub raw_champion_name: Option<String>,

    #[serde(rename = "respawnTimer", default)]
    pub respawn_timer: Option<f64>,

    #[serde(rename = "riotId", default)]
    pub riot_id: Option<String>,

    #[serde(default)]
    pub scores: Option<PlayerScores>,

    #[serde(rename = "skinID", default)]
    pub skin_id: Option<u32>,

    #[serde(rename = "summonerName", default)]
    pub summoner_name: Option<String>,

    #[serde(rename = "summonerSpells", default)]
    pub summoner_spells: Option<SummonerSpells>,

    #[serde(default)]
    pub team: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
    #[serde(rename = "canUse", default)]
    pub can_use: Option<bool>,

    #[serde(rename = "consumable", default)]
    pub consumable: Option<bool>,

    #[serde(default)]
    pub count: Option<u32>,

    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,

    #[serde(rename = "itemID", default)]
    pub item_id: Option<u32>,

    #[serde(default)]
    pub price: Option<u32>,

    #[serde(rename = "rawDescription", default)]
    pub raw_description: Option<String>,

    #[serde(rename = "rawDisplayName", default)]
    pub raw_display_name: Option<String>,

    #[serde(default)]
    pub slot: Option<u32>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerScores {
    #[serde(default)]
    pub assists: Option<u32>,

    #[serde(rename = "creepScore", default)]
    pub creep_score: Option<u32>,

    #[serde(default)]
    pub deaths: Option<u32>,

    #[serde(default)]
    pub kills: Option<u32>,

    #[serde(rename = "wardScore", default)]
    pub ward_score: Option<f64>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SummonerSpells {
    #[serde(rename = "summonerSpellOne", default)]
    pub summoner_spell_one: Option<SummonerSpell>,

    #[serde(rename = "summonerSpellTwo", default)]
    pub summoner_spell_two: Option<SummonerSpell>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SummonerSpell {
    #[serde(rename = "displayName", default)]
    pub display_name: Option<String>,

    #[serde(rename = "rawDescription", default)]
    pub raw_description: Option<String>,

    #[serde(rename = "rawDisplayName", default)]
    pub raw_display_name: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventData {
    #[serde(rename = "Events", default)]
    pub events: Vec<GameEvent>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameEvent {
    #[serde(rename = "EventID", default)]
    pub event_id: Option<u32>,

    #[serde(rename = "EventName")]
    pub event_name: String,

    #[serde(rename = "EventTime", default)]
    pub event_time: Option<f64>,

    #[serde(rename = "KillerName", default)]
    pub killer_name: Option<String>,

    #[serde(rename = "VictimName", default)]
    pub victim_name: Option<String>,

    #[serde(rename = "Assisters", default)]
    pub assisters: Vec<String>,

    #[serde(rename = "TurretKilled", default)]
    pub turret_killed: Option<String>,

    #[serde(rename = "InhibKilled", default)]
    pub inhib_killed: Option<String>,

    #[serde(rename = "DragonType", default)]
    pub dragon_type: Option<String>,

    #[serde(rename = "Stolen", default)]
    pub stolen: Option<String>,

    #[serde(rename = "Acer", default)]
    pub acer: Option<String>,

    #[serde(rename = "AcingTeam", default)]
    pub acing_team: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameData {
    #[serde(rename = "gameMode", default)]
    pub game_mode: Option<String>,

    #[serde(rename = "gameTime", default)]
    pub game_time: Option<f64>,

    #[serde(rename = "mapName", default)]
    pub map_name: Option<String>,

    #[serde(rename = "mapNumber", default)]
    pub map_number: Option<u32>,

    #[serde(rename = "mapTerrain", default)]
    pub map_terrain: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

fn deserialize_items<'de, D>(deserializer: D) -> Result<Vec<Item>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;

    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => Vec::<Item>::deserialize(Value::Array(items)).map_err(D::Error::custom),
        Some(Value::Object(_)) => Ok(Vec::new()),
        Some(other) => Err(D::Error::custom(format!(
            "expected items to be an array or error object, got {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_minimal_all_game_data() {
        let payload = serde_json::json!({
            "activePlayer": {
                "summonerName": "Player",
                "level": 7,
                "currentGold": 850.5
            },
            "allPlayers": [
                {
                    "summonerName": "Player",
                    "riotId": "Player#NA1",
                    "championName": "Ahri",
                    "team": "ORDER",
                    "scores": {
                        "kills": 1,
                        "deaths": 0,
                        "assists": 2,
                        "creepScore": 80
                    }
                }
            ],
            "events": {
                "Events": [
                    {
                        "EventID": 0,
                        "EventName": "GameStart",
                        "EventTime": 0.0
                    }
                ]
            },
            "gameData": {
                "gameMode": "CLASSIC",
                "gameTime": 420.0,
                "mapName": "Map11",
                "mapNumber": 11
            }
        });

        let data: AllGameData = serde_json::from_value(payload).expect("valid allgamedata");

        assert_eq!(data.active_player.summoner_name.as_deref(), Some("Player"));
        assert_eq!(data.all_players.len(), 1);
        assert_eq!(data.events.events[0].event_name, "GameStart");
        assert_eq!(data.game_data.game_time, Some(420.0));
    }

    #[test]
    fn deserializes_player_items_error_object_as_empty_items() {
        let payload = serde_json::json!({
            "activePlayer": {},
            "allPlayers": [
                {
                    "summonerName": "Unknown",
                    "team": "CHAOS",
                    "items": {
                        "error": "Unable to find player"
                    }
                }
            ],
            "events": {
                "Events": []
            },
            "gameData": {}
        });

        let data: AllGameData = serde_json::from_value(payload).expect("valid allgamedata");

        assert_eq!(data.all_players.len(), 1);
        assert!(data.all_players[0].items.is_empty());
    }
}
