FROM rust:1.89-slim-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 --no-create-home service \
    && mkdir -p /data/cut_off_files \
    && chown service /data/cut_off_files
COPY --from=build /app/target/release/prex_processor /usr/local/bin/prex_processor
USER service
ENV HOST=0.0.0.0 \
    PORT=8080 \
    CUT_OFF_DIR=/data/cut_off_files
VOLUME /data/cut_off_files
EXPOSE 8080
ENTRYPOINT ["prex_processor"]
