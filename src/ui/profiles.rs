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

use crate::models::Profile;

// Represents the state of the profile selection screen
pub struct ProfilesState {
    profiles: Vec<Profile>,
    list_state: ListState,
    show_delete_confirmation: bool,
}

impl ProfilesState {
    pub fn new(profiles: Vec<Profile>) -> Self {
        let mut list_state = ListState::default();
        if !profiles.is_empty() {
            list_state.select(Some(0));
        }
        
        Self {
            profiles,
            list_state,
            show_delete_confirmation: false,
        }
    }

    pub fn next(&mut self) {
        if self.profiles.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.profiles.len() - 1 {
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
        if self.profiles.is_empty() {
            return;
        }

        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.profiles.len() - 1
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

    pub fn selected_profile(&self) -> Option<&Profile> {
        self.list_state.selected().and_then(|i| self.profiles.get(i))
    }
    
    pub fn selected_profile_id(&self) -> Option<i32> {
        self.selected_profile().map(|p| p.id)
    }
}

pub enum ProfileAction {
    Exit,
    NewProfile,
    DeleteProfile(i32),
    SelectProfile(i32),
    EditProfile(i32),
}

pub fn render_profiles<B: Backend>(frame: &mut Frame<B>, state: &mut ProfilesState) {
    let size = frame.size();
    
    // Create the layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
        ].as_ref())
        .split(size);

    // Create and render the profiles list
    let items: Vec<ListItem> = state
        .profiles
        .iter()
        .map(|profile| {
            ListItem::new(Spans::from(vec![Span::raw(&profile.name)]))
        })
        .collect();

    let profiles_list = List::new(items)
        .block(Block::default().title("Profiles").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(profiles_list, chunks[0], &mut state.list_state);

    // Create and render the buttons
    let buttons_text = if state.selected_profile().is_some() {
        format!("<N> New Profile | <E> Edit Profile | <D> Delete Profile | <Enter> View Clients | <Esc> Exit")
    } else {
        format!("<N> New Profile | <Esc> Exit")
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
        Spans::from("Are you sure you want to delete this profile?"),
        Spans::from(""),
        Spans::from("All associated clients and projects will also be deleted."),
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

pub fn handle_input(state: &mut ProfilesState) -> Result<Option<ProfileAction>> {
    if let Event::Key(key) = event::read()? {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if state.show_delete_confirmation {
                    state.toggle_delete_confirmation();
                } else {
                    return Ok(Some(ProfileAction::Exit));
                }
            }
            KeyCode::Char('n') => {
                if !state.show_delete_confirmation {
                    return Ok(Some(ProfileAction::NewProfile));
                }
            }
            KeyCode::Char('e') => {
                if !state.show_delete_confirmation && state.selected_profile().is_some() {
                    if let Some(id) = state.selected_profile_id() {
                        return Ok(Some(ProfileAction::EditProfile(id)));
                    }
                }
            }
            KeyCode::Char('d') => {
                if !state.show_delete_confirmation && state.selected_profile().is_some() {
                    state.toggle_delete_confirmation();
                }
            }
            KeyCode::Char('y') => {
                if state.show_delete_confirmation {
                    if let Some(id) = state.selected_profile_id() {
                        state.toggle_delete_confirmation();
                        return Ok(Some(ProfileAction::DeleteProfile(id)));
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
                    if let Some(id) = state.selected_profile_id() {
                        return Ok(Some(ProfileAction::SelectProfile(id)));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(None)
} 