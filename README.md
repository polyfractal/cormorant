## Piccolo

Piccolo is a toy distributed key:value datastore written in Rust.  It is not
intended for any practical use -- it is just a learning project and teaching
tool.

Piccolo is also a small flute. :)

### Building

To test Piccolo out, execute this command in two terminals (the nodes will
find and cluster with each other):  

```
$ RUST_LOG=piccolo=DEBUG cargo run --release
```

If you would like to add more nodes, or change ports, edit the `cargo.toml` file
