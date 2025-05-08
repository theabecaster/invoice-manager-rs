use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::models::Project;
use crate::db::Database;

// Represents the state of the project selection screen
pub struct ProjectsState {
    client_id: i32,
    projects: Vec<Project>,
    list_state: ListState,
    show_delete_confirmation: bool,
}

impl ProjectsState {
    pub fn new(client_id: i32, projects: Vec<Project>) -> Self {
        let mut list_state = ListState::default();
        if !projects.is_empty() {
            list_state.select(Some(0));
        }
        
        Self {
            client_id,
            projects,
            list_state,
            show_delete_confirmation: false,
        }
    }

    pub fn next(&mut self) {
        if self.projects.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.projects.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.projects.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.projects.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn toggle_delete_confirmation(&mut self) {
        self.show_delete_confirmation = !self.show_delete_confirmation;
    }

    pub fn selected_project(&self) -> Option<&Project> {
        self.list_state.selected().and_then(|i| self.projects.get(i))
    }
    
    pub fn selected_project_id(&self) -> Option<i32> {
        self.selected_project().map(|p| p.id)
    }
    
    pub fn client_id(&self) -> i32 {
        self.client_id
    }
}

pub enum ProjectAction {
    Back,
    NewProject(i32), // Contains client_id
    EditProject(i32), // Contains project_id
    DeleteProject(i32), // Contains project_id
    SelectProject(i32), // Contains project_id
}

// DB operations for projects
pub async fn load_projects_by_client(db: &Database, client_id: i32) -> Result<Vec<Project>> {
    db.load_projects_by_client(client_id).await
}

pub async fn delete_project(db: &Database, id: i32) -> Result<()> {
    db.delete_project(id).await
}

pub fn render_projects<B: Backend>(
    frame: &mut Frame<B>,
    state: &mut ProjectsState,
) {
    // Create the layout
    let size = frame.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
        ].as_ref())
        .split(size);

    // Create and render the projects list
    let items: Vec<ListItem> = state
        .projects
        .iter()
        .map(|project| {
            let dates = if let Some(end_date) = project.end_date {
                format!("{} to {}", 
                       project.start_date.format("%Y-%m-%d"),
                       end_date.format("%Y-%m-%d"))
            } else {
                format!("{} to Present", 
                       project.start_date.format("%Y-%m-%d"))
            };
            
            ListItem::new(Spans::from(vec![
                Span::raw(&project.name),
                Span::raw(" ("),
                Span::raw(dates),
                Span::raw(")"),
            ]))
        })
        .collect();

    let projects_list = List::new(items)
        .block(Block::default().title("Projects").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(projects_list, chunks[0], &mut state.list_state);

    // Create and render the buttons
    let buttons_text = if state.selected_project().is_some() {
        "<N> New Project | <E> Edit Project | <D> Delete Project | <Enter> View Invoices | <Esc> Back".to_string()
    } else {
        "<N> New Project | <Esc> Back".to_string()
    };

    let buttons = Paragraph::new(buttons_text)
        .block(Block::default().borders(Borders::TOP))
        .style(Style::default().fg(Color::White));

    frame.render_widget(buttons, chunks[1]);

    // Render delete confirmation popup if needed
    if state.show_delete_confirmation {
        render_delete_confirmation(frame, size);
    }
}

fn render_delete_confirmation<B: Backend>(frame: &mut Frame<B>, size: Rect) {
    let popup_area = centered_rect(50, 20, size);
    
    let popup = Paragraph::new(vec![
        Spans::from(""),
        Spans::from("Are you sure you want to delete this project?"),
        Spans::from(""),
        Spans::from("All associated invoices will also be deleted."),
        Spans::from(""),
        Spans::from("<Y> Yes  <N> No"),
    ])
    .block(Block::default().title("Confirm Delete").borders(Borders::ALL))
    .style(Style::default().fg(Color::White).bg(Color::Black));
    
    frame.render_widget(popup, popup_area);
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

pub fn handle_input(state: &mut ProjectsState) -> Result<Option<ProjectAction>> {
    if let Event::Key(key) = event::read()? {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if state.show_delete_confirmation {
                    state.toggle_delete_confirmation();
                } else {
                    return Ok(Some(ProjectAction::Back));
                }
            }
            KeyCode::Char('n') => {
                if !state.show_delete_confirmation {
                    return Ok(Some(ProjectAction::NewProject(state.client_id())));
                }
            }
            KeyCode::Char('e') => {
                if !state.show_delete_confirmation && state.selected_project().is_some() {
                    if let Some(id) = state.selected_project_id() {
                        return Ok(Some(ProjectAction::EditProject(id)));
                    }
                }
            }
            KeyCode::Char('d') => {
                if !state.show_delete_confirmation && state.selected_project().is_some() {
                    state.toggle_delete_confirmation();
                }
            }
            KeyCode::Char('y') => {
                if state.show_delete_confirmation {
                    if let Some(id) = state.selected_project_id() {
                        state.toggle_delete_confirmation();
                        return Ok(Some(ProjectAction::DeleteProject(id)));
                    }
                }
            }
            KeyCode::Down => {
                if !state.show_delete_confirmation {
                    state.next();
                }
            }
            KeyCode::Up => {
                if !state.show_delete_confirmation {
                    state.previous();
                }
            }
            KeyCode::Enter => {
                if !state.show_delete_confirmation {
                    if let Some(id) = state.selected_project_id() {
                        return Ok(Some(ProjectAction::SelectProject(id)));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(None)
} 