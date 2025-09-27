use crate::core::package_managers::Package;
use crate::core::api::ArchApi;
use crate::core::local::{LocalPackageManager, detect_package_managers};
use std::collections::HashMap;

pub struct UnifiedPackageManager {
    local_managers: Vec<LocalPackageManager>,
    installed_cache: HashMap<String, Vec<Package>>,
}

impl UnifiedPackageManager {
    pub fn new() -> Self {
        let local_managers = detect_package_managers();
        
        Self {
            local_managers,
            installed_cache: HashMap::new(),
        }
    }
    
    pub async fn load_installed_packages(&mut self) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        let mut all_installed = Vec::new();
        
        for manager in &self.local_managers {
            match manager.list_installed() {
                Ok(packages) => {
                    // Cache installed packages by source
                    self.installed_cache.insert(manager.name.clone(), packages.clone());
                    all_installed.extend(packages);
                }
                Err(e) => {
                    eprintln!("Failed to load installed packages from {}: {}", manager.name, e);
                }
            }
        }
        
        Ok(all_installed)
    }
    
    pub async fn search_packages(&self, query: &str) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        let mut all_packages = Vec::new();
        
        // Search using APIs for available packages
        for manager in &self.local_managers {
            match manager.name.as_str() {
                "pacman" => {
                    match ArchApi::search_packages(query).await {
                        Ok(mut packages) => {
                            // Mark installed packages
                            if let Some(installed) = self.installed_cache.get("pacman") {
                                for package in &mut packages {
                                    package.installed = installed.iter().any(|p| p.name == package.name);
                                }
                            }
                            all_packages.extend(packages);
                        }
                        Err(e) => {
                            eprintln!("Failed to search Arch packages: {}", e);
                        }
                    }
                }
                "nix" => {
                    // TODO: Implement Nix search API or local parsing
                    // For now, search through installed packages
                    if let Some(installed) = self.installed_cache.get("nix") {
                        let matching: Vec<Package> = installed
                            .iter()
                            .filter(|p| p.name.to_lowercase().contains(&query.to_lowercase()))
                            .cloned()
                            .collect();
                        all_packages.extend(matching);
                    }
                }
                "emerge" => {
                    // TODO: Implement Gentoo packages.gentoo.org API
                    // For now, search through installed packages
                    if let Some(installed) = self.installed_cache.get("emerge") {
                        let matching: Vec<Package> = installed
                            .iter()
                            .filter(|p| p.name.to_lowercase().contains(&query.to_lowercase()))
                            .cloned()
                            .collect();
                        all_packages.extend(matching);
                    }
                }
                "dnf" => {
                    // TODO: Implement Fedora API
                    // For now, search through installed packages
                    if let Some(installed) = self.installed_cache.get("dnf") {
                        let matching: Vec<Package> = installed
                            .iter()
                            .filter(|p| p.name.to_lowercase().contains(&query.to_lowercase()))
                            .cloned()
                            .collect();
                        all_packages.extend(matching);
                    }
                }
                _ => {}
            }
        }
        
        Ok(all_packages)
    }
    
    pub fn get_installed_packages(&self) -> Vec<Package> {
        let mut all_installed = Vec::new();
        
        for packages in self.installed_cache.values() {
            all_installed.extend(packages.clone());
        }
        
        all_installed
    }
    
    pub fn get_available_managers(&self) -> Vec<String> {
        self.local_managers.iter().map(|m| m.name.clone()).collect()
    }
}