# Build from the repository root:
# docker build -f benchmarks/Dockerfile -t restphp-comparison .
# Run sequentially on a quiet host (the load generator shares these limits):
# docker run --rm --init --network none --cpus 2 --memory 2g --pids-limit 128 \
#   -v "$PWD/benchmarks/results:/app/benchmarks/results" restphp-comparison
ARG FRANKENPHP_IMAGE=dunglas/frankenphp:1-php8.2-bookworm@sha256:6dda03e9f02b2d81d9d6e40d2b443a5197ba296d28e50011d3b42277d4b58578
FROM rust:1-bookworm AS rust
FROM ${FRANKENPHP_IMAGE} AS prerequisites
USER root
COPY --from=rust /usr/local/cargo /usr/local/cargo
COPY --from=rust /usr/local/rustup /usr/local/rustup
ENV RUSTUP_HOME=/usr/local/rustup CARGO_HOME=/usr/local/cargo
ENV PATH="/opt/restphp-php/bin:/usr/local/cargo/bin:${PATH}"
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential autoconf pkg-config ca-certificates curl xz-utils python3 procps wrk \
    && rm -rf /var/lib/apt/lists/*
# Do not guess the PHP patch version from the image tag or an unrelated CLI.
# Compile a separate NTS embed library from the comparator's actual PHP release.
RUN set -eux; version="$(frankenphp php-cli -r 'echo PHP_VERSION;')"; \
    curl --fail --location --retry 3 "https://www.php.net/distributions/php-${version}.tar.xz" -o /tmp/php.tar.xz; \
    mkdir /tmp/php-source; tar -xJf /tmp/php.tar.xz -C /tmp/php-source --strip-components=1; \
    cd /tmp/php-source; \
    ./configure --prefix=/opt/restphp-php --disable-all --enable-embed=shared --enable-cli --without-pear; \
    make -j2; make install; \
    sha256sum /tmp/php.tar.xz > /opt/restphp-php/source-sha256.txt; \
    rm -rf /tmp/php-source /tmp/php.tar.xz
ENV PHP_CONFIG=/opt/restphp-php/bin/php-config PHP_EMBED_LIB_DIR=/opt/restphp-php/lib
ENV LIBRARY_PATH=/opt/restphp-php/lib RESTPHP_LD_LIBRARY_PATH=/opt/restphp-php/lib RUSTFLAGS="-C link-arg=-Wl,-rpath,/opt/restphp-php/lib"
FROM prerequisites AS comparison
WORKDIR /app
COPY Cargo.toml build.rs ./
COPY c ./c
COPY src ./src
COPY benchmarks ./benchmarks
RUN cargo build --release
ENTRYPOINT ["python3", "/app/benchmarks/run.py", "--restphp", "/app/target/release/restphp"]
