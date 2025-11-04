use anyhow;
use mongodb::bson::doc;
use teloxide::{Bot, prelude::Requester};
use tokio;

pub mod bot;
mod init_db;
pub mod structs;
use crate::{bot::run_bot, init_db::init_db, structs::DBBot};
use futures::stream::TryStreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = init_db().await?;

    let bots_cursor = db.bots_coll.find(doc! {"is_active": true}).await?;
    let bots: Vec<DBBot> = bots_cursor.try_collect().await?;

    for bot in bots.into_iter() {
        let db = db.clone();
        match Bot::new(&bot.token).get_me().await {
            Ok(_) => {
                tokio::spawn(async {
                    match run_bot(bot.token, db).await {
                        Err(e) => eprintln!("{}", e),
                        _ => {}
                    }
                });
            }
            Err(e) => {
                eprintln!("Error while starting bot: {}", e);
                db.bots_coll
                    .update_one(
                        doc! {"token": bot.token},
                        doc! {"$set": doc! {"is_active": false}},
                    )
                    .await?;
            }
        }
    }

    tokio::signal::ctrl_c().await?;
    Ok(())
}
