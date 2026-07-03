fn main() {
    // `cargo install --path src-tauri` does not run the Tauri CLI's frontend
    // pipeline. Re-run this build script whenever the already-built frontend
    // bundle changes so the embedded assets do not go stale.
    println!("cargo:rerun-if-changed=../ui/dist/index.html");
    println!("cargo:rerun-if-changed=../ui/dist/assets");
    tauri_build::build()
}
