{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {inherit system overlays;};
        rustNightly = pkgs.rust-bin.nightly.latest.default;

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
          psql -tAc "SELECT 1 FROM pg_database WHERE datname = 'olympdating'" | grep -q 1 || psql -tAc 'CREATE DATABASE "olympdating"'
          export PGDATABASE=$OLD_PGDATABASE
        '';

        pgstop = pkgs.writeShellScriptBin "pgstop" ''
          pg_ctl -D $PGDATA stop | true
        '';

        buildInputs = with pkgs;
          [
            sqlx-cli
            postgresql
            openssl
            pkg-config
            sea-orm-cli
            alejandra
          ]
          ++ [
            pgstart
            pgstop
          ];

        shellHook = ''
          export PGDATA=$PWD/postgres/data
          export PGHOST=$PWD/postgres
          export LOG_PATH=$PWD/postgres/LOG
          export PGDATABASE=olympdating
          export DATABASE_URL=postgresql:///olympdating?host=$PWD/postgres;
        '';
      in {
        devShells = {
          default = pkgs.mkShell {
            inherit shellHook;
            buildInputs = buildInputs ++ [rustNightly];
          };
          norust = pkgs.mkShell {
            inherit shellHook buildInputs;
          };
        };
      }
    );
}
