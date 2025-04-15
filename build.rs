use std::{env, io};
use embed_resource::compile;
use winresource::WindowsResource;

fn main() -> io::Result<()> {
    compile("miniover-manifest.rc", embed_resource::NONE).manifest_required().unwrap();

    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        WindowsResource::new()
            // This path can be absolute, or relative to your crate root.
            .set_icon("icon.ico")
            .compile()?;
    }
    Ok(())
}