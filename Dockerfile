FROM ekidd/rust-musl-builder:latest as planner

ARG CARGO_BUILD_TARGET=x86_64-unknown-linux-musl

WORKDIR /app
RUN sudo chown -R rust:rust .
RUN cargo install cargo-chef 
COPY . .
RUN cargo chef prepare --recipe-path recipe.json
RUN pwd && ls

FROM ekidd/rust-musl-builder:latest as cacher
WORKDIR /app
RUN sudo chown -R rust:rust .
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target=x86_64-unknown-linux-musl --recipe-path recipe.json

FROM ekidd/rust-musl-builder:latest as builder
WORKDIR /app
RUN sudo chown -R rust:rust .
COPY . .
# Copy over the cached dependencies
COPY --from=cacher /app/target target
COPY --from=cacher $CARGO_HOME $CARGO_HOME
RUN cargo build --release --target=x86_64-unknown-linux-musl --bin app

FROM alpine:latest as runtime
WORKDIR /app
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/app /usr/local/bin
CMD ["/usr/local/bin/app"]
