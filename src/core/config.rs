use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub main: MainConfig,
    pub layout: LayoutConfig,
    pub border_colours: HashMap<String, String>,
    pub text_colours: HashMap<String, String>,
    pub pm: PmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainConfig {
    pub sudoers: String,
    pub rounded_borders: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub right_column_width_percent: u16,  // How much of screen width the right column takes
    pub input_field_height: u16,          // Height in lines for input field
    pub installed_list_percent: u16,      // Percentage of right column height for installed list
    pub terminal_percent: u16,            // Percentage of right column height for terminal
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmConfig {
    pub enabled_pm: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        let mut border_colours = HashMap::new();
        border_colours.insert("results_unit".to_string(), "#ffffff".to_string());
        border_colours.insert("description_unit".to_string(), "#ffffff".to_string());
        border_colours.insert("installed_list_unit".to_string(), "#ffffff".to_string());
        border_colours.insert("terminal_unit".to_string(), "#ffffff".to_string());

        let mut text_colours = HashMap::new();
        text_colours.insert("results_unit_highlight_text".to_string(), "#00ff00".to_string());
        text_colours.insert("results_unit_text".to_string(), "#ffffff".to_string());
        text_colours.insert("installed_list_unit_highlight_text".to_string(), "#00ff00".to_string());
        text_colours.insert("installed_list_unit_text".to_string(), "#ffffff".to_string());
        text_colours.insert("description_unit_highlight_text".to_string(), "#00ff00".to_string());
        text_colours.insert("description_unit_text".to_string(), "#ffffff".to_string());
        text_colours.insert("terminal_unit_highlight_text".to_string(), "#00ff00".to_string());
        text_colours.insert("terminal_unit_text".to_string(), "#ffffff".to_string());

        Config {
            main: MainConfig {
                sudoers: "sudo".to_string(),
                rounded_borders: false,
            },
            layout: LayoutConfig {
                right_column_width_percent: 30,
                input_field_height: 3,
                installed_list_percent: 50,
                terminal_percent: 50,
            },
            border_colours,
            text_colours,
            pm: PmConfig {
                enabled_pm: vec!["nix".to_string(), "paru".to_string(), "apt".to_string(), "emerge".to_string(), "dnf".to_string(), "pacman".to_string()],
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        
        if config_path.exists() {
            let content = fs::read_to_string(config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            let default_config = Config::default();
            default_config.save()?;
            Ok(default_config)
        }
    }
    
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::get_config_path()?;
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let toml_content = toml::to_string_pretty(self)?;
        fs::write(config_path, toml_content)?;
        Ok(())
    }
    
    fn get_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let home = std::env::var("HOME")?;
        Ok(PathBuf::from(home).join(".config").join("pmux").join("config.toml"))
    }
    
    pub fn get_config_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let home = std::env::var("HOME")?;
        Ok(PathBuf::from(home).join(".config").join("pmux"))
    }
}