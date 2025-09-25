use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub installed: bool,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManagerConfig {
    pub name: String,
    pub display_name: String,
    pub executable: String,
    pub list_packages_cmd: String,
    pub list_installed_cmd: String,
    pub search_cmd: String,
    pub install_cmd: String,
    pub requires_root: bool,
    pub package_separator: String,
    pub installed_indicator: Option<String>,
    pub cleanup_regex: Option<String>, // Optional regex to clean up package names
    pub version_regex: Option<String>, // Optional regex to extract version
}

#[derive(Debug, Serialize, Deserialize)]
struct PackageManagerToml {
    package_manager: PackageManagerConfig,
}

pub struct PackageManagerRegistry {
    pub managers: HashMap<String, PackageManagerConfig>,
}

impl PackageManagerRegistry {
    pub fn new() -> Self {
        Self {
            managers: HashMap::new(),
        }
    }
    
    pub fn load_from_config_dir(config_dir: &std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut registry = Self::new();
        let pm_dir = config_dir.join("pkgmanagers");
        
        eprintln!("DEBUG: Looking for configs in: {:?}", pm_dir);
        
        if !pm_dir.exists() {
            eprintln!("DEBUG: Config dir doesn't exist, creating and copying defaults");
            std::fs::create_dir_all(&pm_dir)?;
            
            // Create default configs for common package managers
            Self::create_default_configs(&pm_dir)?;
            
            // Create a README explaining how to add package managers
            let readme_content = r#"# Package Manager Configurations

This directory contains TOML configuration files for package managers.
Each .toml file defines a package manager that pmux can use.

## Example Configuration (save as `example.toml`):

```toml
name = "example"
display_name = "Example Package Manager"
executable = "example-pm"
list_packages_cmd = "example-pm list-all"
list_installed_cmd = "example-pm list-installed"
search_cmd = "example-pm search {}"
install_cmd = "example-pm install {}"
requires_root = false
package_separator = " "
installed_indicator = "*"
```

## Fields:
- `name`: Internal identifier
- `display_name`: Human-readable name
- `executable`: Command to check if PM is available
- `list_packages_cmd`: Command to list all available packages
- `list_installed_cmd`: Command to list installed packages
- `search_cmd`: Command to search packages (use {} as placeholder)
- `install_cmd`: Command to install packages (use {} as placeholder)
- `requires_root`: Whether installation needs sudo/root
- `package_separator`: How to separate multiple package names
- `installed_indicator`: Symbol to show for installed packages
"#;
            std::fs::write(pm_dir.join("README.md"), readme_content)?;
        }
        
        // Load all .toml files from pkgmanagers directory
        eprintln!("DEBUG: Loading configs from directory");
        for entry in std::fs::read_dir(&pm_dir)? {
            let entry = entry?;
            let path = entry.path();
            eprintln!("DEBUG: Found file: {:?}", path);
            
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        eprintln!("DEBUG: Read config file: {:?}", path.file_name());
                        match toml::from_str::<PackageManagerToml>(&content) {
                            Ok(toml_config) => {
                                let config = toml_config.package_manager;
                                eprintln!("DEBUG: Loaded package manager: {}", config.name);
                                registry.managers.insert(config.name.clone(), config);
                            }
                            Err(e) => {
                                eprintln!("DEBUG: Failed to parse TOML in {:?}: {}", path, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("DEBUG: Failed to read file {:?}: {}", path, e);
                    }
                }
            }
        }
        
        Ok(registry)
    }
    
    fn create_default_configs(pm_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        // Try to copy from examples directory first
        let examples_dir = std::path::Path::new("examples");
        eprintln!("DEBUG: Looking for examples in: {:?}", examples_dir);
        if examples_dir.exists() {
            eprintln!("DEBUG: Examples directory found, copying configs");
            for entry in std::fs::read_dir(examples_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "toml") {
                    if let Some(filename) = path.file_name() {
                        let dest = pm_dir.join(filename);
                        eprintln!("DEBUG: Copying {:?} to {:?}", path, dest);
                        std::fs::copy(&path, &dest)?;
                    }
                }
            }
            return Ok(());
        } else {
            eprintln!("DEBUG: Examples directory not found, using hardcoded configs");
        }
        
        // Fallback to hardcoded configs if examples don't exist
        
        // Nix config
        let nix_config = r#"[package_manager]
name = "nix"
display_name = "Nix (flakes, nixpkgs/nixos-unstable)"
executable = "nix"
list_packages_cmd = "nix-env -qaP 2>/dev/null | awk '{print $1}' | sed 's/^nixpkgs\\.//' | sort -u"
list_installed_cmd = "nix profile list 2>/dev/null | grep -oP 'nixpkgs#\\K[^@]*' | sort"
search_cmd = "nix search nixpkgs/nixos-unstable {}"
install_cmd = "nix profile install nixpkgs/nixos-unstable#{}"
requires_root = false
package_separator = " "
installed_indicator = "*"
"#;
        std::fs::write(pm_dir.join("nix.toml"), nix_config)?;
        
