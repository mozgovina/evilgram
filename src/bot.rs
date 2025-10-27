use teloxide::{
    Bot,
    dispatching::{HandlerExt, dialogue::InMemStorage},
    dptree,
    prelude::*,
    types::Message,
    utils::command::BotCommands,
};

use crate::init_db::DB;

type MyDialogue = Dialogue<State, InMemStorage<State>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    CreateMirror,
    BroadcastMessage,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    Start,
    CreateMirror,
    Notify,
}

pub async fn run_bot(token: String, db: DB) -> anyhow::Result<()> {
    let bot = Bot::new(token);

    Dispatcher::builder(
        bot,
        Update::filter_message()
            .enter_dialogue::<Message, InMemStorage<State>, State>()
            .branch(
                dptree::case![State::Start]
                    .filter_command::<Command>()
                    .endpoint(start_branch),
            )
            .branch(dptree::case![State::CreateMirror].endpoint(create_mirror))
            .branch(dptree::case![State::BroadcastMessage].endpoint(broadcast_msg)),
    )
    .dependencies(dptree::deps![InMemStorage::<State>::new(), db])
    .build()
    .dispatch()
    .await;

    Ok(())
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
        _ => bot.send_message(msg.chat.id, "Hello").await?,
    };
    Ok(())
}

async fn create_mirror(bot: Bot, dialogue: MyDialogue, msg: Message, db: DB) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, "Hello").await?;
    Ok(())
}

async fn broadcast_msg(bot: Bot, dialogue: MyDialogue, msg: Message, db: DB) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, "Hello").await?;
    Ok(())
}
