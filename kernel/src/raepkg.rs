//! RaePkg - Package manager for RaeenOS

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

// Package metadata
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub dependencies: Vec<String>,
    pub size: u64,
    pub install_path: String,
    pub checksum: String,
    pub installed: bool,
    pub install_time: u64,
}

impl PackageInfo {
    fn new(name: String, version: String, description: String, author: String) -> Self {
        Self {
            name,
            version,
            description,
            author,
            dependencies: Vec::new(),
            size: 0,
            install_path: String::new(),
            checksum: String::new(),
            installed: false,
            install_time: 0,
        }
    }
}

// Package repository
#[derive(Debug, Clone)]
struct Repository {
    name: String,
    url: String,
    enabled: bool,
    packages: BTreeMap<String, PackageInfo>,
}

impl Repository {
    fn new(name: String, url: String) -> Self {
        Self {
            name,
            url,
            enabled: true,
            packages: BTreeMap::new(),
        }
    }
}

// Package installation status
#[derive(Debug, Clone)]
pub enum InstallResult {
    Success,
    AlreadyInstalled,
    DependencyError(String),
    DownloadError(String),
    PermissionDenied,
    InsufficientSpace,
    InvalidPackage(String),
}

// Package removal status
#[derive(Debug, Clone)]
pub enum RemoveResult {
    Success,
    NotInstalled,
    DependencyConflict(Vec<String>),
    PermissionDenied,
    SystemPackage,
}

// Package search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub packages: Vec<PackageInfo>,
    pub total_count: usize,
}

// Package manager system
struct PackageSystem {
    repositories: BTreeMap<String, Repository>,
    installed_packages: BTreeMap<String, PackageInfo>,
    cache_directory: String,
    install_directory: String,
    update_available: bool,
    last_update: u64,
}

lazy_static! {
    static ref PACKAGE_SYSTEM: Mutex<PackageSystem> = {
        let mut system = PackageSystem {
            repositories: BTreeMap::new(),
            installed_packages: BTreeMap::new(),
            cache_directory: "/var/cache/raepkg".to_string(),
            install_directory: "/usr/local".to_string(),
            update_available: false,
            last_update: 0,
        };
        
        // Add default repository
        let mut default_repo = Repository::new(
            "official".to_string(),
            "https://packages.raeenos.org".to_string()
        );
        
        // Add some default packages
        let mut gcc_pkg = PackageInfo::new(
            "gcc".to_string(),
            "11.2.0".to_string(),
            "GNU Compiler Collection".to_string(),
            "GNU Project".to_string()
        );
        gcc_pkg.size = 50 * 1024 * 1024; // 50MB
        gcc_pkg.dependencies.push("binutils".to_string());
        gcc_pkg.dependencies.push("glibc-dev".to_string());
        
        let mut python_pkg = PackageInfo::new(
            "python3".to_string(),
            "3.9.7".to_string(),
            "Python 3 programming language".to_string(),
            "Python Software Foundation".to_string()
        );
        python_pkg.size = 25 * 1024 * 1024; // 25MB
        
        let mut git_pkg = PackageInfo::new(
            "git".to_string(),
            "2.34.1".to_string(),
            "Distributed version control system".to_string(),
            "Linus Torvalds".to_string()
        );
        git_pkg.size = 15 * 1024 * 1024; // 15MB
        git_pkg.dependencies.push("curl".to_string());
        
        let mut vim_pkg = PackageInfo::new(
            "vim".to_string(),
            "8.2.3458".to_string(),
            "Vi IMproved text editor".to_string(),
            "Bram Moolenaar".to_string()
        );
        vim_pkg.size = 8 * 1024 * 1024; // 8MB
        
        let mut curl_pkg = PackageInfo::new(
            "curl".to_string(),
            "7.80.0".to_string(),
            "Command line tool for transferring data".to_string(),
            "Daniel Stenberg".to_string()
        );
        curl_pkg.size = 3 * 1024 * 1024; // 3MB
        
        let mut binutils_pkg = PackageInfo::new(
            "binutils".to_string(),
            "2.37".to_string(),
            "GNU binary utilities".to_string(),
            "GNU Project".to_string()
        );
        binutils_pkg.size = 20 * 1024 * 1024; // 20MB
        
        let mut glibc_dev_pkg = PackageInfo::new(
            "glibc-dev".to_string(),
            "2.34".to_string(),
            "GNU C Library development files".to_string(),
            "GNU Project".to_string()
        );
        glibc_dev_pkg.size = 12 * 1024 * 1024; // 12MB
        
        default_repo.packages.insert("gcc".to_string(), gcc_pkg);
        default_repo.packages.insert("python3".to_string(), python_pkg);
        default_repo.packages.insert("git".to_string(), git_pkg);
        default_repo.packages.insert("vim".to_string(), vim_pkg);
        default_repo.packages.insert("curl".to_string(), curl_pkg);
        default_repo.packages.insert("binutils".to_string(), binutils_pkg);
        default_repo.packages.insert("glibc-dev".to_string(), glibc_dev_pkg);
        
        system.repositories.insert("official".to_string(), default_repo);
        
        Mutex::new(system)
    };
}

