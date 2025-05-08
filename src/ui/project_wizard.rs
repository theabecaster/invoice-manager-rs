use anyhow::Result;
use chrono::{Datelike, NaiveDate};
use crossterm::event::{self, Event, KeyCode};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::models::Project;
use crate::ui::components::date_input::{DateInputState, DatePart};

pub enum ProjectWizardAction {
    Cancel,
    Save(Project),
}

#[derive(Clone, PartialEq, Copy)]
pub enum ProjectField {
    Name,
    StartDate,
    EndDate,
}

pub struct ProjectWizardState {
    pub client_id: i32,
    pub project: Project,
    pub current_field: ProjectField,
    pub editing: bool,
    pub start_date_state: DateInputState,
    pub end_date_state: DateInputState,
}

impl ProjectWizardState {
    pub fn new(client_id: i32) -> Self {
        let today = chrono::Local::now().date_naive();
        
        Self {
            client_id,
            project: Project {
                id: 0,
                client_id,
                name: String::new(),
                start_date: today,
                end_date: None,
            },
            current_field: ProjectField::Name,
            editing: false,
            start_date_state: DateInputState::new(today),
            end_date_state: DateInputState::new(today),
        }
    }

    pub fn from_existing(project: Project) -> Self {
        let end_date = project.end_date.unwrap_or(project.start_date);
        Self {
            client_id: project.client_id,
            project: project.clone(),
            current_field: ProjectField::Name,
            editing: false,
            start_date_state: DateInputState::new(project.start_date),
            end_date_state: DateInputState::new(end_date),
        }
    }

    pub fn client_id(&self) -> i32 {
        self.client_id
    }

    pub fn toggle_editing(&mut self) {
        self.editing = !self.editing;
        if self.editing {
            match self.current_field {
                ProjectField::StartDate => self.start_date_state.toggle_editing(),
                ProjectField::EndDate => self.end_date_state.toggle_editing(),
                _ => {}
            }
        } else {
            self.start_date_state.editing = false;
            self.end_date_state.editing = false;
        }
    }

    pub fn next_field(&mut self) {
        self.current_field = match self.current_field {
            ProjectField::Name => ProjectField::StartDate,
            ProjectField::StartDate => ProjectField::EndDate,
            ProjectField::EndDate => ProjectField::Name,
        };
    }

    pub fn previous_field(&mut self) {
        self.current_field = match self.current_field {
            ProjectField::Name => ProjectField::EndDate,
            ProjectField::StartDate => ProjectField::Name,
            ProjectField::EndDate => ProjectField::StartDate,
        };
    }

    pub fn edit_current_field(&mut self, key: KeyCode) {
        if !self.editing {
            return;
        }

        match self.current_field {
            ProjectField::Name => {
                match key {
                    KeyCode::Char(c) => {
                        self.project.name.push(c);
                    }
                    KeyCode::Backspace => {
                        self.project.name.pop();
                    }
                    _ => {}
                }
            }
            ProjectField::StartDate => {
                self.start_date_state.handle_input(key);
                self.project.start_date = self.start_date_state.date;
            }
            ProjectField::EndDate => {
                if self.project.end_date.is_none() {
                    self.project.end_date = Some(self.project.start_date);
                }
                self.end_date_state.handle_input(key);
                if let Some(end_date) = &mut self.project.end_date {
                    *end_date = self.end_date_state.date;
                }
            }
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.project.name.is_empty()
    }
}

pub fn render_project_wizard<B: Backend>(f: &mut Frame<B>, state: &mut ProjectWizardState) {
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
    let title_text = if state.project.id == 0 {
        "Project Creation Wizard"
    } else {
        "Project Editing Wizard"
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
        match state.current_field {
            ProjectField::Name => "Enter - Save field | Esc - Cancel editing",
            ProjectField::StartDate | ProjectField::EndDate => 
                "Enter - Save field | Left/Right - Switch date part | Esc - Cancel editing",
        }
    } else {
        "Enter - Edit field | Up/Down - Navigate fields | S - Save project | Esc - Cancel"
    };
    
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, chunks[2]);
}

fn render_form<B: Backend>(f: &mut Frame<B>, state: &mut ProjectWizardState, area: Rect) {
    let field_names = [
        "Name",
        "Start Date",
        "End Date",
    ];

    // Format dates
    let end_date_str = match &state.project.end_date {
        Some(date) => format!("{}", date.format("%Y-%m-%d")),
        None => "Not set".to_string(),
    };
    
    let field_values = [
        state.project.name.clone(),
        format!("{}", state.project.start_date.format("%Y-%m-%d")),
        end_date_str,
    ];

    let items: Vec<ListItem> = field_names
        .iter()
        .zip(field_values.iter())
        .enumerate()
        .map(|(i, (name, value))| {
            let content = if i == state.current_field as usize && state.editing {
                let displayed_value = if i == ProjectField::StartDate as usize {
                    state.start_date_state.get_display_string()
                } else if i == ProjectField::EndDate as usize {
                    state.end_date_state.get_display_string()
                } else {
                    format!("{}{}", value, if state.editing { "|" } else { "" })
                };
                
                Spans::from(vec![
                    Span::styled(
                        format!("{}: ", name),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::styled(
                        displayed_value,
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
                    Span::raw(value),
                ])
            };

            ListItem::new(content)
        })
        .collect();

    let form_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Project Details"))
        .highlight_style(Style::default().fg(Color::Yellow));

    f.render_widget(form_list, area);
}

pub fn handle_input(state: &mut ProjectWizardState) -> Result<Option<ProjectWizardAction>> {
    if let Event::Key(key) = event::read()? {
        match key.code {
            KeyCode::Esc => {
                if state.editing {
                    state.toggle_editing();
                } else {
                    return Ok(Some(ProjectWizardAction::Cancel));
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
                    return Ok(Some(ProjectWizardAction::Save(state.project.clone())));
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