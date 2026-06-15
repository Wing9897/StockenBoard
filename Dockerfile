# Stage 1: Build the Rust server binary
FROM rust:1.79-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY src-tauri/ .
RUN cargo build --release --bin server --no-default-features --features server

# Stage 2: Build the frontend SPA
FROM node:20-alpine AS frontend
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

# Stage 3: Minimal runtime image
FROM alpine:3.19
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/server /usr/local/bin/stockenboard-server
COPY --from=frontend /app/dist /app/static

ENV SB_DATA_DIR=/data
ENV SB_BIND=0.0.0.0
ENV SB_PORT=8080
ENV SB_STATIC_DIR=/app/static

EXPOSE 8080
VOLUME ["/data"]

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:8080/ || exit 1

ENTRYPOINT ["stockenboard-server"]