// Initialize package system
pub fn init_package_system() -> Result<(), ()> {
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.manage").unwrap_or(false) {
        return Err(());
    }
    
    // Create cache and install directories
    let _ = crate::fs::create_directory("/var/cache/raepkg");
    let _ = crate::fs::create_directory("/usr/local");
    
    Ok(())
}

// Install a package
pub fn install_package(package_name: &str) -> Result<InstallResult, ()> {
    let mut pkg_system = PACKAGE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.install").unwrap_or(false) {
        return Ok(InstallResult::PermissionDenied);
    }
    
    // Check if already installed
    if pkg_system.installed_packages.contains_key(package_name) {
        return Ok(InstallResult::AlreadyInstalled);
    }
    
    // Find package in repositories
    let mut package_info = None;
    for repo in pkg_system.repositories.values() {
        if repo.enabled {
            if let Some(pkg) = repo.packages.get(package_name) {
                package_info = Some(pkg.clone());
                break;
            }
        }
    }
    
    let mut package = match package_info {
        Some(pkg) => pkg,
        None => return Ok(InstallResult::InvalidPackage("Package not found".to_string())),
    };
    
    // Check dependencies
    for dep in &package.dependencies {
        if !pkg_system.installed_packages.contains_key(dep) {
            // Try to install dependency first
            drop(pkg_system);
            match install_package(dep)? {
                InstallResult::Success | InstallResult::AlreadyInstalled => {
                    pkg_system = PACKAGE_SYSTEM.lock();
                }
                other => return Ok(other),
            }
        }
    }
    
    // Check available space
    let free_space = crate::memory::get_free_memory() as u64;
    if package.size > free_space {
        return Ok(InstallResult::InsufficientSpace);
    }
    
    // Simulate package installation
    package.installed = true;
    package.install_time = crate::time::get_system_uptime();
    package.install_path = format!("{}/{}", pkg_system.install_directory, package_name);
    
    // Create package directory
    let _ = crate::fs::create_directory(&package.install_path);
    
    // Add to installed packages
    pkg_system.installed_packages.insert(package_name.to_string(), package);
    
    Ok(InstallResult::Success)
}

