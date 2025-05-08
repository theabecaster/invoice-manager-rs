#[derive(sqlx::FromRow, Debug, Clone)]

pub struct InvoiceLineItem {
    pub id: i32,
    pub invoice_id: i32,
    pub description: String,
    pub hours: f64,
}