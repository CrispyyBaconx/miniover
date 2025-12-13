fn main() {
    // Check target OS at runtime to support cross-compilation
    // Using CARGO_CFG_TARGET_OS instead of #[cfg(windows)] which checks the host
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    
    if target_os == "windows" {
        println!("cargo:rerun-if-changed=miniover-manifest.rc");
        embed_resource::compile("miniover-manifest.rc", embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }
}
