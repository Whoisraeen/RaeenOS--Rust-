use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let src_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    
    // Compile the assembly file
    let asm_file = PathBuf::from(&src_dir).join("src").join("context_switch.s");
    let obj_file = PathBuf::from(&out_dir).join("context_switch.o");
    
    // Use the system assembler to compile the assembly file
    let output = Command::new("as")
        .args(&["--64", "-o"])
        .arg(&obj_file)
        .arg(&asm_file)
        .output();
    
    match output {
        Ok(output) => {
            if !output.status.success() {
                panic!("Failed to assemble {}: {}", asm_file.display(), String::from_utf8_lossy(&output.stderr));
            }
        }
        Err(e) => {
            // If 'as' is not available, try 'nasm' as an alternative
            let output = Command::new("nasm")
                .args(&["-f", "elf64", "-o"])
                .arg(&obj_file)
                .arg(&asm_file)
                .output();
            
            match output {
                Ok(output) => {
                    if !output.status.success() {
                        panic!("Failed to assemble with nasm {}: {}", asm_file.display(), String::from_utf8_lossy(&output.stderr));
                    }
                }
                Err(_) => {
                    // If neither assembler is available, we'll use inline assembly instead
                    println!("cargo:warning=No assembler found, using inline assembly fallback");
                    return;
                }
            }
        }
    }
    
    // Link the object file
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-arg={}", obj_file.display());
    
    // Tell cargo to rerun if the assembly file changes
    println!("cargo:rerun-if-changed={}", asm_file.display());
}