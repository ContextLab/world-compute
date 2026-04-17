# Stage 1: Build
FROM rust:1.95-bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release --bin worldcompute

# Stage 2: Runtime
FROM gcr.io/distroless/cc-debian12
COPY --from=builder /build/target/release/worldcompute /usr/local/bin/worldcompute
ENTRYPOINT ["worldcompute"]
