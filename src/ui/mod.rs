mod app;
mod render;
mod events;

pub use app::App;
use render::draw;
use events::handle_key_event;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{
    io,
    thread,
    time::{Duration, Instant},
};

use crate::core::{
    local::{detect_package_managers_with_config, LocalPackageManager},
    config::Config,
    package_managers::Package,
};

pub async fn run_tui(initial_query: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();
    if let Some(query) = initial_query {
        app.search_input = query;
        app.cursor_position = app.search_input.len();
    }

    // Load configuration
    let config = Config::load().unwrap_or_default();
    
    // Detect package managers
    let managers = detect_package_managers_with_config(&config);
    app.package_managers = managers.clone();

    // Start background loading
    let (packages_tx, packages_rx) = std::sync::mpsc::channel();
    let (installed_tx, installed_rx) = std::sync::mpsc::channel();
    let (details_tx, details_rx) = std::sync::mpsc::channel();
    let (aur_tx, aur_rx) = std::sync::mpsc::channel();
    
    start_package_loading(managers, packages_tx, installed_tx);

    // Main loop
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(100); // 10 FPS - more reasonable for TUI
    
    loop {
        // Handle incoming packages
        if let Ok(packages) = packages_rx.try_recv() {
            app.set_packages(packages);
        }
        if let Ok(installed) = installed_rx.try_recv() {
            app.set_installed_packages(installed);
        }
        
        // Handle incoming package details
        if let Ok((package, details)) = details_rx.try_recv() {
            app.set_package_details(&package, details);
        }
        
        // Handle incoming AUR packages
        if let Ok(aur_packages) = aur_rx.try_recv() {
            app.add_aur_packages(aur_packages);
        }
        
        // Update search if debounce time has passed
        app.update_search_if_needed();
        
        // Trigger AUR search if search input has changed and contains text
        if !app.search_input.is_empty() && app.should_update_search() {
            let query = app.search_input.clone();
            if query.len() >= 2 { // Only search if query is at least 2 characters
                let aur_tx_clone = aur_tx.clone();
                tokio::spawn(async move {
                    if let Ok(aur_packages) = search_aur_async(&query).await {
                        let _ = aur_tx_clone.send(aur_packages);
                    }
                });
            }
        }
        
        // Fetch package details if needed
        if app.should_fetch_details() {
            if let Some(package) = app.get_selected_package().cloned() {
                if app.get_package_details(&package).is_none() {
                    fetch_package_details_async(package, details_tx.clone());
                }
            }
        }

        // Draw UI
        terminal.draw(|f| draw(f, &app))?;

        // Handle events
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    handle_key_event(&mut app, key);
                }
                Event::Resize(width, height) => {
                    app.terminal_size = (width, height);
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn start_package_loading(
    managers: Vec<LocalPackageManager>,
    packages_tx: std::sync::mpsc::Sender<Vec<Package>>,
    installed_tx: std::sync::mpsc::Sender<Vec<Package>>,
) {
    thread::spawn(move || {
        let mut all_packages = Vec::new();
        let mut all_installed = Vec::new();

        for manager in &managers {
            // Load installed packages
            if let Ok(mut installed) = manager.list_installed() {
                for pkg in &mut installed {
                    pkg.installed = true;
                }
                all_installed.extend(installed.clone());
                all_packages.extend(installed);
            }

            // Load available packages
            if let Ok(available) = manager.list_available() {
                all_packages.extend(available);
            }
        }

        // Send results
        let _ = installed_tx.send(all_installed);
        let _ = packages_tx.send(all_packages);
    });
}

fn fetch_package_details_async(
    package: Package,
    details_tx: std::sync::mpsc::Sender<(Package, String)>,
) {
    thread::spawn(move || {
        let details = match package.source.as_str() {
            "pacman" => {
                std::process::Command::new("pacman")
                    .args(&["-Si", &package.name])
                    .output()
                    .ok()
                    .filter(|output| output.status.success())
                    .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
                    .unwrap_or_else(|| format!("No details available for {}", package.name))
            }
            "paru" => {
                std::process::Command::new("paru")
                    .args(&["-Si", &package.name])
                    .output()
                    .ok()
                    .filter(|output| output.status.success())
                    .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
                    .unwrap_or_else(|| format!("No AUR details available for {}", package.name))
            }
            "dnf" => {
                std::process::Command::new("dnf")
                    .args(&["info", &package.name])
                    .output()
                    .ok()
                    .filter(|output| output.status.success())
                    .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
                    .unwrap_or_else(|| format!("No DNF details available for {}", package.name))
            }
            "emerge" => {
                std::process::Command::new("emerge")
                    .args(&["--info", &package.name])
                    .output()
                    .ok()
                    .filter(|output| output.status.success())
                    .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
                    .unwrap_or_else(|| format!("No Portage details available for {}", package.name))
            }
            "nix" => {
                std::process::Command::new("nix-env")
                    .args(&["-qa", "--description", &package.name])
                    .output()
                    .ok()
                    .filter(|output| output.status.success())
                    .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
                    .unwrap_or_else(|| format!("No Nix details available for {}", package.name))
            }
            "apt" => {
                std::process::Command::new("apt-cache")
                    .args(&["show", &package.name])
                    .output()
                    .ok()
                    .filter(|output| output.status.success())
                    .map(|output| String::from_utf8_lossy(&output.stdout).to_string())
                    .unwrap_or_else(|| format!("No APT details available for {}", package.name))
            }
            _ => format!("Package: {}\nSource: {}", package.name, package.source),
        };

        let _ = details_tx.send((package, details));
    });
}

async fn search_aur_async(query: &str) -> Result<Vec<Package>, Box<dyn std::error::Error + Send + Sync>> {
    use crate::core::aur::AurClient;
    let aur_client = AurClient::new();
    aur_client.search(query).await
}