fn main() -> std::io::Result<()> {
    let builder = tonic_build::configure();
    builder.compile(
        &["proto/auth.proto", "proto/health_check.proto"],
        &["proto"],
    )
}
