FROM rust:1.83-slim AS builder

WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY migrations/ migrations/
COPY plugin/ plugin/
RUN cargo build --release --features cloud

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/cortexmem /usr/local/bin/
EXPOSE 8080
ENTRYPOINT ["cortexmem", "cloud", "serve"]
