use anyhow::Result;
use chrono::{Local, NaiveDate};
use crossterm::event::{self, Event, KeyCode};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::models::{Invoice, InvoiceLineItem};
use crate::ui::components::date_input::{DateInputState, DatePart};

// Represents a field in the invoice form
#[derive(Clone, Copy, PartialEq)]
pub enum InvoiceField {
    SubmitDate,
    DueDate,
    Rate,
    LineItems,
}

// Represents a field being edited in the line items step
#[derive(Clone, Copy, PartialEq)]
pub enum LineItemField {
    Description,
    Hours,
    None,
}

// Represents the wizard state
pub struct InvoiceWizardState {
    project_id: i32,
    invoice_id: Option<i32>,
    submit_date: NaiveDate,
    due_date: NaiveDate,
    rate: f64,
    line_items: Vec<InvoiceLineItem>,
    current_field: InvoiceField,
    line_items_list_state: ListState,
    editing_line_item: Option<(usize, LineItemField, String)>, // (index, field, current value)
    editing: bool,
    active_input: String,
    show_error: Option<String>,
    submit_date_state: DateInputState,
    due_date_state: DateInputState,
}

impl InvoiceWizardState {
    pub fn new(project_id: i32, invoice_id: Option<i32>, existing_invoice: Option<Invoice>, existing_line_items: Option<Vec<InvoiceLineItem>>) -> Self {
        let today = Local::now().date_naive();
        let five_days_later = today + chrono::Duration::days(5);
        
        let mut state = Self {
            project_id,
            invoice_id,
            submit_date: today,
            due_date: five_days_later,
            rate: 0.0,
            line_items: Vec::new(),
            current_field: InvoiceField::SubmitDate,
            line_items_list_state: ListState::default(),
            editing_line_item: None,
            editing: false,
            active_input: String::new(),
            show_error: None,
            submit_date_state: DateInputState::new(today),
            due_date_state: DateInputState::new(five_days_later),
        };
        
        // If editing an existing invoice, load its data
        if let Some(invoice) = existing_invoice {
            state.submit_date = invoice.submit_date;
            state.due_date = invoice.due_date;
            state.rate = invoice.rate;
            state.submit_date_state = DateInputState::new(invoice.submit_date);
            state.due_date_state = DateInputState::new(invoice.due_date);
            
            if let Some(items) = existing_line_items {
                state.line_items = items;
                if !state.line_items.is_empty() {
                    state.line_items_list_state.select(Some(0));
                }
            }
        }
        
        state
    }
    
    pub fn toggle_editing(&mut self) {
        self.editing = !self.editing;
        
        // Handle date fields specially
        if self.editing {
            match self.current_field {
                InvoiceField::SubmitDate => {
                    self.submit_date_state.toggle_editing();
                },
                InvoiceField::DueDate => {
                    self.due_date_state.toggle_editing();
                },
                InvoiceField::Rate => {
                    self.active_input = self.rate.to_string();
                },
                InvoiceField::LineItems => {
                    // Keep line items as they are
                }
            }
        } else {
            self.submit_date_state.editing = false;
            self.due_date_state.editing = false;
            self.editing_line_item = None;
        }
    }
    
    pub fn next_field(&mut self) {
        self.current_field = match self.current_field {
            InvoiceField::SubmitDate => InvoiceField::DueDate,
            InvoiceField::DueDate => InvoiceField::Rate,
            InvoiceField::Rate => InvoiceField::LineItems,
            InvoiceField::LineItems => InvoiceField::SubmitDate,
        };
    }
    
    pub fn previous_field(&mut self) {
        self.current_field = match self.current_field {
            InvoiceField::SubmitDate => InvoiceField::LineItems,
            InvoiceField::DueDate => InvoiceField::SubmitDate,
            InvoiceField::Rate => InvoiceField::DueDate,
            InvoiceField::LineItems => InvoiceField::Rate,
        };
    }
    
