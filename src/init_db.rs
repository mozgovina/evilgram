use std::str::FromStr;

use crate::structs::{DBBot, DBUser};
use anyhow;
use mongodb::{
    Client, Collection, Database,
    bson::{Decimal128, doc},
};

pub struct InitDB {
    pub database: Database,
    pub users_coll: Collection<DBUser>,
    pub bots_coll: Collection<DBBot>,
}

pub async fn init_db() -> anyhow::Result<InitDB> {
    // Enviroment variables
    let uri = std::env::var("MONGODB_URI")?;
    let db_name = std::env::var("BOTS_DATABASE")?;
    let admin_id = std::env::var("ADMIN_ID")?;
    let initial_bot_token = std::env::var("INITIAL_BOT")?;

    // Database checking
    let client = Client::with_uri_str(uri).await?;

    let db_names = client.list_database_names().await?;
    if !db_names.contains(&db_name) {
        return Err(anyhow::anyhow!("Database not exist"));
    }

    let database = client.database(&db_name);

    let coll_names = database.list_collection_names().await?;
    if !coll_names.contains(&"users".to_string()) {
        database.create_collection("users").await?;
    }

    if !coll_names.contains(&"bots".to_string()) {
        database.create_collection("bots").await?;
    }

    // Adding admin if not exist
    let users_coll: Collection<DBUser> = database.collection("users");
    let admin = users_coll
        .find_one(doc! { "user_id": Decimal128::from_str(&admin_id)?})
        .await?;

    if admin.is_none() {
        let new_admin = DBUser {
            user_id: admin_id.parse()?,
            role: "admin".to_string(),
            active_in: Vec::new(),
            created_mirrors: Vec::new(),
            is_active: true,
        };

        users_coll.insert_one(new_admin).await?;
    }

    // Checking initial bot if not exist
    let bots_coll: Collection<DBBot> = database.collection("bots");
    let initial_bot = bots_coll
        .find_one(doc! { "token": &initial_bot_token })
        .await?;

    if initial_bot.is_none() {
        let new_bot = DBBot {
            token: initial_bot_token.to_string(),
            created_by: admin_id.parse()?,
            is_active: true,
        };

        bots_coll.insert_one(new_bot).await?;
    }

    Ok(InitDB {
        database: database,
        users_coll: users_coll,
        bots_coll: bots_coll,
    })
}
