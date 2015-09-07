extern crate capnpc;

use std::io::prelude::*;
use std::fs::{File, OpenOptions};
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

    // Hotpatch for https://github.com/dwrensha/capnpc-rust/issues/5
    // Replace all instances of `::<module_name>` with `super` so we can use
    // capnp in (sub-)modules
    let out_dir = env!("OUT_DIR");
    println!("out dir: {}", out_dir);

    fn is_rs(e: &fs::DirEntry) -> bool {
        let p = e.path();
        p.extension().map(|s| s == "rs").unwrap_or(false)
    }
    for entry in fs::read_dir(out_dir).unwrap().filter_map(|e| e.ok()).filter(is_rs) {
        let path = entry.path();
        let ext = path.extension().unwrap().to_str().unwrap();
        let name = path.file_name().unwrap().to_str().unwrap().replace(ext, "").replace(".", "");
        println!("name: {}", name);

        if name.find("_capnp").is_some() {
            let mut f = OpenOptions::new()
                        .read(true)
                        .write(false)
                        .create(false)
                        .open(entry.path()).unwrap();
            let mut s = String::new();
            f.read_to_string(&mut s).unwrap();
            let module = "::".to_owned() + &name;
            println!("module name: {}", module);

            let s = s.replace(&module, "super");

            let mut f = OpenOptions::new()
                        .read(false)
                        .write(true)
                        .truncate(true)
                        .create(false)
                        .open(entry.path()).unwrap();
            f.write_all(s.as_bytes()).unwrap();
        }
    }



}
