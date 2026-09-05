FROM rust:1.79-bullseye AS builder

WORKDIR /usr/src/app
RUN apt-get update && apt-get install -y php-dev libphp-embed

COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y php-cli libphp-embed && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/restphp /usr/local/bin/restphp
COPY frameworks/Rust/restphp/public /app/public
WORKDIR /app
CMD ["restphp", "serve", "-p", "8080", "--entrypoint", "public/index.php", "--workers", "1"]
