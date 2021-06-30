use anyhow::{anyhow, Result};
use chrono::prelude::*;
use serde::{ser::*, Deserialize, Serialize};
use std::fmt;
use std::result::Result as StdResult;

#[derive(Serialize, Deserialize)]
pub struct HauntingMetadata {
    #[serde(default, rename = "mod")]
    pub modification: String,
}

impl HauntingMetadata {
    pub fn serialize_params<S>(&self, serializer: S) -> StdResult<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("HauntingMetadata", 1)?;
        state.serialize_field("metadata.mod", &self.modification)?;
        state.end()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub description: String,
    pub created: String,
    pub season: isize,
    pub day: isize,
    pub metadata: HauntingMetadata,
    pub player_tags: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerData {
    pub deceased: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    pub first_seen: DateTime<Utc>,
    pub data: PlayerData,
}

#[derive(Copy, Clone)]
pub enum GhostCategory {
    UltraLeague,
    InternetLeague,
    Unknown,
}

impl fmt::Display for GhostCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UltraLeague => write!(f, "Ultra League Blaseball"),
            Self::InternetLeague => write!(f, "Internet League Blaseball"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

pub async fn hauntings() -> Result<Vec<Event>> {
    #[derive(Serialize)]
    struct Query {
        pub limit: usize,

        #[serde(rename = "sortorder")]
        pub sort_order: &'static str,

        #[serde(rename = "sortby")]
        pub sort_by: &'static str,

        #[serde(rename = "type")]
        pub event_type: usize,

        #[serde(flatten, serialize_with = "HauntingMetadata::serialize_params")]
        pub metadata: HauntingMetadata,
    }

    let res = surf::get("https://api.sibr.dev/eventually/v2/events")
        .query(&Query {
            limit: 100,
            sort_order: "desc",
            sort_by: "{created}",
            event_type: 106,
            metadata: HauntingMetadata {
                modification: "INHABITING".to_string(),
            },
        })
        .map_err(|x| anyhow!(x))?
        .await
        .map_err(|x| anyhow!(x))?
        .body_json()
        .await
        .map_err(|x| anyhow!(x))?;

    Ok(res)
}

pub async fn oldest_version(player: &str) -> Result<Option<Player>> {
    #[derive(Serialize)]
    struct Query<'a> {
        pub order: &'static str,
        pub player: &'a str,
        pub limit: usize,
    }

    #[derive(Deserialize)]
    struct Response {
        pub data: Vec<Player>,
    }

    let res: Response = surf::get("https://api.sibr.dev/chronicler/v1/players/updates")
        .query(&Query {
            order: "asc",
            player,
            limit: 1,
        })
        .map_err(|x| anyhow!(x))?
        .await
        .map_err(|x| anyhow!(x))?
        .body_json()
        .await
        .map_err(|x| anyhow!(x))?;

    Ok(res.data.into_iter().next())
}

pub async fn categorize(player: &str) -> Result<GhostCategory> {
    let oldest = oldest_version(player).await?;
    let expansion_era = Utc.ymd(2021, 3, 1).and_hms(0, 0, 0);

    Ok(match oldest {
        Some(Player {
            first_seen,
            data: PlayerData { deceased: true },
        }) if first_seen > expansion_era => GhostCategory::UltraLeague,
        Some(_) => GhostCategory::InternetLeague,
        None => GhostCategory::Unknown,
    })
}
