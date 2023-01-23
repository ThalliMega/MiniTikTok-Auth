fn main() -> std::io::Result<()> {
    let builder = tonic_build::configure();
    // if cfg!(not(test)) {
    //     builder = builder.build_client(false);
    // }
    builder.compile(&["proto/auth.proto"], &["proto"])
}
