# Netplane - Overlay Network

## Dependencies

Install bun latest version: https://bun.sh/

Install sqlx-cli:

```
cargo install sqlx-cli
```

## Building

Netplane uses make to manage build. In order to build all main targets (database, webapp, server and client), just launch:

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

## License

The free version of Netplane is available under the **GNU Affero General Public License v3.0 (AGPL-3.0)**. Components residing under the `ee/` directory require a commercial license. (See LICENSE for details.)
