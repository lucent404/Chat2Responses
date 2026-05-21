FROM --platform=$BUILDPLATFORM node:24-bookworm-slim AS admin-ui-builder

WORKDIR /src/admin-ui

COPY admin-ui/package*.json ./
RUN npm ci

COPY admin-ui/ ./
RUN npm run build

FROM rust:1.95-bookworm AS rust-builder

WORKDIR /src

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY assets ./assets
RUN cargo build --release --locked --bin chat2responses

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --home-dir /app --shell /usr/sbin/nologin chat2responses \
    && mkdir -p /app/data \
    && chown -R chat2responses:chat2responses /app

COPY --from=rust-builder /src/target/release/chat2responses /usr/local/bin/chat2responses
COPY --from=admin-ui-builder /src/admin-ui/dist ./admin-ui/dist

USER chat2responses

ENV CHAT2RESPONSES_HOST=0.0.0.0 \
    CHAT2RESPONSES_PORT=4444 \
    CHAT2RESPONSES_DATABASE_URL=sqlite:///app/data/chat2responses.db \
    CHAT2RESPONSES_ADMIN_UI_DIR=/app/admin-ui/dist \
    RUST_LOG=chat2responses=info,tower_http=warn

EXPOSE 4444
VOLUME ["/app/data"]

CMD ["chat2responses"]
