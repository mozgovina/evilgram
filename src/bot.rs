use std::{collections::HashMap, str::FromStr};

use futures::{FutureExt, StreamExt, future::BoxFuture};
use mongodb::bson::{Decimal128, doc};
use regex::Regex;
use teloxide::{
    Bot,
    dispatching::{HandlerExt, dialogue::InMemStorage},
    dptree,
    payloads::SendMessageSetters,
    prelude::*,
    types::Message,
    utils::command::BotCommands,
};

use crate::{
    init_db::DB,
    structs::{DBBot, DBUser},
};

type MyDialogue = Dialogue<State, InMemStorage<State>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    CreateMirror,
    Notify,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    Start,
    CreateMirror,
    Notify,
    AddAdmin(u64),
}

async fn is_admin(db: &DB, user_id: ChatId) -> anyhow::Result<bool> {
    let user_id = Decimal128::from_str(user_id.to_string().as_str())?;
    match db.users_coll.find_one(doc! {"user_id": user_id}).await? {
        Some(user) => Ok(user.role == "admin".to_string()),
        None => Ok(false),
    }
}

pub fn run_bot(token: String, db: DB) -> BoxFuture<'static, anyhow::Result<()>> {
    async move {
        let bot = Bot::new(token);

        Dispatcher::builder(
            bot,
            Update::filter_message()
                .filter(|msg: Message| msg.chat.is_private())
                .enter_dialogue::<Message, InMemStorage<State>, State>()
                .branch(
                    dptree::case![State::Start]
                        .filter_command::<Command>()
                        .endpoint(start_branch),
                )
                .branch(dptree::case![State::CreateMirror].endpoint(create_mirror))
                .branch(dptree::case![State::Notify].endpoint(broadcast_msg)),
        )
        .dependencies(dptree::deps![InMemStorage::<State>::new(), db])
        .build()
        .dispatch()
        .await;

        Ok(())
    }
    .boxed()
}

async fn start_branch(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    cmd: Command,
    db: DB,
) -> anyhow::Result<()> {
    match cmd {
        Command::Start => {
            start_command(bot, msg, db).await?;
        }
        _ => {
            if is_admin(&db, msg.chat.id).await? {
                match cmd {
                    Command::CreateMirror => {
                        dialogue.update(State::CreateMirror).await?;
                        bot.send_message(msg.chat.id, "Type token").await?;
                    }
                    Command::Notify => {
                        dialogue.update(State::Notify).await?;
                        bot.send_message(msg.chat.id, "Type message you want to send to users")
                            .await?;
                    }
                    Command::AddAdmin(user_id) => {
                        let filter =
                            doc! {"user_id": Decimal128::from_str(user_id.to_string().as_str())?};
                        if let Some(user) = db.users_coll.find_one(filter.clone()).await? {
                            if user.role == "admin" {
                                bot.send_message(msg.chat.id, "This user is already an admin")
                                    .await?;
                            } else {
                                db.users_coll
                                    .update_one(
                                        filter,
                                        doc! {
                                            "$set": doc! {"role": "admin"}
                                        },
                                    )
                                    .await?;
                            }
                        } else {
                            bot.send_message(msg.chat.id, "No such user in database")
                                .await?;
                        }
                    }
                    _ => {
                        bot.send_message(msg.chat.id, "Unknown command").await?;
                    }
                }
            } else {
                bot.send_message(msg.chat.id, "You are not admin").await?;
            };
        }
    };
    Ok(())
}

async fn start_command(bot: Bot, msg: Message, db: DB) -> anyhow::Result<()> {
    let user_id = msg.from.ok_or(anyhow::anyhow!("No sender"))?.id.0;
    let filter = doc! {"user_id": Decimal128::from_str(user_id.to_string().as_str())?};
    let user = db.users_coll.find_one(filter.clone()).await?;
    match user {
        None => {
            db.users_coll
                .insert_one(DBUser {
                    user_id: user_id,
                    role: "default".to_string(),
                    active_in: vec![bot.token().to_string()],
                    created_mirrors: Vec::new(),
                    is_active: true,
                })
                .await?;
        }
        Some(user) => {
            if !user.is_active {
                db.users_coll
                    .update_one(filter.clone(), doc! {"is_active": true})
                    .await?;
            }
            if !user.active_in.contains(&bot.token().to_string()) {
                db.users_coll
                    .update_one(
                        filter.clone(),
                        doc! {"$push": doc! {"active_in": &bot.token().to_string()}},
                    )
                    .await?;
            }
        }
    }
    bot.send_message(msg.chat.id, "Starting Message").await?;
    Ok(())
}

async fn create_mirror(bot: Bot, dialogue: MyDialogue, msg: Message, db: DB) -> anyhow::Result<()> {
    match msg.text() {
        Some(token) => {
            let token = token.to_string();
            let re = Regex::new(r"^\d{9,10}:[a-zA-Z0-9_-]{35}$").unwrap();
            if !re.is_match(&token) {
                bot.send_message(msg.chat.id, "Invalid token").await?;
                return Ok(());
            }

            if Bot::new(&token).get_me().await.is_ok() {
                let creater_id = msg.from.ok_or(anyhow::anyhow!("No sender"))?.id.0;
                db.bots_coll
                    .insert_one(DBBot {
                        token: token.clone().to_string(),
                        created_by: creater_id,
                        is_active: true,
                    })
                    .await?;
                tokio::spawn(async move { run_bot(token, db).await });
                bot.send_message(msg.chat.id, "You added new mirror")
                    .await?;
            } else {
                bot.send_message(msg.chat.id, "Token is expired").await?;
            }
        }
        None => {
            bot.send_message(msg.chat.id, "You didn't send a token")
                .await?;
        }
    }
    dialogue.reset().await?;
    Ok(())
}

async fn broadcast_msg(bot: Bot, dialogue: MyDialogue, msg: Message, db: DB) -> anyhow::Result<()> {
    let mut cursor = db.users_coll.find(doc! {}).await?;

    let mut bot_stats: HashMap<String, u32> = HashMap::new();

    while let Some(user) = cursor.next().await {
        match user {
            Ok(user) => {
                for token in user.active_in {
                    bot_stats.insert(token.clone(), 0);

                    let bot = Bot::new(token.clone());
                    let send = bot.send_message(
                        user.user_id.to_string(),
                        msg.text().ok_or(anyhow::anyhow!("No text to send"))?,
                    );
                    let res = if let Some(entities) = msg.entities() {
                        send.entities(entities.to_vec()).await
                    } else {
                        send.await
                    };

                    match res {
                        Err(e) => {
                            eprintln!("Error: {}", e);
                        }
                        Ok(_) => match bot_stats.get(&token) {
                            Some(i) => {
                                bot_stats.insert(token, i + 1);
                            }
                            None => {
                                bot_stats.insert(token, 1);
                            }
                        },
                    }
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    bot.send_message(msg.chat.id, "Sending completed").await?;
    dialogue.reset().await?;

    for (token, sent) in bot_stats {
        if sent == 0 {
            db.bots_coll
                .update_one(
                    doc! {"token": token},
                    doc! {
                        "$set": doc! {
                            "is_active": false
                        }
                    },
                )
                .await?;
        }
    }

    Ok(())
}
