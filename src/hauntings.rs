use anyhow::{anyhow, Result};
use chrono::prelude::*;
use serde::{ser::*, Deserialize, Serialize};
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
pub struct Event {
    pub id: String,
    pub description: String,
    pub created: String,
    pub season: isize,
    pub day: isize,
    pub metadata: HauntingMetadata,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerData {
    pub id: String,
    pub name: String,

    #[serde(default, rename = "permAttr")]
    pub perm_attrs: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Player {
    pub data: PlayerData,
}

pub async fn hauntings(after: DateTime<Utc>) -> Result<Vec<Event>> {
    #[derive(Serialize)]
    struct Query {
        pub after: DateTime<Utc>,
        pub limit: usize,

        #[serde(rename = "type")]
        pub event_type: usize,

        #[serde(flatten, serialize_with = "HauntingMetadata::serialize_params")]
        pub metadata: HauntingMetadata,
    }

    let res = surf::get("https://api.sibr.dev/eventually/v2/events")
        .query(&Query {
            after,
            metadata: HauntingMetadata {
                modification: "INHABITING".to_string(),
            },
            limit: 100,
            event_type: 106,
        })
        .map_err(|x| anyhow!(x))?
        .await
        .map_err(|x| anyhow!(x))?
        .body_json()
        .await
        .map_err(|x| anyhow!(x))?;

    Ok(res)
}
