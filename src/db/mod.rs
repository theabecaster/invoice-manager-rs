use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use crate::config::Config;
use crate::models::{Profile, Client, Project, Invoice, InvoiceLineItem};

/// Database connection pool
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new Database instance with a connection pool
    pub async fn new(config: &Config) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(config.database_url())
            .await?;
            
        Ok(Self { pool })
    }
    
    /// Get a reference to the connection pool
    pub fn get_pool(&self) -> &PgPool {
        &self.pool
    }

    // Profile operations
    pub async fn get_profiles(&self) -> Result<Vec<Profile>> {
        let profiles = sqlx::query_as!(
            Profile,
            "SELECT * FROM profiles ORDER BY name ASC"
        )
        .fetch_all(self.get_pool())
        .await?;
        
        Ok(profiles)
    }

    pub async fn get_profile(&self, id: i32) -> Result<Profile> {
        let profile = sqlx::query_as!(
            Profile,
            "SELECT * FROM profiles WHERE id = $1",
            id
        )
        .fetch_one(self.get_pool())
        .await?;
        
        Ok(profile)
    }

    pub async fn create_profile(&self, profile: &Profile) -> Result<i32> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO profiles (name, phonenumber, address, email, bank_name, bank_account_number, bank_routing_number)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
            "#,
            profile.name,
            profile.phonenumber,
            profile.address,
            profile.email,
            profile.bank_name,
            profile.bank_account_number,
            profile.bank_routing_number
        )
        .fetch_one(self.get_pool())
        .await?;
        
        Ok(id)
    }

    pub async fn update_profile(&self, profile: &Profile) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE profiles
            SET name = $1, phonenumber = $2, address = $3, email = $4,
                bank_name = $5, bank_account_number = $6, bank_routing_number = $7
            WHERE id = $8
            "#,
            profile.name,
            profile.phonenumber,
            profile.address,
            profile.email,
            profile.bank_name,
            profile.bank_account_number,
            profile.bank_routing_number,
            profile.id
        )
        .execute(self.get_pool())
        .await?;
        
        Ok(())
    }

    pub async fn delete_profile(&self, id: i32) -> Result<()> {
        // Start a transaction
        let mut tx = self.pool.begin().await?;
        
        // Get all clients for this profile
        let clients = sqlx::query_as!(
            Client,
            "SELECT * FROM clients WHERE profile_id = $1",
            id
        )
        .fetch_all(&mut *tx)
        .await?;
        
        // For each client, delete all associated projects and their invoices
        for client in clients {
            // Get all projects for this client
            let projects = sqlx::query_as!(
                Project,
                "SELECT * FROM projects WHERE client_id = $1",
                client.id
            )
            .fetch_all(&mut *tx)
            .await?;
            
            // For each project, delete all associated invoices and their line items
            for project in projects {
                // Delete invoice line items first
                sqlx::query!(
                    "DELETE FROM invoice_line_item WHERE invoice_id IN (SELECT id FROM invoices WHERE project_id = $1)",
                    project.id
                )
                .execute(&mut *tx)
                .await?;
                
                // Delete invoices
                sqlx::query!(
                    "DELETE FROM invoices WHERE project_id = $1",
                    project.id
                )
                .execute(&mut *tx)
                .await?;
            }
            
            // Delete projects
            sqlx::query!(
                "DELETE FROM projects WHERE client_id = $1",
                client.id
            )
            .execute(&mut *tx)
            .await?;
        }
        
        // Delete clients
        sqlx::query!(
            "DELETE FROM clients WHERE profile_id = $1",
            id
        )
        .execute(&mut *tx)
        .await?;
        
        // Finally delete the profile
        sqlx::query!("DELETE FROM profiles WHERE id = $1", id)
            .execute(&mut *tx)
            .await?;
        
        // Commit the transaction
        tx.commit().await?;
        
        Ok(())
    }

    // Client operations
    pub async fn get_clients_by_profile(&self, profile_id: i32) -> Result<Vec<Client>> {
        let clients = sqlx::query_as!(
            Client,
            "SELECT * FROM clients WHERE profile_id = $1 ORDER BY name ASC",
            profile_id
        )
        .fetch_all(self.get_pool())
        .await?;
        
        Ok(clients)
    }

    pub async fn get_client(&self, id: i32) -> Result<Client> {
        let client = sqlx::query_as!(
            Client,
            "SELECT * FROM clients WHERE id = $1",
            id
        )
        .fetch_one(self.get_pool())
        .await?;
        
        Ok(client)
    }

    pub async fn create_client(&self, client: &Client) -> Result<i32> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO clients (name, phone, address, email, profile_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            client.name,
            client.phone,
            client.address,
            client.email,
            client.profile_id
        )
        .fetch_one(self.get_pool())
        .await?;
        
        Ok(id)
    }

    pub async fn update_client(&self, client: &Client) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE clients
            SET name = $1, phone = $2, address = $3, email = $4
            WHERE id = $5
            "#,
            client.name,
            client.phone,
            client.address,
            client.email,
            client.id
        )
        .execute(self.get_pool())
        .await?;
        
        Ok(())
    }

    pub async fn delete_client(&self, id: i32) -> Result<()> {
        // Start a transaction
        let mut tx = self.pool.begin().await?;
        
        // Get all projects for this client
        let projects = sqlx::query_as!(
            Project,
            "SELECT * FROM projects WHERE client_id = $1",
            id
        )
        .fetch_all(&mut *tx)
        .await?;
        
        // For each project, delete all associated invoices and their line items
        for project in projects {
            // Delete invoice line items first
            sqlx::query!(
                "DELETE FROM invoice_line_item WHERE invoice_id IN (SELECT id FROM invoices WHERE project_id = $1)",
                project.id
            )
            .execute(&mut *tx)
            .await?;
            
            // Delete invoices
            sqlx::query!(
                "DELETE FROM invoices WHERE project_id = $1",
                project.id
            )
            .execute(&mut *tx)
            .await?;
        }
        
        // Delete projects
        sqlx::query!(
            "DELETE FROM projects WHERE client_id = $1",
            id
        )
        .execute(&mut *tx)
        .await?;
        
        // Finally delete the client
        sqlx::query!("DELETE FROM clients WHERE id = $1", id)
            .execute(&mut *tx)
            .await?;
        
        // Commit the transaction
        tx.commit().await?;
        
        Ok(())
    }

    // Project operations
    pub async fn get_projects_by_client(&self, client_id: i32) -> Result<Vec<Project>> {
        let projects = sqlx::query_as!(
            Project,
            r#"
            SELECT 
                id,
                client_id,
                name,
                start_date::date as start_date,
                end_date::date as end_date
            FROM projects 
            WHERE client_id = $1 
            ORDER BY name ASC
            "#,
            client_id
        )
        .fetch_all(self.get_pool())
        .await?;
        
        Ok(projects)
    }

    pub async fn get_project(&self, id: i32) -> Result<Project> {
        let project = sqlx::query_as!(
            Project,
            r#"
            SELECT 
                id,
                client_id,
                name,
                start_date::date as start_date,
                end_date::date as end_date
            FROM projects 
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(self.get_pool())
        .await?;
        
        Ok(project)
    }

    pub async fn create_project(&self, project: &Project) -> Result<i32> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO projects (client_id, name, start_date, end_date)
            VALUES ($1, $2, $3::date, $4::date)
            RETURNING id
            "#,
            project.client_id,
            project.name,
            project.start_date as _,
            project.end_date as _
        )
        .fetch_one(self.get_pool())
        .await?;
        
        Ok(id)
    }

    pub async fn update_project(&self, project: &Project) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE projects
            SET name = $1, start_date = $2::date, end_date = $3::date
            WHERE id = $4
            "#,
            project.name,
            project.start_date as _,
            project.end_date as _,
            project.id
        )
        .execute(self.get_pool())
        .await?;
        
        Ok(())
    }

    pub async fn delete_project(&self, id: i32) -> Result<()> {
        sqlx::query!("DELETE FROM projects WHERE id = $1", id)
            .execute(self.get_pool())
            .await?;
        
        Ok(())
    }

    // Invoice operations
    pub async fn get_invoices_by_project(&self, project_id: i32) -> Result<Vec<Invoice>> {
        let invoices = sqlx::query_as!(
            Invoice,
            r#"
            SELECT 
                id,
                project_id,
                number,
                submit_date::date as submit_date,
                due_date::date as due_date,
                COALESCE(rate::float8, 0.0) as "rate!: f64",
                status
            FROM invoices 
            WHERE project_id = $1 
            ORDER BY submit_date DESC
            "#,
            project_id
        )
        .fetch_all(self.get_pool())
        .await?;
        
        Ok(invoices)
    }

    pub async fn get_invoice(&self, id: i32) -> Result<Invoice> {
        let invoice = sqlx::query_as!(
            Invoice,
            r#"
            SELECT 
                id,
                project_id,
                number,
                submit_date::date as submit_date,
                due_date::date as due_date,
                COALESCE(rate::float8, 0.0) as "rate!: f64",
                status
            FROM invoices 
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(self.get_pool())
        .await?;
        
        Ok(invoice)
    }

    pub async fn create_invoice(&self, invoice: &Invoice) -> Result<i32> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO invoices (project_id, number, submit_date, due_date, rate, status)
            VALUES ($1, $2, $3::date, $4::date, $5::float8, $6)
            RETURNING id
            "#,
            invoice.project_id,
            invoice.number,
            invoice.submit_date as _,
            invoice.due_date as _,
            invoice.rate as f64,
            invoice.status
        )
        .fetch_one(self.get_pool())
        .await?;
        
        Ok(id)
    }

    pub async fn update_invoice(&self, invoice: &Invoice) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE invoices
            SET submit_date = $1::date, due_date = $2::date, rate = $3::float8, status = $4
            WHERE id = $5
            "#,
            invoice.submit_date as _,
            invoice.due_date as _,
            invoice.rate as f64,
            invoice.status,
            invoice.id
        )
        .execute(self.get_pool())
        .await?;
        
        Ok(())
    }

    pub async fn delete_invoice(&self, id: i32) -> Result<()> {
        sqlx::query!("DELETE FROM invoices WHERE id = $1", id)
            .execute(self.get_pool())
            .await?;
        
        Ok(())
    }

    // Line item operations
    pub async fn get_line_items_by_invoice(&self, invoice_id: i32) -> Result<Vec<InvoiceLineItem>> {
        let line_items = sqlx::query_as!(
            InvoiceLineItem,
            r#"
            SELECT 
                id,
                invoice_id,
                description,
                hours::float8 as "hours!: f64"
            FROM invoice_line_item 
            WHERE invoice_id = $1 
            ORDER BY id ASC
            "#,
            invoice_id
        )
        .fetch_all(self.get_pool())
        .await?;
        
        Ok(line_items)
    }

    pub async fn create_line_item(&self, line_item: &InvoiceLineItem) -> Result<i32> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO invoice_line_item (invoice_id, description, hours)
            VALUES ($1, $2, $3::float8)
            RETURNING id
            "#,
            line_item.invoice_id,
            line_item.description,
            line_item.hours as f64
        )
        .fetch_one(self.get_pool())
        .await?;
        
        Ok(id)
    }

    pub async fn update_line_item(&self, line_item: &InvoiceLineItem) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE invoice_line_item
            SET description = $1, hours = $2::float8
            WHERE id = $3
            "#,
            line_item.description,
            line_item.hours as f64,
            line_item.id
        )
        .execute(self.get_pool())
        .await?;
        
        Ok(())
    }

    pub async fn delete_line_item(&self, id: i32) -> Result<()> {
        sqlx::query!("DELETE FROM invoice_line_item WHERE id = $1", id)
            .execute(self.get_pool())
            .await?;
        
        Ok(())
    }

    pub async fn delete_line_items_by_invoice(&self, invoice_id: i32) -> Result<()> {
        sqlx::query!("DELETE FROM invoice_line_item WHERE invoice_id = $1", invoice_id)
            .execute(self.get_pool())
            .await?;
        
        Ok(())
    }

    // Transaction methods
    pub async fn save_invoice_with_line_items(
        &self,
        invoice: &Invoice, 
        line_items: &[InvoiceLineItem]
    ) -> Result<i32> {
        // Begin a transaction
        let mut tx = self.pool.begin().await?;
        
        // Create or update the invoice
        let invoice_id = if invoice.id == 0 {
            // New invoice
            let id = sqlx::query_scalar!(
                r#"
                INSERT INTO invoices (project_id, number, submit_date, due_date, rate, status)
                VALUES ($1, $2, $3::date, $4::date, $5::float8, $6)
                RETURNING id
                "#,
                invoice.project_id,
                invoice.number,
                invoice.submit_date,
                invoice.due_date,
                invoice.rate as f64,
                invoice.status
            )
            .fetch_one(&mut *tx)
            .await?;
            
            id
        } else {
            // Update existing invoice
            sqlx::query!(
                r#"
                UPDATE invoices
                SET submit_date = $1::date, due_date = $2::date, rate = $3::float8, status = $4
                WHERE id = $5
                "#,
                invoice.submit_date,
                invoice.due_date,
                invoice.rate as f64,
                invoice.status,
                invoice.id
            )
            .execute(&mut *tx)
            .await?;
            
            invoice.id
        };
        
        // Delete existing line items if updating
        if invoice.id > 0 {
            sqlx::query!("DELETE FROM invoice_line_item WHERE invoice_id = $1", invoice_id)
                .execute(&mut *tx)
                .await?;
        }
        
        // Insert all line items
        for line_item in line_items {
            sqlx::query!(
                r#"
                INSERT INTO invoice_line_item (invoice_id, description, hours)
                VALUES ($1, $2, $3::float8)
                "#,
                invoice_id,
                line_item.description,
                line_item.hours as f64
            )
            .execute(&mut *tx)
            .await?;
        }
        
        // Commit the transaction
        tx.commit().await?;
        
        Ok(invoice_id)
    }

    pub async fn get_invoice_with_line_items(&self, id: i32) -> Result<(Invoice, Vec<InvoiceLineItem>)> {
        let invoice = self.get_invoice(id).await?;
        let line_items = self.get_line_items_by_invoice(id).await?;
        Ok((invoice, line_items))
    }

    // Additional invoice operations used by UI layer
    pub async fn load_invoices_by_project(&self, project_id: i32) -> Result<Vec<Invoice>> {
        // This is similar to get_invoices_by_project but with explicit type handling
        let invoices = sqlx::query_as!(
            Invoice,
            r#"
            SELECT 
                id,
                project_id,
                number,
                submit_date::date as submit_date,
                due_date::date as due_date,
                COALESCE(rate::float8, 0.0) as "rate!: f64",
                status
            FROM invoices 
            WHERE project_id = $1 
            ORDER BY submit_date DESC
            "#,
            project_id
        )
        .fetch_all(self.get_pool())
        .await?;
        
        Ok(invoices)
    }

    // Projects methods
    pub async fn load_projects_by_client(&self, client_id: i32) -> Result<Vec<Project>> {
        let projects = sqlx::query_as!(
            Project,
            "SELECT * FROM projects WHERE client_id = $1 ORDER BY name ASC",
            client_id
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(projects)
    }

    // Clients methods
    pub async fn load_clients_by_profile(&self, profile_id: i32) -> Result<Vec<Client>> {
        let clients = sqlx::query_as!(
            Client,
            "SELECT * FROM clients WHERE profile_id = $1 ORDER BY name ASC",
            profile_id
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(clients)
    }

    // Profiles methods
    pub async fn load_profiles(&self) -> Result<Vec<Profile>> {
        let profiles = sqlx::query_as!(
            Profile,
            "SELECT * FROM profiles ORDER BY name ASC"
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(profiles)
    }
}

/// Initialize the database connection pool
pub async fn init(config: &Config) -> Result<Database> {
    let db = Database::new(config).await?;
    
    // Run migrations if you want to automatically migrate the database
    // sqlx::migrate!().run(db.get_pool()).await?;
    
    Ok(db)
} 