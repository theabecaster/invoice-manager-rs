#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Client {
    pub id: i32,
    pub name: String,
    pub phone: String,
    pub address: Option<String>,
    pub email: String,
    pub profile_id: i32,
} 