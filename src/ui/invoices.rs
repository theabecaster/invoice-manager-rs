use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState, Paragraph},
    Frame,
};

use crate::models::Invoice;
use crate::ui::email_wizard::{
    self, EmailWizardState, EmailWizardAction, 
    render_email_wizard, handle_input as handle_email_input, send_invoice_email,
    generate_invoice_files
};

// Represents the state of the invoice table screen
pub struct InvoicesState {
    project_id: i32,
    project_name: String,
    invoices: Vec<Invoice>,
    table_state: TableState,
    email_wizard_state: Option<EmailWizardState>,
}

impl InvoicesState {
    pub fn new(project_id: i32, project_name: String, invoices: Vec<Invoice>) -> Self {
        let mut table_state = TableState::default();
        if !invoices.is_empty() {
            table_state.select(Some(0));
        }
        
        Self {
            project_id,
            project_name,
            invoices,
            table_state,
            email_wizard_state: None,
        }
    }

    pub fn next(&mut self) {
        if self.invoices.is_empty() {
            return;
        }

        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.invoices.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.invoices.is_empty() {
            return;
        }

        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.invoices.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }
    
    pub fn selected_invoice(&self) -> Option<&Invoice> {
        self.table_state.selected().and_then(|i| self.invoices.get(i))
    }
    
    pub fn selected_invoice_id(&self) -> Option<i32> {
        self.selected_invoice().map(|i| i.id)
    }
    
    pub fn project_id(&self) -> i32 {
        self.project_id
    }
    
    pub fn project_name(&self) -> &str {
        &self.project_name
    }
    
    pub fn start_email_wizard(&mut self, invoice_id: i32) {
        self.email_wizard_state = Some(EmailWizardState::new(invoice_id));
    }
    
    pub fn close_email_wizard(&mut self) {
        self.email_wizard_state = None;
    }
    
    pub async fn force_close_email_wizard(&mut self) -> Result<()> {
        if let Some(email_state) = &mut self.email_wizard_state {
            // Force cleanup of any generated files
            email_state.cleanup_files()?;
        }
        
        // Close the email wizard state
        self.email_wizard_state = None;
        
        Ok(())
    }
    
    pub fn is_in_email_wizard(&self) -> bool {
        self.email_wizard_state.is_some()
    }
}

pub enum InvoiceAction {
    Back,
    NewInvoice(i32), // Contains project_id
    EditInvoice(i32), // Contains invoice_id
    EmailInvoice(i32), // Contains invoice_id
}

// DB operations for invoices
pub async fn load_invoices_by_project(db: &crate::db::Database, project_id: i32) -> Result<Vec<Invoice>> {
    // Use the database layer instead of direct access
    db.load_invoices_by_project(project_id).await
}

pub async fn delete_invoice(db: &crate::db::Database, id: i32) -> Result<()> {
    // Use the database layer instead of direct access
    db.delete_invoice(id).await
}

pub async fn get_invoice_with_line_items(db: &crate::db::Database, id: i32) -> Result<(Invoice, Vec<crate::models::InvoiceLineItem>)> {
    // Use the database layer instead of direct access
    db.get_invoice_with_line_items(id).await
}

