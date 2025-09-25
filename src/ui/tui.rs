use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, MouseEvent},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};
use std::io::{self, BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc;
use arboard::Clipboard;
use crate::core::package_managers::{Package, PackageManagerRegistry};
use crate::core::config::Config;

pub struct App {
    pub query: String,
    pub packages: Arc<Mutex<Vec<Package>>>, // Shared cached packages
    pub filtered_packages: Vec<Package>, // Packages matching current search
    pub selected_packages: Vec<Package>,
    pub cursor_position: usize,
    pub scroll_offset: usize,
    pub active_unit: ActiveUnit,
    pub should_quit: bool,
    pub installed_packages: Vec<Package>,
    pub terminal_output: Arc<Mutex<Vec<String>>>,
    pub config: Config,
    pub pm_registry: PackageManagerRegistry,
    pub terminal_scroll: usize,
    pub packages_loaded: Arc<Mutex<bool>>,
    pub loading_in_progress: bool,
    pub package_receiver: Option<mpsc::Receiver<Vec<Package>>>,
    pub installed_scroll: usize,
    pub installed_cursor: usize,
    pub package_cache: std::collections::HashMap<String, Vec<Package>>, // Cache per PM
}

#[derive(Clone, Copy, PartialEq)]
pub enum ActiveUnit {
    Results,
    Input,
    Description,
    InstalledList,
    Terminal,
}

impl App {
    pub fn new(initial_query: Option<String>) -> Result<Self, Box<dyn std::error::Error>> {
        let config = Config::load().unwrap_or_else(|_| {
            eprintln!("Warning: Could not load config, using defaults");
            Config::default()
        });
        
        let config_dir = Config::get_config_dir().unwrap_or_else(|_| {
            std::path::PathBuf::from("/tmp/pmux")
        });
        
        let pm_registry = PackageManagerRegistry::load_from_config_dir(&config_dir).unwrap_or_else(|_| {
            eprintln!("Warning: Could not load package manager configs, using empty registry");
            PackageManagerRegistry::new()
        });
        
        let (package_sender, package_receiver) = mpsc::channel();
        
        let mut app = Self {
            query: initial_query.unwrap_or_default(),
            packages: Arc::new(Mutex::new(Vec::new())),
            filtered_packages: Vec::new(),
            selected_packages: Vec::new(),
            cursor_position: 0,
            scroll_offset: 0,
            active_unit: ActiveUnit::Input, // Start in input mode
            should_quit: false,
            installed_packages: Vec::new(),
            terminal_output: Arc::new(Mutex::new(Vec::new())),
            config,
            pm_registry,
            terminal_scroll: 0,
            packages_loaded: Arc::new(Mutex::new(false)),
            loading_in_progress: false,
            package_receiver: Some(package_receiver),
            installed_scroll: 0,
            installed_cursor: 0,
            package_cache: std::collections::HashMap::new(),
        };
        
        // Add welcome message
        if let Ok(mut output) = app.terminal_output.lock() {
            output.push("pmux - Package Manager Multiplexer".to_string());
            output.push("Terminal Unit: All command output appears here".to_string());
            output.push("Ctrl+C: Copy all, y: Copy visible, j/k: Scroll".to_string());
            output.push("Type to search packages, Enter to install selected".to_string());
            output.push("Loading package managers...".to_string());
            output.push("".to_string());
        }
        
        // Start with just installed packages for fast startup
        app.load_installed_packages_only();
        
        // Load packages in background
        app.start_background_loading(package_sender);
        
        Ok(app)
    }
    
    fn load_installed_packages_only(&mut self) {
        self.installed_packages.clear();
        
        for manager in self.pm_registry.get_enabled_managers(&self.config.pm.enabled_pm) {
            if let Ok(mut output) = self.terminal_output.lock() {
                output.push(format!("Loading installed packages from {}...", manager.name));
            }
            
            match self.pm_registry.list_installed(manager) {
                Ok(installed) => {
                    if let Ok(mut output) = self.terminal_output.lock() {
                        output.push(format!("Loaded {} installed packages from {}", installed.len(), manager.name));
                    }
                    self.installed_packages.extend(installed);
                }
                Err(e) => {
                    if let Ok(mut output) = self.terminal_output.lock() {
                        output.push(format!("Failed to load installed packages from {}: {}", manager.name, e));
                    }
                }
            }
        }
        
        // Start with installed packages in the cache
        if let Ok(mut packages) = self.packages.lock() {
            *packages = self.installed_packages.clone();
        }
        self.filter_packages();
    }
    
    fn start_background_loading(&mut self, package_sender: mpsc::Sender<Vec<Package>>) {
        if self.loading_in_progress {
            return;
        }
        
        self.loading_in_progress = true;
        
        if let Ok(mut output) = self.terminal_output.lock() {
            output.push("Loading packages in background...".to_string());
        }
        
        // Clone what we need for the background thread
        let enabled_pm = self.config.pm.enabled_pm.clone();
        let terminal_output = Arc::clone(&self.terminal_output);
        let packages_loaded = Arc::clone(&self.packages_loaded);
        let packages_cache = Arc::clone(&self.packages);
        
        // Create a new registry for the background thread (can't clone the existing one)
        let config_dir = Config::get_config_dir().unwrap_or_else(|_| {
            std::path::PathBuf::from("/tmp/pmux")
        });
        
        thread::spawn(move || {
            // Load registry in background thread
            let pm_registry = match PackageManagerRegistry::load_from_config_dir(&config_dir) {
                Ok(registry) => registry,
                Err(_) => {
                    if let Ok(mut output) = terminal_output.lock() {
                        output.push("Failed to load package manager registry in background".to_string());
                    }
                    return;
                }
            };
            
            let enabled_managers = pm_registry.get_enabled_managers(&enabled_pm);
            let mut all_packages = Vec::new();
            
            for manager in enabled_managers {
                if let Ok(mut output) = terminal_output.lock() {
                    output.push(format!("Loading packages from {} (executable: {})...", manager.name, manager.executable));
                }
                
                match pm_registry.list_packages(manager) {
                    Ok(packages) => {
                        if let Ok(mut output) = terminal_output.lock() {
                            output.push(format!("Loaded {} packages from {}", packages.len(), manager.name));
                            if packages.len() > 0 {
                                output.push(format!("Sample package: {}", packages[0].name));
                            }
                        }
                        
                        // Cache packages per PM to avoid conflicts
                        // TODO: Send cache update to main thread
                        all_packages.extend(packages);
                    }
                    Err(e) => {
                        if let Ok(mut output) = terminal_output.lock() {
                            output.push(format!("Failed to load packages from {}: {}", manager.name, e));
                        }
                    }
                }
            }
            
            // Update the shared cache
            if let Ok(mut packages) = packages_cache.lock() {
                *packages = all_packages.clone();
            }
            
            // Send packages to main thread
            let _ = package_sender.send(all_packages);
            
            // Mark as loaded
            if let Ok(mut loaded) = packages_loaded.lock() {
                *loaded = true;
            }
            
            if let Ok(mut output) = terminal_output.lock() {
                output.push("Package loading complete!".to_string());
            }
        });
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        crossterm::execute!(
            stdout,
            EnterAlternateScreen
        )?;
        
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        let mut list_state = ListState::default();
        
        while !self.should_quit {
            // Check for new packages from background loading
            if let Some(ref receiver) = self.package_receiver {
                if let Ok(new_packages) = receiver.try_recv() {
                    if let Ok(mut packages) = self.packages.lock() {
                        *packages = new_packages;
                    }
                    // Re-filter with new packages if we have a query
                    if !self.query.is_empty() {
                        self.filter_packages();
                    }
                }
            }
            
            terminal.draw(|frame| {
                self.draw_ui(frame, &mut list_state);
            })?;
            self.handle_events()?;
        }

        disable_raw_mode()?;
        crossterm::execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen
        )?;
        Ok(())
    }

    fn draw_ui(&mut self, frame: &mut ratatui::Frame, list_state: &mut ListState) {
        let size = frame.size();
        
        // Check minimum size
        if size.width < 40 || size.height < 10 {
            let text = format!("Terminal too small: {}x{}", size.width, size.height);
            let paragraph = Paragraph::new(text);
            frame.render_widget(paragraph, size);
            return;
        }
        
        // Calculate layout based on config
        let right_col_percent = self.config.layout.right_column_width_percent;
        let input_height = self.config.layout.input_field_height;
        
        // Main horizontal split
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(100 - right_col_percent),
                Constraint::Percentage(right_col_percent),
            ])
            .split(size);
        
        // Left column split (Results, Input, Description)
        let left_vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(5),                          // Results (expandable)
                Constraint::Length(input_height),            // Input (fixed)
                Constraint::Min(3),                          // Description (expandable)
            ])
            .split(horizontal[0]);
        
        // Right column split (Installed list, Terminal)
        let right_vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(self.config.layout.installed_list_percent),
                Constraint::Percentage(self.config.layout.terminal_percent),
            ])
            .split(horizontal[1]);
        
        // Update list state selection
        list_state.select(if self.filtered_packages.is_empty() { None } else { Some(self.cursor_position) });
        
        // Draw all components
        self.draw_results_list(frame, left_vertical[0], list_state);
        self.draw_input_field_widget(frame, left_vertical[1]);
        self.draw_description_widget(frame, left_vertical[2]);
        self.draw_installed_list_widget(frame, right_vertical[0]);
        self.draw_terminal_widget(frame, right_vertical[1]);
    }

    fn filter_packages(&mut self) {
        if self.query.is_empty() {
            // Show installed packages when no query
            self.filtered_packages = self.installed_packages.clone();
        } else if self.query.len() < 2 {
            // Don't search with very short queries
            self.filtered_packages.clear();
        } else {
            // Fast local search through cached packages
            self.perform_cached_search();
        }
        
        // Reset cursor to first result when filtering
        self.cursor_position = 0;
        self.scroll_offset = 0;
    }
    
    fn perform_cached_search(&mut self) {
        let packages = if let Ok(packages) = self.packages.lock() {
            packages.clone()
        } else {
            return;
        };
        
        let query_lower = self.query.to_lowercase();
        
        // Check for package manager filter (e.g., "nix*", "dnf*")
        if query_lower.ends_with('*') {
            let pm_name = &query_lower[..query_lower.len() - 1];
            let search_results: Vec<Package> = packages
                .iter()
                .filter(|package| package.source == pm_name)
                .cloned()
                .collect();
            
            self.filtered_packages = search_results.into_iter().take(500).collect();
            return;
        }
        
        // Check if query specifies a package manager (e.g., "nix firefox")
        let parts: Vec<&str> = self.query.split_whitespace().collect();
        
        let search_results: Vec<Package> = if parts.len() >= 2 {
            let pm_name = parts[0];
            let search_term = parts[1..].join(" ").to_lowercase();
            
            packages
                .iter()
                .filter(|package| {
                    package.source == pm_name && 
                    package.name.to_lowercase().contains(&search_term)
                })
                .cloned()
                .collect()
        } else {
            // Single term search across all packages
            packages
                .iter()
                .filter(|package| {
                    package.name.to_lowercase().contains(&query_lower) ||
                    package.description.as_ref().map_or(false, |desc| desc.to_lowercase().contains(&query_lower))
                })
                .cloned()
                .collect()
        };
        
        // Limit results to prevent UI slowdown and sort by relevance
        let mut limited_results: Vec<Package> = search_results.into_iter().take(500).collect();
        
        // Sort by name match quality (exact matches first, then starts_with, then contains)
        limited_results.sort_by(|a, b| {
            let a_exact = a.name.to_lowercase() == query_lower;
            let b_exact = b.name.to_lowercase() == query_lower;
            if a_exact != b_exact {
                return b_exact.cmp(&a_exact);
            }
            
            let a_starts = a.name.to_lowercase().starts_with(&query_lower);
            let b_starts = b.name.to_lowercase().starts_with(&query_lower);
            if a_starts != b_starts {
                return b_starts.cmp(&a_starts);
            }
            
            a.name.cmp(&b.name)
        });
        
        self.filtered_packages = limited_results;
    }
    
    fn draw_results_list(&self, frame: &mut ratatui::Frame, area: Rect, list_state: &mut ListState) {
        let items: Vec<ListItem> = self.filtered_packages
            .iter()
            .map(|package| {
                let is_selected = self.selected_packages.iter().any(|p| p.name == package.name && p.source == package.source);
                let (prefix, style) = if is_selected {
                    ("* ", Style::default().fg(Color::Yellow))
                } else if package.installed {
                    ("* ", Style::default().fg(Color::Green))
                } else {
                    ("  ", Style::default())
                };
                let content = format!("{}{:<40} {}*", prefix, package.name, package.source);
                ListItem::new(content).style(style)
            })
            .collect();
            
        let block = Block::default()
            .title(" Results ")
            .borders(Borders::ALL)
            .border_style(if self.active_unit == ActiveUnit::Results {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            });
            
        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");
            
        frame.render_stateful_widget(list, area, list_state);
    }
    
    fn draw_input_field_widget(&self, frame: &mut ratatui::Frame, area: Rect) {
        let packages_loaded = self.packages_loaded.lock().map(|loaded| *loaded).unwrap_or(false);
        let package_count = self.packages.lock().map(|p| p.len()).unwrap_or(0);
        
        let status = if !packages_loaded {
            " (loading packages...)".to_string()
        } else if self.query.len() > 0 && self.query.len() < 2 {
            " (type 2+ chars, use pm* to filter)".to_string()
        } else if self.query.len() >= 2 && self.filtered_packages.is_empty() {
            " (no matches)".to_string()
        } else if self.query.ends_with('*') {
            let pm_name = &self.query[..self.query.len() - 1];
            format!(" (showing {} packages)", pm_name)
        } else {
            format!(" (cached: {})", package_count)
        };
        
        let input_text = format!("({}/{}) >> {}{}", 
            self.selected_packages.len(), 
            self.filtered_packages.len(), 
            self.query,
            status
        );
        
        let block = Block::default()
            .title(" Input ")
            .borders(Borders::ALL)
            .border_style(if self.active_unit == ActiveUnit::Input {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            });
            
        let paragraph = Paragraph::new(input_text)
            .block(block)
            .wrap(Wrap { trim: true });
            
        frame.render_widget(paragraph, area);
    }
    
    fn draw_description_widget(&self, frame: &mut ratatui::Frame, area: Rect) {
        let content = if let Some(package) = self.filtered_packages.get(self.cursor_position) {
            let mut lines = vec![
                Line::from(Span::styled(
                    format!("Package: {}", package.name),
                    Style::default().fg(Color::Yellow)
                )),
                Line::from(format!("Source: {}", package.source)),
            ];
            
            if let Some(ref version) = package.version {
                lines.push(Line::from(format!("Version: {}", version)));
            }
            
            if let Some(ref description) = package.description {
                lines.push(Line::from(""));
                lines.push(Line::from(description.as_str()));
            }
            
            lines
        } else {
            vec![Line::from("No package selected")]
        };
        
        let block = Block::default()
            .title(" Description ")
            .borders(Borders::ALL)
            .border_style(if self.active_unit == ActiveUnit::Description {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            });
            
        let paragraph = Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: false });
            
        frame.render_widget(paragraph, area);
    }
    
    fn draw_installed_list_widget(&self, frame: &mut ratatui::Frame, area: Rect) {
        let visible_lines = (area.height as usize).saturating_sub(3); // Account for borders and header
        let start_idx = self.installed_scroll;
        let end_idx = (start_idx + visible_lines).min(self.installed_packages.len());
        
        let mut content = vec![
            Line::from(format!("Count: {} (scroll: {})", self.installed_packages.len(), self.installed_scroll)),
        ];
        
        if self.installed_packages.is_empty() {
            content.push(Line::from("No installed packages found"));
        } else {
            for (i, package) in self.installed_packages.iter().enumerate().skip(start_idx).take(visible_lines) {
                let prefix = if self.active_unit == ActiveUnit::InstalledList && i == self.installed_cursor {
                    "> ✓ "
                } else {
                    "  ✓ "
                };
                let style = if self.active_unit == ActiveUnit::InstalledList && i == self.installed_cursor {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                content.push(Line::from(Span::styled(format!("{}{}", prefix, package.name), style)));
            }
        }
        
        let scroll_indicator = if self.installed_packages.len() > visible_lines {
            format!(" Installed ({}/{}) ", end_idx, self.installed_packages.len())
        } else {
            " Installed ".to_string()
        };
        
        let block = Block::default()
            .title(scroll_indicator)
            .borders(Borders::ALL)
            .border_style(if self.active_unit == ActiveUnit::InstalledList {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            });
            
        let paragraph = Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: false });
            
        frame.render_widget(paragraph, area);
    }
    
    fn draw_terminal_widget(&self, frame: &mut ratatui::Frame, area: Rect) {
        let content: Vec<Line> = if let Ok(output) = self.terminal_output.lock() {
            let visible_lines = (area.height as usize).saturating_sub(2); // Account for borders
            
            // Show latest messages by default, unless user has scrolled up
            let start_idx = if output.len() > visible_lines {
                if self.terminal_scroll == 0 {
                    // Auto-scroll: show latest messages
                    output.len().saturating_sub(visible_lines)
                } else {
                    // User scrolled: show from scrolled position
                    output.len().saturating_sub(visible_lines).saturating_sub(self.terminal_scroll)
                }
            } else {
                0
            };
            
            output
                .iter()
                .skip(start_idx)
                .take(visible_lines)
                .map(|line| Line::from(line.clone()))
                .collect()
        } else {
            vec![Line::from("Terminal output unavailable")]
        };
        
        let title = if self.terminal_scroll > 0 {
            format!(" Terminal (↑{}) [Ctrl+C: copy, y: copy visible, mouse: select text] ", self.terminal_scroll)
        } else {
            " Terminal [Ctrl+C: copy, y: copy visible, mouse: select text] ".to_string()
        };
            
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(if self.active_unit == ActiveUnit::Terminal {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            });
            
        let paragraph = Paragraph::new(content)
            .block(block)
            .wrap(Wrap { trim: false });
            
        frame.render_widget(paragraph, area);
    }

    fn handle_events(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Reduce polling frequency to save CPU - 30 FPS is plenty for a TUI
        if event::poll(std::time::Duration::from_millis(33))? {
            match event::read()? {
                Event::Key(key) => {
                    self.handle_key_event(key);
                }
                // Mouse events disabled for text selection
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        // Auto-focus input field when typing (except in terminal)
        if let KeyCode::Char(c) = key.code {
            if self.active_unit != ActiveUnit::Terminal && self.active_unit != ActiveUnit::Input {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == '*' {
                    self.active_unit = ActiveUnit::Input;
                    self.query.push(c);
                    self.filter_packages();
                    return;
                }
            }
        }
        
        match self.active_unit {
            ActiveUnit::Input => {
                match key.code {
                    // Quit
                    KeyCode::Esc => {
                        if self.query.is_empty() {
                            self.should_quit = true;
                        } else {
                            self.query.clear();
                            self.filter_packages();
                        }
                    }
                    KeyCode::Char('q') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        self.should_quit = true;
                    }
                    // Navigation (vim bindings)
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.active_unit = ActiveUnit::Results;
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        self.active_unit = ActiveUnit::Results;
                        if !self.filtered_packages.is_empty() {
                            self.cursor_position = self.filtered_packages.len() - 1;
                        }
                    }
                    // Tab switching
                    KeyCode::Tab => self.switch_unit(),
                    // Enter to install
                    KeyCode::Enter => {
                        if !self.filtered_packages.is_empty() {
                            self.install_selected();
                        }
                    }
                    // Text input
                    KeyCode::Char(c) => {
                        self.query.push(c);
                        self.filter_packages();
                    }
                    KeyCode::Backspace => {
                        self.query.pop();
                        self.filter_packages();
                    }
                    _ => {}
                }
            }
            ActiveUnit::Results => {
                match key.code {
                    // Quit
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Esc => self.active_unit = ActiveUnit::Input,
                    // Navigation (vim bindings)
                    KeyCode::Char('j') | KeyCode::Down => self.move_cursor_down(),
                    KeyCode::Char('k') | KeyCode::Up => self.move_cursor_up(),
                    KeyCode::Char('g') => {
                        if !self.filtered_packages.is_empty() {
                            self.cursor_position = 0;
                            self.scroll_offset = 0;
                        }
                    }
                    KeyCode::Char('G') => {
                        if !self.filtered_packages.is_empty() {
                            self.cursor_position = self.filtered_packages.len() - 1;
                        }
                    }
                    // Selection
                    KeyCode::Char(' ') => self.toggle_selection(),
                    KeyCode::Enter => self.install_selected(),
                    // Switch to input
                    KeyCode::Char('i') | KeyCode::Char('/') => {
                        self.active_unit = ActiveUnit::Input;
                    }
                    // Tab switching
                    KeyCode::Tab => self.switch_unit(),
                    _ => {}
                }
            }
            ActiveUnit::Terminal => {
                match key.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Esc | KeyCode::Char('i') => self.active_unit = ActiveUnit::Input,
                    KeyCode::Tab => self.switch_unit(),
                    // Terminal scrolling
                    KeyCode::Char('j') | KeyCode::Down => {
                        if let Ok(_output) = self.terminal_output.lock() {
                            if self.terminal_scroll > 0 {
                                self.terminal_scroll -= 1;
                            }
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if let Ok(output) = self.terminal_output.lock() {
                            let max_scroll = output.len().saturating_sub(10);
                            if self.terminal_scroll < max_scroll {
                                self.terminal_scroll += 1;
                            }
                        }
                    }
                    KeyCode::Char('g') => self.terminal_scroll = 0,
                    KeyCode::Char('G') => {
                        if let Ok(output) = self.terminal_output.lock() {
                            self.terminal_scroll = output.len().saturating_sub(10);
                        }
                    }
                    // Copy terminal content to clipboard
                    KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        if let Ok(output) = self.terminal_output.lock() {
                            let content = output.join("\n");
                            if let Ok(mut clipboard) = Clipboard::new() {
                                let _ = clipboard.set_text(content);
                            }
                        }
                    }

                    // Copy visible terminal content
                    KeyCode::Char('y') => {
                        if let Ok(output) = self.terminal_output.lock() {
                            let visible_lines = 10; // Approximate
                            let start_idx = if output.len() > visible_lines {
                                if self.terminal_scroll == 0 {
                                    output.len().saturating_sub(visible_lines)
                                } else {
                                    output.len().saturating_sub(visible_lines).saturating_sub(self.terminal_scroll)
                                }
                            } else {
                                0
                            };
                            
                            let visible_content: Vec<String> = output
                                .iter()
                                .skip(start_idx)
                                .take(visible_lines)
                                .cloned()
                                .collect();
                            
                            if let Ok(mut clipboard) = Clipboard::new() {
                                let _ = clipboard.set_text(visible_content.join("\n"));
                            }
                        }
                    }
                    _ => {}
                }
            }
            ActiveUnit::InstalledList => {
                match key.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Esc | KeyCode::Char('i') => self.active_unit = ActiveUnit::Input,
                    KeyCode::Tab => self.switch_unit(),
                    // Installed list scrolling
                    KeyCode::Char('j') | KeyCode::Down => {
                        if self.installed_cursor < self.installed_packages.len().saturating_sub(1) {
                            self.installed_cursor += 1;
                            // Auto-scroll if cursor goes off screen
                            let visible_lines = 10; // Approximate
                            if self.installed_cursor >= self.installed_scroll + visible_lines {
                                self.installed_scroll += 1;
                            }
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if self.installed_cursor > 0 {
                            self.installed_cursor -= 1;
                            if self.installed_cursor < self.installed_scroll {
                                self.installed_scroll = self.installed_cursor;
                            }
                        }
                    }
                    KeyCode::Char('g') => {
                        self.installed_cursor = 0;
                        self.installed_scroll = 0;
                    }
                    KeyCode::Char('G') => {
                        self.installed_cursor = self.installed_packages.len().saturating_sub(1);
                        let visible_lines = 10;
                        self.installed_scroll = self.installed_packages.len().saturating_sub(visible_lines);
                    }
                    _ => {}
                }
            }
            _ => {
                // Other units: basic navigation
                match key.code {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Esc | KeyCode::Char('i') => self.active_unit = ActiveUnit::Input,
                    KeyCode::Tab => self.switch_unit(),
                    _ => {}
                }
            }
        }
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        use crossterm::event::{MouseEventKind, MouseButton};
        
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Determine which unit was clicked based on coordinates
                // This is approximate - you'd need to track actual widget areas for precision
                let x = mouse.column;
                let y = mouse.row;
                
                // Rough layout detection (adjust based on your actual layout)
                if x < 50 { // Left column
                    if y < 20 { // Top area - Results
                        self.active_unit = ActiveUnit::Results;
                        // Just move cursor, no auto-selection
                        if y > 2 && !self.filtered_packages.is_empty() {
                            let clicked_index = (y as usize).saturating_sub(3);
                            if clicked_index < self.filtered_packages.len() {
                                self.cursor_position = clicked_index;
                            }
                        }
                    } else if y < 25 { // Middle area - Input
                        self.active_unit = ActiveUnit::Input;
                    } else { // Bottom area - Description
                        self.active_unit = ActiveUnit::Description;
                    }
                } else { // Right column
                    if y < 20 { // Top right - Installed list
                        self.active_unit = ActiveUnit::InstalledList;
                        // Calculate which installed package was clicked
                        if y > 2 && !self.installed_packages.is_empty() {
                            let clicked_index = (y as usize).saturating_sub(3) + self.installed_scroll;
                            if clicked_index < self.installed_packages.len() {
                                self.installed_cursor = clicked_index;
                            }
                        }
                    } else { // Bottom right - Terminal
                        self.active_unit = ActiveUnit::Terminal;
                    }
                }
            }
            MouseEventKind::ScrollUp => {
                match self.active_unit {
                    ActiveUnit::Results => self.move_cursor_up(),
                    ActiveUnit::InstalledList => {
                        if self.installed_cursor > 0 {
                            self.installed_cursor -= 1;
                            if self.installed_cursor < self.installed_scroll {
                                self.installed_scroll = self.installed_cursor;
                            }
                        }
                    }
                    ActiveUnit::Terminal => {
                        if let Ok(output) = self.terminal_output.lock() {
                            let max_scroll = output.len().saturating_sub(10);
                            if self.terminal_scroll < max_scroll {
                                self.terminal_scroll += 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
            MouseEventKind::ScrollDown => {
                match self.active_unit {
                    ActiveUnit::Results => self.move_cursor_down(),
                    ActiveUnit::InstalledList => {
                        if self.installed_cursor < self.installed_packages.len().saturating_sub(1) {
                            self.installed_cursor += 1;
                            let visible_lines = 10;
                            if self.installed_cursor >= self.installed_scroll + visible_lines {
                                self.installed_scroll += 1;
                            }
                        }
                    }
                    ActiveUnit::Terminal => {
                        if self.terminal_scroll > 0 {
                            self.terminal_scroll -= 1;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn move_cursor_up(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position -= 1;
            if self.cursor_position < self.scroll_offset {
                self.scroll_offset = self.cursor_position;
            }
        }
    }

    fn move_cursor_down(&mut self) {
        if !self.filtered_packages.is_empty() && self.cursor_position < self.filtered_packages.len() - 1 {
            self.cursor_position += 1;
            let visible_height = 10; // adjust based on actual height
            if self.cursor_position >= self.scroll_offset + visible_height {
                self.scroll_offset += 1;
            }
        }
    }

    fn toggle_selection(&mut self) {
        if let Some(package) = self.filtered_packages.get(self.cursor_position) {
            if let Some(pos) = self.selected_packages.iter().position(|p| p.name == package.name && p.source == package.source) {
                self.selected_packages.remove(pos);
            } else {
                self.selected_packages.push(package.clone());
            }
        }
    }

    fn install_selected(&mut self) {
        if self.selected_packages.is_empty() {
            return;
        }
        
        // Group packages by source (package manager)
        let mut packages_by_source: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        
        for package in &self.selected_packages {
            packages_by_source.entry(package.source.clone())
                .or_insert_with(Vec::new)
                .push(package.name.clone());
        }
        
        // Execute install commands for each package manager
        for (source, package_names) in packages_by_source {
            if let Some(manager) = self.pm_registry.get_manager(&source) {
                let command = self.pm_registry.get_install_command(manager, &package_names);
                
                // Log the installation attempt
                if let Ok(mut output) = self.terminal_output.lock() {
                    output.push(format!("Installing from {}: {}", source, package_names.join(", ")));
                    output.push(format!("Command: {}", command));
                    output.push("".to_string());
                }
                
                // Execute the command in a separate thread
                self.execute_command(&command);
            }
        }
        
        // Clear selection after installation
        self.selected_packages.clear();
    }
    
    fn execute_command(&self, command: &str) {
        let terminal_output = Arc::clone(&self.terminal_output);
        let command = command.to_string();
        
        thread::spawn(move || {
            // Parse command into program and args
            let parts: Vec<&str> = command.split_whitespace().collect();
            if parts.is_empty() {
                return;
            }
            
            let program = parts[0];
            let args = &parts[1..];
            
            // Log command start
            if let Ok(mut output) = terminal_output.lock() {
                output.push(format!("$ {}", command));
            }
            
            // Execute command
            match Command::new(program)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    // Read stdout
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        let terminal_output_clone = Arc::clone(&terminal_output);
                        
                        thread::spawn(move || {
                            for line in reader.lines() {
                                if let Ok(line) = line {
                                    if let Ok(mut output) = terminal_output_clone.lock() {
                                        output.push(line);
                                    }
                                }
                            }
                        });
                    }
                    
                    // Read stderr
                    if let Some(stderr) = child.stderr.take() {
                        let reader = BufReader::new(stderr);
                        let terminal_output_clone = Arc::clone(&terminal_output);
                        
                        thread::spawn(move || {
                            for line in reader.lines() {
                                if let Ok(line) = line {
                                    if let Ok(mut output) = terminal_output_clone.lock() {
                                        output.push(format!("ERROR: {}", line));
                                    }
                                }
                            }
                        });
                    }
                    
                    // Wait for command to complete
                    match child.wait() {
                        Ok(status) => {
                            if let Ok(mut output) = terminal_output.lock() {
                                if status.success() {
                                    output.push("Command completed successfully".to_string());
                                } else {
                                    output.push(format!("Command failed with exit code: {:?}", status.code()));
                                }
                                output.push("".to_string());
                            }
                        }
                        Err(e) => {
                            if let Ok(mut output) = terminal_output.lock() {
                                output.push(format!("Failed to wait for command: {}", e));
                                output.push("".to_string());
                            }
                        }
                    }
                }
                Err(e) => {
                    if let Ok(mut output) = terminal_output.lock() {
                        output.push(format!("Failed to execute command: {}", e));
                        output.push("".to_string());
                    }
                }
            }
        });
    }

    fn switch_unit(&mut self) {
        self.active_unit = match self.active_unit {
            ActiveUnit::Results => ActiveUnit::Input,
            ActiveUnit::Input => ActiveUnit::Description,
            ActiveUnit::Description => ActiveUnit::InstalledList,
            ActiveUnit::InstalledList => ActiveUnit::Terminal,
            ActiveUnit::Terminal => ActiveUnit::Results,
        };
    }
    

}
