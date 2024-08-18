odbc-cli
========
[![Build and release](https://github.com/SamuelMarks/odbc-cli/actions/workflows/build-and-release.yml/badge.svg)](https://github.com/SamuelMarks/odbc-cli/actions/workflows/build-and-release.yml)

Database abstracted CLI—using Open Database Connectivity (ODBC)—intended for basic and batch Create Read Update Delete (CRUD) operations, and negotiating database connection using a secret manager.

NOTE: You may need to install `unixodbc` on macOS, Linux (though the provided `musl` binary has it statically linked), and others; and on Windows you may need to install the ODBC driver for your database. For example, PostgreSQL: https://www.postgresql.org/ftp/odbc/releases/

## Usage

### Example

Start an ODBC compatible database. For example, to start PostgreSQL with `docker`:

    docker run -p 5432:5432 -e POSTGRES_USER=rest_user -e POSTGRES_PASSWORD=rest_pass -e POSTGRES_DB=rest_db postgres:alpine

(will start and initialise a new PostgreSQL instance accessible via connection string `postgresql://rest_user:rest_pass@127.0.0.1:5432/rest_db`)

Now you can execute `odbc-cli` (or `cargo run -- --conn […]`):

    odbc-cli \
      --conn "Driver={PostgreSQL UNICODE};Server=localhost;Port=5432;Database=rest_db;Uid=rest_user;Password=rest_pass;" \
      -c "SELECT atan(1)*4 AS pi"

### Vault / OpenBao

To avoid explicitly providing credential information, you can use Vault / OpenBao.

First, start or point to your secret store: [OpenBao](https://openbao.org) || [Hashicorp Vault](https://www.vaultproject.io) || [AWS Secret Engine](https://developer.hashicorp.com/vault/api-docs/secret/aws).

For example, for **testing only**, after [installing `bao` (guide)](https://openbao.org/docs/install):

    VAULT_TOKEN='dev-only-token'
    bao server -dev -dev-root-token-id="$VAULT_TOKEN"

(`docker run openbao/openbao` will do similar if you prefer Docker)

Then you can populate the secret store like so:

    odbc-cli \
      --conn "Driver={PostgreSQL UNICODE};Server=localhost;Port=5432;Database=rest_db;Uid=rest_user;Password=rest_pass;" \
      --store-secret \
      -c "SELECT version();"

(there's also a full Docker Compose example in the root of this repo)

#### Secret store environment variables

  - `VAULT_ADDR`
  - `VAULT_CACERT`
  - `VAULT_CAPATH`
  - `VAULT_CLIENT_CERT`
  - `VAULT_CLIENT_KEY`
  - `VAULT_SKIP_VERIFY`
  - `VAULT_TOKEN`

### `--help`

    CLI for basic CRUD across many databases using ODBC
    
    Usage: odbc-cli [OPTIONS]
    
    Options:
      -d, --data-source-name <DATA_SOURCE_NAME>
              DataSourceName
          --hostname <HOSTNAME>
              Hostname to connect with
          --port <PORT>
              Port to connect with
          --database <DATABASE>
              Database name to connect to
      -u, --username <USERNAME>
              Username to connect as
      -P, --password <PASSWORD>
              Password to connect with
          --conn <CONNECTION_STRING>
              Connect string to connect with. Takes precedence over `data_source_name`, `username`, `password`
      -c, --command <COMMAND>
              Query to execute
      -f, --command-file <COMMAND_FILE>
              Alternative query to execute from file or stdin
      -p, --params <PARAMS>
              Parameters to provide sanitarily to SQL query `--command`
          --secret-store-engine <SECRET_STORE_ENGINE>
              Secret storage service engine name [default: vault] [possible values: infisical, vault]
      -a, --address <ADDRESS>
              Secret storage service address [env: VAULT_ADDR=]
          --ca-cert <CA_CERT>
              Secret storage Certificate Authority (CA) certificate [env: VAULT_CACERT=]
          --ca-path <CA_PATH>
              Secret storage CA path [env: VAULT_CAPATH=]
          --client-cert <CLIENT_CERT>
              Secret storage client certificate [env: VAULT_CLIENT_CERT=]
          --client-key <CLIENT_KEY>
              Secret storage client key [env: VAULT_CLIENT_KEY=]
          --skip-verify <SKIP_VERIFY>
              Whether to skip verification on secret storage [env: VAULT_SKIP_VERIFY=] [possible values: true, false]
          --token <TOKEN>
              Secret storage service vault token [env: VAULT_TOKEN=]
          --store-secret
              Whether to store the provided `connection_string` in the secret store
          --secret-mount <SECRET_MOUNT>
              mount of secret within secret storage engine
          --secret-path <SECRET_PATH>
              path of secret within secret storage engine
          --output-format <OUTPUT_FORMAT>
              Output format for SQL query result [default: csv] [possible values: csv, json, parquet]
          --print-connection-str-and-exit
              Whether to just print the connection string and then exit
      -v, --verbose...
              Increase logging verbosity
      -q, --quiet...
              Decrease logging verbosity
      -h, --help
              Print help
      -V, --version
              Print version

---

<small>

## Development guide

### Install Rust

Follow the [official alt-guide](https://forge.rust-lang.org/infra/other-installation-methods.html#other-ways-to-install-rustup) or alternatively run one of the following:

#### Non-Windows
```sh
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh sh -s -- --default-toolchain nightly
```

#### Windows
```cmd
> curl --proto '=https' --tlsv1.2 -sSfO https://static.rust-lang.org/rustup/dist/i686-pc-windows-gnu/rustup-init.exe
> rustup-init --default-toolchain nightly
```

### Build and run project
```sh
$ cargo run
```

### Docker
```sh
$ docker build . -t odbc-cli:latest
```

#### Docker Compose
```sh
$ docker compose up
```

Then if you want to be fancy and get the executable out so it can be added to the releases:
```sh
$ docker save odbc-cli:latest -o odbc-cli.tar
# Find and extract the binary, for example:
$ odbc-cli.tar\blobs\sha256\7e3cb62b89edc58a47effa0f9a4e1fa792d3013911ab29f6bcbd6b60a64b5ffb\usr\local\bin\odbc-cli
$ mv odbc-cli odbc-cli-x86_64-unknown-linux-musl
# Upload this manually to the release or run
$ gh release upload "tag-name" "odbc-cli-x86_64-unknown-linux-musl"
```

## Contribution guide
Ensure all tests are passing [`cargo test`](https://doc.rust-lang.org/cargo/commands/cargo-test.html) and [`rustfmt`](https://github.com/rust-lang/rustfmt) has been run. This can be with [`cargo make`](https://github.com/sagiegurari/cargo-make); installable with:

```sh
$ cargo install --force cargo-make
```

Then run:
```sh
$ cargo make
```

Finally, we recommend [feature-branches](https://martinfowler.com/bliki/FeatureBranch.html) with an accompanying [pull-request](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/proposing-changes-to-your-work-with-pull-requests/about-pull-requests).
</small>
