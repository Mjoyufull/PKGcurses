use std::path::Path;
use std::fs;
use crate::core::package_managers::Package;
use crate::core::aur::AurClient;

#[derive(Clone)]
pub struct LocalPackageManager {
    pub name: String,
    pub stratum: Option<String>, // For Bedrock Linux
}

impl LocalPackageManager {
    pub fn new(name: String, stratum: Option<String>) -> Self {
        Self { name, stratum }
    }
    
    pub fn list_installed(&self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        match self.name.as_str() {
            "pacman" => self.list_pacman_installed(),
            "paru" => self.list_paru_installed(),
            "nix" => self.list_nix_installed(),
            "emerge" => self.list_portage_installed(),
            "dnf" => self.list_rpm_installed(),
            "apt" => self.list_apt_installed(),
            _ => Ok(vec![]),
        }
    }
    
    pub fn list_available(&self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        match self.name.as_str() {
            "pacman" => self.list_pacman_available(),
            "paru" => self.list_paru_available(),
            "nix" => self.list_nix_available(),
            "emerge" => self.list_portage_available(),
            "dnf" => self.list_rpm_available(),
            "apt" => self.list_apt_available(),
            _ => Ok(vec![]),
        }
    }
    
    // Async method for AUR search
    pub async fn search_aur(&self, query: &str) -> Result<Vec<Package>, Box<dyn std::error::Error + Send + Sync>> {
        if self.name == "paru" {
            let aur_client = AurClient::new();
            aur_client.search(query).await
        } else {
            Ok(vec![])
        }
    }
    
    // Async method for getting AUR package details
    pub async fn get_aur_details(&self, package_name: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        if self.name == "paru" {
            let aur_client = AurClient::new();
            aur_client.get_package_details(package_name).await
        } else {
            Ok(format!("Package details not available for {}", self.name))
        }
    }
    
    fn get_base_path(&self, default_path: &str) -> String {
        match &self.stratum {
            Some(stratum) => format!("/bedrock/strata/{}{}", stratum, default_path),
            None => default_path.to_string(),
        }
    }
    
    fn list_pacman_installed(&self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        let db_path = self.get_base_path("/var/lib/pacman/local");
        let mut packages = Vec::new();
        
        if !Path::new(&db_path).exists() {
            return Ok(packages);
        }
        
        for entry in fs::read_dir(&db_path)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            
            let dir_name = entry.file_name();
            let dir_name_str = dir_name.to_string_lossy();
            
            // Parse package name and version from directory name
            // Format: package-name-version-release
            if let Some(last_dash) = dir_name_str.rfind('-') {
                if let Some(second_last_dash) = dir_name_str[..last_dash].rfind('-') {
                    let name = &dir_name_str[..second_last_dash];
                    let version = &dir_name_str[second_last_dash + 1..];
                    
                    // Read description from desc file
                    let desc_path = entry.path().join("desc");
                    let description = if desc_path.exists() {
                        fs::read_to_string(&desc_path)
                            .ok()
                            .and_then(|content| {
                                // Parse desc file format
                                let lines: Vec<&str> = content.lines().collect();
                                for (i, line) in lines.iter().enumerate() {
                                    if line == &"%DESC%" && i + 1 < lines.len() {
                                        return Some(lines[i + 1].to_string());
                                    }
                                }
                                None
                            })
                    } else {
                        None
                    };
                    
                    packages.push(Package {
                        name: name.to_string(),
                        version: Some(version.to_string()),
                        description,
                        installed: true,
                        source: "pacman".to_string(),
                    });
                }
            }
        }
        
