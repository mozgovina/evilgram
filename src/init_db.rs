use crate::structs::{DBBot, DBUser};
use anyhow;
use mongodb::{Client, Collection, Database, bson::doc};

#[derive(Clone)]
pub struct DB {
    pub database: Database,
    pub users_coll: Collection<DBUser>,
    pub bots_coll: Collection<DBBot>,
}

pub async fn init_db() -> anyhow::Result<DB> {
    // Enviroment variables
    let uri = std::env::var("MONGODB_URI")?;
    let db_name = std::env::var("DATABASE")?;
    let admin_id = std::env::var("ADMIN_ID");
    let initial_bot_token = std::env::var("BOT_TOKEN");

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
    let count_users = users_coll.count_documents(doc! {}).await?;
    if count_users == 0 {
        if admin_id.is_err() {
            return Err(anyhow::anyhow!(
                "No users in database and initial admin is not set in enviroment variables"
            ));
        } else {
            let new_admin = DBUser {
                user_id: admin_id?.parse()?,
                role: "admin".to_string(),
                active_in: Vec::new(),
                created_mirrors: Vec::new(),
                is_active: true,
            };

            users_coll.insert_one(new_admin).await?;
        }
    }

    // Checking initial bot if not exist
    let bots_coll: Collection<DBBot> = database.collection("bots");
    let count_bots = bots_coll.count_documents(doc! {}).await?;
    if count_bots == 0 {
        if initial_bot_token.is_err() {
            return Err(anyhow::anyhow!(
                "No bots in database and initial bot token is not set in enviroment variables"
            ));
        } else {
            let new_bot = DBBot {
                token: initial_bot_token?.to_string(),
                created_by: 0,
                is_active: true,
            };

            bots_coll.insert_one(new_bot).await?;
        }
    }

    Ok(DB {
        database: database,
        users_coll: users_coll,
        bots_coll: bots_coll,
    })
}
