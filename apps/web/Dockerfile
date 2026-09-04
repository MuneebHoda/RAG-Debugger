# syntax=docker/dockerfile:1.7@sha256:a57df69d0ea827fb7266491f2813635de6f17269be881f696fbfdf2d83dda33e

FROM --platform=$BUILDPLATFORM node:24.8.0-alpine3.22@sha256:3e843c608bb5232f39ecb2b25e41214b958b0795914707374c8acc28487dea17 AS builder

WORKDIR /src/apps/web
ENV VITE_API_BASE_URL=http://127.0.0.1:8080
COPY apps/web/package.json apps/web/package-lock.json ./
RUN npm ci
COPY apps/web ./
RUN npm run build

FROM nginxinc/nginx-unprivileged:1.29.1-alpine3.22@sha256:27985295bdb22a1ef8f712863210bd5877c0f3006494a593e86b3fe0fa55467e

ARG OCI_REVISION=unknown
ARG OCI_VERSION=0.1.0
LABEL org.opencontainers.image.source="https://github.com/MuneebHoda/RAG-Debugger" \
      org.opencontainers.image.revision=$OCI_REVISION \
      org.opencontainers.image.version=$OCI_VERSION \
      org.opencontainers.image.licenses="MIT"

COPY --from=builder --chown=101:101 /src/apps/web/dist /usr/share/nginx/html
COPY --chown=101:101 apps/web/docker/nginx.conf /etc/nginx/nginx.conf
COPY --chmod=755 --chown=101:101 apps/web/docker/start-web.sh /usr/local/bin/start-web

EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/start-web"]
