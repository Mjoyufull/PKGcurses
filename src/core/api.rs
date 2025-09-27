use serde::Deserialize;
use crate::core::package_managers::Package;

#[derive(Debug, Deserialize)]
pub struct ArchPackageResult {
    pub results: Vec<ArchPackage>,
    pub version: u32,
    pub limit: u32,
    pub valid: bool,
}

#[derive(Debug, Deserialize)]
pub struct ArchPackage {
    pub pkgname: String,
    pub pkgver: String,
    pub pkgdesc: Option<String>,
    pub repo: String,
    pub arch: String,
    pub maintainers: Vec<String>,
    pub packager: String,
    pub url: Option<String>,
}

pub struct ArchApi;

impl ArchApi {
    pub async fn search_packages(query: &str) -> Result<Vec<Package>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("https://archlinux.org/packages/search/json/?q={}", 
                         urlencoding::encode(query));
        
        let response = reqwest::get(&url).await?;
        let arch_result: ArchPackageResult = response.json().await?;
        
        let packages: Vec<Package> = arch_result.results
            .into_iter()
            .map(|arch_pkg| Package {
                name: arch_pkg.pkgname,
                version: Some(arch_pkg.pkgver),
                description: arch_pkg.pkgdesc,
                installed: false, // Will be determined later
                source: "pacman".to_string(),
            })
            .collect();
        
        Ok(packages)
    }
    
    pub async fn get_package_details(repo: &str, arch: &str, name: &str) -> Result<Option<Package>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("https://archlinux.org/packages/{}/{}/{}/json/", repo, arch, name);
        
        let response = reqwest::get(&url).await?;
        if !response.status().is_success() {
            return Ok(None);
        }
        
        let arch_pkg: ArchPackage = response.json().await?;
        
        Ok(Some(Package {
            name: arch_pkg.pkgname,
            version: Some(arch_pkg.pkgver),
            description: arch_pkg.pkgdesc,
            installed: false,
            source: "pacman".to_string(),
        }))
    }
}