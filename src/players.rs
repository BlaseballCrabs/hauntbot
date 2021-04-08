use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

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

pub async fn players() -> Result<Vec<Player>> {
    #[derive(Deserialize)]
    struct Response {
        pub data: Vec<Player>,
    }

    let resp: Response = surf::get("https://api.sibr.dev/chronicler/v1/players")
        .await
        .map_err(|x| anyhow!(x))?
        .body_json()
        .await
        .map_err(|x| anyhow!(x))?;

    Ok(resp.data)
}

pub async fn redacted() -> Result<impl Iterator<Item = PlayerData>> {
    Ok(players()
        .await?
        .into_iter()
        .map(|x| x.data)
        .filter(|x| x.perm_attrs.iter().any(|y| y == "REDACTED")))
}