// Remove a package
pub fn remove_package(package_name: &str) -> Result<RemoveResult, ()> {
    let mut pkg_system = PACKAGE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.remove").unwrap_or(false) {
        return Ok(RemoveResult::PermissionDenied);
    }
    
    // Check if package is installed
    let package = match pkg_system.installed_packages.get(package_name) {
        Some(pkg) => pkg.clone(),
        None => return Ok(RemoveResult::NotInstalled),
    };
    
    // Check for system packages
    if package_name == "kernel" || package_name == "init" {
        return Ok(RemoveResult::SystemPackage);
    }
    
    // Check for dependency conflicts
    let mut dependent_packages = Vec::new();
    for (name, pkg) in &pkg_system.installed_packages {
        if pkg.dependencies.contains(&package_name.to_string()) {
            dependent_packages.push(name.clone());
        }
    }
    
    if !dependent_packages.is_empty() {
        return Ok(RemoveResult::DependencyConflict(dependent_packages));
    }
    
    // Remove package directory
    let _ = crate::fs::remove_directory(&package.install_path);
    
    // Remove from installed packages
    pkg_system.installed_packages.remove(package_name);
    
    Ok(RemoveResult::Success)
}

// Search for packages
pub fn search_packages(query: &str) -> Result<SearchResult, ()> {
    let pkg_system = PACKAGE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.search").unwrap_or(false) {
        return Err(());
    }
    
    let mut results = Vec::new();
    let query_lower = query.to_lowercase();
    
    // Search in all repositories
    for repo in pkg_system.repositories.values() {
        if repo.enabled {
            for pkg in repo.packages.values() {
                if pkg.name.to_lowercase().contains(&query_lower) ||
                   pkg.description.to_lowercase().contains(&query_lower) {
                    results.push(pkg.clone());
                }
            }
        }
    }
    
    let total_count = results.len();
    
    Ok(SearchResult {
        packages: results,
        total_count,
    })
}

// List installed packages
pub fn list_installed_packages() -> Result<Vec<PackageInfo>, ()> {
    let pkg_system = PACKAGE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.list").unwrap_or(false) {
        return Err(());
    }
    
    Ok(pkg_system.installed_packages.values().cloned().collect())
}

// Get package information
pub fn get_package_info(package_name: &str) -> Result<Option<PackageInfo>, ()> {
    let pkg_system = PACKAGE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.info").unwrap_or(false) {
        return Err(());
    }
    
    // Check installed packages first
    if let Some(pkg) = pkg_system.installed_packages.get(package_name) {
        return Ok(Some(pkg.clone()));
    }
    
    // Search in repositories
    for repo in pkg_system.repositories.values() {
        if repo.enabled {
            if let Some(pkg) = repo.packages.get(package_name) {
                return Ok(Some(pkg.clone()));
            }
        }
    }
    
    Ok(None)
}

// Update package database
pub fn update_package_database() -> Result<usize, ()> {
    let mut pkg_system = PACKAGE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.update").unwrap_or(false) {
        return Err(());
    }
    
    // Simulate updating package database
    let mut updated_count = 0;
    
    for repo in pkg_system.repositories.values_mut() {
        if repo.enabled {
            // Simulate fetching updates
            updated_count += repo.packages.len();
        }
    }
    
    pkg_system.last_update = crate::time::get_system_uptime();
    pkg_system.update_available = false;
    
    Ok(updated_count)
}

// Upgrade installed packages
pub fn upgrade_packages() -> Result<Vec<String>, ()> {
    let mut pkg_system = PACKAGE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.upgrade").unwrap_or(false) {
        return Err(());
    }
    
    let mut upgraded_packages = Vec::new();
    
    // Check for upgrades
    for (name, installed_pkg) in pkg_system.installed_packages.iter_mut() {
        for repo in pkg_system.repositories.values() {
            if repo.enabled {
                if let Some(repo_pkg) = repo.packages.get(name) {
                    if repo_pkg.version != installed_pkg.version {
                        // Simulate upgrade
                        installed_pkg.version = repo_pkg.version.clone();
                        upgraded_packages.push(name.clone());
                        break;
                    }
                }
            }
        }
    }
    
    Ok(upgraded_packages)
}

