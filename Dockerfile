FROM node:20-slim AS web-builder
ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
RUN corepack enable

WORKDIR /usr/src/web
COPY web/package.json ./
COPY web/pnpm-lock.yaml ./
COPY web/app ./app
COPY web/public ./public
COPY web/tsconfig.json ./
COPY web/vite.config.ts ./
COPY web/react-router.config.ts ./
RUN --mount=type=cache,id=pnpm,target=/pnpm/store pnpm install --frozen-lockfile
RUN pnpm run build

FROM rust:1.85-slim-bullseye AS builder

WORKDIR /usr/src/reticula
COPY . .
RUN cargo update
RUN cargo build --bin server

FROM debian:bullseye-slim
RUN apt update && apt -y install build-essential
COPY --from=web-builder /usr/src/web/build /usr/local/bin/web/build
COPY --from=builder /usr/src/reticula/target/debug/server /usr/local/bin/reticula

WORKDIR /usr/local/bin
ENTRYPOINT ["reticula"]
