//! RaeKit - Application development framework for RaeenOS

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

// Application metadata
#[derive(Debug, Clone)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub executable_path: String,
    pub icon_path: String,
    pub permissions: Vec<String>,
    pub dependencies: Vec<String>,
    pub app_type: AppType,
    pub install_time: u64,
    pub last_run: u64,
    pub run_count: u32,
}

// Application types
#[derive(Debug, Clone, PartialEq)]
pub enum AppType {
    System,
    User,
    Service,
    Library,
    Game,
    Utility,
    Development,
}

// Application framework capabilities
#[derive(Debug, Clone)]
pub enum FrameworkCapability {
    Graphics2D,
    Graphics3D,
    Audio,
    Network,
    FileSystem,
    Database,
    Cryptography,
    AI,
    WebView,
    Notifications,
}

// Application lifecycle state
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Stopped,
    Starting,
    Running,
    Paused,
    Stopping,
    Crashed,
}

// Application runtime context
#[derive(Debug, Clone)]
struct AppContext {
    app_id: u32,
    process_id: u32,
    state: AppState,
    capabilities: Vec<FrameworkCapability>,
    resource_limits: ResourceLimits,
    start_time: u64,
    memory_usage: u64,
    cpu_usage: f32,
}

// Resource limits for applications
#[derive(Debug, Clone)]
struct ResourceLimits {
    max_memory: u64,
    max_cpu_percent: f32,
    max_file_handles: u32,
    max_network_connections: u32,
    max_threads: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: 64 * 1024 * 1024, // 64MB
            max_cpu_percent: 50.0,
            max_file_handles: 100,
            max_network_connections: 50,
            max_threads: 10,
        }
    }
}

// Application development project
#[derive(Debug, Clone)]
pub struct DevProject {
    pub name: String,
    pub path: String,
    pub language: String,
    pub framework: String,
    pub target_type: AppType,
    pub created_time: u64,
    pub last_modified: u64,
    pub build_config: BuildConfig,
}

// Build configuration
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub optimization_level: OptimizationLevel,
    pub debug_symbols: bool,
    pub target_arch: String,
    pub output_format: OutputFormat,
    pub custom_flags: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum OptimizationLevel {
    None,
    Basic,
    Aggressive,
    Size,
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Executable,
    Library,
    Service,
    Module,
}

// RaeKit development system
struct RaeKitSystem {
    registered_apps: BTreeMap<String, AppInfo>,
    running_apps: BTreeMap<u32, AppContext>,
    dev_projects: BTreeMap<String, DevProject>,
    next_app_id: u32,
    framework_templates: BTreeMap<String, String>,
    build_tools: BTreeMap<String, String>,
}

