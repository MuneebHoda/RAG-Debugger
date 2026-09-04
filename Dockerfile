# syntax=docker/dockerfile:1.7@sha256:a57df69d0ea827fb7266491f2813635de6f17269be881f696fbfdf2d83dda33e

ARG RUST_VERSION=1.88.0
FROM --platform=$TARGETPLATFORM rust:${RUST_VERSION}-alpine3.22@sha256:9dfaae478ecd298b6b5a039e1f2cc4fc040fc818a2de9aa78fa714dea036574d AS builder

RUN apk add --no-cache ca-certificates musl-dev
WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY apps/api/Cargo.toml apps/api/Cargo.toml
COPY crates/core/Cargo.toml crates/core/Cargo.toml
COPY crates/rag/Cargo.toml crates/rag/Cargo.toml
COPY crates/storage/Cargo.toml crates/storage/Cargo.toml
COPY apps/api apps/api
COPY crates crates
COPY fixtures fixtures
COPY migrations migrations

RUN cargo build --locked --release -p rag-debugger-api

FROM scratch

ARG OCI_REVISION=unknown
ARG OCI_VERSION=0.1.0
LABEL org.opencontainers.image.source="https://github.com/MuneebHoda/RAG-Debugger" \
      org.opencontainers.image.revision=$OCI_REVISION \
      org.opencontainers.image.version=$OCI_VERSION \
      org.opencontainers.image.licenses="MIT"

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /src/target/release/rag-debugger-api /usr/local/bin/rag-debugger-api

ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
EXPOSE 8080
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/rag-debugger-api"]
HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=6 CMD ["/usr/local/bin/rag-debugger-api", "readycheck"]
