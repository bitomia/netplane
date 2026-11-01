#!/bin/sh
# Install server as a systemd service running in podman.
#
#   curl -fsSL https://raw.githubusercontent.com/bitomia/netplane/main/scripts/install-server.sh \
#     | sudo sh -s -- [--admin-email=<email>]
#
# Re-running the script pulls the latest image and restarts the server.

set -eu

IMAGE="ghcr.io/bitomia/netplane-server"
ENV_FILE="/root/netplane-server.env"
DATA_DIR="/var/lib/netplane"
UNIT_FILE="/etc/systemd/system/netplane-server.service"

usage() {
    cat <<EOF
Usage: install-server.sh [options]

Options:
  --port=<port>              VPN port, UDP or TCP depending on transport (default: 5000)
  --web-port=<port>          Web/auth port (default: 8000)
  --transport=<udp|websocket>
                             Transport mode (default: udp)
  --dynamic-clients=<key>    Allow dynamic clients using this auth key
  --admin-email=<email>      Create or update a web user (password is prompted)
  --tag=<tag>                Image tag (default: latest)

--port, --web-port and --transport only apply when $ENV_FILE is created;
edit that file to change them later.
EOF
}

die() {
    echo "error: $*" >&2
    exit 1
}

random_secret() {
    head -c 32 /dev/urandom | base64 | tr -d '\n/+='
}

install_podman() {
    echo "Installing podman..."
    if command -v apt-get >/dev/null; then
        apt-get update -q && apt-get install -y -q podman
    elif command -v dnf >/dev/null; then
        dnf install -y -q podman
    else
        die "podman is not installed and no supported package manager was found"
    fi
}

main() {
    port="5000"
    web_port="8000"
    transport="udp"
    dynamic_clients=""
    admin_email=""
    tag="latest"

    for arg in "$@"; do
        case "$arg" in
            --port=*) port="${arg#*=}" ;;
            --web-port=*) web_port="${arg#*=}" ;;
            --transport=*) transport="${arg#*=}" ;;
            --dynamic-clients=*) dynamic_clients="${arg#*=}" ;;
            --admin-email=*) admin_email="${arg#*=}" ;;
            --tag=*) tag="${arg#*=}" ;;
            -h | --help) usage; exit 0 ;;
            *) usage >&2; die "unknown argument: $arg" ;;
        esac
    done

    [ "$(id -u)" -eq 0 ] || die "must run as root (pipe into 'sudo sh')"
    [ "$(uname -s)" = "Linux" ] || die "only Linux is supported"
    command -v systemctl >/dev/null || die "systemd is required"
    command -v podman >/dev/null || install_podman

    admin_password=""
    if [ -n "$admin_email" ]; then
        [ -r /dev/tty ] || die "--admin-email needs a terminal to prompt for the password"
        stty -echo </dev/tty
        printf "Password for %s: " "$admin_email" >/dev/tty
        read -r admin_password </dev/tty
        printf "\nConfirm password: " >/dev/tty
        read -r admin_password_confirm </dev/tty
        stty echo </dev/tty
        printf "\n" >/dev/tty
        [ -n "$admin_password" ] || die "password cannot be empty"
        [ "$admin_password" = "$admin_password_confirm" ] || die "passwords do not match"
    fi

    mkdir -p "$DATA_DIR"
    chmod 700 "$DATA_DIR"

    if [ -f "$ENV_FILE" ]; then
        echo "Keeping existing $ENV_FILE"
    else
        umask 077
        cat >"$ENV_FILE" <<EOF
RUST_LOG=info
AUTH_SECRET=$(random_secret)
JWT_SECRET=$(random_secret)
DATABASE_URL=sqlite:///data/netplane.db
SERVER=0.0.0.0:$port
WEBSERVER=0.0.0.0:$web_port
TRANSPORT=$transport
EOF
        umask 022
        echo "Created $ENV_FILE"
    fi

    echo "Pulling $IMAGE:$tag..."
    podman pull -q "$IMAGE:$tag" >/dev/null

    podman_bin="$(command -v podman)"
    run_opts="--env-file $ENV_FILE --network=host -v $DATA_DIR:/data -w /data"

    if [ -n "$admin_email" ]; then
        # shellcheck disable=SC2086
        printf "%s" "$admin_password" \
            | podman run --rm -i $run_opts "$IMAGE:$tag" --create-user="$admin_email"
    fi

    server_args=""
    [ -z "$dynamic_clients" ] || server_args=" --dynamic-clients=$dynamic_clients"

    cat >"$UNIT_FILE" <<EOF
[Unit]
Description=netplane-server
Wants=network-online.target
After=network-online.target

[Service]
Restart=on-failure
RestartSec=5
ExecStartPre=-$podman_bin rm -f netplane-server
ExecStart=$podman_bin run --rm --name netplane-server $run_opts $IMAGE:$tag$server_args
ExecStop=$podman_bin stop netplane-server
StandardOutput=append:/var/log/netplane-server.out.log
StandardError=append:/var/log/netplane-server.err.log

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    systemctl enable netplane-server
    systemctl restart netplane-server

    echo "netplane-server is running. Logs: /var/log/netplane-server.{out,err}.log"
    echo "Make sure the VPN port (UDP/TCP) and the web port are open in the firewall."
    echo "To add users later:"
    echo "  podman run --rm -it $run_opts $IMAGE:$tag --create-user=<email>"
}

main "$@"
