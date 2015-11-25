## Cormorant

Cormorant is a toy distributed key:value datastore written in Rust.  It is not
intended for any practical use -- it is just a learning project and teaching
tool.

I am writing up my experience/thoughts as Cormorant is written.  [You can follow
along at my blog.](https://polyfractal.com/categories/cormorant/)


Cormorant are coastal seabirds that are excellent swimmers, but quite poor
at flying.  They nest in giant colonies and look rather prehistoric.

### Building

To test Cormorant out, execute this command in two terminals (the nodes will
find and cluster with each other):  

```
$ RUST_LOG=cormorant=DEBUG cargo run --release
```

If you would like to add more nodes, or change ports, edit the `cargo.toml` file
