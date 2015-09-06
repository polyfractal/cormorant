extern crate capnpc;

use std::fs;

fn main() {

    let mut files = Vec::new();
    let src = "src/schema";

    // Kudos to BurntSushi for this snippet:
    // https://users.rust-lang.org/t/comparing-file-extensions-without-type-conversion-hell/1330/3
    fn is_capnp(e: &fs::DirEntry) -> bool {
        let p = e.path();
        p.extension().map(|s| s == "capnp").unwrap_or(false)
    }
    for entry in fs::read_dir(src).unwrap().filter_map(|e| e.ok()).filter(is_capnp) {
        let p = entry.path().to_str().unwrap().to_owned();
        files.push(p);
    }

    ::capnpc::compile(".", &files).unwrap();
}
