use anyhow::{ensure, Context, Result};
use async_std::task::sleep;
use db::Database;
use futures::prelude::*;
use log::*;
use serde::Serialize;
use std::collections::HashSet;
use std::fmt::Write;
use std::time::Duration;

pub mod db;
pub mod logger;
pub mod oauth_listener;
pub mod players;

#[derive(Debug, Serialize)]
pub struct WebhookPayload<'a> {
    pub content: &'a str,
    pub avatar_url: &'static str,
}

async fn send_message(db: &Database, url: &str, content: &str) -> Result<()> {
    let hook = WebhookPayload {
        content,
        avatar_url: "http://hs.hiveswap.com/ezodiac/images/aspect_8.png",
    };

    let resp = surf::post(url)
        .body(surf::Body::from_json(&hook).map_err(|x| x.into_inner())?)
        .send()
        .await
        .map_err(|x| x.into_inner())?;

    let remaining: usize = resp
        .header("X-RateLimit-Remaining")
        .context("missing remaining requests")?
        .last()
        .as_str()
        .parse()?;

    debug!("{} requests left", remaining);

    if remaining == 0 {
        let time: f64 = resp
            .header("X-RateLimit-Reset-After")
            .context("missing remaining time")?
            .last()
            .as_str()
            .parse()?;

        debug!("sleeping for {}s...", time);

        sleep(Duration::from_secs_f64(time)).await;

        debug!("slept");
    }

    let status = resp.status();
    if status == surf::StatusCode::NotFound {
        debug!("webhook removed, deleting from database");
        db.remove_url(url).await?;
    } else {
        ensure!(status.is_success(), "Couldn't send webhook: {}", status);
    }

    Ok(())
}

async fn send_messages(db: &Database, content: &str) -> Result<()> {
    db.webhooks()
        .try_for_each_concurrent(None, |hook| async move {
            send_message(db, &hook.url, content).await?;
            Ok(())
        })
        .await?;
    Ok(())
}

pub fn watch(db: &Database) -> impl Future<Output = Result<()>> {
    let db = db.clone();

    async move {
        loop {
            debug!("fetching players from db...");
            let known: HashSet<_> = db.player_uuids().try_collect().await?;

            let mut message = "".to_string();

            debug!("fetching redacted players...");
            for found in players::redacted().await? {
                debug!("checking {} ({:?})", found.name, found.id);

                if known.contains(&found.id) {
                    debug!("already seen");
                    continue;
                }

                writeln!(message, "{} is Redacted!", found.name)?;

                debug!("adding {:?} to db", found.id);
                db.add_player(&found.id).await?;
            }

            if message.is_empty() {
                debug!("no players found");
            } else {
                info!("{}", message);
                send_messages(&db, &message).await?;
            }

            debug!("sleeping...");
            sleep(Duration::from_secs(60)).await;
        }
    }
}
