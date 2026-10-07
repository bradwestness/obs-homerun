FROM alpine:3.21

LABEL maintainer="Brad Westness"
LABEL org.opencontainers.image.title="OBS HomeRun"
LABEL org.opencontainers.image.description="Turn your OBS stream into a virtual HDTV tuner for Smart TVs"
LABEL org.opencontainers.image.source="https://github.com/bradwestness/obs-homerun"

# Install runtime dependencies (Python 3, FFmpeg)
RUN apk add --no-cache \
    python3 \
    ffmpeg \
    curl \
    ca-certificates \
    tar

WORKDIR /app

# Download MediaMTX binary according to target architecture (amd64 / arm64)
ARG TARGETARCH
ARG MEDIAMTX_VERSION=v1.21.1
RUN case "${TARGETARCH}" in \
      "arm64") MTX_ARCH="linux_arm64v8" ;; \
      "amd64"|*) MTX_ARCH="linux_amd64" ;; \
    esac && \
    echo "Downloading MediaMTX ${MEDIAMTX_VERSION} for ${MTX_ARCH}..." && \
    curl -sSL "https://github.com/bluenviron/mediamtx/releases/download/${MEDIAMTX_VERSION}/mediamtx_${MEDIAMTX_VERSION}_${MTX_ARCH}.tar.gz" \
      | tar -xz -C /app mediamtx && \
    chmod +x /app/mediamtx

# Copy application files
COPY mediamtx.yml /app/mediamtx.yml
COPY broadcaster.py /app/broadcaster.py
COPY entrypoint.sh /app/entrypoint.sh

RUN chmod +x /app/entrypoint.sh /app/broadcaster.py

# 1935: RTMP (OBS stream ingest)
# 5004: HTTP (virtual HDTV / DLNA stream & metadata)
# 1900: UDP (SSDP UPnP multicast discovery)
EXPOSE 1935/tcp 5004/tcp 1900/udp

ENTRYPOINT ["/app/entrypoint.sh"]
