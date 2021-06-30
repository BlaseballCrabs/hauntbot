use anyhow::{ensure, Context, Result};
use async_std::task::sleep;
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

    async move {
        loop {
            let mut message = Vec::new();

            debug!("fetching data...");
            let (known, hauntings): (Result<HashSet<_>>, _) =
                futures::join!(db.haunting_uuids().try_collect(), hauntings::hauntings());

            let known = known?;

            let hauntings = match hauntings {
                Ok(hauntings) => hauntings,
                Err(err) => {
                    error!("error fetching hauntings: {:?}", err);
                    sleep(Duration::from_secs(30)).await;
                    continue;
                }
            };

            let mut seen = 0;

            for found in hauntings {
                if known.contains(&found.id) {
                    seen += 1;
                    continue;
                }

                info!("{}: {}", found.id, found.description);

                let category = hauntings::categorize(&found.player_tags[0]).await?;

                message.push(Embed {
                    title: found.description,
                    footer: Footer {
                        text: format!(
                            "Season {} Day {} • Origin: {}",
                            found.season + 1,
                            found.day + 1,
                            category
                        ),
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
                debug!("{} already seen, sleeping...", seen);
            } else {
                if seen > 0 {
                    debug!("+ {} already seen", seen);
                }

                send_messages(&db, &message).await?;
                debug!("sleeping...");
            }

            sleep(Duration::from_secs(5)).await;
        }
    }
}