pub fn render_invoices<B: Backend>(frame: &mut Frame<B>, state: &mut InvoicesState) {
    // Clear the frame completely first
    let clear_block = Block::default().style(Style::default().bg(Color::Black));
    frame.render_widget(clear_block, frame.size());
    
    // If in email wizard mode, render the email wizard instead
    if let Some(email_state) = &mut state.email_wizard_state {
        // Don't render the email wizard if it's being dismissed
        if !email_state.is_dismissing() {
            render_email_wizard(frame, email_state);
            return;
        }
        // If the email wizard is dismissing, we'll fall through to render the invoice table
    }
    
    let size = frame.size();
    
    // Create the layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
        ].as_ref())
        .split(size);

    // Define the header cells
    let header_cells = ["Number", "Submit Date", "Due Date", "Status", "Actions"]
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(Color::Yellow)));
    let header = Row::new(header_cells)
        .style(Style::default())
        .height(1)
        .bottom_margin(1);

    // Create the rows with data and action buttons
    let rows = state.invoices.iter().map(|invoice| {
        let submit_date = invoice.submit_date.format("%Y-%m-%d").to_string();
        let due_date = invoice.due_date.format("%Y-%m-%d").to_string();
        
        let cells = vec![
            Cell::from(invoice.number.to_string()),
            Cell::from(submit_date),
            Cell::from(due_date),
            Cell::from(invoice.status.as_str()),
            Cell::from("Edit | Email"),
        ];
        
        Row::new(cells).height(1)
    });

    // Create the table
    let title = format!("Invoices for {}", state.project_name());
    let table = Table::new(rows)
        .header(header)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .widths(&[
            Constraint::Percentage(15),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(25),
        ]);

    frame.render_stateful_widget(table, chunks[0], &mut state.table_state);

    // Create and render the buttons
    let selected = state.selected_invoice().is_some();
    let buttons_text = if selected {
        format!("<N> New Invoice | <E> Edit Invoice | <M> Email Invoice | <Esc> Back")
    } else {
        format!("<N> New Invoice | <Esc> Back")
    };

    let buttons = Paragraph::new(buttons_text)
        .block(Block::default().borders(Borders::TOP))
        .style(Style::default().fg(Color::White));

    frame.render_widget(buttons, chunks[1]);
}

pub async fn handle_input(db: &crate::db::Database, state: &mut InvoicesState) -> Result<Option<InvoiceAction>> {
    // If in email wizard mode, handle email input instead
    if state.is_in_email_wizard() {
        if let Some(email_state) = &mut state.email_wizard_state {
            // If the email_wizard is being dismissed, complete the dismissal immediately
            if email_state.is_dismissing() {
                // Force a complete cleanup
                email_state.cleanup_files()?;
                state.close_email_wizard();
                return Ok(None);
            }
            
            match handle_email_input(email_state)? {
                Some(EmailWizardAction::Cancel) => {
                    // Mark the email wizard for dismissal and force cleanup
                    email_state.dismiss();
                    // Immediately close the email wizard
                    state.close_email_wizard();
                    return Ok(None);
                }
                Some(EmailWizardAction::Send) => {
                    // Send the email
                    send_invoice_email(email_state).await?;
                    
                    // Check if we've successfully sent the email - we'll need to add a method to check this
                    if email_state.has_success_message() {
                        // Add a short delay to let user see the success message
                        std::thread::sleep(std::time::Duration::from_millis(1500));
                        // Then dismiss and immediately close
                        email_state.dismiss();
                        state.close_email_wizard();
                        return Ok(None);
                    }
                }
                None => {}
            }
            return Ok(None);
        }
    }
    
    if let Event::Key(key) = event::read()? {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                return Ok(Some(InvoiceAction::Back));
            }
            KeyCode::Char('n') => {
                return Ok(Some(InvoiceAction::NewInvoice(state.project_id())));
            }
            KeyCode::Char('e') => {
                if let Some(id) = state.selected_invoice_id() {
                    return Ok(Some(InvoiceAction::EditInvoice(id)));
                }
            }
            KeyCode::Char('m') => {
                if let Some(id) = state.selected_invoice_id() {
                    // Initialize the email wizard and load invoice data
                    state.start_email_wizard(id);
                    
                    if let Some(email_state) = &mut state.email_wizard_state {
                        // Load the invoice and line items
                        let (invoice, line_items) = get_invoice_with_line_items(db, id).await?;
                        
                        // Get the project to access its name and client
                        let project = db.get_project(invoice.project_id).await?;
                        
                        // Get the client to access email
                        let client = db.get_client(project.client_id).await?;
                        
                        // Now load invoice with project name and client email
                        email_state.load_invoice(invoice, line_items, project.name, client.email);
                        
                        // Generate invoice files on-demand
                        generate_invoice_files(db, email_state).await?;
                    }
                    
                    return Ok(None);
                }
            }
            KeyCode::Down => {
                state.next();
            }
            KeyCode::Up => {
                state.previous();
            }
            _ => {}
        }
    }
    Ok(None)
} 