lazy_static! {
    static ref RAEKIT_SYSTEM: Mutex<RaeKitSystem> = {
        let mut system = RaeKitSystem {
            registered_apps: BTreeMap::new(),
            running_apps: BTreeMap::new(),
            dev_projects: BTreeMap::new(),
            next_app_id: 1,
            framework_templates: BTreeMap::new(),
            build_tools: BTreeMap::new(),
        };
        
        // Register some default applications
        let text_editor = AppInfo {
            name: "RaeEdit".to_string(),
            version: "1.0.0".to_string(),
            description: "Simple text editor for RaeenOS".to_string(),
            author: "RaeenOS Team".to_string(),
            executable_path: "/usr/bin/raeedit".to_string(),
            icon_path: "/usr/share/icons/raeedit.svg".to_string(),
            permissions: vec!["fs.read".to_string(), "fs.write".to_string()],
            dependencies: Vec::new(),
            app_type: AppType::Utility,
            install_time: 0,
            last_run: 0,
            run_count: 0,
        };
        
        let file_manager = AppInfo {
            name: "RaeFiles".to_string(),
            version: "1.0.0".to_string(),
            description: "File manager for RaeenOS".to_string(),
            author: "RaeenOS Team".to_string(),
            executable_path: "/usr/bin/raefiles".to_string(),
            icon_path: "/usr/share/icons/raefiles.svg".to_string(),
            permissions: vec!["fs.read".to_string(), "fs.write".to_string(), "fs.delete".to_string()],
            dependencies: Vec::new(),
            app_type: AppType::Utility,
            install_time: 0,
            last_run: 0,
            run_count: 0,
        };
        
        let terminal = AppInfo {
            name: "RaeTerminal".to_string(),
            version: "1.0.0".to_string(),
            description: "Terminal emulator for RaeenOS".to_string(),
            author: "RaeenOS Team".to_string(),
            executable_path: "/usr/bin/raeterminal".to_string(),
            icon_path: "/usr/share/icons/raeterminal.svg".to_string(),
            permissions: vec!["shell.access".to_string(), "process.spawn".to_string()],
            dependencies: Vec::new(),
            app_type: AppType::System,
            install_time: 0,
            last_run: 0,
            run_count: 0,
        };
        
        system.registered_apps.insert("raeedit".to_string(), text_editor);
        system.registered_apps.insert("raefiles".to_string(), file_manager);
        system.registered_apps.insert("raeterminal".to_string(), terminal);
        
        // Add framework templates
        system.framework_templates.insert("console".to_string(), 
            "Basic console application template".to_string());
        system.framework_templates.insert("gui".to_string(), 
            "Graphical user interface application template".to_string());
        system.framework_templates.insert("service".to_string(), 
            "Background service template".to_string());
        system.framework_templates.insert("library".to_string(), 
            "Shared library template".to_string());
        
        // Add build tools
        system.build_tools.insert("gcc".to_string(), "/usr/bin/gcc".to_string());
        system.build_tools.insert("rustc".to_string(), "/usr/bin/rustc".to_string());
        system.build_tools.insert("ld".to_string(), "/usr/bin/ld".to_string());
        system.build_tools.insert("make".to_string(), "/usr/bin/make".to_string());
        
        Mutex::new(system)
    };
}

// Initialize RaeKit framework
pub fn init_raekit() -> Result<(), ()> {
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.init").unwrap_or(false) {
        return Err(());
    }
    
    // Create framework directories
    let _ = crate::fs::create_directory("/usr/share/raekit");
    let _ = crate::fs::create_directory("/usr/share/raekit/templates");
    let _ = crate::fs::create_directory("/var/lib/raekit");
    
    Ok(())
}

// Register an application
pub fn register_application(app_info: AppInfo) -> Result<(), ()> {
    let mut raekit = RAEKIT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.register").unwrap_or(false) {
        return Err(());
    }
    
    raekit.registered_apps.insert(app_info.name.clone(), app_info);
    Ok(())
}

// Launch an application
pub fn launch_application(app_name: &str, args: &[&str]) -> Result<u32, ()> {
    let mut raekit = RAEKIT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.launch").unwrap_or(false) {
        return Err(());
    }
    
    // Find application
    let app_info = raekit.registered_apps.get(app_name)
        .ok_or(())?
        .clone();
    
    // Check application permissions
    for permission in &app_info.permissions {
        if !crate::security::request_permission(current_pid, permission).unwrap_or(false) {
            return Err(());
        }
    }
    
    // Launch process
    let process_id = crate::process::exec_process(&app_info.executable_path, args)
        .map_err(|_| ())?;
    
    // Create application context
    let app_id = raekit.next_app_id;
    raekit.next_app_id += 1;
    
    let context = AppContext {
        app_id,
        process_id,
        state: AppState::Starting,
        capabilities: Vec::new(),
        resource_limits: ResourceLimits::default(),
        start_time: crate::time::get_system_uptime(),
        memory_usage: 0,
        cpu_usage: 0.0,
    };
    
    raekit.running_apps.insert(app_id, context);
    
    // Update app statistics
    if let Some(app) = raekit.registered_apps.get_mut(app_name) {
        app.last_run = crate::time::get_system_uptime();
        app.run_count += 1;
    }
    
    Ok(app_id)
}

