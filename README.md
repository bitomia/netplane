# Netplane - Private networking built in Rust

Connect servers, laptops and devices into one private, encrypted network, managed from a central control plane and web dashboard. No port forwarding, no complex VPN setup.

Self-host it on your own infrastructure or embed it directly into your applications through the Go and C++ bindings.

## Installation

**Server**

Prerequisites:

* Linux x86
* Systemd
* Podman

```shell
curl -fsSL https://raw.githubusercontent.com/bitomia/netplane/main/scripts/install-server.sh | sudo sh -s -- --admin-email=<email>
```

**Client**

Prerequisites:

* Linux x86
* Systemd


```shell
curl -fsSL https://raw.githubusercontent.com/bitomia/netplane/main/scripts/install-client.sh | sudo sh -s -- --server=<host> --link=<code>
```

Run either script with `--help` for all options. Re-run to upgrade.

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

Server (reads `.env`, see `example.env`):

```shell
cp example.env .env
./netplane-server --create-user=<email>
./netplane-server
```

Client, linking on first run and reusing the stored key afterwards:

```shell
sudo ./netplane 127.0.0.1 --port=5050 --link=<code>
sudo ./netplane 127.0.0.1 --port=5050
```

## License

The free version of Netplane is available under the **GNU Affero General Public License v3.0 (AGPL-3.0)**. Components residing under the `ee/` directory require a commercial license. (See LICENSE for details.)
