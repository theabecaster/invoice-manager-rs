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

use crate::models::Client;

// Represents the state of the client selection screen
pub struct ClientsState {
    profile_id: i32,
    clients: Vec<Client>,
    list_state: ListState,
    show_delete_confirmation: bool,
}

impl ClientsState {
    pub fn new(profile_id: i32, clients: Vec<Client>) -> Self {
        let mut list_state = ListState::default();
        if !clients.is_empty() {
            list_state.select(Some(0));
        }
        
        Self {
            profile_id,
            clients,
            list_state,
            show_delete_confirmation: false,
        }
    }

    pub fn next(&mut self) {
        if self.clients.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.clients.len() - 1 {
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
        if self.clients.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.clients.len() - 1
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

    pub fn selected_client(&self) -> Option<&Client> {
        self.list_state.selected().and_then(|i| self.clients.get(i))
    }
    
    pub fn selected_client_id(&self) -> Option<i32> {
        self.selected_client().map(|c| c.id)
    }
    
    pub fn profile_id(&self) -> i32 {
        self.profile_id
    }
}

pub enum ClientAction {
    Back,
    NewClient(i32), // Contains profile_id
    EditClient(i32), // Contains client_id
    DeleteClient(i32), // Contains client_id
    SelectClient(i32), // Contains client_id
}

// DB operations for clients
pub async fn load_clients_by_profile(db: &crate::db::Database, profile_id: i32) -> Result<Vec<Client>> {
    db.load_clients_by_profile(profile_id).await
}

pub async fn delete_client(db: &crate::db::Database, id: i32) -> Result<()> {
    db.delete_client(id).await
}

pub fn render_clients<B: Backend>(frame: &mut Frame<B>, state: &mut ClientsState) {
    let size = frame.size();
    
    // Create the layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
        ].as_ref())
        .split(size);

    // Create and render the clients list
    let items: Vec<ListItem> = state
        .clients
        .iter()
        .map(|client| {
            ListItem::new(Spans::from(vec![Span::raw(&client.name)]))
        })
        .collect();

    let clients_list = List::new(items)
        .block(Block::default().title("Clients").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(clients_list, chunks[0], &mut state.list_state);

    // Create and render the buttons
    let buttons_text = if state.selected_client().is_some() {
        format!("<N> New Client | <E> Edit Client | <D> Delete Client | <Enter> View Projects | <Esc> Back")
    } else {
        format!("<N> New Client | <Esc> Back")
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
        Spans::from("Are you sure you want to delete this client?"),
        Spans::from(""),
        Spans::from("All associated projects will also be deleted."),
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

pub fn handle_input(state: &mut ClientsState) -> Result<Option<ClientAction>> {
    if let Event::Key(key) = event::read()? {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if state.show_delete_confirmation {
                    state.toggle_delete_confirmation();
                } else {
                    return Ok(Some(ClientAction::Back));
                }
            }
            KeyCode::Char('n') => {
                if !state.show_delete_confirmation {
                    return Ok(Some(ClientAction::NewClient(state.profile_id())));
                }
            }
            KeyCode::Char('e') => {
                if !state.show_delete_confirmation && state.selected_client().is_some() {
                    if let Some(id) = state.selected_client_id() {
                        return Ok(Some(ClientAction::EditClient(id)));
                    }
                }
            }
            KeyCode::Char('d') => {
                if !state.show_delete_confirmation && state.selected_client().is_some() {
                    state.toggle_delete_confirmation();
                }
            }
            KeyCode::Char('y') => {
                if state.show_delete_confirmation {
                    if let Some(id) = state.selected_client_id() {
                        state.toggle_delete_confirmation();
                        return Ok(Some(ClientAction::DeleteClient(id)));
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
                    if let Some(id) = state.selected_client_id() {
                        return Ok(Some(ClientAction::SelectClient(id)));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(None)
} 