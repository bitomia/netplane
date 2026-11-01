#!/bin/sh
# Install client as a systemd service.
#
#   curl -fsSL https://raw.githubusercontent.com/bitomia/netplane/main/scripts/install-client.sh \
#     | sudo sh -s -- --server=<host> [--link=<code>|--dynamic-link=<code>]
#
# Re-running the script upgrades binary.

set -eu

REPO="bitomia/netplane"
ARCHIVE="netplane-linux-amd64.tgz"
INSTALL_DIR="/opt/netplane"
UNIT_FILE="/etc/systemd/system/netplane-client.service"
LINK_TIMEOUT=30

usage() {
    cat <<EOF
Usage: install-client.sh --server=<host> [options]

Options:
  --server=<host>           Server address (required)
  --port=<port>             Server port (default: 5000)
  --tun=<device>            TUN device name (default: tun0)
  --transport=<udp|websocket>
  --link=<code>             Link this client with a link code
  --dynamic-link=<code>     Link this client with a dynamic link code
  --auth-port=<port>        Server auth (web) port used for linking (default: 8000)
  --version=<tag>           Release tag to install (default: latest)
EOF
}

die() {
    echo "error: $*" >&2
    exit 1
}

main() {
    server=""
    port="5000"
    tun="tun0"
    transport=""
    link=""
    dynamic_link=""
    auth_port=""
    version="latest"

    for arg in "$@"; do
        case "$arg" in
            --server=*) server="${arg#*=}" ;;
            --port=*) port="${arg#*=}" ;;
            --tun=*) tun="${arg#*=}" ;;
            --transport=*) transport="${arg#*=}" ;;
            --link=*) link="${arg#*=}" ;;
            --dynamic-link=*) dynamic_link="${arg#*=}" ;;
            --auth-port=*) auth_port="${arg#*=}" ;;
            --version=*) version="${arg#*=}" ;;
            -h | --help) usage; exit 0 ;;
            *) usage >&2; die "unknown argument: $arg" ;;
        esac
    done

    [ -n "$server" ] || { usage >&2; die "--server is required"; }
    [ -z "$link" ] || [ -z "$dynamic_link" ] || die "--link and --dynamic-link are mutually exclusive"
    [ "$(id -u)" -eq 0 ] || die "must run as root (pipe into 'sudo sh')"
    [ "$(uname -s)" = "Linux" ] || die "only Linux is supported"
    [ "$(uname -m)" = "x86_64" ] || die "only x86_64 is supported (got $(uname -m))"
    command -v systemctl >/dev/null || die "systemd is required"
    command -v curl >/dev/null || die "curl is required"
    command -v sha256sum >/dev/null || die "sha256sum is required"
    [ -c /dev/net/tun ] || die "/dev/net/tun not found (is the tun module loaded?)"

    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT

    url="https://github.com/$REPO/releases/download/$version"
    echo "Downloading $ARCHIVE ($version)..."
    curl -fsSL -o "$tmp/$ARCHIVE" "$url/$ARCHIVE"
    curl -fsSL -o "$tmp/$ARCHIVE.sha256" "$url/$ARCHIVE.sha256"
    (cd "$tmp" && sha256sum -c "$ARCHIVE.sha256" >/dev/null) || die "checksum verification failed"
    tar -xzf "$tmp/$ARCHIVE" -C "$tmp"

    if systemctl is-active --quiet netplane-client; then
        systemctl stop netplane-client
    fi

    mkdir -p "$INSTALL_DIR"
    install -m 0755 "$tmp/netplane" "$INSTALL_DIR/netplane"
    echo "Installed $INSTALL_DIR/netplane"

    args="$server --port=$port --tun=$tun"
    [ -z "$transport" ] || args="$args --transport=$transport"

    # The client has no link-only mode: it links, writes the key file and then
    # keeps running, so run it until the key appears and stop it.
    if [ -n "$link" ] || [ -n "$dynamic_link" ]; then
        if [ -n "$link" ]; then
            key_file="auth.key"
            link_arg="--link=$link"
        else
            key_file="dynamic.key"
            link_arg="--dynamic-link=$dynamic_link"
        fi
        [ -z "$auth_port" ] || link_arg="$link_arg --auth-port=$auth_port"

        rm -f "$INSTALL_DIR/auth.key" "$INSTALL_DIR/dynamic.key"
        echo "Linking client with $server..."
        # shellcheck disable=SC2086
        (cd "$INSTALL_DIR" && exec ./netplane $args $link_arg) >"$tmp/link.log" 2>&1 &
        pid=$!
        i=0
        while [ ! -s "$INSTALL_DIR/$key_file" ] && kill -0 "$pid" 2>/dev/null && [ "$i" -lt "$LINK_TIMEOUT" ]; do
            sleep 1
            i=$((i + 1))
        done
        kill "$pid" 2>/dev/null || true
        wait "$pid" 2>/dev/null || true

        if [ ! -s "$INSTALL_DIR/$key_file" ]; then
            cat "$tmp/link.log" >&2
            die "linking failed"
        fi
        chmod 600 "$INSTALL_DIR/$key_file"
        echo "Client linked"
    elif [ ! -s "$INSTALL_DIR/auth.key" ] && [ ! -s "$INSTALL_DIR/dynamic.key" ]; then
        die "client is not linked yet: pass --link=<code> or --dynamic-link=<code>"
    fi

    cat >"$UNIT_FILE" <<EOF
[Unit]
Description=netplane-client
Wants=network-online.target
After=network-online.target

[Service]
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/netplane $args
Restart=on-failure
RestartSec=5
StandardOutput=append:/var/log/netplane-client.out.log
StandardError=append:/var/log/netplane-client.err.log

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    systemctl enable --now netplane-client
    echo "netplane-client is running. Logs: /var/log/netplane-client.{out,err}.log"
}

main "$@"
