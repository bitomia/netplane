# Netplane - Software defined networks

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

Netplane is **dual-licensed**:

### Open Source License: AGPL-3.0

The open-source version of Netplane is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0)**.

### Commercial License

If you cannot comply with AGPL-3.0 requirements, we offer **commercial licenses** that allow you to:

- 🔓 Use Netplane in proprietary/closed-source applications
- 🔓 Embed Netplane via the C API without open-sourcing your code
- 🔓 Offer Netplane-based services without releasing your source code
- 🎯 Receive enterprise support and SLA guarantees
- 🎯 Access professional services and custom development

**Commercial licenses include**:
- Freedom from AGPL-3.0 requirements
- Enterprise support with SLA
- Professional services options
- Legal indemnification

For commercial licensing options and pricing, contact **licensing@bitomia.com**.

Copyright (C) 2024-2025 Bitomia Software SLU. All rights reserved.