        // Paru config
        let paru_config = r#"[package_manager]
name = "paru"
display_name = "Paru (AUR)"
executable = "paru"
list_packages_cmd = "paru -Slqa"
list_installed_cmd = "paru -Q"
search_cmd = "paru -Ss {}"
install_cmd = "paru -S {}"
requires_root = false
package_separator = " "
installed_indicator = "*"
"#;
        std::fs::write(pm_dir.join("paru.toml"), paru_config)?;
        
        // APT config
        let apt_config = r#"[package_manager]
name = "apt"
display_name = "APT (Debian/Ubuntu)"
executable = "apt"
list_packages_cmd = "apt list"
list_installed_cmd = "apt list --installed"
search_cmd = "apt search {}"
install_cmd = "apt install {}"
requires_root = true
package_separator = " "
installed_indicator = "*"
"#;
        std::fs::write(pm_dir.join("apt.toml"), apt_config)?;
        
        // Emerge config
        let emerge_config = r#"[package_manager]
name = "emerge"
display_name = "Portage (Gentoo)"
executable = "equery"
list_packages_cmd = "equery list --portage-tree '*'"
list_installed_cmd = "equery list '*'"
search_cmd = "emerge --search {}"
install_cmd = "emerge {}"
requires_root = true
package_separator = " "
installed_indicator = "[I"
cleanup_regex = "^\\[.{3}\\] \\[..\\] (.+?):"
version_regex = "([^/]+/[^-]+)-(.+)"
"#;
        std::fs::write(pm_dir.join("emerge.toml"), emerge_config)?;
        
        // DNF config (supports both dnf and dnf5)
        let dnf_config = r#"[package_manager]
name = "dnf"
display_name = "DNF (Fedora/RHEL)"
executable = "dnf"
list_packages_cmd = "dnf list --available"
list_installed_cmd = "dnf list --installed"
search_cmd = "dnf search {}"
install_cmd = "dnf install {}"
requires_root = true
package_separator = " "
installed_indicator = "@"
"#;
        std::fs::write(pm_dir.join("dnf.toml"), dnf_config)?;
        
        // Pacman config
        let pacman_config = r#"[package_manager]
name = "pacman"
display_name = "Pacman (Arch Linux)"
executable = "pacman"
list_packages_cmd = "pacman -Slq"
list_installed_cmd = "pacman -Q"
search_cmd = "pacman -Ss {}"
install_cmd = "pacman -S {}"
requires_root = true
package_separator = " "
installed_indicator = "*"
"#;
        std::fs::write(pm_dir.join("pacman.toml"), pacman_config)?;
        
