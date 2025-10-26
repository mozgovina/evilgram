use anyhow;
use mongodb::Client;
use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let uri = std::env::var("MONGODB_URI")?;
    let db_name = std::env::var("BOTS_DATABASE")?;
    let client = Client::with_uri_str(uri).await?;

    let db_names = client.list_database_names().await?;
    if !db_names.contains(&db_name) {
        println!("Database not exist");
        return Ok(());
    }

    let database = client.database(&db_name);

    let coll_names = database.list_collection_names().await?;
    if !coll_names.contains(&"users".to_string()) {
        database.create_collection("users").await?;
    }

    println!("Hello, world!");
    Ok(())
}
