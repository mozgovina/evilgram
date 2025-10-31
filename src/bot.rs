use std::str::FromStr;

use futures::{FutureExt, future::BoxFuture};
use mongodb::bson::{Decimal128, doc};
use teloxide::{
    Bot,
    dispatching::{HandlerExt, dialogue::InMemStorage},
    dptree,
    prelude::*,
    types::Message,
    utils::command::BotCommands,
};

use crate::{init_db::DB, structs::DBBot};

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
        Command::Start => bot.send_message(msg.chat.id, "Starting Message").await?,
        _ => {
            let user_id = msg.chat.id;
            if is_admin(&db, user_id).await? {
                match cmd {
                    Command::CreateMirror => {
                        dialogue.update(State::CreateMirror).await?;
                        bot.send_message(msg.chat.id, "Type token").await?
                    }
                    Command::Notify => {
                        dialogue.update(State::Notify).await?;
                        bot.send_message(msg.chat.id, "Type message you want to send to users")
                            .await?
                    }
                    _ => bot.send_message(msg.chat.id, "Unknown command").await?,
                }
            } else {
                bot.send_message(msg.chat.id, "You are not admin").await?
            }
        }
    };
    Ok(())
}

async fn create_mirror(bot: Bot, dialogue: MyDialogue, msg: Message, db: DB) -> anyhow::Result<()> {
    match msg.text() {
        Some(token) => {
            let token = token.to_string();
            // TODO: add token validation

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
    bot.send_message(msg.chat.id, "Notify is not implemented")
        .await?;
    dialogue.reset().await?;
    Ok(())
}
