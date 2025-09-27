use crate::core::package_managers::Package;
use crate::core::local::LocalPackageManager;
use crate::core::aur::AurClient;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActivePane {
    Results,
    Search,
    Details,
    Installed,
    Terminal,
}

pub struct App {
    // Core state
    pub should_quit: bool,
    pub input_mode: InputMode,
    pub active_pane: ActivePane,
    
    // Search state
    pub search_input: String,
    pub cursor_position: usize,
    
    // Results state
    pub packages: Vec<Package>,
    pub filtered_packages: Vec<Package>,
    pub selected_index: usize,
    pub scroll_offset: usize,
    
    // Installed packages
    pub installed_packages: Vec<Package>,
    pub installed_selected: usize,
    pub installed_scroll: usize,
    
    // Package details
    pub package_details: HashMap<String, String>,
    pub details_loading: bool,
    pub last_selection_time: Instant,
    
    // Search debouncing
    pub last_search_time: Instant,
    pub search_debounce_ms: u64,
    
    // Package managers
    pub package_managers: Vec<LocalPackageManager>,
    pub loading_complete: bool,
    
    // Multi-selection
    pub selected_packages: HashSet<String>, // Package names selected for installation
    
    // AUR client
    pub aur_client: AurClient,
    
    // UI state
    pub terminal_size: (u16, u16),
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_quit: false,
            input_mode: InputMode::Normal,
            active_pane: ActivePane::Results,
            
            search_input: String::new(),
            cursor_position: 0,
            
            packages: Vec::new(),
            filtered_packages: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            
            installed_packages: Vec::new(),
            installed_selected: 0,
            installed_scroll: 0,
            
            package_details: HashMap::new(),
            details_loading: false,
            last_selection_time: Instant::now(),
            
            last_search_time: Instant::now(),
            search_debounce_ms: 150,
            
            package_managers: Vec::new(),
            loading_complete: false,
            
            selected_packages: HashSet::new(),
            aur_client: AurClient::new(),
            
            terminal_size: (80, 24),
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
    
    pub fn set_packages(&mut self, packages: Vec<Package>) {
        self.packages = packages;
        self.filter_packages();
    }
    
    pub fn set_installed_packages(&mut self, packages: Vec<Package>) {
        self.installed_packages = packages;
    }
    
    pub fn filter_packages(&mut self) {
        if self.search_input.is_empty() {
            self.filtered_packages = self.packages.clone();
        } else {
            let query = self.search_input.to_lowercase();
            self.filtered_packages = self.packages
                .iter()
                .filter(|pkg| {
                    pkg.name.to_lowercase().contains(&query) ||
                    pkg.description.as_ref().map_or(false, |desc| desc.to_lowercase().contains(&query))
                })
                .cloned()
                .collect();
        }
        
        // Reset selection
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.last_selection_time = Instant::now();
    }
    
