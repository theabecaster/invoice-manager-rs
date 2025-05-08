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

use crate::models::Profile;

pub enum ProfileWizardAction {
    Cancel,
    Save(Profile),
}

#[derive(Clone, PartialEq, Copy)]
pub enum ProfileField {
    Name,
    Email,
    PhoneNumber,
    Address,
    BankName,
    BankAccountNumber,
    BankRoutingNumber,
}

pub struct ProfileWizardState {
    pub profile: Profile,
    pub current_field: ProfileField,
    pub editing: bool,
}

impl ProfileWizardState {
    pub fn new() -> Self {
        Self {
            profile: Profile {
                id: 0,
                name: String::new(),
                email: String::new(),
                phonenumber: String::new(),
                address: Some(String::new()),
                bank_name: String::new(),
                bank_account_number: String::new(),
                bank_routing_number: String::new(),
            },
            current_field: ProfileField::Name,
            editing: false,
        }
    }

    pub fn from_existing(profile: Profile) -> Self {
        Self {
            profile,
            current_field: ProfileField::Name,
            editing: false,
        }
    }

    pub fn toggle_editing(&mut self) {
        self.editing = !self.editing;
    }

    pub fn next_field(&mut self) {
        self.current_field = match self.current_field {
            ProfileField::Name => ProfileField::Email,
            ProfileField::Email => ProfileField::PhoneNumber,
            ProfileField::PhoneNumber => ProfileField::Address,
            ProfileField::Address => ProfileField::BankName,
            ProfileField::BankName => ProfileField::BankAccountNumber,
            ProfileField::BankAccountNumber => ProfileField::BankRoutingNumber,
            ProfileField::BankRoutingNumber => ProfileField::Name,
        };
    }

    pub fn previous_field(&mut self) {
        self.current_field = match self.current_field {
            ProfileField::Name => ProfileField::BankRoutingNumber,
            ProfileField::Email => ProfileField::Name,
            ProfileField::PhoneNumber => ProfileField::Email,
            ProfileField::Address => ProfileField::PhoneNumber,
            ProfileField::BankName => ProfileField::Address,
            ProfileField::BankAccountNumber => ProfileField::BankName,
            ProfileField::BankRoutingNumber => ProfileField::BankAccountNumber,
        };
    }

    pub fn edit_current_field(&mut self, key: KeyCode) {
        if !self.editing {
            return;
        }

        let field_value = match self.current_field {
            ProfileField::Name => &mut self.profile.name,
            ProfileField::Email => &mut self.profile.email,
            ProfileField::PhoneNumber => &mut self.profile.phonenumber,
            ProfileField::Address => {
                if self.profile.address.is_none() {
                    self.profile.address = Some(String::new());
                }
                self.profile.address.as_mut().unwrap()
            }
            ProfileField::BankName => &mut self.profile.bank_name,
            ProfileField::BankAccountNumber => &mut self.profile.bank_account_number,
            ProfileField::BankRoutingNumber => &mut self.profile.bank_routing_number,
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
        !self.profile.name.is_empty() &&
        !self.profile.email.is_empty() &&
        !self.profile.phonenumber.is_empty() &&
        !self.profile.bank_name.is_empty() &&
        !self.profile.bank_account_number.is_empty() &&
        !self.profile.bank_routing_number.is_empty()
    }
}

pub fn render_profile_wizard<B: Backend>(f: &mut Frame<B>, state: &mut ProfileWizardState) {
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
    let title_text = if state.profile.id == 0 {
        "Profile Creation Wizard"
    } else {
        "Profile Editing Wizard"
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
        "Enter - Edit field | Up/Down - Navigate fields | S - Save profile | Esc - Cancel"
    };
    
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

fn render_form<B: Backend>(f: &mut Frame<B>, state: &mut ProfileWizardState, area: Rect) {
    let field_names = [
        "Name",
        "Email",
        "Phone Number",
        "Address",
        "Bank Name",
        "Bank Account Number",
        "Bank Routing Number",
    ];

    let empty_string = String::new();
    
    let field_values = [
        &state.profile.name,
        &state.profile.email,
        &state.profile.phonenumber,
        state.profile.address.as_ref().unwrap_or(&empty_string),
        &state.profile.bank_name,
        &state.profile.bank_account_number,
        &state.profile.bank_routing_number,
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
        .block(Block::default().borders(Borders::ALL).title("Profile Details"))
        .highlight_style(Style::default().fg(Color::Yellow));

    f.render_widget(form_list, area);
}

pub fn handle_input(state: &mut ProfileWizardState) -> Result<Option<ProfileWizardAction>> {
    if let Event::Key(key) = event::read()? {
        match key.code {
            KeyCode::Esc => {
                if state.editing {
                    state.toggle_editing();
                } else {
                    return Ok(Some(ProfileWizardAction::Cancel));
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
                    return Ok(Some(ProfileWizardAction::Save(state.profile.clone())));
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