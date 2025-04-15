use std::io;
use embed_resource::compile;

fn main() -> io::Result<()> {
    compile("miniover-manifest.rc", embed_resource::NONE).manifest_required().unwrap();

    Ok(())
}