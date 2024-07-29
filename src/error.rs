use std::cmp::PartialEq;
use std::string::ToString;

#[derive(
    strum_macros::AsRefStr,
    strum_macros::Display,
    strum_macros::EnumDiscriminants,
    strum_macros::IntoStaticStr,
    Debug,
)]
#[repr(u16)]
pub enum OdbcCliError {
    #[strum(to_string = "{0}")]
    NotFound(String) = 404,

    // ************************
    // * Library level errors *
    // ************************
    #[strum(to_string = "`std::io::Error` error. {error:?}")]
    StdIoError { error: std::io::Error } = 700,

    #[strum(to_string = "`std::fmt::Error` error. {error:?}")]
    StdFmtError { error: std::fmt::Error } = 709,

    #[strum(to_string = "{0:?}")]
    ExitCode(std::process::ExitCode) = 710,

    #[strum(to_string = "`serde_json::Error` error. {error:?}")]
    SerdeJsonError { error: serde_json::Error } = 721,

    #[strum(to_string = "`clap::error::Error` error. {error:?}")]
    ClapError { error: clap::error::Error } = 739,

    #[strum(to_string = "`odbc_api::Error` error. {error:?}")]
    OdbcApiError { error: odbc_api::Error } = 740,

    #[strum(to_string = "`csv::Error` error. {error:?}")]
    CsvError { error: csv::Error } = 741,
}

impl OdbcCliError {
    fn discriminant(&self) -> u16 {
        unsafe { *(self as *const Self as *const u16) }
    }
}

impl From<std::io::Error> for OdbcCliError {
    fn from(error: std::io::Error) -> Self {
        Self::StdIoError { error }
    }
}

impl From<std::fmt::Error> for OdbcCliError {
    fn from(error: std::fmt::Error) -> Self {
        Self::StdFmtError { error }
    }
}

impl From<clap::error::Error> for OdbcCliError {
    fn from(error: clap::error::Error) -> Self {
        Self::ClapError { error }
    }
}

impl From<odbc_api::Error> for OdbcCliError {
    fn from(error: odbc_api::Error) -> Self {
        Self::OdbcApiError { error }
    }
}

impl From<csv::Error> for OdbcCliError {
    fn from(error: csv::Error) -> Self {
        Self::CsvError { error }
    }
}

impl From<serde_json::Error> for OdbcCliError {
    fn from(error: serde_json::Error) -> Self {
        Self::SerdeJsonError { error }
    }
}

impl From<std::process::ExitCode> for OdbcCliError {
    fn from(error: std::process::ExitCode) -> Self {
        Self::ExitCode(error)
    }
}

impl std::process::Termination for OdbcCliError {
    fn report(self) -> std::process::ExitCode {
        if let OdbcCliError::ExitCode(exit_code) = self {
            return exit_code;
        }
        let status_code = self.discriminant();
        if status_code > u8::MAX as u16 {
            eprintln!("exit code {}", status_code);
            std::process::ExitCode::FAILURE
        } else {
            std::process::ExitCode::from(status_code as u8)
        }
    }
}

pub enum SuccessOrOdbcCliError<T> {
    Ok(T),
    Err(OdbcCliError),
}

impl<T> From<Result<T, OdbcCliError>> for SuccessOrOdbcCliError<T> {
    fn from(value: Result<T, OdbcCliError>) -> Self {
        match value {
            Ok(val) => SuccessOrOdbcCliError::Ok(val),
            Err(error) => SuccessOrOdbcCliError::Err(error),
        }
    }
}

// Can't use `Result` because
// [E0117] Only traits defined in the current crate can be implemented for arbitrary types
impl<T: std::any::Any> std::process::Termination for SuccessOrOdbcCliError<T> {
    fn report(self) -> std::process::ExitCode {
        const PROCESS_EXIT_CODE: fn(i32) -> std::process::ExitCode = |e: i32| {
            if e > u8::MAX as i32 {
                eprintln!("exit code {}", e);
                std::process::ExitCode::FAILURE
            } else {
                std::process::ExitCode::from(e as u8)
            }
        };

        match self {
            SuccessOrOdbcCliError::Ok(e)
                if std::any::TypeId::of::<T>()
                    == std::any::TypeId::of::<std::process::ExitCode>() =>
            {
                *(&e as &dyn std::any::Any)
                    .downcast_ref::<std::process::ExitCode>()
                    .unwrap()
            }
            SuccessOrOdbcCliError::Ok(_) => std::process::ExitCode::SUCCESS,
            SuccessOrOdbcCliError::Err(err) => match err {
                OdbcCliError::StdIoError { error } if error.raw_os_error().is_some() => {
                    let e = unsafe { error.raw_os_error().unwrap_unchecked() };
                    eprintln!("{}", e.to_string());
                    PROCESS_EXIT_CODE(e)
                }
                OdbcCliError::ExitCode(error) => error,
                _ => {
                    eprintln!("{}", err.to_string());
                    PROCESS_EXIT_CODE(err.discriminant() as i32)
                }
            },
        }
    }
}
