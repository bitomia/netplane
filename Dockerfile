FROM rust:1.84-slim-bullseye AS builder

WORKDIR /usr/src/reticula
COPY . .
RUN cargo install --path .

FROM debian:bullseye-slim
RUN apt update && apt -y install build-essential
COPY --from=builder /usr/local/cargo/bin/reticula /usr/local/bin/reticula
ENTRYPOINT ["/usr/local/bin/reticula"]
