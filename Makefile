.PHONY: client server admin

#all: client server admin
all: client server

client:
	cargo build -p client

server: reticuladb
	cargo build -p server

# admin:
# 	cargo build -p server --bin admin

reticuladb:
	sqlx database create
	sqlx migrate run --source ./server/src/migrations

webapp:
	cd web; pnpm install --frozen-lockfile; pnpm run build; cd -

docker: reticuladb webapp
	cargo build --bin server --target x86_64-unknown-linux-musl
	docker build -t ghcr.io/bitomia/reticula-server -f Dockerfile.server .

clean:
	cargo clean
