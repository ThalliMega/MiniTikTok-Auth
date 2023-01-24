FROM rust:alpine AS build
RUN mkdir -p Auth/src && touch Auth/src/main.rs
COPY Cargo.toml ./
COPY Auth/Cargo.toml Auth/
RUN cargo vendor
COPY ./ ./
RUN cargo build --frozen --release --bins

FROM alpine
COPY --from=build target/release/mini_tiktok_auth ./auth
ENTRYPOINT [ "./auth" ]

EXPOSE 14514
