use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Spans,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::path::Path;
use std::fs;
use lettre::{
    Message, SmtpTransport, Transport, message::{MultiPart, SinglePart, Attachment, header},
    transport::smtp::authentication::Credentials,
};

use crate::models::{Invoice, InvoiceLineItem};

// Represents the state of the email wizard
pub struct EmailWizardState {
    invoice_id: i32,
    invoice: Option<Invoice>,
    line_items: Option<Vec<InvoiceLineItem>>,
    recipient_email: String,
    subject: String,
    message: String,
    current_field: EmailField,
    show_error: Option<String>,
    show_success: Option<String>,
    // Track the generated files so we can clean them up
    generated_md_path: Option<String>,
    generated_pdf_path: Option<String>,
    // Flag to indicate the wizard should be dismissed
    dismissing: bool,
}

// Represents the current field being edited
#[derive(Clone, Copy, PartialEq)]
pub enum EmailField {
    RecipientEmail,
    Subject,
    Message,
    None,
}

// Possible actions from the email wizard
pub enum EmailWizardAction {
    Cancel,
    Send,
}

impl EmailWizardState {
    pub fn new(invoice_id: i32) -> Self {
        Self {
            invoice_id,
            invoice: None,
            line_items: None,
            recipient_email: String::new(),
            subject: String::new(),
            message: String::new(),
            current_field: EmailField::RecipientEmail,
            show_error: None,
            show_success: None,
            generated_md_path: None,
            generated_pdf_path: None,
            dismissing: false,
        }
    }

    pub fn load_invoice(&mut self, invoice: Invoice, line_items: Vec<InvoiceLineItem>, project_name: String, client_email: String) {
        // Extract values we need for formatting before moving the invoice
        let invoice_number = invoice.number;
        
        // Set default subject with project name instead of ID
        self.subject = format!("Invoice #{} for {}", invoice_number, project_name);
            
        // Set the recipient email from client
        self.recipient_email = client_email;
            
        // Store the invoice and line items
        self.invoice = Some(invoice);
        self.line_items = Some(line_items);
        
        // Set default message
        self.message = self.get_default_message();
    }
    
    fn get_default_message(&self) -> String {
        if let Some(invoice) = &self.invoice {
            format!(
                "Dear Client,\n\nPlease find attached the invoice #{} for our recent work.\n\n\
                Invoice Number: {}\n\
                Submit Date: {}\n\
                Due Date: {}\n\
                Total Amount: ${:.2}\n\n\
                Thank you for your business.\n\
                Please let me know if you have any questions.\n\n\
                Regards,\n\
                Your Name",
                invoice.number,
                invoice.number,
                invoice.submit_date.format("%Y-%m-%d"),
                invoice.due_date.format("%Y-%m-%d"),
                self.calculate_total_amount(),
            )
        } else {
            String::new()
        }
    }
    
    fn calculate_total_amount(&self) -> f64 {
        if let (Some(invoice), Some(line_items)) = (&self.invoice, &self.line_items) {
            line_items.iter().map(|item| item.hours * invoice.rate).sum()
        } else {
            0.0
        }
    }
    
    pub fn next_field(&mut self) {
        match self.current_field {
            EmailField::RecipientEmail => self.current_field = EmailField::Subject,
            EmailField::Subject => self.current_field = EmailField::Message,
            EmailField::Message => self.current_field = EmailField::None,
            EmailField::None => {}
        }
    }
    
    pub fn previous_field(&mut self) {
        match self.current_field {
            EmailField::RecipientEmail => {},
            EmailField::Subject => self.current_field = EmailField::RecipientEmail,
            EmailField::Message => self.current_field = EmailField::Subject,
            EmailField::None => self.current_field = EmailField::Message,
        }
    }
    
    pub fn handle_input(&mut self, input: char) {
        match self.current_field {
            EmailField::RecipientEmail => {
                if input == '\u{7f}' { // Backspace
                    self.recipient_email.pop();
                } else {
                    self.recipient_email.push(input);
                }
            },
            EmailField::Subject => {
                if input == '\u{7f}' { // Backspace
                    self.subject.pop();
                } else {
                    self.subject.push(input);
                }
            },
            EmailField::Message => {
                if input == '\u{7f}' { // Backspace
                    self.message.pop();
                } else if input == '\n' {
                    self.message.push('\n');
                } else {
                    self.message.push(input);
                }
            },
            EmailField::None => {}
        }
    }
    
    pub fn validate(&self) -> Result<(), String> {
        // Validate email
        if self.recipient_email.is_empty() {
            return Err("Recipient email cannot be empty".into());
        }
        
        if !self.recipient_email.contains('@') {
            return Err("Invalid email address".into());
        }
        
        // Validate subject
        if self.subject.is_empty() {
            return Err("Subject cannot be empty".into());
        }
        
        Ok(())
    }
    
