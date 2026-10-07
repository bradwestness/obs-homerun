# Stage 1: Build the Rust binary
FROM rust:alpine AS builder

WORKDIR /src
COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/

RUN mkdir -p /app && \
    cargo build --release && \
    cp target/release/obs-homerun /app/obs-homerun

# Stage 2: Runtime image
FROM alpine:3.21

LABEL maintainer="Brad Westness"
LABEL org.opencontainers.image.title="OBS HomeRun"
LABEL org.opencontainers.image.description="Turn your OBS Studio stream into a virtual HDTV tuner for Smart TVs"
LABEL org.opencontainers.image.source="https://github.com/bradwestness/obs-homerun"

# Install runtime dependencies (FFmpeg and curl for HEALTHCHECK)
RUN apk add --no-cache ffmpeg curl

WORKDIR /app

# Copy latest MediaMTX binary from official image
COPY --from=docker.io/bluenviron/mediamtx:latest /mediamtx /app/mediamtx

# Copy compiled Rust binary and configuration
COPY --from=builder /app/obs-homerun /app/obs-homerun
COPY mediamtx.yml /app/mediamtx.yml

RUN chmod +x /app/obs-homerun /app/mediamtx

# 1935: RTMP (OBS stream ingest)
# 5004: HTTP (virtual HDTV / DLNA stream & metadata)
# 1900: UDP (SSDP UPnP multicast discovery)
EXPOSE 1935/tcp 5004/tcp 1900/udp

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD curl -f http://127.0.0.1:5004/discover.json || exit 1

ENTRYPOINT ["/app/obs-homerun"]
