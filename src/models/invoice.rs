#[derive(sqlx::FromRow, Debug)]

pub struct Invoice {
    pub id: i32,
    pub project_id: i32,
    pub number: i32,
    pub submit_date: chrono::NaiveDate,
    pub due_date: chrono::NaiveDate,
    pub rate: f64,
    pub status: String,
}