#[derive(sqlx::FromRow, Debug)]
pub struct Project {
    pub id: i32,
    pub client_id: i32,
    pub name: String,
    pub start_date: chrono::NaiveDate,
    pub end_date: Option<chrono::NaiveDate>,
} 