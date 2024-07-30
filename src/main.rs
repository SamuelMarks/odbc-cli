mod error;

use clap::{Parser, ValueEnum};

use crate::error::OdbcSecretsCliError;

#[derive(serde::Serialize, serde::Deserialize)]
struct OdbcConnection {
    odbc_conn: String,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
enum SecretStoreEngine {
    #[default]
    VAULT,
    INFISICAL,
}

impl SecretStoreEngine {
    /// Report all `possible_values`
    pub fn possible_values() -> impl Iterator<Item = clap::builder::PossibleValue> {
        Self::value_variants()
            .iter()
            .filter_map(clap::ValueEnum::to_possible_value)
    }
}

impl std::fmt::Display for SecretStoreEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}

impl std::str::FromStr for SecretStoreEngine {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("invalid variant: {s}"))
    }
}

impl clap::ValueEnum for SecretStoreEngine {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::INFISICAL, Self::VAULT]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Self::INFISICAL => clap::builder::PossibleValue::new("infisical"),
            Self::VAULT => clap::builder::PossibleValue::new("vault"),
        })
    }
}

/// CLI for basic CRUD across many databases using ODBC
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// DataSourceName
    #[arg(short, long)]
    data_source_name: Option<String>,

    /// Username to connect as
    #[arg(short, long)]
    username: Option<String>,

    /// Password to connect with
    #[arg(short = 'P', long)]
    password: Option<String>,

    /// Connect string to connect with. Takes precedence over `data_source_name`, `username`, `password`.
    #[arg(short, long = "conn")]
    connection_string: Option<String>,

    /// Query to execute
    #[arg(short, long)]
    query: String,

    /// Parameters to provide sanitarily to SQL query `--query`
    #[arg(short, long)]
    params: Option<String>,

    /// Secret storage service engine name
    #[arg(long, default_value_t=SecretStoreEngine::default())]
    secret_store_engine: SecretStoreEngine,

    /// Secret storage service address
    #[arg(short, long, env = "VAULT_ADDR")]
    address: Option<String>,

    /// Secret storage Certificate Authority (CA) certificate
    #[arg(long, env = "VAULT_CACERT")]
    ca_cert: Option<String>,

    /// Secret storage CA path
    #[arg(long, env = "VAULT_CAPATH")]
    ca_path: Option<String>,

    /// Secret storage client certificate
    #[arg(long, env = "VAULT_CLIENT_CERT")]
    client_cert: Option<String>,

    /// Secret storage client key
    #[arg(long, env = "VAULT_CLIENT_KEY")]
    client_key: Option<String>,

    /// Whether to skip verification on secret storage
    #[arg(long, env = "VAULT_SKIP_VERIFY")]
    skip_verify: Option<bool>,

    /// Secret storage service vault token
    #[arg(long, env = "VAULT_TOKEN")]
    token: Option<String>,

    /// Whether to store the provided `connection_string` in the secret store
    #[arg(long)]
    store_secret: Option<bool>,

    /// mount of secret within secret storage engine
    #[arg(long)] // default_value=("secret".to_string()))]
    secret_mount: Option<String>,

    /// path of secret within secret storage engine
    #[arg(long)] // default_value=("odbc-conn".to_string()))]
    secret_path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), OdbcSecretsCliError> {
    let mut args = Args::parse();
    let secret_mount = args.secret_mount.unwrap_or(String::from("secret"));
    let secret_path = args.secret_path.unwrap_or(String::from("odbc-conn"));
    if args.connection_string.is_none()
        && (args.data_source_name.is_none() || args.username.is_none() || args.password.is_none())
    {
        match args.secret_store_engine {
            SecretStoreEngine::VAULT => {
                let vault_client = odbc_secrets_lib::secrets::vault_openbao::connect(
                    args.address.expect("Specify secret service `--address`"),
                    args.token.expect("Specify secret service `--token`"),
                )?;
                /* println!(
                    "{} version {}",
                    args.secret_store_engine, vault_client.settings.version
                ); */
                let secret: OdbcConnection =
                    vaultrs::kv2::read(&vault_client, &secret_mount, &secret_path).await?;
                args.connection_string = Some(secret.odbc_conn);
            }
            SecretStoreEngine::INFISICAL => {}
        }

        if args.connection_string.is_none() {
            eprintln!(
                "Provide either `--conn` or all of `--data_source_name`, `--username`, `--password`"
            );
            return Err(clap::Error::new(clap::error::ErrorKind::MissingRequiredArgument).into());
        }
    } else if args.store_secret.unwrap_or(false) {
        let vault_client = odbc_secrets_lib::secrets::vault_openbao::connect(
            args.address.expect("Specify secret service `--address`"),
            args.token.expect("Specify secret service `--token`"),
        )?;
        let connection_string_struct = OdbcConnection {
            odbc_conn: args.connection_string.clone().unwrap(),
        };
        vaultrs::kv2::set(
            &vault_client,
            &secret_mount,
            &secret_path,
            &connection_string_struct,
        )
        .await?;
    }

    Ok(odbc_secrets_lib::odbc_runner::odbc_runner(
        args.connection_string,
        args.data_source_name,
        args.username,
        args.password,
        args.params,
        args.query,
    )?)
}
