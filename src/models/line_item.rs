#[derive(sqlx::FromRow, Debug)]

pub struct LineItem {
    pub id: i32,
    pub invoice_id: i32,
    pub description: String,
    pub hours: f64,
}