// Stop an application
pub fn stop_application(app_id: u32) -> Result<(), ()> {
    let mut raekit = RAEKIT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.stop").unwrap_or(false) {
        return Err(());
    }
    
    // Find running application
    let context = raekit.running_apps.get_mut(&app_id)
        .ok_or(())?;
    
    // Terminate process
    let _ = crate::process::terminate_process(context.process_id);
    
    // Update state
    context.state = AppState::Stopping;
    
    // Remove from running apps
    raekit.running_apps.remove(&app_id);
    
    Ok(())
}

// Get application information
pub fn get_application_info(app_name: &str) -> Result<Option<AppInfo>, ()> {
    let raekit = RAEKIT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.info").unwrap_or(false) {
        return Err(());
    }
    
    Ok(raekit.registered_apps.get(app_name).cloned())
}

// List all registered applications
pub fn list_applications() -> Result<Vec<AppInfo>, ()> {
    let raekit = RAEKIT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.list").unwrap_or(false) {
        return Err(());
    }
    
    Ok(raekit.registered_apps.values().cloned().collect())
}

// List running applications
pub fn list_running_applications() -> Result<Vec<(u32, String, AppState)>, ()> {
    let raekit = RAEKIT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.status").unwrap_or(false) {
        return Err(());
    }
    
    let mut running_apps = Vec::new();
    
    for (app_id, context) in &raekit.running_apps {
        // Find app name by process ID
        let mut app_name = format!("Process {}", context.process_id);
        for (name, app_info) in &raekit.registered_apps {
            if app_info.executable_path.contains(&format!("{}", context.process_id)) {
                app_name = name.clone();
                break;
            }
        }
        
        running_apps.push((*app_id, app_name, context.state.clone()));
    }
    
    Ok(running_apps)
}

// Create a new development project
pub fn create_project(name: &str, path: &str, template: &str) -> Result<(), ()> {
    let mut raekit = RAEKIT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.develop").unwrap_or(false) {
        return Err(());
    }
    
    // Check if template exists
    if !raekit.framework_templates.contains_key(template) {
        return Err(());
    }
    
    // Create project directory
    let _ = crate::fs::create_directory(path);
    
    // Create project structure based on template
    match template {
        "console" => {
            let _ = crate::fs::create_directory(&format!("{}/src", path));
            let _ = crate::fs::create_file(&format!("{}/src/main.rs", path));
            let _ = crate::fs::create_file(&format!("{}/Cargo.toml", path));
        }
        "gui" => {
            let _ = crate::fs::create_directory(&format!("{}/src", path));
            let _ = crate::fs::create_directory(&format!("{}/resources", path));
            let _ = crate::fs::create_file(&format!("{}/src/main.rs", path));
            let _ = crate::fs::create_file(&format!("{}/Cargo.toml", path));
        }
        "service" => {
            let _ = crate::fs::create_directory(&format!("{}/src", path));
            let _ = crate::fs::create_file(&format!("{}/src/service.rs", path));
            let _ = crate::fs::create_file(&format!("{}/service.toml", path));
        }
        "library" => {
            let _ = crate::fs::create_directory(&format!("{}/src", path));
            let _ = crate::fs::create_file(&format!("{}/src/lib.rs", path));
            let _ = crate::fs::create_file(&format!("{}/Cargo.toml", path));
        }
        _ => return Err(()),
    }
    
    // Create project metadata
    let project = DevProject {
        name: name.to_string(),
        path: path.to_string(),
        language: "rust".to_string(),
        framework: template.to_string(),
        target_type: match template {
            "console" => AppType::Utility,
            "gui" => AppType::User,
            "service" => AppType::Service,
            "library" => AppType::Library,
            _ => AppType::Utility,
        },
        created_time: crate::time::get_system_uptime(),
        last_modified: crate::time::get_system_uptime(),
        build_config: BuildConfig {
            optimization_level: OptimizationLevel::Basic,
            debug_symbols: true,
            target_arch: "x86_64".to_string(),
            output_format: OutputFormat::Executable,
            custom_flags: Vec::new(),
        },
    };
    
    raekit.dev_projects.insert(name.to_string(), project);
    Ok(())
}

