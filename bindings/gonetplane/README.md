# Netplane Go Bindings

Go bindings for the Netplane client library using CGO.

## Prerequisites

1. Build the Netplane Rust library first:
   ```bash
   cd ../../../crates/client
   cargo build --release
   ```

2. Ensure Go 1.26+ and cbindgen are installed

## Installation

```bash
go get github.com/netplane/bindings/go/netplane
```

## Building

The library links against the Rust library, so you need to build the Rust library first:

```bash
# From the root of the netplane repository
cd crates/client
cargo build --release
```

## Usage Example

```go
package main

import (
    "log"
    "github.com/netplane/bindings/go/netplane"
)

func main() {
    // Initialize logger
    netplane.InitLogger()

    // Generate crypto keys if needed
    err := netplane.TryGenerateCryptoKeys("public.key", "private.key")
    if err != nil {
        log.Fatalf("Failed to generate keys: %v", err)
    }

    // Authenticate with the server
    err = netplane.ClientAuth(
        "auth.key",
        "public.key",
        "private.key",
        "example.com",
        "my-link-code",
        8080,
    )
    if err != nil {
        log.Fatalf("Failed to authenticate: %v", err)
    }

    // Create a transport
    transport, err := netplane.CreateTransport("example.com", 8080, "")
    if err != nil {
        log.Fatalf("Failed to create transport: %v", err)
    }
    defer transport.Free()

    // Perform handshake
    handshake, err := netplane.ClientHandshake("auth.key", transport)
    if err != nil {
        log.Fatalf("Failed to handshake: %v", err)
    }
    defer handshake.Free()

    log.Printf("IP: %s, Netmask: %s, Destination: %s",
        handshake.IPAddr, handshake.Netmask, handshake.Destination)

    // Run the client (you need to create the TUN device first)
    // tunFd := createTunDevice() // Platform-specific
    // err = netplane.ClientRun(tunFd, transport, handshake)
    // if err != nil {
    //     log.Fatalf("Failed to run client: %v", err)
    // }

    // Stop the client when done
    defer netplane.ClientStop()
}
```

## Notes

- Always call `Free()` on `Transport` and `HandshakeResult` when done to prevent memory leaks
- The library requires the netplane Rust library to be built and available in `../../../target/release`
- On macOS, the Security and SystemConfiguration frameworks are automatically linked
- Error codes from the C library are returned as Go errors with descriptive messages