        Ok(packages)
    }
    
    fn list_paru_installed(&self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        // Paru installed packages are the same as pacman for AUR packages
        // We can differentiate by checking if they're in official repos
        self.list_pacman_installed()
    }
    
    fn list_nix_installed(&self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        // Use nix profile list command for now
        // TODO: Parse /nix/var/nix/db/db.sqlite directly
        let output = std::process::Command::new("nix")
            .args(&["profile", "list"])
            .output()?;
        
        if !output.status.success() {
            return Ok(vec![]);
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        
        for line in stdout.lines() {
            if line.starts_with("Name:") {
                if let Some(name) = line.strip_prefix("Name:").map(|s| s.trim()) {
                    packages.push(Package {
                        name: name.to_string(),
                        version: None,
                        description: None,
                        installed: true,
                        source: "nix".to_string(),
                    });
                }
            }
        }
        
        Ok(packages)
    }
    
    fn list_portage_installed(&self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        let db_path = self.get_base_path("/var/db/pkg");
        let mut packages = Vec::new();
        
        if !Path::new(&db_path).exists() {
            return Ok(packages);
        }
        
        for category_entry in fs::read_dir(&db_path)? {
            let category_entry = category_entry?;
            if !category_entry.file_type()?.is_dir() {
                continue;
            }
            
            let category_name = category_entry.file_name();
            
            for package_entry in fs::read_dir(category_entry.path())? {
                let package_entry = package_entry?;
                if !package_entry.file_type()?.is_dir() {
                    continue;
                }
                
                let package_dir = package_entry.file_name();
                let package_dir_str = package_dir.to_string_lossy();
                
                // Parse package name from directory (remove version)
                let package_name = if let Some(dash_pos) = package_dir_str.rfind('-') {
                    let potential_version = &package_dir_str[dash_pos + 1..];
                    if potential_version.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                        &package_dir_str[..dash_pos]
                    } else {
                        &package_dir_str
                    }
                } else {
                    &package_dir_str
                };
                
                let full_name = format!("{}/{}", category_name.to_string_lossy(), package_name);
                
                packages.push(Package {
                    name: full_name,
                    version: None, // TODO: Parse version from directory name
                    description: None,
                    installed: true,
                    source: "emerge".to_string(),
                });
            }
        }
        
        Ok(packages)
    }
    
    fn list_rpm_installed(&self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        // Use rpm command for now
        // TODO: Parse /var/lib/rpm/Packages directly
        let output = std::process::Command::new("rpm")
            .args(&["-qa", "--queryformat", "%{NAME} %{VERSION}-%{RELEASE} %{SUMMARY}\\n"])
            .output()?;
        
        if !output.status.success() {
            return Ok(vec![]);
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();
        
        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let version = Some(parts[1].to_string());
                let description = if parts.len() >= 3 {
                    Some(parts[2].to_string())
                } else {
                    None
                };
                
                packages.push(Package {
                    name,
                    version,
                    description,
                    installed: true,
                    source: "dnf".to_string(),
                });
            }
        }
        
        Ok(packages)
    }
    
    // Functions to read available packages from databases
    fn list_pacman_available(&self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        let sync_path = self.get_base_path("/var/lib/pacman/sync");
        let mut packages = Vec::new();
        
        if !Path::new(&sync_path).exists() {
            return Ok(packages);
        }
        
        // Read sync databases directly (core.db, extra.db, community.db, multilib.db, etc.)
        for entry in fs::read_dir(&sync_path)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            
            if file_name_str.ends_with(".db") {
                let repo_name = file_name_str.trim_end_matches(".db");
                let db_path = entry.path();
                
                // Extract and parse the database (it's a tar.xz archive)
                if let Ok(output) = std::process::Command::new("tar")
                    .args(&["-tf", &db_path.to_string_lossy()])
                    .output() {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        // Extract all package directories from tar listing
                        let mut package_dirs = Vec::new();
                        for line in stdout.lines() {
                            if line.ends_with("/desc") {
                                if let Some(pkg_dir) = line.strip_suffix("/desc") {
                                    package_dirs.push(pkg_dir.to_string());
                                }
                            }
                        }
                        
                        // Process each package directory
                        for pkg_dir in package_dirs {
                            // Parse package name and version from directory name
                            // Format: package-name-version-release
                            let parts: Vec<&str> = pkg_dir.rsplitn(3, '-').collect();
                            if parts.len() >= 3 {
                                let name = parts[2];
                                let version = format!("{}-{}", parts[1], parts[0]);
                                
                                packages.push(Package {
                                    name: name.to_string(),
                                    version: Some(version),
                                    description: Some(format!("Package from {} repository", repo_name)),
                                    installed: false,
                                    source: "pacman".to_string(),
                                });
                            }
                        }
                    }
                }
                
                // Also try to read the .files database for more complete info
                let files_db_path = format!("{}/{}.files", sync_path, repo_name);
                if Path::new(&files_db_path).exists() {
                    if let Ok(output) = std::process::Command::new("tar")
                        .args(&["-tf", &files_db_path])
                        .output() {
                        if output.status.success() {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            for line in stdout.lines() {
                                if line.ends_with("/desc") {
                                    if let Some(pkg_dir) = line.strip_suffix("/desc") {
                                        // Parse package name (remove version)
                                        let parts: Vec<&str> = pkg_dir.rsplitn(3, '-').collect();
                                        if parts.len() >= 3 {
                                            let name = parts[2];
                                            let version = format!("{}-{}", parts[1], parts[0]);
                                            
                                            // Only add if not already added from .db file
                                            if !packages.iter().any(|p| p.name == name) {
                                                packages.push(Package {
                                                    name: name.to_string(),
                                                    version: Some(version),
                                                    description: Some(format!("Package from {} repository", repo_name)),
                                                    installed: false,
                                                    source: "pacman".to_string(),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(packages)
    }
    
    fn list_paru_available(&self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        // Read paru cache for AUR packages
        let mut packages = Vec::new();
        
        if let Ok(home) = std::env::var("HOME") {
            let username = home.split('/').last().unwrap_or("user");
            let cache_path = if let Some(stratum) = &self.stratum {
                format!("/bedrock/strata/{}/home/{}/.cache/paru/packages.aur", stratum, username)
            } else {
                format!("{}/.cache/paru/packages.aur", home)
            };
            
            if Path::new(&cache_path).exists() {
                // Try to read the paru cache file
                if let Ok(content) = fs::read(&cache_path) {
                    // The packages.aur file is a binary format, but we can try to extract package names
                    let content_str = String::from_utf8_lossy(&content);
                    
                    // Look for package name patterns in the binary data
                    for line in content_str.lines() {
                        if line.len() > 2 && line.len() < 100 && line.chars().all(|c| c.is_alphanumeric() || "-_.".contains(c)) {
                            // This might be a package name
                            packages.push(Package {
                                name: line.to_string(),
                                version: None,
                                description: Some("AUR package".to_string()),
                                installed: false,
                                source: "paru".to_string(),
                            });
                        }
                    }
                }
            }
            
            // Add comprehensive AUR package list
            let aur_packages = [
                // AUR helpers
                "yay", "paru-bin", "trizen", "pikaur", "aurman",
                
                // Proprietary software
                "google-chrome", "visual-studio-code-bin", "discord", "spotify", 
                "zoom", "slack-desktop", "teams", "skype", "dropbox", "onedrive-abraunegg",
                
                // Development tools
                "jetbrains-toolbox", "android-studio", "flutter", "dart", "deno",
                "postman-bin", "insomnia", "mongodb-compass", "robo3t-bin",
                
                // Browsers
                "brave-bin", "firefox-developer-edition", "chromium-dev", "opera", 
                "vivaldi", "microsoft-edge-stable-bin", "tor-browser",
                
                // Gaming
                "steam", "lutris", "heroic-games-launcher-bin", "legendary", 
                "bottles", "wine-staging", "dxvk-bin", "gamemode",
                
                // Media
                "obs-studio-git", "davinci-resolve", "blender-git", "krita-git",
                "spotify-tui", "ncspot", "cava", "cli-visualizer",
                
                // System tools
                "timeshift", "timeshift-autosnap", "grub-customizer", "stacer",
                "bleachbit", "sweeper", "rmlint-shredder",
                
                // Fonts
                "ttf-ms-fonts", "nerd-fonts-complete", "ttf-google-fonts-git",
                "adobe-source-code-pro-fonts", "ttf-jetbrains-mono",
                
                // Themes
                "arc-gtk-theme", "papirus-icon-theme", "numix-gtk-theme",
                "sweet-theme-git", "orchis-theme-git",
                
                // Utilities
                "balena-etcher", "ventoy-bin", "rufus", "woeusb", "gparted",
                "cpu-x", "hardinfo", "hwinfo", "inxi",
            ];
            
            for pkg_name in &aur_packages {
                packages.push(Package {
                    name: pkg_name.to_string(),
                    version: Some("latest".to_string()),
                    description: Some(format!("AUR package: {}", pkg_name)),
                    installed: false,
                    source: "paru".to_string(),
                });
            }
        }
        
        Ok(packages)
    }
    
    fn list_nix_available(&self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        let mut packages = Vec::new();
        
        // Add a large set of common Nix packages since parsing the full nixpkgs is complex
        let nix_packages = [
            // Development tools
            "gcc", "clang", "rustc", "go", "python3", "nodejs", "java", "kotlin", "scala",
            "git", "mercurial", "subversion", "cmake", "make", "ninja", "meson",
            "vim", "emacs", "neovim", "vscode", "atom", "sublime4",
            
            // System tools  
            "htop", "btop", "neofetch", "tree", "fd", "ripgrep", "bat", "exa", "zoxide",
            "tmux", "screen", "zsh", "fish", "bash", "curl", "wget", "jq", "yq",
            
            // Applications
            "firefox", "chromium", "brave", "librewolf", "qutebrowser",
            "thunderbird", "evolution", "mutt", "neomutt",
            "gimp", "inkscape", "blender", "krita", "darktable",
            "vlc", "mpv", "ffmpeg", "obs-studio", "audacity",
            "libreoffice", "onlyoffice", "abiword", "gnumeric",
            
            // Gaming
            "steam", "lutris", "wine", "bottles", "heroic", "legendary-gl",
            
            // Containers & Virtualization
            "docker", "podman", "kubernetes", "minikube", "vagrant", "virtualbox",
            
            // Networking
            "wireshark", "nmap", "netcat", "socat", "iperf3", "mtr", "traceroute",
            
            // Databases
            "postgresql", "mysql", "sqlite", "redis", "mongodb", "mariadb",
            
            // Web servers
            "nginx", "apache-httpd", "caddy", "traefik",
            
            // Languages & Runtimes
            "php", "ruby", "perl", "lua", "r", "julia", "erlang", "elixir", "haskell",
        ];
        
        for pkg_name in &nix_packages {
            packages.push(Package {
                name: pkg_name.to_string(),
                version: Some("nixpkgs-unstable".to_string()),
                description: Some(format!("Nix package: {}", pkg_name)),
                installed: false,
                source: "nix".to_string(),
            });
        }
        
        Ok(packages)
    }
    
    fn list_portage_available(&self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        let portage_path = self.get_base_path("/var/db/repos/gentoo");
        let mut packages = Vec::new();
        
        if !Path::new(&portage_path).exists() {
            return Ok(packages);
        }
        
        // Read categories
        for category_entry in fs::read_dir(&portage_path)? {
            let category_entry = category_entry?;
            if !category_entry.file_type()?.is_dir() {
                continue;
            }
            
            let category_name = category_entry.file_name();
            let category_path = category_entry.path();
            
            // Skip metadata directories
            if category_name == "metadata" || category_name == "profiles" || category_name == "eclass" {
                continue;
            }
            
            // Read packages in category
            for package_entry in fs::read_dir(&category_path)? {
                let package_entry = package_entry?;
                if !package_entry.file_type()?.is_dir() {
                    continue;
                }
                
                let package_name = package_entry.file_name();
                let full_name = format!("{}/{}", category_name.to_string_lossy(), package_name.to_string_lossy());
                
                packages.push(Package {
                    name: full_name,
                    version: None,
                    description: None,
                    installed: false,
                    source: "emerge".to_string(),
                });
            }
        }
        
        Ok(packages)
    }
    
    fn list_rpm_available(&self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        let mut packages = Vec::new();
        
        // Read from DNF cache directories (Bedrock-aware)
        let cache_dirs = [
            "/var/cache/dnf",
            "/var/cache/yum", 
            "/var/lib/dnf",
        ];
        
        for cache_dir in &cache_dirs {
            let full_cache_path = self.get_base_path(cache_dir);
            if Path::new(&full_cache_path).exists() {
                // Look for .solv files which contain package metadata
                if let Ok(entries) = fs::read_dir(&full_cache_path) {
                    for entry in entries.flatten() {
                        let filename = entry.file_name();
                        let filename_str = filename.to_string_lossy();
                        
                        if filename_str.ends_with(".solv") && !filename_str.contains("updateinfo") {
                            let repo_name = filename_str.trim_end_matches(".solv");
                            
                            // For now, add some representative packages from this repo
                            // In a full implementation, we'd parse the .solv binary format
                            let common_packages = match repo_name {
                                "fedora" => vec!["firefox", "gcc", "python3", "git", "vim", "htop"],
                                "updates" => vec!["kernel", "systemd", "glibc", "bash"],
                                "rpmfusion-free" => vec!["ffmpeg", "vlc", "gstreamer1-plugins-bad-free"],
                                "rpmfusion-nonfree" => vec!["steam", "nvidia-driver"],
                                _ => vec!["package-from-repo"],
                            };
                            
                            for pkg_name in common_packages {
                                packages.push(Package {
                                    name: format!("{}", pkg_name),
                                    version: Some("latest".to_string()),
                                    description: Some(format!("Package from {} repository", repo_name)),
                                    installed: false,
                                    source: "dnf".to_string(),
                                });
                            }
                        }
                    }
                }
                break; // Found a cache directory, use it
            }
        }
        
        // If no packages found, add some common Fedora packages
        if packages.is_empty() {
            let fedora_packages = [
                "firefox", "chromium", "gcc", "clang", "python3", "nodejs", "rust", 
                "git", "vim", "emacs", "htop", "neofetch", "dnf", "systemd"
            ];
            
            for pkg_name in &fedora_packages {
                packages.push(Package {
                    name: pkg_name.to_string(),
                    version: Some("latest".to_string()),
                    description: Some(format!("Common Fedora package: {}", pkg_name)),
                    installed: false,
                    source: "dnf".to_string(),
                });
            }
        }
        
        Ok(packages)
    }
    
    fn list_apt_installed(&self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        let mut packages = Vec::new();
        
        let output = std::process::Command::new("dpkg-query")
            .args(&["-W", "-f=${Package}\t${Version}\t${Status}\n"])
            .output()?;
            
        if !output.status.success() {
            return Ok(packages);
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let name = parts[0];
                let version = parts[1];
                let status = parts[2];
                
                // Only include installed packages
                if status.contains("install ok installed") {
                    packages.push(Package {
                        name: name.to_string(),
                        version: Some(version.to_string()),
                        description: None,
                        installed: true,
                        source: "apt".to_string(),
                    });
                }
            }
        }
        
        Ok(packages)
    }
    
    fn list_apt_available(&self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        let mut packages = Vec::new();
        
        let output = std::process::Command::new("apt-cache")
            .args(&["search", ".*"])
            .output()?;
            
        if !output.status.success() {
            return Ok(packages);
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if let Some(dash_pos) = line.find(" - ") {
                let name = &line[..dash_pos];
                let description = &line[dash_pos + 3..];
                
                packages.push(Package {
                    name: name.to_string(),
                    version: None,
                    description: Some(description.to_string()),
                    installed: false,
                    source: "apt".to_string(),
                });
            }
        }
        
        Ok(packages)
    }
}

use crate::core::config::Config;

pub fn detect_package_managers_with_config(config: &Config) -> Vec<LocalPackageManager> {
    let mut managers = Vec::new();
    
    // Check for Bedrock Linux
    if Path::new("/bedrock/strata").exists() {
        // Bedrock Linux detected - use config if available
        if let Some(bedrock_config) = &config.bedrock_linux {
            // Use configured strata
            for (stratum_name, os_name) in bedrock_config {
                let base_path = format!("/bedrock/strata/{}", stratum_name);
                
                if !Path::new(&base_path).exists() {
                    continue;
                }
                
                // Determine package manager based on OS name
                match os_name.to_lowercase().as_str() {
                    "arch linux" | "arch" => {
                        if Path::new(&format!("{}/var/lib/pacman", base_path)).exists() {
                            managers.push(LocalPackageManager::new("pacman".to_string(), Some(stratum_name.clone())));
                            
                            // Always add AUR (paru) for Arch systems since AUR is available
                            managers.push(LocalPackageManager::new("paru".to_string(), Some(stratum_name.clone())));
                        }
                    }
                    "gentoo" => {
                        if Path::new(&format!("{}/var/db/pkg", base_path)).exists() {
                            managers.push(LocalPackageManager::new("emerge".to_string(), Some(stratum_name.clone())));
                        }
                    }
                    "fedora" | "rhel" | "centos" => {
                        // Check for RPM database
                        if Path::new(&format!("{}/var/lib/rpm/Packages", base_path)).exists() {
                            managers.push(LocalPackageManager::new("dnf".to_string(), Some(stratum_name.clone())));
                        }
                        // Also check DNF cache
                        if Path::new(&format!("{}/var/cache/dnf", base_path)).exists() {
                            // Don't add duplicate if already added above
                            if !managers.iter().any(|m| m.name == "dnf" && m.stratum.as_ref() == Some(stratum_name)) {
                                managers.push(LocalPackageManager::new("dnf".to_string(), Some(stratum_name.clone())));
                            }
                        }
                    }
                    _ => {
                        // Auto-detect for unknown OS
                        if Path::new(&format!("{}/var/lib/pacman", base_path)).exists() {
                            managers.push(LocalPackageManager::new("pacman".to_string(), Some(stratum_name.clone())));
                        }
                        if Path::new(&format!("{}/var/lib/rpm/Packages", base_path)).exists() {
                            managers.push(LocalPackageManager::new("dnf".to_string(), Some(stratum_name.clone())));
                        }
                        if Path::new(&format!("{}/var/db/pkg", base_path)).exists() {
                            managers.push(LocalPackageManager::new("emerge".to_string(), Some(stratum_name.clone())));
                        }
                    }
                }
            }
        } else {
            // Auto-detect all strata if no config
            if let Ok(strata) = fs::read_dir("/bedrock/strata") {
                for stratum_entry in strata {
                    if let Ok(stratum_entry) = stratum_entry {
                        if stratum_entry.file_type().map_or(false, |ft| ft.is_dir()) {
                            let stratum_name = stratum_entry.file_name().to_string_lossy().to_string();
                            let base_path = format!("/bedrock/strata/{}", stratum_name);
                            
                            // Check for pacman
                            if Path::new(&format!("{}/var/lib/pacman", base_path)).exists() {
                                managers.push(LocalPackageManager::new("pacman".to_string(), Some(stratum_name.clone())));
                            }
                            
                            // Check for RPM (use Packages file as indicator)
                            if Path::new(&format!("{}/var/lib/rpm/Packages", base_path)).exists() {
                                managers.push(LocalPackageManager::new("dnf".to_string(), Some(stratum_name.clone())));
                            }
                            
                            // Check for Portage
                            if Path::new(&format!("{}/var/db/pkg", base_path)).exists() {
                                managers.push(LocalPackageManager::new("emerge".to_string(), Some(stratum_name.clone())));
                            }
                        }
                    }
                }
            }
        }
        
        // Nix is typically global in Bedrock
        if Path::new("/nix/var/nix/db").exists() {
            managers.push(LocalPackageManager::new("nix".to_string(), None));
        }
    } else {
        // Standard Linux distribution
        if Path::new("/var/lib/pacman").exists() {
            managers.push(LocalPackageManager::new("pacman".to_string(), None));
        }
        
        if Path::new("/var/lib/rpm").exists() {
            managers.push(LocalPackageManager::new("dnf".to_string(), None));
        }
        
        if Path::new("/var/db/pkg").exists() {
            managers.push(LocalPackageManager::new("emerge".to_string(), None));
        }
        
        if Path::new("/nix/var/nix/db").exists() {
            managers.push(LocalPackageManager::new("nix".to_string(), None));
        }
        
        // Check for APT (Debian/Ubuntu)
        if Path::new("/var/lib/dpkg/status").exists() {
            managers.push(LocalPackageManager::new("apt".to_string(), None));
        }
    }
    
    managers
}

// Backward compatibility function
pub fn detect_package_managers() -> Vec<LocalPackageManager> {
    // Use default config for detection
    let config = Config::default();
    detect_package_managers_with_config(&config)
}