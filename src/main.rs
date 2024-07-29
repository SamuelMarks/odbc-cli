pub mod error;

use crate::error::OdbcCliError;
use clap::Parser;
use odbc_api::{buffers::TextRowSet, ConnectionOptions, Cursor, Environment, ResultSetMetadata};
use std::io::stdout;

/// Maximum number of rows fetched with one row set. Fetching batches of rows is usually much
/// faster than fetching individual rows.
const BATCH_SIZE: usize = 5000;
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
    #[arg(short, long = "uri")]
    connection_string: Option<String>,

    /// Query to execute
    #[arg(short, long)]
    query: String,

    /// Parameters to provide sanitarily to SQL query `--query`
    #[arg(short, long)]
    params: Option<String>,
}

fn main() -> Result<(), OdbcCliError> {
    let args = Args::parse();

    // Write csv to standard out
    let out = stdout();
    let mut writer = csv::Writer::from_writer(out);

    // If you do not do anything fancy it is recommended to have only one Environment in the
    // entire process.
    let environment = Environment::new()?;
    if args.connection_string.is_none()
        && (args.data_source_name.is_none() || args.username.is_none() || args.password.is_none())
    {
        eprintln!(
            "Provide either `--uri` or all of `--data_source_name`, `--username`, `--password`"
        );
        return Err(clap::Error::new(clap::error::ErrorKind::ValueValidation).into());
    }

    let connection = if args.connection_string.is_none() {
        environment.connect(
            args.data_source_name.unwrap().as_str(),
            args.username.unwrap().as_str(),
            args.password.unwrap().as_str(),
            ConnectionOptions::default(),
        )
    } else {
        environment.connect_with_connection_string(
            args.connection_string.unwrap().as_str(),
            ConnectionOptions::default(),
        )
    }?;
    let params = match args.params {
        None => (),
        Some(_params) => serde_json::from_str(_params.as_str())?,
    };

    // Execute a one of query without any parameters.
    match connection.execute(args.query.as_str(), params)? {
        Some(mut cursor) => {
            // Write the column names to stdout
            let headline: Vec<String> = cursor.column_names()?.collect::<Result<_, _>>()?;
            writer.write_record(headline)?;

            // Use schema in cursor to initialize a text buffer large enough to hold the largest
            // possible strings for each column up to an upper limit of 4KiB.
            let mut buffers = TextRowSet::for_cursor(BATCH_SIZE, &mut cursor, Some(4096))?;
            // Bind the buffer to the cursor. It is now being filled with every call to fetch.
            let mut row_set_cursor = cursor.bind_buffer(&mut buffers)?;

            // Iterate over batches
            while let Some(batch) = row_set_cursor.fetch()? {
                // Within a batch, iterate over every row
                for row_index in 0..batch.num_rows() {
                    // Within a row iterate over every column
                    let record = (0..batch.num_cols())
                        .map(|col_index| batch.at(col_index, row_index).unwrap_or(&[]));
                    // Writes row as csv
                    writer.write_record(record)?;
                }
            }
        }
        None => {
            eprintln!("Query came back empty. No output has been created.");
        }
    }

    Ok(())
}
