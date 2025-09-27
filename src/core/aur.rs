use serde::{Deserialize, Serialize};
use crate::core::package_managers::Package;
use std::error::Error;

#[derive(Debug, Deserialize)]
pub struct AurResponse {
    pub resultcount: u32,
    pub results: Vec<AurPackage>,
    #[serde(rename = "type")]
    pub response_type: String,
    pub version: u32,
}

#[derive(Debug, Deserialize)]
pub struct AurPackage {
    #[serde(rename = "ID")]
    pub id: u32,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "PackageBaseID")]
    pub package_base_id: u32,
    #[serde(rename = "PackageBase")]
    pub package_base: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "URL")]
    pub url: Option<String>,
    #[serde(rename = "NumVotes")]
    pub num_votes: u32,
    #[serde(rename = "Popularity")]
    pub popularity: f64,
    #[serde(rename = "OutOfDate")]
    pub out_of_date: Option<u64>,
    #[serde(rename = "Maintainer")]
    pub maintainer: Option<String>,
    #[serde(rename = "FirstSubmitted")]
    pub first_submitted: u64,
    #[serde(rename = "LastModified")]
    pub last_modified: u64,
    #[serde(rename = "URLPath")]
    pub url_path: String,
}

pub struct AurClient {
    base_url: String,
    client: reqwest::Client,
}

impl AurClient {
    pub fn new() -> Self {
        Self {
            base_url: "https://aur.archlinux.org/rpc/".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Package>, Box<dyn Error + Send + Sync>> {
        let url = format!("{}?v=5&type=search&arg={}", self.base_url, urlencoding::encode(query));
        
        let response = self.client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        let aur_response: AurResponse = response.json().await?;
        
        let packages = aur_response.results
            .into_iter()
            .map(|aur_pkg| Package {
                name: aur_pkg.name,
                version: Some(aur_pkg.version),
                description: aur_pkg.description,
                installed: false, // We'll check this separately
                source: "aur".to_string(),
            })
            .collect();

        Ok(packages)
    }

    pub async fn get_info(&self, package_names: &[String]) -> Result<Vec<Package>, Box<dyn Error + Send + Sync>> {
        if package_names.is_empty() {
            return Ok(Vec::new());
        }

        let names = package_names.join("&arg[]=");
        let url = format!("{}?v=5&type=info&arg[]={}", self.base_url, names);
        
        let response = self.client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        let aur_response: AurResponse = response.json().await?;
        
        let packages = aur_response.results
            .into_iter()
            .map(|aur_pkg| Package {
                name: aur_pkg.name,
                version: Some(aur_pkg.version),
                description: aur_pkg.description,
                installed: false,
                source: "aur".to_string(),
            })
            .collect();

        Ok(packages)
    }

    pub async fn get_package_details(&self, package_name: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let packages = self.get_info(&[package_name.to_string()]).await?;
        
        if let Some(package) = packages.first() {
            let mut details = format!("Package: {}\n", package.name);
            details.push_str(&format!("Source: AUR\n"));
            
            if let Some(version) = &package.version {
                details.push_str(&format!("Version: {}\n", version));
            }
            
            if let Some(description) = &package.description {
                details.push_str(&format!("Description: {}\n", description));
            }
            
            details.push_str("\nThis is an AUR (Arch User Repository) package.\n");
            details.push_str("Installation requires building from source.\n");
            
            Ok(details)
        } else {
            Ok(format!("Package '{}' not found in AUR", package_name))
        }
    }
}

impl Default for AurClient {
    fn default() -> Self {
        Self::new()
    }
}