// Add repository
pub fn add_repository(name: &str, url: &str) -> Result<(), ()> {
    let mut pkg_system = PACKAGE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.repo.add").unwrap_or(false) {
        return Err(());
    }
    
    let repo = Repository::new(name.to_string(), url.to_string());
    pkg_system.repositories.insert(name.to_string(), repo);
    
    Ok(())
}

// Remove repository
pub fn remove_repository(name: &str) -> Result<(), ()> {
    let mut pkg_system = PACKAGE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.repo.remove").unwrap_or(false) {
        return Err(());
    }
    
    // Don't allow removing the official repository
    if name == "official" {
        return Err(());
    }
    
    pkg_system.repositories.remove(name);
    Ok(())
}

// List repositories
pub fn list_repositories() -> Result<Vec<(String, String, bool)>, ()> {
    let pkg_system = PACKAGE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.repo.list").unwrap_or(false) {
        return Err(());
    }
    
    let repos = pkg_system.repositories
        .values()
        .map(|repo| (repo.name.clone(), repo.url.clone(), repo.enabled))
        .collect();
    
    Ok(repos)
}

// Enable/disable repository
pub fn set_repository_enabled(name: &str, enabled: bool) -> Result<(), ()> {
    let mut pkg_system = PACKAGE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.repo.manage").unwrap_or(false) {
        return Err(());
    }
    
    if let Some(repo) = pkg_system.repositories.get_mut(name) {
        repo.enabled = enabled;
        Ok(())
    } else {
        Err(())
    }
}

// Get package statistics
pub fn get_package_statistics() -> Result<(usize, usize, u64, u64), ()> {
    let pkg_system = PACKAGE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.stats").unwrap_or(false) {
        return Err(());
    }
    
    let installed_count = pkg_system.installed_packages.len();
    
    let mut available_count = 0;
    for repo in pkg_system.repositories.values() {
        if repo.enabled {
            available_count += repo.packages.len();
        }
    }
    
    let total_installed_size: u64 = pkg_system.installed_packages
        .values()
        .map(|pkg| pkg.size)
        .sum();
    
    let cache_size = 0u64; // Simulate cache size
    
    Ok((installed_count, available_count, total_installed_size, cache_size))
}

// Clean package cache
pub fn clean_package_cache() -> Result<u64, ()> {
    let pkg_system = PACKAGE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.clean").unwrap_or(false) {
        return Err(());
    }
    
    // Simulate cleaning cache
    let cleaned_size = 50 * 1024 * 1024; // 50MB
    
    // Remove cache files
    let _ = crate::fs::remove_directory(&pkg_system.cache_directory);
    let _ = crate::fs::create_directory(&pkg_system.cache_directory);
    
    Ok(cleaned_size)
}

// Check for package updates
pub fn check_for_updates() -> Result<Vec<String>, ()> {
    let pkg_system = PACKAGE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.check").unwrap_or(false) {
        return Err(());
    }
    
    let mut updates_available = Vec::new();
    
    // Check for updates
    for (name, installed_pkg) in &pkg_system.installed_packages {
        for repo in pkg_system.repositories.values() {
            if repo.enabled {
                if let Some(repo_pkg) = repo.packages.get(name) {
                    if repo_pkg.version != installed_pkg.version {
                        updates_available.push(name.clone());
                        break;
                    }
                }
            }
        }
    }
    
    Ok(updates_available)
}

// Verify package integrity
pub fn verify_package(package_name: &str) -> Result<bool, ()> {
    let pkg_system = PACKAGE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "package.verify").unwrap_or(false) {
        return Err(());
    }
    
    // Check if package is installed
    if let Some(package) = pkg_system.installed_packages.get(package_name) {
        // Simulate integrity check
        let _ = crate::fs::get_metadata(&package.install_path);
        Ok(true) // Assume package is valid
    } else {
        Ok(false)
    }
}

// Clean up package system for a process
pub fn cleanup_process_packages(process_id: u32) {
    // Package system is global, no per-process cleanup needed
    let _ = process_id;
}