    pub fn add_line_item(&mut self) {
        let new_id = if let Some(last) = self.line_items.last() {
            last.id + 1
        } else {
            1
        };
        
        let line_item = InvoiceLineItem {
            id: new_id,
            invoice_id: self.invoice_id.unwrap_or(0), // Will be updated when invoice is saved
            description: String::new(),
            hours: 0.0,
        };
        
        self.line_items.push(line_item);
        self.line_items_list_state.select(Some(self.line_items.len() - 1));
        self.editing_line_item = Some((
            self.line_items.len() - 1,
            LineItemField::Description,
            String::new(),
        ));
    }
    
    pub fn edit_line_item(&mut self) {
        if let Some(selected) = self.line_items_list_state.selected() {
            if selected < self.line_items.len() {
                self.editing_line_item = Some((
                    selected,
                    LineItemField::Description,
                    self.line_items[selected].description.clone(),
                ));
            }
        }
    }
    
    pub fn delete_line_item(&mut self) {
        if let Some(selected) = self.line_items_list_state.selected() {
            if selected < self.line_items.len() {
                self.line_items.remove(selected);
                
                // Adjust selection after deletion
                if !self.line_items.is_empty() {
                    let new_selection = if selected >= self.line_items.len() {
                        self.line_items.len() - 1
                    } else {
                        selected
                    };
                    self.line_items_list_state.select(Some(new_selection));
                } else {
                    self.line_items_list_state.select(None);
                }
                
                self.editing_line_item = None;
            }
        }
    }
    
    pub fn next_field_in_line_item(&mut self) {
        if let Some((idx, field, value)) = &self.editing_line_item {
            let idx = *idx;
            match field {
                LineItemField::Description => {
                    // Save current value and move to Hours field
                    if idx < self.line_items.len() {
                        self.line_items[idx].description = value.clone();
                        self.editing_line_item = Some((
                            idx,
                            LineItemField::Hours,
                            self.line_items[idx].hours.to_string(),
                        ));
                    }
                }
                LineItemField::Hours => {
                    // Save current value and finish editing
                    if idx < self.line_items.len() {
                        match value.parse::<f64>() {
                            Ok(hours) => {
                                self.line_items[idx].hours = hours;
                                self.editing_line_item = None;
                            }
                            Err(_) => {
                                self.show_error = Some("Invalid hours. Please enter a valid number.".to_string());
                            }
                        }
                    }
                }
                LineItemField::None => {}
            }
        }
    }
    
