# Netplane

## Building

Installing cargo-make:

```shell
make
```

For musl:

```shell
cargo make musl
```

## Running example

Server running at 127.0.0.1:5000:

```shell
./netplane-server
```

Client mode:

```shell
sudo ./netplane tun0 127.0.0.1:5000 --auth=http://127.0.0.1:8000/auth/XXXXX
sudo ./netplane tun0 127.0.0.1:5000
```


