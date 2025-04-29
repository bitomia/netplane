FROM rust:1.85-slim-bullseye AS builder

WORKDIR /usr/src/reticula
COPY . .
RUN cargo update
RUN cargo build --bin server

FROM debian:bullseye-slim
RUN apt update && apt -y install build-essential
COPY --from=builder /usr/src/reticula/target/debug/server /usr/local/bin/reticula
ENTRYPOINT ["/usr/local/bin/reticula"]