        Ok(())
    }
    
    pub fn get_manager(&self, name: &str) -> Option<&PackageManagerConfig> {
        self.managers.get(name)
    }
    
    pub fn get_enabled_managers(&self, enabled_list: &[String]) -> Vec<&PackageManagerConfig> {
        enabled_list.iter()
            .filter_map(|name| self.managers.get(name))
            .collect()
    }
    
    pub fn list_packages(&self, manager: &PackageManagerConfig) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        // Check if package manager is available
        if !self.is_available(manager) {
            return Ok(vec![]);
        }
        
        let output = Command::new(&manager.executable)
            .args(manager.list_packages_cmd.split_whitespace().skip(1))
            .output()?;
            
        if !output.status.success() {
            return Ok(vec![]);
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_package_list(&stdout, manager)
    }
    
    pub fn list_installed(&self, manager: &PackageManagerConfig) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        if !self.is_available(manager) {
            return Ok(vec![]);
        }
        
        let output = Command::new(&manager.executable)
            .args(manager.list_installed_cmd.split_whitespace().skip(1))
            .output()?;
            
        if !output.status.success() {
            return Ok(vec![]);
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_package_list(&stdout, manager)
    }
    
    pub fn search(&self, manager: &PackageManagerConfig, query: &str) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        if !self.is_available(manager) {
            return Ok(vec![]);
        }
        
        let cmd = manager.search_cmd.replace("{}", query);
        let args: Vec<&str> = cmd.split_whitespace().collect();
        
        let output = Command::new(args[0])
            .args(&args[1..])
            .output()?;
            
        if !output.status.success() {
            return Ok(vec![]);
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_package_list(&stdout, manager)
    }
    
    pub fn get_install_command(&self, manager: &PackageManagerConfig, packages: &[String]) -> String {
        let package_list = packages.join(&manager.package_separator);
        let cmd = manager.install_cmd.replace("{}", &package_list);
        
        if manager.requires_root {
            format!("sudo {}", cmd)
        } else {
            cmd
        }
    }
    
    fn is_available(&self, manager: &PackageManagerConfig) -> bool {
        let available = Command::new("which")
            .arg(&manager.executable)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        
        eprintln!("DEBUG: {} executable '{}' available: {}", manager.name, manager.executable, available);
        available
    }
    
    fn parse_package_list(&self, output: &str, manager: &PackageManagerConfig) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        let mut packages = Vec::new();
        
        match manager.name.as_str() {
            "nix" => {
                // Handle different nix command outputs
                if output.trim().starts_with('{') {
                    // Handle nix search JSON output
                    if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(output) {
                        if let Some(obj) = json_data.as_object() {
                            for (key, value) in obj {
                                if let Some(pkg_obj) = value.as_object() {
                                    let package = Package {
                                        name: key.clone(),
                                        version: pkg_obj.get("version").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                        description: pkg_obj.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                        installed: false,
                                        source: manager.name.clone(),
                                    };
                                    packages.push(package);
                                }
                            }
                        }
                    }
                } else {
                    // Handle nix-env -qaP or nix profile list output
                    for line in output.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }
                        
                        if line.contains("nixpkgs#") {
                            // nix profile list output
                            if let Some(pkg_name) = line.split("nixpkgs#").nth(1) {
                                if let Some(name) = pkg_name.split('@').next() {
                                    let package = Package {
                                        name: name.to_string(),
                                        version: None,
                                        description: None,
                                        installed: true,
                                        source: manager.name.clone(),
                                    };
                                    packages.push(package);
                                }
                            }
                        } else {
                            // nix-env -qaP output: just package names
                            let package = Package {
                                name: line.trim().to_string(),
                                version: None,
                                description: None,
                                installed: false,
                                source: manager.name.clone(),
                            };
                            packages.push(package);
                        }
                    }
                }
            }
            "paru" => {
                // Handle paru output
                for line in output.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(name) = parts.get(0) {
                        let package = Package {
                            name: name.to_string(),
                            version: parts.get(1).map(|v| v.to_string()),
                            description: parts.get(2..).map(|d| d.join(" ")),
                            installed: false,
                            source: manager.name.clone(),
                        };
                        packages.push(package);
                    }
                }
            }
            "apt" => {
                // Handle apt output
                for line in output.lines() {
                    if line.starts_with("WARNING") || line.starts_with("Listing") || line.trim().is_empty() {
                        continue;
                    }
                    
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(name_version) = parts.get(0) {
                        let name = name_version.split('/').next().unwrap_or(name_version);
                        let package = Package {
                            name: name.to_string(),
                            version: parts.get(1).map(|v| v.to_string()),
                            description: parts.get(3..).map(|d| d.join(" ")),
                            installed: line.contains("[installed"),
                            source: manager.name.clone(),
                        };
                        packages.push(package);
                    }
                }
            }
            "emerge" => {
                // Handle emerge/equery output
                // Format: [-P-] [  ] acct-group/3proxy-0:0
                // Format: [IP-] [  ] acct-group/audio-0-r3:0
                for line in output.lines() {
                    if line.trim().is_empty() || !line.starts_with('[') {
                        continue;
                    }
                    
                    // Skip the "* Searching for * ..." line
                    if line.contains("Searching for") {
                        continue;
                    }
                    
                    // Parse the status flags [I--] or [-P-]
                    let installed = line.starts_with("[I");
                    
                    // Simple approach: split by spaces and get the 3rd element (index 2)
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let atom_full = parts[2]; // The package atom
                        
                        // Split by : to remove slot
                        let atom = atom_full.split(':').next().unwrap_or(atom_full);
                        
                        let package = Package {
                            name: atom.to_string(),
                            version: None, // Version is embedded in atom
                            description: None,
                            installed,
                            source: manager.name.clone(),
                        };
                        packages.push(package);
                    }
                }
            }
            _ => {
                // Generic parsing with optional regex cleanup
                for line in output.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    
                    let mut processed_line = line;
                    
                    // Apply cleanup regex if provided
                    if let Some(cleanup_pattern) = &manager.cleanup_regex {
                        if let Ok(regex) = Regex::new(cleanup_pattern) {
                            if let Some(captures) = regex.captures(line) {
                                if let Some(matched) = captures.get(1) {
                                    processed_line = matched.as_str();
                                }
                            }
                        }
                    }
                    
                    // Extract name and version using version regex
                    let (name, version) = if let Some(version_pattern) = &manager.version_regex {
                        if let Ok(regex) = Regex::new(version_pattern) {
                            if let Some(captures) = regex.captures(processed_line) {
                                let pkg_name = captures.get(1).map(|m| m.as_str()).unwrap_or(processed_line);
                                let pkg_version = captures.get(2).map(|m| m.as_str().to_string());
                                (pkg_name.to_string(), pkg_version)
                            } else {
                                (processed_line.to_string(), None)
                            }
                        } else {
                            (processed_line.to_string(), None)
                        }
                    } else {
                        // Fallback to simple whitespace parsing
                        let parts: Vec<&str> = processed_line.split_whitespace().collect();
                        let name = parts.get(0).unwrap_or(&processed_line).to_string();
                        let version = parts.get(1).map(|v| v.to_string());
                        (name, version)
                    };
                    
                    // Check if installed using indicator
                    let installed = if let Some(indicator) = &manager.installed_indicator {
                        line.contains(indicator)
                    } else {
                        false
                    };
                    
                    let package = Package {
                        name,
                        version,
                        description: None, // Generic parser doesn't extract descriptions
                        installed,
                        source: manager.name.clone(),
                    };
                    packages.push(package);
                }
            }
        }
        
        Ok(packages)
    }
}
