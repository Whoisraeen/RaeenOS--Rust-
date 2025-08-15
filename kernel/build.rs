fn main() {
    // Build script for kernel - using inline assembly for context switching
    println!("cargo:rerun-if-changed=src/");
}