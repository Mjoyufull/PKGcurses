use crate::core::package_managers::Package;
use std::fs;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

pub struct Cache {
    cache_dir: PathBuf,
    max_age_hours: u64,
}

impl Cache {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let home = std::env::var("HOME")?;
        let cache_dir = PathBuf::from(home).join(".cache").join("pmux");
        fs::create_dir_all(&cache_dir)?;
        
        Ok(Cache {
            cache_dir,
            max_age_hours: 24,
        })
    }
    
    pub fn is_fresh(&self, pm_name: &str) -> bool {
        let cache_file = self.cache_dir.join(format!("{}_packages.txt", pm_name));
        if !cache_file.exists() {
            return false;
        }
        
        if let Ok(metadata) = fs::metadata(&cache_file) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                    let age_hours = duration.as_secs() / 3600;
                    return age_hours < self.max_age_hours;
                }
            }
        }
        false
    }
    
    pub fn save_packages(&self, pm_name: &str, packages: &[Package]) -> Result<(), Box<dyn std::error::Error>> {
        let cache_file = self.cache_dir.join(format!("{}_packages.txt", pm_name));
        let content: Vec<String> = packages.iter().map(|p| p.name.clone()).collect();
        fs::write(cache_file, content.join("\n"))?;
        Ok(())
    }
    
    pub fn load_packages(&self, pm_name: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let cache_file = self.cache_dir.join(format!("{}_packages.txt", pm_name));
        if cache_file.exists() {
            let content = fs::read_to_string(cache_file)?;
            Ok(content.lines().map(|l| l.to_string()).collect())
        } else {
            Ok(vec![])
        }
    }
    
    pub fn save_installed(&self, pm_name: &str, packages: &[Package]) -> Result<(), Box<dyn std::error::Error>> {
        let cache_file = self.cache_dir.join(format!("{}_installed.txt", pm_name));
        let content: Vec<String> = packages.iter().map(|p| p.name.clone()).collect();
        fs::write(cache_file, content.join("\n"))?;
        Ok(())
    }
    
    pub fn load_installed(&self, pm_name: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let cache_file = self.cache_dir.join(format!("{}_installed.txt", pm_name));
        if cache_file.exists() {
            let content = fs::read_to_string(cache_file)?;
            Ok(content.lines().map(|l| l.to_string()).collect())
        } else {
            Ok(vec![])
        }
    }
}