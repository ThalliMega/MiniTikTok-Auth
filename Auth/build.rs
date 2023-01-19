fn main() -> std::io::Result<()> {
    tonic_build::configure()
        .build_client(false)
        .compile(&["proto/auth.proto"], &["proto"])
}
