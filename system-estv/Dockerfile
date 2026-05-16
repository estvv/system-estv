FROM rust:1.82-alpine AS builder

RUN apk add --no-cache musl-dev

ARG GIT_HASH=unknown
ENV GIT_HASH=$GIT_HASH

WORKDIR /app

COPY Cargo.toml ./
COPY src ./src
COPY static ./static

RUN RUSTFLAGS='-C target-feature=+crt-static' \
    cargo build --release --target x86_64-unknown-linux-musl

FROM scratch

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/rust-exporter /rust-exporter

USER 65534:65534

EXPOSE 8080

ENTRYPOINT ["/rust-exporter"]