    pub fn edit_current_field(&mut self, key: KeyCode) {
        if !self.editing {
            return;
        }

        match self.current_field {
            InvoiceField::SubmitDate => {
                self.submit_date_state.handle_input(key);
                self.submit_date = self.submit_date_state.date;
            }
            InvoiceField::DueDate => {
                self.due_date_state.handle_input(key);
                self.due_date = self.due_date_state.date;
            }
            InvoiceField::Rate => {
                match key {
                    KeyCode::Char(c) if c.is_digit(10) || c == '.' => {
                        self.active_input.push(c);
                    }
                    KeyCode::Backspace => {
                        self.active_input.pop();
                    }
                    _ => {}
                }
            }
            InvoiceField::LineItems => {
                if let Some((_, _, ref mut value)) = self.editing_line_item {
                    match key {
                        KeyCode::Char(c) => {
                            value.push(c);
                        }
                        KeyCode::Backspace => {
                            value.pop();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    
    pub fn to_invoice(&self) -> Invoice {
        // Generate a new invoice number (in real app, this would be more sophisticated)
        let number = self.invoice_id.unwrap_or_else(|| {
            // Simple logic for demo purposes
            let timestamp = Local::now().timestamp() as i32;
            timestamp % 10000 // Last 4 digits of timestamp
        });
        
        Invoice {
            id: self.invoice_id.unwrap_or(0), // DB will assign real id for new invoices
            project_id: self.project_id,
            number,
            submit_date: self.submit_date,
            due_date: self.due_date,
            rate: if self.active_input.is_empty() { 
                self.rate 
            } else { 
                self.active_input.parse().unwrap_or(self.rate) 
            },
            status: "Draft".to_string(),
        }
    }
    
    pub fn is_valid(&self) -> bool {
        // Basic validation
        let rate_valid = if self.active_input.is_empty() {
            self.rate > 0.0
        } else {
            self.active_input.parse::<f64>().unwrap_or(0.0) > 0.0
        };
        
        !self.line_items.is_empty() && rate_valid
    }
}

pub enum InvoiceWizardAction {
    Cancel,
    Save(Invoice, Vec<InvoiceLineItem>),
}

pub fn render_invoice_wizard<B: Backend>(frame: &mut Frame<B>, state: &mut InvoiceWizardState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3),  // Title
                Constraint::Min(10),    // Form
                Constraint::Length(3),  // Help
            ]
            .as_ref(),
        )
        .split(frame.size());

    // Title with appropriate text based on whether we're editing or creating
    let title_text = if state.invoice_id.is_some() {
        "Invoice Editing Wizard"
    } else {
        "Invoice Creation Wizard"
    };
    
    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Form
    let form_area = chunks[1];
    render_form(frame, state, form_area);

    // Help text
    let help_text = match (state.editing, state.current_field) {
        (false, _) => "Enter - Edit field | Up/Down - Navigate fields | S - Save invoice | Esc - Cancel",
        (true, InvoiceField::SubmitDate | InvoiceField::DueDate) => 
            "Enter - Save field | Left/Right - Switch date part | Esc - Cancel editing",
        (true, InvoiceField::Rate) => 
            "Enter - Save field | Esc - Cancel editing",
        (true, InvoiceField::LineItems) => {
            if state.editing_line_item.is_some() {
                "Enter - Next field | Tab - Next field | Esc - Cancel editing"
            } else {
                "A - Add item | E - Edit selected | D - Delete selected | Enter - Done | Esc - Cancel"
            }
        }
    };
    
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, chunks[2]);
    
    // Show error if needed
    if let Some(error) = &state.show_error {
        render_error(frame, frame.size(), error);
    }
}

fn render_form<B: Backend>(frame: &mut Frame<B>, state: &mut InvoiceWizardState, area: Rect) {
    let form_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),  // Submit Date
                Constraint::Length(3),  // Due Date
                Constraint::Length(3),  // Rate
                Constraint::Min(6),     // Line Items
            ]
            .as_ref(),
        )
        .split(area);
    
    // Submit Date
    let submit_date_style = if state.current_field == InvoiceField::SubmitDate {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    
    let submit_date_value = if state.current_field == InvoiceField::SubmitDate && state.editing {
        state.submit_date_state.get_display_string()
    } else {
        format!("{}", state.submit_date.format("%Y-%m-%d"))
    };
    
    let submit_date = Paragraph::new(Spans::from(vec![
        Span::styled("Submit Date: ", submit_date_style),
        Span::raw(submit_date_value),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(submit_date, form_chunks[0]);
    
    // Due Date
    let due_date_style = if state.current_field == InvoiceField::DueDate {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    
    let due_date_value = if state.current_field == InvoiceField::DueDate && state.editing {
        state.due_date_state.get_display_string()
    } else {
        format!("{}", state.due_date.format("%Y-%m-%d"))
    };
    
    let due_date = Paragraph::new(Spans::from(vec![
        Span::styled("Due Date: ", due_date_style),
        Span::raw(due_date_value),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(due_date, form_chunks[1]);
    
    // Rate
    let rate_style = if state.current_field == InvoiceField::Rate {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };
    
    let rate_value = if state.current_field == InvoiceField::Rate && state.editing {
        format!("{}{}", state.active_input, if state.editing { "|" } else { "" })
    } else {
        format!("{:.2}", state.rate)
    };
    
    let rate = Paragraph::new(Spans::from(vec![
        Span::styled("Hourly Rate: $", rate_style),
        Span::raw(rate_value),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(rate, form_chunks[2]);
    
    // Line Items
    let line_items_block = Block::default()
        .title(if state.current_field == InvoiceField::LineItems {
            "Line Items (selected)"
        } else {
            "Line Items"
        })
        .borders(Borders::ALL)
        .style(if state.current_field == InvoiceField::LineItems {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });
    
    if state.current_field == InvoiceField::LineItems && state.editing {
        if let Some((idx, field, value)) = &state.editing_line_item {
            // Editing a line item
            let line_items_area = line_items_block.inner(form_chunks[3]);
            frame.render_widget(line_items_block, form_chunks[3]);
            
            let edit_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Description
                    Constraint::Length(3),  // Hours
                ])
                .split(line_items_area);
            
            // Description field
            let desc_style = if *field == LineItemField::Description {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            
            let desc_value = if *field == LineItemField::Description {
                format!("{}{}", value, if *field == LineItemField::Description { "|" } else { "" })
            } else if *idx < state.line_items.len() {
                state.line_items[*idx].description.clone()
            } else {
                String::new()
            };
            
            let desc_paragraph = Paragraph::new(Spans::from(vec![
                Span::raw("Description: "),
                Span::styled(desc_value, desc_style),
            ]))
            .block(Block::default().borders(Borders::ALL));
            frame.render_widget(desc_paragraph, edit_chunks[0]);
            
            // Hours field
            let hours_style = if *field == LineItemField::Hours {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            
            let hours_value = if *field == LineItemField::Hours {
                format!("{}{}", value, if *field == LineItemField::Hours { "|" } else { "" })
            } else if *idx < state.line_items.len() {
                state.line_items[*idx].hours.to_string()
            } else {
                String::new()
            };
            
            let hours_paragraph = Paragraph::new(Spans::from(vec![
                Span::raw("Hours: "),
                Span::styled(hours_value, hours_style),
            ]))
            .block(Block::default().borders(Borders::ALL));
            frame.render_widget(hours_paragraph, edit_chunks[1]);
            
        } else {
            // Viewing line items with controls
            let line_items = state.line_items
                .iter()
                .map(|item| {
                    ListItem::new(format!("{}: {} hours (${:.2})", 
                                      item.description, 
                                      item.hours, 
                                      item.hours * state.rate))
                })
                .collect::<Vec<_>>();
            
            let list = List::new(line_items)
                .block(line_items_block)
                .highlight_style(Style::default().bg(Color::Blue).fg(Color::White));
            
            frame.render_stateful_widget(list, form_chunks[3], &mut state.line_items_list_state);
        }
    } else {
        // Just showing line items as part of the form
        let mut content = Vec::new();
        
        // Calculate total
        let total_hours: f64 = state.line_items.iter().map(|item| item.hours).sum();
        let total_amount = total_hours * state.rate;
        
        if state.line_items.is_empty() {
            content.push(Spans::from("No line items added yet"));
        } else {
            for item in &state.line_items {
                content.push(Spans::from(format!("- {}: {} hours (${:.2})", 
                                        item.description, 
                                        item.hours, 
                                        item.hours * state.rate)));
            }
            
            content.push(Spans::from(""));
            content.push(Spans::from(format!("Total Hours: {}", total_hours)));
            content.push(Spans::from(format!("Total Amount: ${:.2}", total_amount)));
        }
        
        let paragraph = Paragraph::new(content)
            .block(line_items_block);
        
        frame.render_widget(paragraph, form_chunks[3]);
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

pub fn handle_input(state: &mut InvoiceWizardState) -> Result<Option<InvoiceWizardAction>> {
    // Clear any existing error message
    state.show_error = None;
    
    if let Event::Key(key) = event::read()? {
        match key.code {
            KeyCode::Esc => {
                if state.editing {
                    state.toggle_editing();
                } else {
                    return Ok(Some(InvoiceWizardAction::Cancel));
                }
            }
            KeyCode::Enter => {
                if state.editing {
                    if state.current_field == InvoiceField::Rate {
                        // Validate rate
                        if let Ok(rate) = state.active_input.parse::<f64>() {
                            state.rate = rate;
                            state.toggle_editing();
                        } else {
                            state.show_error = Some("Invalid rate. Please enter a valid number.".to_string());
                        }
                    } else if state.current_field == InvoiceField::LineItems {
                        if state.editing_line_item.is_some() {
                            state.next_field_in_line_item();
                        } else {
                            state.toggle_editing();
                        }
                    } else {
                        state.toggle_editing();
                    }
                } else {
                    state.toggle_editing();
                }
            }
            KeyCode::Char('s') if !state.editing => {
                if state.is_valid() {
                    let invoice = state.to_invoice();
                    return Ok(Some(InvoiceWizardAction::Save(
                        invoice,
                        state.line_items.clone(),
                    )));
                } else {
                    state.show_error = Some("Please complete all required fields. Rate must be > 0 and at least one line item is required.".to_string());
                }
            }
            KeyCode::Char('a') => {
                if state.current_field == InvoiceField::LineItems && state.editing && state.editing_line_item.is_none() {
                    state.add_line_item();
                } else if state.editing {
                    state.edit_current_field(key.code);
                }
            }
            KeyCode::Char('e') => {
                if state.current_field == InvoiceField::LineItems && state.editing && 
                   state.editing_line_item.is_none() && state.line_items_list_state.selected().is_some() {
                    state.edit_line_item();
                } else if state.editing {
                    state.edit_current_field(key.code);
                }
            }
            KeyCode::Char('d') => {
                if state.current_field == InvoiceField::LineItems && state.editing && 
                   state.editing_line_item.is_none() && state.line_items_list_state.selected().is_some() {
                    state.delete_line_item();
                } else if state.editing {
                    state.edit_current_field(key.code);
                }
            }
            KeyCode::Tab => {
                if state.current_field == InvoiceField::LineItems && 
                   state.editing && state.editing_line_item.is_some() {
                    state.next_field_in_line_item();
                }
            }
            KeyCode::Up if !state.editing => {
                state.previous_field();
            }
            KeyCode::Down if !state.editing => {
                state.next_field();
            }
            KeyCode::Up if state.current_field == InvoiceField::LineItems && 
                          state.editing && state.editing_line_item.is_none() => {
                let len = state.line_items.len();
                if len > 0 {
                    let i = match state.line_items_list_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                len - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    state.line_items_list_state.select(Some(i));
                }
            }
            KeyCode::Down if state.current_field == InvoiceField::LineItems && 
                            state.editing && state.editing_line_item.is_none() => {
                let len = state.line_items.len();
                if len > 0 {
                    let i = match state.line_items_list_state.selected() {
                        Some(i) => {
                            if i >= len - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    state.line_items_list_state.select(Some(i));
                }
            }
            _ if state.editing => {
                state.edit_current_field(key.code);
            }
            _ => {}
        }
    }
    
    Ok(None)
}

pub async fn save_invoice_with_line_items(
    db: &crate::db::Database, 
    invoice: &Invoice, 
    line_items: &[InvoiceLineItem]
) -> Result<i32> {
    // Use the database layer's method instead of direct access
    let invoice_id = db.save_invoice_with_line_items(invoice, line_items).await?;
    
    // No longer generating invoice files here - will be done on-demand when email wizard is opened
    
    Ok(invoice_id)
}

pub async fn get_invoice_with_line_items(db: &crate::db::Database, id: i32) -> Result<(Invoice, Vec<InvoiceLineItem>)> {
    // Use the database layer instead of direct access
    db.get_invoice_with_line_items(id).await
} 