fn main() {
    // Only embed Windows resources on Windows
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=miniover-manifest.rc");
        use embed_resource::compile;
        compile("miniover-manifest.rc", embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }
}
