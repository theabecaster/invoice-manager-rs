#[derive(sqlx::FromRow, Debug)]
pub struct Profile {
    pub id: i32,
    pub name: String,
    pub phone: String,
    pub address: Option<String>,
    pub email: String,
    pub bank_name: String,
    pub bank_account: String,
    pub routing_number: String,
} 