## Cormorant

Cormorant is a toy distributed key:value datastore written in Rust.  It is not
intended for any practical use -- it is just a learning project and teaching
tool.


Cormorant are coastal seabirds that are excellent swimmers, but rather poor
at flying.  They nest in giant colonies and look rather prehistoric.

### Building

To test Cormorant out, execute this command in two terminals (the nodes will
find and cluster with each other):  

```
$ RUST_LOG=piccolo=DEBUG cargo run --release
```

If you would like to add more nodes, or change ports, edit the `cargo.toml` file
