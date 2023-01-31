FROM rust:alpine3.17 AS build
WORKDIR /src
ARG REPLACE_ALPINE=""
RUN mkdir -p Auth/src \
    && touch Auth/src/main.rs \
    && printenv REPLACE_ALPINE > reposcript \
    && sed -i -f reposcript /etc/apk/repositories
RUN apk add --no-cache -U musl-dev protoc protobuf-dev
COPY .cargo/ .cargo/
COPY Cargo.toml ./
COPY Auth/Cargo.toml Auth/
RUN cargo vendor --respect-source-config
COPY ./ ./
RUN cargo build --release --frozen --bins

FROM alpine:3.17
WORKDIR /app
COPY --from=build /src/target/release/mini_tiktok_auth ./auth
ENTRYPOINT [ "./auth" ]

EXPOSE 14514
