mod error;

use clap::{Parser, ValueEnum};
use odbc_secrets_lib::odbc_runner::OutputFormat;

use crate::error::OdbcSecretsCliError;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
enum SecretStoreEngine {
    #[default]
    VAULT,
    INFISICAL,
}

impl SecretStoreEngine {
    /// Report all `possible_values`
    #[allow(dead_code)]
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

    /// Hostname to connect with
    #[arg(long)]
    hostname: Option<String>,

    /// Port to connect with
    #[arg(long)]
    port: Option<u16>,

    /// Database name to connect to
    #[arg(long)]
    database: Option<String>,

    /// Username to connect as
    #[arg(short, long)]
    username: Option<String>,

    /// Password to connect with
    #[arg(short = 'P', long)]
    password: Option<String>,

    /// Connect string to connect with. Takes precedence over `data_source_name`, `username`, `password`.
    #[arg(long = "conn")]
    connection_string: Option<String>,

    /// Query to execute
    #[arg(short, long)]
    command: Option<String>,

    /// Alternative query to execute from file or stdin
    #[arg(short = 'f', long)]
    command_file: Option<clap_stdin::FileOrStdin>,

    /// Parameters to provide sanitarily to SQL query `--command`
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
    #[arg(long, default_value_t = true)]
    store_secret: bool,

    /// mount of secret within secret storage engine
    #[arg(long)] // default_value_t=Some("secret".to_string())
    secret_mount: Option<String>,

    /// path of secret within secret storage engine
    #[arg(long)] // default_value=Some("odbc-conn".to_string())
    secret_path: Option<String>,

    /// Output format for SQL query result
    #[arg(long, default_value_t = Default::default())]
    output_format: OutputFormat,

    /// Whether to just print the connection string and then exit
    #[arg(long, default_value_t = false)]
    print_connection_str_and_exit: bool,

    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

#[tokio::main]
async fn main() -> Result<(), OdbcSecretsCliError> {
    let mut args = Args::parse();

    simple_logger::SimpleLogger::new()
        .with_level(args.verbose.log_level_filter())
        .init()
        .unwrap();

    let secret_mount = args.secret_mount.unwrap_or(String::from("secret"));
    let secret_path = args.secret_path.unwrap_or(String::from("odbc-conn"));

    if args.connection_string.is_none()
        && (args.data_source_name.is_none()
            || args.hostname.is_none()
            || args.port.is_none()
            || args.database.is_none()
            || args.username.is_none()
            || args.password.is_none())
    {
        match args.secret_store_engine {
            SecretStoreEngine::VAULT => {
                let vault_client = odbc_secrets_lib::secrets::vault_openbao::connect(
                    args.address.expect("Specify secret service `--address`"),
                    args.token.expect("Specify secret service `--token`"),
                )?;
                println!(
                    "{} version {}",
                    args.secret_store_engine, vault_client.settings.version
                );
                let secret: std::collections::HashMap<String, String> =
                    vaultrs::kv2::read(&vault_client, &secret_mount, &secret_path).await?;

                args.connection_string = match secret.get(&secret_path) {
                    None => None,
                    Some(s) => Some(s.to_owned()),
                };
            }
            SecretStoreEngine::INFISICAL => unimplemented!(),
        }

        if args.connection_string.is_none() {
            eprintln!(
                "Provide either `--conn` or all of `--data_source_name`, `--username`, `--password`"
            );
            return Err(clap::Error::new(clap::error::ErrorKind::MissingRequiredArgument).into());
        }
    } else if args.store_secret && args.address.is_some() && args.token.is_some() {
        let vault_client = odbc_secrets_lib::secrets::vault_openbao::connect(
            unsafe { args.address.unwrap_unchecked() },
            unsafe { args.token.unwrap_unchecked() },
        )?;

        if args.connection_string.is_none() {
            let data_source_name = args.data_source_name.clone().unwrap();
            let hostname = args.hostname.clone().unwrap();
            let port = args.port.unwrap();
            let database = args.database.clone().unwrap();
            let username = args.username.clone().unwrap();
            let password = args.password.clone().unwrap();
            args.connection_string = Some(format!(
                "Driver={{{}}};Server={};Port={};Database={};Uid={};Password={}",
                data_source_name, hostname, port, database, username, password
            ));
        }

        let connection_string_obj = std::collections::HashMap::<String, String>::from([(
            secret_path.to_owned(),
            args.connection_string.clone().unwrap(),
        )]);

        vaultrs::kv2::set(
            &vault_client,
            &secret_mount,
            &secret_path,
            &connection_string_obj,
        )
        .await?;
    }
    if args.connection_string.is_none() {
        let data_source_name = args.data_source_name.unwrap();
        let hostname = args.hostname.unwrap();
        let port = args.port.unwrap();
        let database = args.database.unwrap();
        let username = args.username.unwrap();
        let password = args.password.unwrap();
        args.connection_string = Some(format!(
            "Driver={{{}}};Server={};Port={};Database={};Uid={};Password={}",
            data_source_name, hostname, port, database, username, password
        ));
    }

    if args.print_connection_str_and_exit {
        println!("{}", args.connection_string.unwrap());
        return Ok(());
    }

    if args.command_file.is_none() && args.command.is_none() {
        eprintln!("Provide either `--command-file` or `--command`");
        return Err(clap::Error::new(clap::error::ErrorKind::MissingRequiredArgument).into());
    }

    Ok(odbc_secrets_lib::odbc_runner::odbc_runner(
        args.connection_string.unwrap(),
        args.params,
        match args.command {
            Some(q) => q,
            None => args.command_file.unwrap().contents()?,
        },
        args.output_format,
    )?)
}
