use anyhow;
use mongodb::bson::doc;
use tokio;

pub mod bot;
mod init_db;
pub mod structs;
use crate::{bot::run_bot, init_db::init_db, structs::DBBot};
use futures::stream::TryStreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = init_db().await?;

    let bots_cursor = db.bots_coll.find(doc! {}).await?;
    let bots: Vec<DBBot> = bots_cursor.try_collect().await?;

    for bot in bots.into_iter() {
        tokio::spawn(async {
            match run_bot(bot.token).await {
                Err(e) => eprintln!("{}", e),
                _ => {}
            }
        });
    }

    tokio::signal::ctrl_c().await?;
    Ok(())
}