    // Clean up any generated files
    pub fn cleanup_files(&self) -> Result<()> {
        if let Some(md_path) = &self.generated_md_path {
            if Path::new(md_path).exists() {
                fs::remove_file(md_path)?;
                println!("Removed temporary file: {}", md_path);
            }
        }
        
        if let Some(pdf_path) = &self.generated_pdf_path {
            if Path::new(pdf_path).exists() {
                fs::remove_file(pdf_path)?;
                println!("Removed temporary file: {}", pdf_path);
            }
        }
        
        Ok(())
    }

    // Mark the wizard for dismissal
    pub fn dismiss(&mut self) {
        self.dismissing = true;
        // Clean up files immediately
        if let Err(e) = self.cleanup_files() {
            eprintln!("Error cleaning up files during dismissal: {}", e);
        }
    }
    
    // Check if the wizard is being dismissed
    pub fn is_dismissing(&self) -> bool {
        self.dismissing
    }

    // Add this new method
    pub fn has_success_message(&self) -> bool {
        self.show_success.is_some()
    }
}

// Clean up on drop to ensure we always clean up files even if there's an error
impl Drop for EmailWizardState {
    fn drop(&mut self) {
        if let Err(e) = self.cleanup_files() {
            eprintln!("Error cleaning up files: {}", e);
        }
    }
}

pub fn render_email_wizard<B: Backend>(frame: &mut Frame<B>, state: &mut EmailWizardState) {
    let size = frame.size();
    
    // Create the layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Email recipient
            Constraint::Length(3), // Subject
            Constraint::Min(8),    // Message body
            Constraint::Length(3), // Buttons/navigation
        ].as_ref())
        .split(size);
    
    // Render title
    let title_text = if let Some(invoice) = &state.invoice {
        format!("Email Invoice #{}", invoice.number)
    } else {
        "Email Invoice".to_string()
    };
    
    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    
    frame.render_widget(title, chunks[0]);
    
    // Render email recipient field
    let email_style = if state.current_field == EmailField::RecipientEmail {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    
    let email_field = Paragraph::new(state.recipient_email.clone())
        .style(email_style)
        .block(Block::default().borders(Borders::ALL).title("Recipient Email"));
    
    frame.render_widget(email_field, chunks[1]);
    
    // Render subject field
    let subject_style = if state.current_field == EmailField::Subject {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    
    let subject_field = Paragraph::new(state.subject.clone())
        .style(subject_style)
        .block(Block::default().borders(Borders::ALL).title("Subject"));
    
    frame.render_widget(subject_field, chunks[2]);
    
    // Render message field
    let message_style = if state.current_field == EmailField::Message {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    
    let message_field = Paragraph::new(state.message.clone())
        .style(message_style)
        .block(Block::default().borders(Borders::ALL).title("Message"));
    
    frame.render_widget(message_field, chunks[3]);
    
    // Render navigation/buttons
    let buttons_text = match state.current_field {
        EmailField::None => "<Enter> Send | <Tab> Back to Fields | <Esc> Cancel",
        _ => "<Tab> Next Field | <Shift+Tab> Previous Field | <Enter> Send | <Esc> Cancel",
    };
    
    let buttons = Paragraph::new(buttons_text)
        .block(Block::default().borders(Borders::TOP))
        .style(Style::default().fg(Color::White));
    
    frame.render_widget(buttons, chunks[4]);
    
    // Show error if needed
    if let Some(error) = &state.show_error {
        render_error(frame, size, error);
    }
    
    // Show success message if needed
    if let Some(message) = &state.show_success {
        render_success(frame, size, message);
    }
}

fn render_error<B: Backend>(frame: &mut Frame<B>, size: Rect, error: &str) {
    let popup_area = centered_rect(60, 20, size);
    
    let error_msg = Paragraph::new(vec![
        Spans::from(""),
        Spans::from(error),
        Spans::from(""),
        Spans::from("Press any key to continue"),
    ])
    .block(Block::default().title("Error").borders(Borders::ALL))
    .style(Style::default().fg(Color::Red));
    
    frame.render_widget(error_msg, popup_area);
}

fn render_success<B: Backend>(frame: &mut Frame<B>, size: Rect, message: &str) {
    let popup_area = centered_rect(60, 20, size);
    
    let success_msg = Paragraph::new(vec![
        Spans::from(""),
        Spans::from(message),
        Spans::from(""),
        Spans::from("Press any key to continue"),
    ])
    .block(Block::default().title("Success").borders(Borders::ALL))
    .style(Style::default().fg(Color::Green));
    
    frame.render_widget(success_msg, popup_area);
}

// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn handle_input(state: &mut EmailWizardState) -> Result<Option<EmailWizardAction>> {
    // Clear any existing error message
    state.show_error = None;
    state.show_success = None;
    
    if let Event::Key(key) = event::read()? {
        match key.code {
            KeyCode::Backspace => {
                state.handle_input('\u{7f}'); // Pass backspace char
            }
            KeyCode::Esc => {
                return Ok(Some(EmailWizardAction::Cancel));
            }
            KeyCode::Char(c) => {
                state.handle_input(c);
            }
            KeyCode::Enter => {
                if state.current_field == EmailField::None {
                    // Try to send email
                    match state.validate() {
                        Ok(_) => return Ok(Some(EmailWizardAction::Send)),
                        Err(e) => state.show_error = Some(e),
                    }
                } else {
                    state.next_field();
                }
            }
            KeyCode::Tab => {
                if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                    state.previous_field();
                } else {
                    state.next_field();
                }
            }
            _ => {}
        }
    }
    
    Ok(None)
}

