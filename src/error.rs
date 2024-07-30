use std::cmp::PartialEq;
use std::string::ToString;

use odbc_secrets_lib::error::OdbcSecretsLibError;

#[derive(
    strum_macros::AsRefStr,
    strum_macros::Display,
    strum_macros::EnumDiscriminants,
    strum_macros::IntoStaticStr,
    Debug,
)]
#[repr(u16)]
pub enum OdbcSecretsCliError {
    #[strum(to_string = "`clap::error::Error` error. {error:?}")]
    OdbcSecretsLibError {
        error: odbc_secrets_lib::error::OdbcSecretsLibError,
    } = 668,

    // ************************
    // * Library level errors *
    // ************************
    #[strum(to_string = "`clap::error::Error` error. {error:?}")]
    ClapError { error: clap::error::Error } = 739,
}

impl OdbcSecretsCliError {
    fn discriminant(&self) -> u16 {
        unsafe { *(self as *const Self as *const u16) }
    }
}

impl From<odbc_secrets_lib::error::OdbcSecretsLibError> for OdbcSecretsCliError {
    fn from(error: odbc_secrets_lib::error::OdbcSecretsLibError) -> Self {
        Self::OdbcSecretsLibError { error }
    }
}

impl From<clap::error::Error> for OdbcSecretsCliError {
    fn from(error: clap::error::Error) -> Self {
        Self::ClapError { error }
    }
}

impl From<vaultrs::error::ClientError> for OdbcSecretsCliError {
    fn from(error: vaultrs::error::ClientError) -> Self {
        Self::OdbcSecretsLibError {
            error: OdbcSecretsLibError::VaultClientError { error },
        }
    }
}

impl std::process::Termination for OdbcSecretsCliError {
    fn report(self) -> std::process::ExitCode {
        match self {
            OdbcSecretsCliError::OdbcSecretsLibError { error } => error.report(),
            _ => {
                let status_code = self.discriminant();
                if status_code > u8::MAX as u16 {
                    eprintln!("exit code {}", status_code);
                    std::process::ExitCode::FAILURE
                } else {
                    std::process::ExitCode::from(status_code as u8)
                }
            }
        }
    }
}

pub enum SuccessOrOdbcSecretsLibError<T> {
    Ok(T),
    Err(OdbcSecretsCliError),
}

impl<T> From<Result<T, OdbcSecretsCliError>> for SuccessOrOdbcSecretsLibError<T> {
    fn from(value: Result<T, OdbcSecretsCliError>) -> Self {
        match value {
            Ok(val) => SuccessOrOdbcSecretsLibError::Ok(val),
            Err(error) => SuccessOrOdbcSecretsLibError::Err(error),
        }
    }
}

// Can't use `Result` because
// [E0117] Only traits defined in the current crate can be implemented for arbitrary types
impl<T: std::any::Any> std::process::Termination for SuccessOrOdbcSecretsLibError<T> {
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
            SuccessOrOdbcSecretsLibError::Ok(e)
                if std::any::TypeId::of::<T>()
                    == std::any::TypeId::of::<std::process::ExitCode>() =>
            {
                *(&e as &dyn std::any::Any)
                    .downcast_ref::<std::process::ExitCode>()
                    .unwrap()
            }
            SuccessOrOdbcSecretsLibError::Ok(_) => std::process::ExitCode::SUCCESS,
            SuccessOrOdbcSecretsLibError::Err(err) => match err {
                OdbcSecretsCliError::OdbcSecretsLibError {
                    error: odbc_secrets_lib::error::OdbcSecretsLibError::StdIoError { error },
                } if error.raw_os_error().is_some() => {
                    let e = unsafe { error.raw_os_error().unwrap_unchecked() };
                    eprintln!("{}", e.to_string());
                    PROCESS_EXIT_CODE(e)
                }
                _ => {
                    eprintln!("{}", err.to_string());
                    PROCESS_EXIT_CODE(err.discriminant() as i32)
                }
            },
        }
    }
}
