mod config;
mod db;
mod models;
mod ui;
mod invoice_gen;

use std::io;
use anyhow::Result;
use crossterm::{
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

use crate::ui::{
    profiles::{ProfilesState, ProfileAction, render_profiles, handle_input as handle_profiles_input},
    clients::{ClientsState, ClientAction, render_clients, handle_input as handle_clients_input, load_clients_by_profile},
    projects::{ProjectsState, ProjectAction, render_projects, handle_input as handle_projects_input, load_projects_by_client},
    invoices::{InvoicesState, InvoiceAction, render_invoices, handle_input as handle_invoices_input, load_invoices_by_project},
    invoice_wizard::{InvoiceWizardState, InvoiceWizardAction, render_invoice_wizard, handle_input as handle_invoice_wizard_input, save_invoice_with_line_items, get_invoice_with_line_items},
    profile_wizard::{ProfileWizardState, ProfileWizardAction, render_profile_wizard, handle_input as handle_profile_wizard_input},
    client_wizard::{ClientWizardState, ClientWizardAction, render_client_wizard, handle_input as handle_client_wizard_input},
    project_wizard::{ProjectWizardState, ProjectWizardAction, render_project_wizard, handle_input as handle_project_wizard_input},
};

// Represents the current screen in the app
enum AppScreen {
    Profiles,
    ProfileWizard,
    Clients(i32),         // Contains profile_id
    ClientWizard(i32),    // Contains profile_id
    Projects(i32),        // Contains client_id
    ProjectWizard(i32),   // Contains client_id
    Invoices(i32),        // Contains project_id
    InvoiceWizard(i32, Option<i32>),  // Contains project_id and optional invoice_id
}

// Main application state
struct AppState {
    db: db::Database,
    screen: AppScreen,
    profiles_state: Option<ProfilesState>,
    profile_wizard_state: Option<ProfileWizardState>,
    clients_state: Option<ClientsState>,
    client_wizard_state: Option<ClientWizardState>,
    projects_state: Option<ProjectsState>,
    project_wizard_state: Option<ProjectWizardState>,
    invoices_state: Option<InvoicesState>,
    invoice_wizard_state: Option<InvoiceWizardState>,
}

impl AppState {
    fn new(db: db::Database) -> Self {
        Self {
            db,
            screen: AppScreen::Profiles,
            profiles_state: None,
            profile_wizard_state: None,
            clients_state: None,
            client_wizard_state: None,
            projects_state: None,
            project_wizard_state: None,
            invoices_state: None,
            invoice_wizard_state: None,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = config::init()?;
    println!("Initializing invoice manager...");
    
    // Initialize database connection
    let db = db::init(&config).await?;
    println!("Database connection established");
    
    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create app state
    let mut app_state = AppState::new(db);
    
    // Initialize the profiles state
    load_profiles_screen(&mut app_state).await?;
    
    // Run the main app loop
    let result = run_app(&mut terminal, &mut app_state).await;
    
    // Restore terminal
    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    // Show any error message
    if let Err(err) = result {
        println!("Error: {}", err);
    }
    
    println!("Thanks for using Invoice Manager!");
    
    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app_state: &mut AppState) -> Result<()> {
    loop {
        // Render current screen
        terminal.draw(|f| {
            match app_state.screen {
                AppScreen::Profiles => {
                    if let Some(state) = &mut app_state.profiles_state {
                        render_profiles(f, state);
                    }
                }
                AppScreen::ProfileWizard => {
                    if let Some(state) = &mut app_state.profile_wizard_state {
                        render_profile_wizard(f, state);
                    }
                }
                AppScreen::Clients(_) => {
                    if let Some(state) = &mut app_state.clients_state {
                        render_clients(f, state);
                    }
                }
                AppScreen::ClientWizard(_) => {
                    if let Some(state) = &mut app_state.client_wizard_state {
                        render_client_wizard(f, state);
                    }
                }
                AppScreen::Projects(_) => {
                    if let Some(state) = &mut app_state.projects_state {
                        render_projects(f, state);
                    }
                }
                AppScreen::ProjectWizard(_) => {
                    if let Some(state) = &mut app_state.project_wizard_state {
                        render_project_wizard(f, state);
                    }
                }
                AppScreen::Invoices(_) => {
                    if let Some(state) = &mut app_state.invoices_state {
                        render_invoices(f, state);
                    }
                }
                AppScreen::InvoiceWizard(_, _) => {
                    if let Some(state) = &mut app_state.invoice_wizard_state {
                        render_invoice_wizard(f, state);
                    }
                }
            }
        })?;
        
        // Handle input for current screen
        let should_quit = match app_state.screen {
            AppScreen::Profiles => handle_profiles_screen(app_state).await?,
            AppScreen::ProfileWizard => handle_profile_wizard_screen(app_state).await?,
            AppScreen::Clients(_) => handle_clients_screen(app_state).await?,
            AppScreen::ClientWizard(_) => handle_client_wizard_screen(app_state).await?,
            AppScreen::Projects(_) => handle_projects_screen(app_state).await?,
            AppScreen::ProjectWizard(_) => handle_project_wizard_screen(app_state).await?,
            AppScreen::Invoices(_) => handle_invoices_screen(app_state).await?,
            AppScreen::InvoiceWizard(_, _) => handle_invoice_wizard_screen(app_state).await?,
        };
        
        if should_quit {
            break;
        }
    }
    
    Ok(())
}

async fn load_profiles_screen(app_state: &mut AppState) -> Result<()> {
    // Load profiles from database
    let profiles = app_state.db.load_profiles().await?;
    
    // Create profiles state
    app_state.profiles_state = Some(ProfilesState::new(profiles));
    app_state.screen = AppScreen::Profiles;
    
    Ok(())
}

async fn handle_profiles_screen(app_state: &mut AppState) -> Result<bool> {
    if let Some(state) = &mut app_state.profiles_state {
        match handle_profiles_input(state)? {
            Some(ProfileAction::Exit) => {
                return Ok(true);
            }
            Some(ProfileAction::SelectProfile(profile_id)) => {
                // Load clients for the selected profile
                let clients = load_clients_by_profile(&app_state.db, profile_id).await?;
                
                // Create clients state
                app_state.clients_state = Some(ClientsState::new(profile_id, clients));
                app_state.screen = AppScreen::Clients(profile_id);
            }
            Some(ProfileAction::DeleteProfile(profile_id)) => {
                // Delete profile from database
                app_state.db.delete_profile(profile_id).await?;
                
                // Reload profiles
                load_profiles_screen(app_state).await?;
            }
            Some(ProfileAction::EditProfile(profile_id)) => {
                // Load the profile from database
                let profile = app_state.db.get_profile(profile_id).await?;
                
                // Create profile wizard state for editing
                app_state.profile_wizard_state = Some(ProfileWizardState::from_existing(profile));
                app_state.screen = AppScreen::ProfileWizard;
            }
            Some(ProfileAction::NewProfile) => {
                // Create a new profile wizard state
                app_state.profile_wizard_state = Some(ProfileWizardState::new());
                app_state.screen = AppScreen::ProfileWizard;
            }
            None => {}
        }
    }
    
    Ok(false)
}

async fn handle_clients_screen(app_state: &mut AppState) -> Result<bool> {
    if let Some(state) = &mut app_state.clients_state {
        match handle_clients_input(state)? {
            Some(ClientAction::Back) => {
                // Go back to profiles screen
                load_profiles_screen(app_state).await?;
            }
            Some(ClientAction::SelectClient(client_id)) => {
                // Load projects for the selected client
                let projects = load_projects_by_client(&app_state.db, client_id).await?;
                
                // Create projects state
                app_state.projects_state = Some(ProjectsState::new(client_id, projects));
                app_state.screen = AppScreen::Projects(client_id);
            }
            Some(ClientAction::DeleteClient(client_id)) => {
                // Delete client from database
                app_state.db.delete_client(client_id).await?;
                
                // Reload clients
                let profile_id = state.profile_id();
                let clients = load_clients_by_profile(&app_state.db, profile_id).await?;
                app_state.clients_state = Some(ClientsState::new(profile_id, clients));
            }
            Some(ClientAction::EditClient(client_id)) => {
                // Load the client from database
                let client = app_state.db.get_client(client_id).await?;
                
                // Store the profile_id before moving the client
                let profile_id = client.profile_id;
                
                // Create client wizard state for editing
                app_state.client_wizard_state = Some(ClientWizardState::from_existing(client));
                app_state.screen = AppScreen::ClientWizard(profile_id);
            }
            Some(ClientAction::NewClient(profile_id)) => {
                // Create client wizard state
                app_state.client_wizard_state = Some(ClientWizardState::new(profile_id));
                app_state.screen = AppScreen::ClientWizard(profile_id);
            }
            None => {}
        }
    }
    
    Ok(false)
}

async fn handle_projects_screen(app_state: &mut AppState) -> Result<bool> {
    if let Some(state) = &mut app_state.projects_state {
        match handle_projects_input(state)? {
            Some(ProjectAction::Back) => {
                // Go back to clients screen
                let client_id = state.client_id();
                let client = app_state.db.get_client(client_id).await?;
                let profile_id = client.profile_id;
                
                // Reload clients
                let clients = load_clients_by_profile(&app_state.db, profile_id).await?;
                app_state.clients_state = Some(ClientsState::new(profile_id, clients));
                app_state.screen = AppScreen::Clients(profile_id);
            }
            Some(ProjectAction::SelectProject(project_id)) => {
                // Load invoices for the selected project
                let invoices = load_invoices_by_project(&app_state.db, project_id).await?;
                
                // Get the project to access its name
                let project = app_state.db.get_project(project_id).await?;
                
                // Create invoices state
                app_state.invoices_state = Some(InvoicesState::new(project_id, project.name, invoices));
                app_state.screen = AppScreen::Invoices(project_id);
            }
            Some(ProjectAction::DeleteProject(project_id)) => {
                // Delete project from database
                app_state.db.delete_project(project_id).await?;
                
                // Reload projects
                let client_id = state.client_id();
                let projects = load_projects_by_client(&app_state.db, client_id).await?;
                app_state.projects_state = Some(ProjectsState::new(client_id, projects));
            }
            Some(ProjectAction::EditProject(project_id)) => {
                // Load the project from database
                let project = app_state.db.get_project(project_id).await?;
                
                // Store the client_id before moving the project
                let client_id = project.client_id;
                
                // Create project wizard state for editing
                app_state.project_wizard_state = Some(ProjectWizardState::from_existing(project));
                app_state.screen = AppScreen::ProjectWizard(client_id);
            }
            Some(ProjectAction::NewProject(client_id)) => {
                // Create a new project wizard state
                app_state.project_wizard_state = Some(ProjectWizardState::new(client_id));
                app_state.screen = AppScreen::ProjectWizard(client_id);
            }
            None => {}
        }
    }
    
    Ok(false)
}

async fn handle_invoices_screen(app_state: &mut AppState) -> Result<bool> {
    if let Some(state) = &mut app_state.invoices_state {
        // Before handling input, make sure any lingering email wizard state is cleared
        if state.is_in_email_wizard() {
            // Force the email wizard to close if it's active
            state.force_close_email_wizard().await?;
        }
        
        match handle_invoices_input(&app_state.db, state).await? {
            Some(InvoiceAction::Back) => {
                // Ensure email wizard is properly cleaned up before switching screens
                if state.is_in_email_wizard() {
                    state.force_close_email_wizard().await?;
                }
                
                // Go back to projects screen
                let project_id = state.project_id();
                let project = app_state.db.get_project(project_id).await?;
                let client_id = project.client_id;
                
                // Reload projects
                let projects = load_projects_by_client(&app_state.db, client_id).await?;
                app_state.projects_state = Some(ProjectsState::new(client_id, projects));
                app_state.screen = AppScreen::Projects(client_id);
            }
            Some(InvoiceAction::EditInvoice(invoice_id)) => {
                // Load invoice data
                let (invoice, line_items) = get_invoice_with_line_items(&app_state.db, invoice_id).await?;
                let project_id = invoice.project_id;
                
                // Create invoice wizard state for editing
                app_state.invoice_wizard_state = Some(InvoiceWizardState::new(
                    project_id,
                    Some(invoice_id),
                    Some(invoice),
                    Some(line_items),
                ));
                app_state.screen = AppScreen::InvoiceWizard(project_id, Some(invoice_id));
            }
            Some(InvoiceAction::NewInvoice(project_id)) => {
                // Create new invoice wizard state
                app_state.invoice_wizard_state = Some(InvoiceWizardState::new(
                    project_id,
                    None,
                    None,
                    None,
                ));
                app_state.screen = AppScreen::InvoiceWizard(project_id, None);
            }
            Some(InvoiceAction::EmailInvoice(_)) => {
                // This is handled within the InvoicesState with its email_wizard_state
                // in the updated invoices module
            }
            None => {}
        }
    }
    
    Ok(false)
}

async fn handle_invoice_wizard_screen(app_state: &mut AppState) -> Result<bool> {
    if let Some(state) = &mut app_state.invoice_wizard_state {
        match handle_invoice_wizard_input(state)? {
            Some(InvoiceWizardAction::Cancel) => {
                // Go back to invoices screen
                if let AppScreen::InvoiceWizard(project_id, _) = app_state.screen {
                    // Reload invoices
                    let invoices = load_invoices_by_project(&app_state.db, project_id).await?;
                    
                    // Get the project to access its name
                    let project = app_state.db.get_project(project_id).await?;
                    
                    app_state.invoices_state = Some(InvoicesState::new(project_id, project.name, invoices));
                    app_state.screen = AppScreen::Invoices(project_id);
                }
            }
            Some(InvoiceWizardAction::Save(invoice, line_items)) => {
                // Save the invoice
                save_invoice_with_line_items(&app_state.db, &invoice, &line_items).await?;
                
                // Go back to invoices screen
                if let AppScreen::InvoiceWizard(project_id, _) = app_state.screen {
                    // Reload invoices
                    let invoices = load_invoices_by_project(&app_state.db, project_id).await?;
                    
                    // Get the project to access its name
                    let project = app_state.db.get_project(project_id).await?;
                    
                    app_state.invoices_state = Some(InvoicesState::new(project_id, project.name, invoices));
                    app_state.screen = AppScreen::Invoices(project_id);
                }
            }
            None => {}
        }
    }
    
    Ok(false)
}

async fn handle_profile_wizard_screen(app_state: &mut AppState) -> Result<bool> {
    if let Some(state) = &mut app_state.profile_wizard_state {
        match handle_profile_wizard_input(state)? {
            Some(ProfileWizardAction::Cancel) => {
                // Go back to profiles screen
                load_profiles_screen(app_state).await?;
            }
            Some(ProfileWizardAction::Save(profile)) => {
                if profile.id == 0 {
                    // Create new profile
                    app_state.db.create_profile(&profile).await?;
                } else {
                    // Update existing profile
                    app_state.db.update_profile(&profile).await?;
                }
                
                // Reload profiles
                load_profiles_screen(app_state).await?;
            }
            None => {}
        }
    }
    
    Ok(false)
}

async fn handle_client_wizard_screen(app_state: &mut AppState) -> Result<bool> {
    if let Some(state) = &mut app_state.client_wizard_state {
        match handle_client_wizard_input(state)? {
            Some(ClientWizardAction::Cancel) => {
                // Go back to clients screen
                if let AppScreen::ClientWizard(profile_id) = app_state.screen {
                    // Reload clients
                    let clients = load_clients_by_profile(&app_state.db, profile_id).await?;
                    app_state.clients_state = Some(ClientsState::new(profile_id, clients));
                    app_state.screen = AppScreen::Clients(profile_id);
                }
            }
            Some(ClientWizardAction::Save(client)) => {
                if client.id == 0 {
                    // Create new client
                    app_state.db.create_client(&client).await?;
                } else {
                    // Update existing client
                    app_state.db.update_client(&client).await?;
                }
                
                // Reload clients and go back to clients screen
                if let AppScreen::ClientWizard(profile_id) = app_state.screen {
                    let clients = load_clients_by_profile(&app_state.db, profile_id).await?;
                    app_state.clients_state = Some(ClientsState::new(profile_id, clients));
                    app_state.screen = AppScreen::Clients(profile_id);
                }
            }
            None => {}
        }
    }
    
    Ok(false)
}

async fn handle_project_wizard_screen(app_state: &mut AppState) -> Result<bool> {
    if let Some(state) = &mut app_state.project_wizard_state {
        match handle_project_wizard_input(state)? {
            Some(ProjectWizardAction::Cancel) => {
                // Go back to projects screen
                let client_id = state.client_id();
                let projects = load_projects_by_client(&app_state.db, client_id).await?;
                app_state.projects_state = Some(ProjectsState::new(client_id, projects));
                app_state.screen = AppScreen::Projects(client_id);
            }
            Some(ProjectWizardAction::Save(project)) => {
                if project.id == 0 {
                    // Create new project
                    app_state.db.create_project(&project).await?;
                } else {
                    // Update existing project
                    app_state.db.update_project(&project).await?;
                }
                
                // Reload projects
                let projects = load_projects_by_client(&app_state.db, project.client_id).await?;
                app_state.projects_state = Some(ProjectsState::new(project.client_id, projects));
                app_state.screen = AppScreen::Projects(project.client_id);
            }
            None => {}
        }
    }
    
    Ok(false)
}