// Function to generate invoice files when the email wizard is opened
pub async fn generate_invoice_files(
    db: &crate::db::Database,
    state: &mut EmailWizardState
) -> Result<()> {
    if let (Some(invoice), Some(line_items)) = (&state.invoice, &state.line_items) {
        // Get the project for this invoice
        let project = db.get_project(invoice.project_id).await?;
        
        // Get the client that owns the project
        let client = db.get_client(project.client_id).await?;
        
        // Get the profile that owns the client
        let profile = db.get_profile(client.profile_id).await?;
        
        // Ensure the invoices directory exists
        let invoices_dir = "invoices";
        if !Path::new(invoices_dir).exists() {
            fs::create_dir_all(invoices_dir)?;
        }
        
        // Create the invoice generator instance
        let generator = crate::invoice_gen::InvoiceGenerator::new(invoices_dir)?;
        
        // Generate the invoice files with the additional information
        match generator.generate_invoice(invoice, line_items, &profile, &client, &project) {
            Ok((md_path, pdf_path)) => {
                println!("Invoice files generated on-demand:");
                println!("Markdown: {}", md_path);
                println!("PDF: {}", pdf_path);
                
                // Store the paths for later cleanup
                state.generated_md_path = Some(md_path);
                state.generated_pdf_path = Some(pdf_path);
            }
            Err(e) => {
                state.show_error = Some(format!("Failed to generate invoice files: {}", e));
            }
        }
    }
    
    Ok(())
}

// Function to send invoice email
pub async fn send_invoice_email(state: &mut EmailWizardState) -> Result<()> {
    if let (Some(invoice), Some(_)) = (&state.invoice, &state.line_items) {
        // Build the file path for the PDF - use the one we generated
        let pdf_path = if let Some(path) = &state.generated_pdf_path {
            path.clone()
        } else {
            // Fallback if not generated yet
            format!("invoices/invoice_{}.pdf", invoice.number)
        };
        
        // Check if the PDF file exists
        if !Path::new(&pdf_path).exists() {
            state.show_error = Some(format!("PDF file not found: {}", pdf_path));
            return Ok(());
        }
        
        // Read PDF file
        let pdf_content = std::fs::read(&pdf_path)?;
        
        // Determine if it's likely a real PDF (starts with %PDF magic number) or a text file
        let content_type = if pdf_content.starts_with(b"%PDF") {
            header::ContentType::parse("application/pdf")?
        } else {
            // It's our text fallback
            header::ContentType::parse("text/plain")?
        };
        
        // Create email
        let email = Message::builder()
            .from("invoicemanager@example.com".parse()?)
            .to(state.recipient_email.parse()?)
            .subject(&state.subject)
            .multipart(
                MultiPart::mixed()
                    .singlepart(
                        SinglePart::plain(state.message.clone())
                    )
                    .singlepart(
                        Attachment::new(format!("invoice_{}.pdf", invoice.number))
                            .body(pdf_content, content_type)
                    )
            )?;
        
        // Configure SMTP client - This should come from a config in a real app
        let smtp_username = "username@example.com"; // Replace with env var or config
        let smtp_password = "password"; // Replace with env var or config
        let smtp_server = "smtp.example.com"; // Replace with env var or config
        
        let creds = Credentials::new(smtp_username.to_string(), smtp_password.to_string());
        
        // Open connection and send email
        let mailer = SmtpTransport::relay(smtp_server)?
            .credentials(creds)
            .build();
        
        match mailer.send(&email) {
            Ok(_) => {
                state.show_success = Some(format!("Email with invoice #{} sent successfully", invoice.number));
                Ok(())
            }
            Err(e) => {
                state.show_error = Some(format!("Failed to send email: {}", e));
                Ok(())
            }
        }
    } else {
        state.show_error = Some("Invoice data is missing".to_string());
        Ok(())
    }
} 