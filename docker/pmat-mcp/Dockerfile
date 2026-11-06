# pmat MCP server over streamable HTTP.
#
# Builds from a pinned commit of the fork, not the local tree, so the image is
# reproducible, sources are LF on a Windows host, and local secrets such as
# .mcp.json never enter the build. Change PMAT_REF to build another commit.

FROM rust:1.96-slim-trixie AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates cmake g++ git make perl pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

ARG PMAT_REPO=https://github.com/JacobSal/paiml-mcp-agent-toolkit.git
ARG PMAT_REF=64ee4a3cff04fc31770b2d92b507e6b9de56f86e
ARG BUILD_JOBS=6

WORKDIR /build
RUN git init -q . \
    && git remote add origin "$PMAT_REPO" \
    && git fetch -q --depth 1 origin "$PMAT_REF" \
    && git checkout -q FETCH_HEAD

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --locked --bin pmat \
        --features r-ast,matlab-ast -j "$BUILD_JOBS" \
    && cp target/release/pmat /usr/local/bin/pmat

FROM debian:trixie-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 1000 pmat \
    && git config --system --add safe.directory '*' \
    && mkdir /work \
    && chown pmat:pmat /work

COPY --from=builder /usr/local/bin/pmat /usr/local/bin/pmat

# pmat writes its search index to .pmat under the working directory, so /work
# is a writable volume and the source repos are mounted read-only beneath it.
USER pmat
WORKDIR /work
EXPOSE 8765

ENTRYPOINT ["pmat"]
CMD ["serve", "--transport", "http", "--host", "0.0.0.0", "--port", "8765"]
