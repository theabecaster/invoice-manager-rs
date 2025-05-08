#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Profile {
    pub id: i32,
    pub name: String,
    pub phonenumber: String,
    pub address: Option<String>,
    pub email: String,
    pub bank_name: String,
    pub bank_account_number: String,
    pub bank_routing_number: String,
} 