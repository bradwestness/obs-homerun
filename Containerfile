# Stage 1: Build the Go binary
FROM --platform=$BUILDPLATFORM golang:1.23-alpine AS builder

WORKDIR /src
COPY go.mod ./
COPY main.go ./

ARG TARGETOS TARGETARCH
RUN CGO_ENABLED=0 GOOS=${TARGETOS:-linux} GOARCH=${TARGETARCH:-amd64} \
    go build -ldflags="-s -w" -o /app/obs-homerun .

# Stage 2: Runtime image
FROM alpine:3.21

LABEL maintainer="Brad Westness"
LABEL org.opencontainers.image.title="OBS HomeRun"
LABEL org.opencontainers.image.description="Turn your OBS stream into a virtual HDTV tuner for Smart TVs"
LABEL org.opencontainers.image.source="https://github.com/bradwestness/obs-homerun"

# Install runtime dependencies (FFmpeg only - NO Python needed!)
RUN apk add --no-cache \
    ffmpeg \
    curl \
    ca-certificates \
    tar

WORKDIR /app

# Download MediaMTX binary according to target architecture (amd64 / arm64)
ARG TARGETARCH
ARG MEDIAMTX_VERSION=v1.21.1
RUN case "${TARGETARCH}" in \
      "arm64") MTX_ARCH="linux_arm64" ;; \
      "amd64"|*) MTX_ARCH="linux_amd64" ;; \
    esac && \
    echo "Downloading MediaMTX ${MEDIAMTX_VERSION} for ${MTX_ARCH}..." && \
    curl -sSL "https://github.com/bluenviron/mediamtx/releases/download/${MEDIAMTX_VERSION}/mediamtx_${MEDIAMTX_VERSION}_${MTX_ARCH}.tar.gz" \
      | tar -xz -C /app mediamtx && \
    chmod +x /app/mediamtx

# Copy compiled Go binary and configuration
COPY --from=builder /app/obs-homerun /app/obs-homerun
COPY mediamtx.yml /app/mediamtx.yml

RUN chmod +x /app/obs-homerun

# 1935: RTMP (OBS stream ingest)
# 5004: HTTP (virtual HDTV / DLNA stream & metadata)
# 1900: UDP (SSDP UPnP multicast discovery)
EXPOSE 1935/tcp 5004/tcp 1900/udp

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD curl -f http://127.0.0.1:5004/discover.json || exit 1

ENTRYPOINT ["/app/obs-homerun"]
