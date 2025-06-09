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

## Running example

Server running at 172.16.140.1:5000:

```shell
./reticula server
```

Client mode:

```shell
sudo ./reticula client tun0 12.0.0.0 255.255.255.0 12.0.0.2 172.16.140.1:5000
```
