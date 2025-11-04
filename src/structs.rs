use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DBUser {
    pub user_id: u64,
    pub role: String,
    pub active_in: Vec<String>,
    pub created_mirrors: Vec<String>,
    pub is_active: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DBBot {
    pub token: String,
    pub created_by: u64,
    pub is_active: bool,
}
