use anyhow::Result;
use async_std::prelude::*;
use async_std::task;
use hauntbot::{
    db::Database,
    logger,
    oauth_listener::{self, OAuth},
    watch,
};
use log::*;

#[async_std::main]
async fn main() -> Result<()> {
    logger::init()?;

    let db_uri = dotenv::var("DATABASE_URL")?;

    let db = Database::connect(&db_uri).await?;
    debug!("Connected to database");

    let redirect_uri = dotenv::var("REDIRECT_URI")?;
    let client_id = dotenv::var("CLIENT_ID")?;
    let client_secret = dotenv::var("CLIENT_SECRET")?;

    let manual_webhook_urls = dotenv::var("WEBHOOK_URL");
    db.add_urls(
        manual_webhook_urls
            .as_deref()
            .unwrap_or("")
            .split(',')
            .map(|x| x.trim())
            .filter(|x| !x.is_empty()),
    )
    .await?;

    let bot = task::spawn(watch(&db));
    let listener = task::spawn(oauth_listener::listen(
        &db,
        OAuth {
            redirect_uri,
            client_id,
            client_secret,
        },
    ));
    bot.race(listener).await?;

    Ok(())
}
