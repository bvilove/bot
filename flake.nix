{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
    flake-utils.url = "github:numtide/flake-utils";
    import-cargo.url = "github:edolstra/import-cargo";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    import-cargo,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {inherit system overlays;};
        inherit (import-cargo.builders) importCargo;
        rustNightly = pkgs.rust-bin.nightly.latest.default;
        rustStable = pkgs.rust-bin.stable.latest.minimal;

        pgstart = pkgs.writeShellScriptBin "pgstart" ''
          if [ ! -d $PGHOST ]; then
            mkdir -p $PGHOST
          fi
          if [ ! -d $PGDATA ]; then
            echo 'Initializing postgresql database...'
            LC_ALL=C.utf8 initdb $PGDATA --auth=trust >/dev/null
          fi
          OLD_PGDATABASE=$PGDATABASE
          export PGDATABASE=postgres
          pg_ctl start -l $LOG_PATH -o "-c listen_addresses= -c unix_socket_directories=$PGHOST"
          psql -tAc "SELECT 1 FROM pg_database WHERE datname = 'bvilovebot'" | grep -q 1 || psql -tAc 'CREATE DATABASE "bvilovebot"'
          export PGDATABASE=$OLD_PGDATABASE
        '';

        pgstop = pkgs.writeShellScriptBin "pgstop" ''
          pg_ctl -D $PGDATA stop | true
        '';

        buildInputs = with pkgs; [openssl];
        nativeBuildInputs = with pkgs; [pkg-config];

        shellInputs =
          buildInputs
          ++ nativeBuildInputs
          ++ (with pkgs; [
            alejandra
            postgresql
            sea-orm-cli
            sqlx-cli
          ])
          ++ [
            pgstart
            pgstop
          ];

        bvilovebot = pkgs.stdenv.mkDerivation {
          name = "bvilovebot";
          src = self;

          buildInputs = buildInputs;

          nativeBuildInputs =
            nativeBuildInputs
            ++ [
              (importCargo {
                lockFile = ./Cargo.lock;
                inherit pkgs;
              })
              .cargoHome
            ]
            ++ [rustStable];

          buildPhase = ''
            cargo build --release --offline
          '';

          installPhase = ''
            install -Dm775 ./target/release/bvilovebot $out/bin/bvilovebot
          '';
        };

        shellHook = ''
          export PGDATA=$PWD/postgres/data
          export PGHOST=$PWD/postgres
          export LOG_PATH=$PWD/postgres/LOG
          export PGDATABASE=bvilovebot
          export DATABASE_URL=postgresql:///bvilovebot?host=$PWD/postgres;
        '';
      in {
        packages = {
          default = bvilovebot;
        };
        devShells = {
          default = pkgs.mkShell {
            inherit shellHook;
            buildInputs = shellInputs ++ [rustNightly];
          };
          norust = pkgs.mkShell {
            inherit shellHook;
            buildInputs = shellInputs;
          };
        };
      }
    );
}