// Build a project
pub fn build_project(project_name: &str) -> Result<String, ()> {
    let mut raekit = RAEKIT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.build").unwrap_or(false) {
        return Err(());
    }
    
    // Find project
    let project = raekit.dev_projects.get_mut(project_name)
        .ok_or(())?;
    
    // Simulate build process
    let output_path = format!("{}/target/release/{}", project.path, project_name);
    
    // Update last modified time
    project.last_modified = crate::time::get_system_uptime();
    
    Ok(output_path)
}

// Get project information
pub fn get_project_info(project_name: &str) -> Result<Option<DevProject>, ()> {
    let raekit = RAEKIT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.project.info").unwrap_or(false) {
        return Err(());
    }
    
    Ok(raekit.dev_projects.get(project_name).cloned())
}

// List development projects
pub fn list_projects() -> Result<Vec<DevProject>, ()> {
    let raekit = RAEKIT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.project.list").unwrap_or(false) {
        return Err(());
    }
    
    Ok(raekit.dev_projects.values().cloned().collect())
}

// Get available templates
pub fn get_available_templates() -> Result<Vec<(String, String)>, ()> {
    let raekit = RAEKIT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.templates").unwrap_or(false) {
        return Err(());
    }
    
    let templates = raekit.framework_templates
        .iter()
        .map(|(name, desc)| (name.clone(), desc.clone()))
        .collect();
    
    Ok(templates)
}

// Get build tools
pub fn get_build_tools() -> Result<Vec<(String, String)>, ()> {
    let raekit = RAEKIT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.tools").unwrap_or(false) {
        return Err(());
    }
    
    let tools = raekit.build_tools
        .iter()
        .map(|(name, path)| (name.clone(), path.clone()))
        .collect();
    
    Ok(tools)
}

// Set resource limits for an application
pub fn set_resource_limits(app_id: u32, limits: ResourceLimits) -> Result<(), ()> {
    let mut raekit = RAEKIT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.limits").unwrap_or(false) {
        return Err(());
    }
    
    if let Some(context) = raekit.running_apps.get_mut(&app_id) {
        context.resource_limits = limits;
        Ok(())
    } else {
        Err(())
    }
}

// Get application resource usage
pub fn get_resource_usage(app_id: u32) -> Result<(u64, f32), ()> {
    let raekit = RAEKIT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.monitor").unwrap_or(false) {
        return Err(());
    }
    
    if let Some(context) = raekit.running_apps.get(&app_id) {
        Ok((context.memory_usage, context.cpu_usage))
    } else {
        Err(())
    }
}

// Enable framework capability for an application
pub fn enable_capability(app_id: u32, capability: FrameworkCapability) -> Result<(), ()> {
    let mut raekit = RAEKIT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raekit.capability").unwrap_or(false) {
        return Err(());
    }
    
    if let Some(context) = raekit.running_apps.get_mut(&app_id) {
        if !context.capabilities.contains(&capability) {
            context.capabilities.push(capability);
        }
        Ok(())
    } else {
        Err(())
    }
}

// Clean up RaeKit resources for a process
pub fn cleanup_process_raekit(process_id: u32) {
    let mut raekit = RAEKIT_SYSTEM.lock();
    
    // Remove running applications for this process
    let apps_to_remove: Vec<u32> = raekit.running_apps
        .iter()
        .filter(|(_, context)| context.process_id == process_id)
        .map(|(&app_id, _)| app_id)
        .collect();
    
    for app_id in apps_to_remove {
        raekit.running_apps.remove(&app_id);
    }
}