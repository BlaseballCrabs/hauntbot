use anyhow::{ensure, Context, Result};
use async_std::task::sleep;
use chrono::prelude::*;
use db::Database;
use futures::prelude::*;
use log::*;
use serde::Serialize;
use std::collections::HashSet;
use std::time::Duration;

pub mod db;
pub mod hauntings;
pub mod logger;
pub mod oauth_listener;

#[derive(Debug, Serialize)]
pub struct Footer {
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct Embed {
    pub title: String,
    pub footer: Footer,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct WebhookPayload<'a> {
    pub embeds: &'a [Embed],
}

async fn send_message(db: &Database, url: &str, embeds: &[Embed]) -> Result<()> {
    let hook = WebhookPayload { embeds };

    let resp = surf::post(url)
        .body(surf::Body::from_json(&hook).map_err(|x| x.into_inner())?)
        .send()
        .await
        .map_err(|x| x.into_inner())?;

    let remaining: usize = resp
        .header("X-RateLimit-Remaining")
        .map(|x| x.last().as_str().parse())
        .transpose()?
        .unwrap_or(1);

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

async fn send_messages(db: &Database, embeds: &[Embed]) -> Result<()> {
    db.webhooks()
        .try_for_each(|hook| async move {
            send_message(db, &hook.url, embeds).await?;
            Ok(())
        })
        .await?;
    Ok(())
}

pub fn watch(db: &Database) -> impl Future<Output = Result<()>> {
    let db = db.clone();
    let mut time = Utc::now();

    async move {
        loop {
            debug!("fetching hauntings from db...");
            let known: HashSet<_> = db.haunting_uuids().try_collect().await?;

            let mut message = Vec::new();

            debug!("fetching hauntings from feed...");

            let hauntings = match hauntings::hauntings(time - chrono::Duration::hours(6)).await {
                Ok(hauntings) => hauntings,
                Err(err) => {
                    error!("error fetching hauntings: {:?}", err);
                    sleep(Duration::from_secs(30)).await;
                    continue;
                }
            };

            for found in hauntings {
                debug!("checking {:?}", found.id);

                if known.contains(&found.id) {
                    debug!("already seen");
                    continue;
                }

                info!("{}", found.description);

                message.push(Embed {
                    title: found.description,
                    footer: Footer {
                        text: format!("Season {} Day {}", found.season + 1, found.day + 1),
                    },
                    timestamp: found.created,
                });

                debug!("adding {:?} to db", found.id);
                db.add_haunting(&found.id).await?;

                if message.len() >= 10 {
                    debug!("hit max embeds, sending early");
                    send_messages(&db, &message).await?;
                    message.clear();
                }
            }

            if message.is_empty() {
                debug!("no hauntings found");
            } else {
                send_messages(&db, &message).await?;
            }

            time = Utc::now();

            debug!("sleeping...");
            sleep(Duration::from_secs(5)).await;
        }
    }
}
