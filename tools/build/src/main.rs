use clap::{Arg, Command, ArgMatches};
use log::{info, warn, error, debug};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};
use walkdir::WalkDir;
use toml_edit::Document;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BuildConfig {
    kernel_target: String,
    userspace_target: String,
    bootloader: String,
    iso_name: String,
    vmdk_name: String,
    qemu_args: Vec<String>,
    packages: PackageConfig,
    security: SecurityConfig,
    development: DevelopmentConfig,
    compatibility: CompatibilityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageConfig {
    repository_url: String,
    mirrors: Vec<String>,
    signing_key: String,
    compression: String,
    format_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecurityConfig {
    sandbox_default: String,
    code_signing_required: bool,
    verified_boot: bool,
    secure_boot: bool,
    tpm_required: bool,
    encryption_default: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DevelopmentConfig {
    test_runner: String,
    benchmark_runner: String,
    documentation_tool: String,
    package_tool: String,
    debugging_symbols: bool,
    profiling_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompatibilityConfig {
    wine_version: String,
    android_api_level: u32,
    web_engine: String,
    flatpak_runtime: String,
    snap_base: String,
    appimage_runtime: String,
}

#[derive(Debug, Clone)]
struct BuildContext {
    config: BuildConfig,
    workspace_root: PathBuf,
    build_dir: PathBuf,
    output_dir: PathBuf,
    target_dir: PathBuf,
    verbose: bool,
    parallel_jobs: usize,
}

#[derive(Debug, Clone)]
struct BuildTarget {
    name: String,
    path: PathBuf,
    target_type: TargetType,
    dependencies: Vec<String>,
    features: Vec<String>,
    profile: String,
}

#[derive(Debug, Clone, PartialEq)]
enum TargetType {
    Kernel,
    Userspace,
    Bootloader,
    Tool,
    Library,
    Application,
    Compatibility,
}

#[derive(Debug, Clone)]
struct BuildResult {
    target: String,
    success: bool,
    duration: std::time::Duration,
    output_files: Vec<PathBuf>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

fn main() {
    env_logger::init();
    
    let matches = Command::new("raeen-build")
        .version("0.1.0")
        .author("RaeenOS Team")
        .about("Build system for RaeenOS")
        .arg(Arg::new("command")
            .help("Build command to execute")
            .value_parser(["all", "kernel", "userspace", "bootloader", "iso", "vmdk", "test", "bench", "docs", "clean", "check"])
            .required(true)
            .index(1))
        .arg(Arg::new("target")
            .help("Specific target to build")
            .short('t')
            .long("target")
            .value_name("TARGET"))
        .arg(Arg::new("profile")
            .help("Build profile to use")
            .short('p')
            .long("profile")
            .value_name("PROFILE")
            .default_value("release"))
        .arg(Arg::new("features")
            .help("Features to enable")
            .short('f')
            .long("features")
            .value_name("FEATURES")
            .action(clap::ArgAction::Append))
        .arg(Arg::new("jobs")
            .help("Number of parallel jobs")
            .short('j')
            .long("jobs")
            .value_name("N")
            .value_parser(clap::value_parser!(usize))
            .default_value("4"))
        .arg(Arg::new("verbose")
            .help("Enable verbose output")
            .short('v')
            .long("verbose")
            .action(clap::ArgAction::SetTrue))
        .arg(Arg::new("output")
            .help("Output directory")
            .short('o')
            .long("output")
            .value_name("DIR")
            .default_value("build"))
        .arg(Arg::new("workspace")
            .help("Workspace root directory")
            .short('w')
            .long("workspace")
            .value_name("DIR")
            .default_value("."))
        .get_matches();
    
    let result = match run_build(&matches) {
        Ok(results) => {
            info!("Build completed successfully!");
            print_build_summary(&results);
            0
        }
        Err(e) => {
            error!("Build failed: {}", e);
            1
        }
    };
    
    std::process::exit(result);
}

fn run_build(matches: &ArgMatches) -> Result<Vec<BuildResult>, Box<dyn std::error::Error>> {
    let workspace_root = PathBuf::from(matches.get_one::<String>("workspace").unwrap());
    let config = load_build_config(&workspace_root)?;
    
    let context = BuildContext {
        config,
        workspace_root: workspace_root.clone(),
        build_dir: workspace_root.join("build"),
        output_dir: PathBuf::from(matches.get_one::<String>("output").unwrap()),
        target_dir: workspace_root.join("target"),
        verbose: matches.get_flag("verbose"),
        parallel_jobs: *matches.get_one::<usize>("jobs").unwrap(),
    };
    
    // Create build directories
    fs::create_dir_all(&context.build_dir)?;
    fs::create_dir_all(&context.output_dir)?;
    fs::create_dir_all(&context.target_dir)?;
    
    let command = matches.get_one::<String>("command").unwrap();
    let profile = matches.get_one::<String>("profile").unwrap();
    let features: Vec<String> = matches.get_many::<String>("features")
        .unwrap_or_default()
        .cloned()
        .collect();
    
    match command.as_str() {
        "all" => build_all(&context, profile, &features),
        "kernel" => build_kernel(&context, profile, &features),
        "userspace" => build_userspace(&context, profile, &features),
        "bootloader" => build_bootloader(&context, profile, &features),
        "iso" => build_iso(&context),
        "vmdk" => build_vmdk(&context),
        "test" => run_tests(&context),
        "bench" => run_benchmarks(&context),
        "docs" => build_documentation(&context),
        "clean" => clean_build(&context),
        "check" => check_code(&context),
        _ => Err(format!("Unknown command: {}", command).into()),
    }
}

fn load_build_config(workspace_root: &Path) -> Result<BuildConfig, Box<dyn std::error::Error>> {
    let cargo_toml_path = workspace_root.join("Cargo.toml");
    let content = fs::read_to_string(cargo_toml_path)?;
    let doc: Document = content.parse()?;
    
    let metadata = doc["workspace"]["metadata"]["raeen"].as_table()
        .ok_or("Missing raeen metadata in Cargo.toml")?;
    
    let config = BuildConfig {
        kernel_target: metadata["kernel_target"].as_str().unwrap_or("x86_64-raeen").to_string(),
        userspace_target: metadata["userspace_target"].as_str().unwrap_or("x86_64-unknown-linux-gnu").to_string(),
        bootloader: metadata["bootloader"].as_str().unwrap_or("raeen-bootloader").to_string(),
        iso_name: metadata["iso_name"].as_str().unwrap_or("raeen-os.iso").to_string(),
        vmdk_name: metadata["vmdk_name"].as_str().unwrap_or("raeen-os.vmdk").to_string(),
        qemu_args: metadata["qemu_args"].as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect())
            .unwrap_or_default(),
        packages: PackageConfig {
            repository_url: "https://packages.raeen.dev".to_string(),
            mirrors: vec!["https://mirror1.raeen.dev".to_string()],
            signing_key: "raeen-packages.pub".to_string(),
            compression: "zstd".to_string(),
            format_version: "1.0".to_string(),
        },
        security: SecurityConfig {
            sandbox_default: "strict".to_string(),
            code_signing_required: true,
            verified_boot: true,
            secure_boot: true,
            tpm_required: false,
            encryption_default: "aes256".to_string(),
        },
        development: DevelopmentConfig {
            test_runner: "raeen-test".to_string(),
            benchmark_runner: "criterion".to_string(),
            documentation_tool: "rustdoc".to_string(),
            package_tool: "raeen-pkg".to_string(),
            debugging_symbols: true,
            profiling_enabled: true,
        },
        compatibility: CompatibilityConfig {
            wine_version: "8.0".to_string(),
            android_api_level: 33,
            web_engine: "webkit".to_string(),
            flatpak_runtime: "org.freedesktop.Platform".to_string(),
            snap_base: "core22".to_string(),
            appimage_runtime: "runtime-x86_64".to_string(),
        },
    };
    
    Ok(config)
}

fn build_all(context: &BuildContext, profile: &str, features: &[String]) -> Result<Vec<BuildResult>, Box<dyn std::error::Error>> {
    info!("Building all components...");
    
    let mut results = Vec::new();
    
    // Build in dependency order
    results.extend(build_kernel(context, profile, features)?);
    results.extend(build_userspace(context, profile, features)?);
    results.extend(build_bootloader(context, profile, features)?);
    results.extend(build_compatibility_layers(context, profile, features)?);
    
    Ok(results)
}

fn build_kernel(context: &BuildContext, profile: &str, features: &[String]) -> Result<Vec<BuildResult>, Box<dyn std::error::Error>> {
    info!("Building kernel...");
    
    let start_time = std::time::Instant::now();
    let kernel_dir = context.workspace_root.join("kernel");
    
    let mut cmd = ProcessCommand::new("cargo");
    cmd.current_dir(&kernel_dir)
        .arg("build")
        .arg("--profile")
        .arg(profile)
        .arg("--target")
        .arg(&context.config.kernel_target);
    
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }
    
    if context.verbose {
        cmd.arg("--verbose");
    }
    
    let output = cmd.output()?;
    let duration = start_time.elapsed();
    
    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    if context.verbose {
        println!("Kernel build stdout: {}", stdout);
        println!("Kernel build stderr: {}", stderr);
    }
    
    let result = BuildResult {
        target: "kernel".to_string(),
        success,
        duration,
        output_files: if success {
            vec![context.target_dir.join(&context.config.kernel_target).join(profile).join("raeen_kernel")]
        } else {
            Vec::new()
        },
        errors: if success { Vec::new() } else { vec![stderr.to_string()] },
        warnings: Vec::new(),
    };
    
    if success {
        info!("Kernel build completed in {:?}", duration);
    } else {
        error!("Kernel build failed: {}", stderr);
    }
    
    Ok(vec![result])
}

fn build_userspace(context: &BuildContext, profile: &str, features: &[String]) -> Result<Vec<BuildResult>, Box<dyn std::error::Error>> {
    info!("Building userspace components...");
    
    let mut results = Vec::new();
    let userspace_dirs = [
        "userspace/init",
        "userspace/shell",
        "userspace/desktop",
    ];
    
    for dir in &userspace_dirs {
        let component_dir = context.workspace_root.join(dir);
        if component_dir.exists() {
            let start_time = std::time::Instant::now();
            
            let mut cmd = ProcessCommand::new("cargo");
            cmd.current_dir(&component_dir)
                .arg("build")
                .arg("--profile")
                .arg(profile)
                .arg("--target")
                .arg(&context.config.userspace_target);
            
            if !features.is_empty() {
                cmd.arg("--features").arg(features.join(","));
            }
            
            if context.verbose {
                cmd.arg("--verbose");
            }
            
            let output = cmd.output()?;
            let duration = start_time.elapsed();
            
            let success = output.status.success();
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            let result = BuildResult {
                target: dir.to_string(),
                success,
                duration,
                output_files: Vec::new(), // TODO: Determine actual output files
                errors: if success { Vec::new() } else { vec![stderr.to_string()] },
                warnings: Vec::new(),
            };
            
            if success {
                info!("{} build completed in {:?}", dir, duration);
            } else {
                error!("{} build failed: {}", dir, stderr);
            }
            
            results.push(result);
        }
    }
    
    Ok(results)
}

fn build_bootloader(context: &BuildContext, profile: &str, features: &[String]) -> Result<Vec<BuildResult>, Box<dyn std::error::Error>> {
    info!("Building bootloader...");
    
    let start_time = std::time::Instant::now();
    let bootloader_dir = context.workspace_root.join("bootloader");
    
    if !bootloader_dir.exists() {
        warn!("Bootloader directory not found, skipping...");
        return Ok(Vec::new());
    }
    
    let mut cmd = ProcessCommand::new("cargo");
    cmd.current_dir(&bootloader_dir)
        .arg("build")
        .arg("--profile")
        .arg(profile);
    
    if !features.is_empty() {
        cmd.arg("--features").arg(features.join(","));
    }
    
    if context.verbose {
        cmd.arg("--verbose");
    }
    
    let output = cmd.output()?;
    let duration = start_time.elapsed();
    
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    let result = BuildResult {
        target: "bootloader".to_string(),
        success,
        duration,
        output_files: Vec::new(),
        errors: if success { Vec::new() } else { vec![stderr.to_string()] },
        warnings: Vec::new(),
    };
    
    if success {
        info!("Bootloader build completed in {:?}", duration);
    } else {
        error!("Bootloader build failed: {}", stderr);
    }
    
    Ok(vec![result])
}

fn build_compatibility_layers(context: &BuildContext, profile: &str, features: &[String]) -> Result<Vec<BuildResult>, Box<dyn std::error::Error>> {
    info!("Building compatibility layers...");
    
    let mut results = Vec::new();
    let compat_dirs = [
        "compatibility/wine",
        "compatibility/android",
        "compatibility/web",
    ];
    
    for dir in &compat_dirs {
        let compat_dir = context.workspace_root.join(dir);
        if compat_dir.exists() {
            let start_time = std::time::Instant::now();
            
            let mut cmd = ProcessCommand::new("cargo");
            cmd.current_dir(&compat_dir)
                .arg("build")
                .arg("--profile")
                .arg("compatibility"); // Use compatibility profile
            
            if !features.is_empty() {
                cmd.arg("--features").arg(features.join(","));
            }
            
            if context.verbose {
                cmd.arg("--verbose");
            }
            
            let output = cmd.output()?;
            let duration = start_time.elapsed();
            
            let success = output.status.success();
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            let result = BuildResult {
                target: dir.to_string(),
                success,
                duration,
                output_files: Vec::new(),
                errors: if success { Vec::new() } else { vec![stderr.to_string()] },
                warnings: Vec::new(),
            };
            
            if success {
                info!("{} build completed in {:?}", dir, duration);
            } else {
                error!("{} build failed: {}", dir, stderr);
            }
            
            results.push(result);
        }
    }
    
    Ok(results)
}

fn build_iso(context: &BuildContext) -> Result<Vec<BuildResult>, Box<dyn std::error::Error>> {
    info!("Building ISO image...");
    
    let start_time = std::time::Instant::now();
    
    // TODO: Implement ISO creation logic
    // This would involve:
    // 1. Creating a temporary directory structure
    // 2. Copying kernel, bootloader, and userspace binaries
    // 3. Creating boot configuration
    // 4. Using tools like genisoimage or xorriso to create the ISO
    
    let duration = start_time.elapsed();
    
    let result = BuildResult {
        target: "iso".to_string(),
        success: true, // TODO: Implement actual ISO creation
        duration,
        output_files: vec![context.output_dir.join(&context.config.iso_name)],
        errors: Vec::new(),
        warnings: vec!["ISO creation not yet implemented".to_string()],
    };
    
    info!("ISO build completed in {:?}", duration);
    
    Ok(vec![result])
}

fn build_vmdk(context: &BuildContext) -> Result<Vec<BuildResult>, Box<dyn std::error::Error>> {
    info!("Building VMDK image...");
    
    let start_time = std::time::Instant::now();
    
    // TODO: Implement VMDK creation logic
    
    let duration = start_time.elapsed();
    
    let result = BuildResult {
        target: "vmdk".to_string(),
        success: true, // TODO: Implement actual VMDK creation
        duration,
        output_files: vec![context.output_dir.join(&context.config.vmdk_name)],
        errors: Vec::new(),
        warnings: vec!["VMDK creation not yet implemented".to_string()],
    };
    
    info!("VMDK build completed in {:?}", duration);
    
    Ok(vec![result])
}

fn run_tests(context: &BuildContext) -> Result<Vec<BuildResult>, Box<dyn std::error::Error>> {
    info!("Running tests...");
    
    let start_time = std::time::Instant::now();
    
    let mut cmd = ProcessCommand::new("cargo");
    cmd.current_dir(&context.workspace_root)
        .arg("test")
        .arg("--workspace");
    
    if context.verbose {
        cmd.arg("--verbose");
    }
    
    let output = cmd.output()?;
    let duration = start_time.elapsed();
    
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    let result = BuildResult {
        target: "tests".to_string(),
        success,
        duration,
        output_files: Vec::new(),
        errors: if success { Vec::new() } else { vec![stderr.to_string()] },
        warnings: Vec::new(),
    };
    
    if success {
        info!("Tests completed in {:?}", duration);
    } else {
        error!("Tests failed: {}", stderr);
    }
    
    Ok(vec![result])
}

fn run_benchmarks(context: &BuildContext) -> Result<Vec<BuildResult>, Box<dyn std::error::Error>> {
    info!("Running benchmarks...");
    
    let start_time = std::time::Instant::now();
    
    let mut cmd = ProcessCommand::new("cargo");
    cmd.current_dir(&context.workspace_root)
        .arg("bench")
        .arg("--workspace");
    
    if context.verbose {
        cmd.arg("--verbose");
    }
    
    let output = cmd.output()?;
    let duration = start_time.elapsed();
    
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    let result = BuildResult {
        target: "benchmarks".to_string(),
        success,
        duration,
        output_files: Vec::new(),
        errors: if success { Vec::new() } else { vec![stderr.to_string()] },
        warnings: Vec::new(),
    };
    
    if success {
        info!("Benchmarks completed in {:?}", duration);
    } else {
        error!("Benchmarks failed: {}", stderr);
    }
    
    Ok(vec![result])
}

fn build_documentation(context: &BuildContext) -> Result<Vec<BuildResult>, Box<dyn std::error::Error>> {
    info!("Building documentation...");
    
    let start_time = std::time::Instant::now();
    
    let mut cmd = ProcessCommand::new("cargo");
    cmd.current_dir(&context.workspace_root)
        .arg("doc")
        .arg("--workspace")
        .arg("--no-deps");
    
    if context.verbose {
        cmd.arg("--verbose");
    }
    
    let output = cmd.output()?;
    let duration = start_time.elapsed();
    
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    let result = BuildResult {
        target: "documentation".to_string(),
        success,
        duration,
        output_files: vec![context.target_dir.join("doc")],
        errors: if success { Vec::new() } else { vec![stderr.to_string()] },
        warnings: Vec::new(),
    };
    
    if success {
        info!("Documentation build completed in {:?}", duration);
    } else {
        error!("Documentation build failed: {}", stderr);
    }
    
    Ok(vec![result])
}

fn clean_build(context: &BuildContext) -> Result<Vec<BuildResult>, Box<dyn std::error::Error>> {
    info!("Cleaning build artifacts...");
    
    let start_time = std::time::Instant::now();
    
    // Clean cargo target directory
    let mut cmd = ProcessCommand::new("cargo");
    cmd.current_dir(&context.workspace_root)
        .arg("clean");
    
    let output = cmd.output()?;
    
    // Clean custom build directories
    if context.build_dir.exists() {
        fs::remove_dir_all(&context.build_dir)?;
    }
    
    if context.output_dir.exists() {
        fs::remove_dir_all(&context.output_dir)?;
    }
    
    let duration = start_time.elapsed();
    let success = output.status.success();
    
    let result = BuildResult {
        target: "clean".to_string(),
        success,
        duration,
        output_files: Vec::new(),
        errors: Vec::new(),
        warnings: Vec::new(),
    };
    
    info!("Clean completed in {:?}", duration);
    
    Ok(vec![result])
}

fn check_code(context: &BuildContext) -> Result<Vec<BuildResult>, Box<dyn std::error::Error>> {
    info!("Checking code...");
    
    let start_time = std::time::Instant::now();
    
    let mut cmd = ProcessCommand::new("cargo");
    cmd.current_dir(&context.workspace_root)
        .arg("check")
        .arg("--workspace");
    
    if context.verbose {
        cmd.arg("--verbose");
    }
    
    let output = cmd.output()?;
    let duration = start_time.elapsed();
    
    let success = output.status.success();
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    let result = BuildResult {
        target: "check".to_string(),
        success,
        duration,
        output_files: Vec::new(),
        errors: if success { Vec::new() } else { vec![stderr.to_string()] },
        warnings: Vec::new(),
    };
    
    if success {
        info!("Code check completed in {:?}", duration);
    } else {
        error!("Code check failed: {}", stderr);
    }
    
    Ok(vec![result])
}

fn print_build_summary(results: &[BuildResult]) {
    println!("\n=== Build Summary ===");
    
    let total_duration: std::time::Duration = results.iter().map(|r| r.duration).sum();
    let successful = results.iter().filter(|r| r.success).count();
    let failed = results.len() - successful;
    
    println!("Total targets: {}", results.len());
    println!("Successful: {}", successful);
    println!("Failed: {}", failed);
    println!("Total time: {:?}", total_duration);
    
    if failed > 0 {
        println!("\nFailed targets:");
        for result in results.iter().filter(|r| !r.success) {
            println!("  - {} ({:?})", result.target, result.duration);
            for error in &result.errors {
                println!("    Error: {}", error);
            }
        }
    }
    
    if results.iter().any(|r| !r.warnings.is_empty()) {
        println!("\nWarnings:");
        for result in results {
            for warning in &result.warnings {
                println!("  - {}: {}", result.target, warning);
            }
        }
    }
    
    println!("\nOutput files:");
    for result in results {
        for file in &result.output_files {
            println!("  - {}", file.display());
        }
    }
}