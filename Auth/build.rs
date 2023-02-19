fn main() -> std::io::Result<()> {
    let builder = tonic_build::configure().build_client(false);
    builder.compile(&["proto/auth.proto"], &["proto"])
}
