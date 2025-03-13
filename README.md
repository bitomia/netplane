# Reticula

## Building

Installing cargo-make:

```shell
cargo install --force cargo-make
```

For musl:

```shell
cargo make musl
```

## Initializing

```shell
cargo install sqlx-cli
# Check .env exists with a valid DATABSE_URL
sqlx database create --source src/migrations
```

## Running example

Server running at 172.16.140.1:5000:

```shell
RUST_LOG=debug cargo run server
```

Client mode:

```shell
RUST_LOG=debug cargo run client tun0 12.0.0.0 255.255.255.0 12.0.0.2 172.16.140.1:5000
```
