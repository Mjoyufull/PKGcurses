use crate::core::package_managers::Package;
use crate::core::local::LocalPackageManager;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActivePane {
    Search,
    Results,
    Details,
    Installed,
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
    
    // Package managers
    pub package_managers: Vec<LocalPackageManager>,
    pub loading_complete: bool,
    
    // UI state
    pub terminal_size: (u16, u16),
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_quit: false,
            input_mode: InputMode::Editing,
            active_pane: ActivePane::Search,
            
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
            
            package_managers: Vec::new(),
            loading_complete: false,
            
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
                    
                    // Adjust scroll (assuming 20 visible items)
                    let visible_items = 20;
                    if self.selected_index >= self.scroll_offset + visible_items {
                        self.scroll_offset = self.selected_index.saturating_sub(visible_items - 1);
                    }
                }
            }
            ActivePane::Installed => {
                if self.installed_selected < self.installed_packages.len().saturating_sub(1) {
                    self.installed_selected += 1;
                    
                    let visible_items = 20;
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
            ActivePane::Search => ActivePane::Results,
            ActivePane::Results => ActivePane::Details,
            ActivePane::Details => ActivePane::Installed,
            ActivePane::Installed => ActivePane::Search,
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
            self.filter_packages();
        }
    }
    
    pub fn delete_char(&mut self) {
        if self.input_mode == InputMode::Editing && self.cursor_position > 0 {
            self.search_input.remove(self.cursor_position - 1);
            self.cursor_position -= 1;
            self.filter_packages();
        }
    }
    
    pub fn clear_search(&mut self) {
        self.search_input.clear();
        self.cursor_position = 0;
        self.filter_packages();
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
}