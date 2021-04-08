use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct HauntingMetadata {
    #[serde(rename = "mod")]
    pub modification: String,
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

pub async fn events() -> Result<Vec<Event>> {
    Ok(
        surf::get("https://www.blaseball.com/database/feed/global?type=106&limit=100")
            .await
            .map_err(|x| anyhow!(x))?
            .body_json()
            .await
            .map_err(|x| anyhow!(x))?,
    )
}

pub async fn hauntings() -> Result<impl Iterator<Item = Event>> {
    Ok(events()
        .await?
        .into_iter()
        .filter(|x| x.metadata.modification == "INHABITING"))
}
