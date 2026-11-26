FROM rust:1.83-bookworm AS server-build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY server/ server/
COPY cli/ cli/
RUN cargo build --release --bin jebat

FROM node:22-bookworm-slim AS web-build
WORKDIR /build
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ .
RUN npm run build

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates openssl dnsutils && rm -rf /var/lib/apt/lists/*
RUN useradd -m -s /bin/bash jebat
WORKDIR /app

COPY --from=server-build /build/target/release/jebat /app/jebat
COPY --from=web-build /build/dist /app/web/dist

RUN mkdir -p /app/config /app/data /app/loot && chown -R jebat:jebat /app
USER jebat

EXPOSE 9090 443

ENTRYPOINT ["/app/jebat"]
CMD ["serve", "--config", "/app/config/jebat.toml"]
