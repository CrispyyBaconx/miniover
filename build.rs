fn main() {
    // Only embed Windows resources on Windows
    #[cfg(windows)]
    {
        use embed_resource::compile;
        compile("miniover-manifest.rc", embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }
}
