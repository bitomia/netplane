{
  description = "Reticula server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages = {
          default = self.packages.${system}.reticula-server;
          
          reticula-server = pkgs.rustPlatform.buildRustPackage rec {
            pname = "reticula-server";
            version = "0.1.0";

            src = ./.;
            # For git sources with workspace:
            # src = pkgs.fetchFromGitHub {
            #   owner = "username";
            #   repo = "repository";
            #   rev = "commit-hash-or-tag";
            #   sha256 = "sha256-hash";
            # };

            cargoBuildFlags = [ "--package" "server" ];
            cargoTestFlags = [ "--package" "server" ];

            cargoLock = {
              lockFile = ./Cargo.lock;
            };
            nativeBuildInputs = with pkgs; [
              pkg-config
              sqlx-cli
              sqlite
            ];

            buildInputs = with pkgs; [
              openssl
            ];

            # Environment variables for the build
            # PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
            # OPENSSL_DIR = "${pkgs.openssl.dev}";

            preBuild = ''
              echo "Running pre-build steps..."
              sqlx database create
              sqlx migrate run --source ./server/src/migrations
            '';

            meta = with pkgs.lib; {
            #   description = "A Rust application from workspace";
            #   homepage = "https://github.com/user/repo";
            #   license = licenses.mit;
            #   maintainers = [ maintainers.yourname ];
              platforms = platforms.all;
            };
          };
        };

        packages.reticula-server-static = pkgs.pkgsStatic.rustPlatform.buildRustPackage rec {
          pname = "reticula-server";
          version = "0.1.0";
          src = ./.;

          cargoBuildFlags = [ "--package" "server" ];
          cargoTestFlags = [ "--package" "server" ];
          
          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs.pkgsStatic; [
            pkg-config
            sqlx-cli
            sqlite
          ];

          buildInputs = with pkgs.pkgsStatic; [
            openssl
          ];

          preBuild = ''
            echo "Setting up database for static SQLx build..."
            sqlx database create
            # sqlx migrate run
            echo "Database setup complete"
          '';

          # Ensure static linking
          CARGO_BUILD_TARGET = "x86_64-unknown-linux-musl";
          RUSTFLAGS = "-C target-feature=+crt-static";

          meta = with pkgs.lib; {
            description = "Static build of Rust application for containers";
            platforms = [ "x86_64-linux" ];
          };
        };
        
        # Development shell
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustc
            cargo
            rustfmt
            clippy
            rust-analyzer
            pkg-config
            openssl.dev
            git
          ];

          RUST_BACKTRACE = "1";
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
        };

        packages.docker = pkgs.dockerTools.buildImage {
          name = "reticula-server";
          tag = "latest";
          contents = [ self.packages.${system}.reticula-server-static ];
          config = {
            Cmd = [ "${self.packages.${system}.reticula-server-static}/bin/reticula-server" ];
            ExposedPorts = {
              "8080/tcp" = {};
              "6000/tcp" = {};
              "6000/udp" = {};
            };
          };
        };
      });
}
