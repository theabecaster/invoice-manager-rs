use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::models::Client;

pub enum ClientWizardAction {
    Cancel,
    Save(Client),
}

#[derive(Clone, PartialEq, Copy)]
pub enum ClientField {
    Name,
    Email,
    Phone,
    Address,
}

pub struct ClientWizardState {
    pub profile_id: i32,
    pub client: Client,
    pub current_field: ClientField,
    pub editing: bool,
}

impl ClientWizardState {
    pub fn new(profile_id: i32) -> Self {
        Self {
            profile_id,
            client: Client {
                id: 0,
                profile_id,
                name: String::new(),
                email: String::new(),
                phone: String::new(),
                address: Some(String::new()),
            },
            current_field: ClientField::Name,
            editing: false,
        }
    }

    pub fn from_existing(client: Client) -> Self {
        Self {
            profile_id: client.profile_id,
            client,
            current_field: ClientField::Name,
            editing: false,
        }
    }

    pub fn profile_id(&self) -> i32 {
        self.profile_id
    }

    pub fn toggle_editing(&mut self) {
        self.editing = !self.editing;
    }

    pub fn next_field(&mut self) {
        self.current_field = match self.current_field {
            ClientField::Name => ClientField::Email,
            ClientField::Email => ClientField::Phone,
            ClientField::Phone => ClientField::Address,
            ClientField::Address => ClientField::Name,
        };
    }

    pub fn previous_field(&mut self) {
        self.current_field = match self.current_field {
            ClientField::Name => ClientField::Address,
            ClientField::Email => ClientField::Name,
            ClientField::Phone => ClientField::Email,
            ClientField::Address => ClientField::Phone,
        };
    }

    pub fn edit_current_field(&mut self, key: KeyCode) {
        if !self.editing {
            return;
        }

        let field_value = match self.current_field {
            ClientField::Name => &mut self.client.name,
            ClientField::Email => &mut self.client.email,
            ClientField::Phone => &mut self.client.phone,
            ClientField::Address => {
                if self.client.address.is_none() {
                    self.client.address = Some(String::new());
                }
                self.client.address.as_mut().unwrap()
            }
        };

        match key {
            KeyCode::Char(c) => {
                field_value.push(c);
            }
            KeyCode::Backspace => {
                field_value.pop();
            }
            _ => {}
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.client.name.is_empty() &&
        !self.client.email.is_empty() &&
        !self.client.phone.is_empty()
    }
}

pub fn render_client_wizard<B: Backend>(f: &mut Frame<B>, state: &mut ClientWizardState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.size());

    // Title with appropriate text based on whether we're editing or creating
    let title_text = if state.client.id == 0 {
        "Client Creation Wizard"
    } else {
        "Client Editing Wizard"
    };
    
    let title = Paragraph::new(title_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Form fields
    let form_area = chunks[1];
    render_form(f, state, form_area);

    // Help text
    let help_text = if state.editing {
        "Enter - Save field | Esc - Cancel editing"
    } else {
        "Enter - Edit field | Up/Down - Navigate fields | S - Save client | Esc - Cancel"
    };
    
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

fn render_form<B: Backend>(f: &mut Frame<B>, state: &mut ClientWizardState, area: Rect) {
    let field_names = [
        "Name",
        "Email",
        "Phone",
        "Address",
    ];

    // Create a new empty string for the case when address is None
    let empty_string = String::new();
    
    let field_values = [
        &state.client.name,
        &state.client.email,
        &state.client.phone,
        state.client.address.as_ref().unwrap_or(&empty_string),
    ];

    let items: Vec<ListItem> = field_names
        .iter()
        .zip(field_values.iter())
        .enumerate()
        .map(|(i, (name, value))| {
            let content = if i == state.current_field as usize && state.editing {
                Spans::from(vec![
                    Span::styled(
                        format!("{}: ", name),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(
                        format!("{}{}", value, if state.editing { "|" } else { "" }),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                let style = if i == state.current_field as usize {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };
                
                Spans::from(vec![
                    Span::styled(format!("{}: ", name), style),
                    Span::raw(value.as_str()),
                ])
            };

            ListItem::new(content)
        })
        .collect();

    let form_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Client Details"))
        .highlight_style(Style::default().fg(Color::Yellow));

    f.render_widget(form_list, area);
}

pub fn handle_input(state: &mut ClientWizardState) -> Result<Option<ClientWizardAction>> {
    if let Event::Key(key) = event::read()? {
        match key.code {
            KeyCode::Esc => {
                if state.editing {
                    state.toggle_editing();
                } else {
                    return Ok(Some(ClientWizardAction::Cancel));
                }
            }
            KeyCode::Enter => {
                if state.editing {
                    state.toggle_editing();
                } else {
                    state.toggle_editing();
                }
            }
            KeyCode::Up if !state.editing => {
                state.previous_field();
            }
            KeyCode::Down if !state.editing => {
                state.next_field();
            }
            KeyCode::Char('s') if !state.editing => {
                if state.is_valid() {
                    return Ok(Some(ClientWizardAction::Save(state.client.clone())));
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