    pub fn move_up(&mut self) {
        match self.active_pane {
            ActivePane::Results => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.last_selection_time = Instant::now();
                    
                    // Adjust scroll
                    if self.selected_index < self.scroll_offset {
                        self.scroll_offset = self.selected_index;
                    }
                }
            }
            ActivePane::Installed => {
                if self.installed_selected > 0 {
                    self.installed_selected -= 1;
                    
                    if self.installed_selected < self.installed_scroll {
                        self.installed_scroll = self.installed_selected;
                    }
                }
            }
            _ => {}
        }
    }
    
    pub fn move_down(&mut self) {
        match self.active_pane {
            ActivePane::Results => {
                if self.selected_index < self.filtered_packages.len().saturating_sub(1) {
                    self.selected_index += 1;
                    self.last_selection_time = Instant::now();
                    
                    // Adjust scroll based on terminal size
                    let visible_items = self.get_results_visible_items();
                    if self.selected_index >= self.scroll_offset + visible_items {
                        self.scroll_offset = self.selected_index.saturating_sub(visible_items - 1);
                    }
                }
            }
            ActivePane::Installed => {
                if self.installed_selected < self.installed_packages.len().saturating_sub(1) {
                    self.installed_selected += 1;
                    
                    let visible_items = self.get_installed_visible_items();
                    if self.installed_selected >= self.installed_scroll + visible_items {
                        self.installed_scroll = self.installed_selected.saturating_sub(visible_items - 1);
                    }
                }
            }
            _ => {}
        }
    }
    
    pub fn switch_pane(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::Results => ActivePane::Search,
            ActivePane::Search => ActivePane::Details,
            ActivePane::Details => ActivePane::Installed,
            ActivePane::Installed => ActivePane::Terminal,
            ActivePane::Terminal => ActivePane::Results,
        };
        
        if self.active_pane == ActivePane::Search {
            self.input_mode = InputMode::Editing;
        } else {
            self.input_mode = InputMode::Normal;
        }
    }
    
    pub fn enter_search_mode(&mut self) {
        self.active_pane = ActivePane::Search;
        self.input_mode = InputMode::Editing;
    }
    
    pub fn add_char(&mut self, c: char) {
        if self.input_mode == InputMode::Editing {
            self.search_input.insert(self.cursor_position, c);
            self.cursor_position += 1;
            self.last_search_time = Instant::now();
        }
    }
    
    pub fn delete_char(&mut self) {
        if self.input_mode == InputMode::Editing && self.cursor_position > 0 {
            self.search_input.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
            self.last_search_time = Instant::now();
        }
    }
    
    pub fn clear_search(&mut self) {
        self.search_input.clear();
        self.cursor_position = 0;
        self.last_search_time = Instant::now();
        self.filter_packages();
    }
    
    pub fn should_update_search(&self) -> bool {
        self.last_search_time.elapsed() > Duration::from_millis(self.search_debounce_ms)
    }
    
    pub fn update_search_if_needed(&mut self) {
        if self.should_update_search() {
            self.filter_packages();
        }
    }
    
    pub fn get_selected_package(&self) -> Option<&Package> {
        self.filtered_packages.get(self.selected_index)
    }
    
    pub fn should_fetch_details(&self) -> bool {
        self.last_selection_time.elapsed() > Duration::from_millis(300) &&
        self.get_selected_package().is_some()
    }
    
    pub fn get_package_details(&self, package: &Package) -> Option<&String> {
        let key = format!("{}:{}", package.source, package.name);
        self.package_details.get(&key)
    }
    
    pub fn set_package_details(&mut self, package: &Package, details: String) {
        let key = format!("{}:{}", package.source, package.name);
        self.package_details.insert(key, details);
    }
    
    pub fn get_results_visible_items(&self) -> usize {
        // Calculate based on terminal height: total height - search (3) - details (8) - borders
        let available_height = self.terminal_size.1.saturating_sub(13);
        (available_height as usize).max(5) // Minimum 5 items visible
    }
    
    pub fn get_installed_visible_items(&self) -> usize {
        // Right panel gets full height minus borders
        let available_height = self.terminal_size.1.saturating_sub(2);
        (available_height as usize).max(5) // Minimum 5 items visible
    }
    
    // Multi-selection methods
    pub fn toggle_package_selection(&mut self) {
        if let Some(package) = self.get_selected_package() {
            let package_key = format!("{}:{}", package.source, package.name);
            if self.selected_packages.contains(&package_key) {
                self.selected_packages.remove(&package_key);
            } else {
                self.selected_packages.insert(package_key);
            }
        }
    }
    
    pub fn is_package_selected(&self, package: &Package) -> bool {
        let package_key = format!("{}:{}", package.source, package.name);
        self.selected_packages.contains(&package_key)
    }
    
    pub fn clear_selection(&mut self) {
        self.selected_packages.clear();
    }
    
    pub fn get_selected_count(&self) -> usize {
        self.selected_packages.len()
    }
    
    pub fn get_selected_packages_list(&self) -> Vec<String> {
        self.selected_packages.iter().cloned().collect()
    }
    
    // Async search method for AUR integration
    pub async fn search_aur_packages(&mut self, query: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Search AUR packages using the AUR client
        let aur_results = self.aur_client.search(query).await?;
        
        // Add AUR results to the packages list
        for aur_package in aur_results {
            // Check if package already exists (avoid duplicates)
            if !self.packages.iter().any(|p| p.name == aur_package.name && p.source == aur_package.source) {
                self.packages.push(aur_package);
            }
        }
        
        // Re-filter packages with current search
        self.filter_packages();
        
        Ok(())
    }
    
    // Method to get package details (for the details pane)
    pub async fn get_package_details(&self, package: &Package) -> String {
        if package.source == "paru" {
            match self.aur_client.get_package_details(&package.name).await {
                Ok(details) => details,
                Err(_) => format!("Failed to fetch details for {}", package.name),
            }
        } else {
            // For non-AUR packages, show basic info
            let mut details = format!("Package: {}\n", package.name);
            if let Some(version) = &package.version {
                details.push_str(&format!("Version: {}\n", version));
            }
            if let Some(description) = &package.description {
                details.push_str(&format!("Description: {}\n", description));
            }
            details.push_str(&format!("Source: {}\n", package.source));
            details.push_str(&format!("Installed: {}\n", if package.installed { "Yes" } else { "No" }));
            details
        }
    }
    
    // Method to add AUR packages from async search
    pub fn add_aur_packages(&mut self, aur_packages: Vec<Package>) {
        for aur_package in aur_packages {
            // Check if package already exists (avoid duplicates)
            if !self.packages.iter().any(|p| p.name == aur_package.name && p.source == aur_package.source) {
                self.packages.push(aur_package);
            }
        }
        
        // Re-filter packages with current search
        self.filter_packages();
    }
    
    // Start installation of selected packages
    pub fn start_installation(&mut self) {
        if self.selected_packages.is_empty() {
            return;
        }
        
        // Group packages by source/package manager
        let mut install_commands = std::collections::HashMap::new();
        
        for package_key in &self.selected_packages {
            let parts: Vec<&str> = package_key.split(':').collect();
            if parts.len() == 2 {
                let source = parts[0];
                let package_name = parts[1];
                
                install_commands.entry(source.to_string())
                    .or_insert_with(Vec::new)
                    .push(package_name.to_string());
            }
        }
        
        // Execute installation commands
        for (source, packages) in install_commands {
            let package_list = packages.join(" ");
            let command = match source.as_str() {
                "pacman" => format!("sudo pacman -S {}", package_list),
                "paru" => format!("paru -S {}", package_list),
                "dnf" => format!("sudo dnf install {}", package_list),
                "emerge" => format!("sudo emerge {}", package_list),
                "nix" => format!("nix-env -iA {}", package_list),
                "apt" => format!("sudo apt install {}", package_list),
                _ => continue,
            };
            
            // For now, just print the command (in a real implementation, this would execute)
            println!("Would execute: {}", command);
        }
        
        // Clear selection after installation
        self.clear_selection();
    }
}