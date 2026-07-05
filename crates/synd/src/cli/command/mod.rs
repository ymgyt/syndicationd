pub mod clean;
pub mod config;
pub mod daemon;
pub mod doctor;
pub mod export;
pub mod feed;
pub mod import;
pub mod term;

use std::{io, process::ExitCode};

/// User-facing failure report for CLI command boundaries.
pub(crate) struct CommandFailure {
    error: anyhow::Error,
}

impl CommandFailure {
    pub(crate) fn report(error: impl Into<anyhow::Error>) -> ExitCode {
        let failure = Self {
            error: error.into(),
        };
        if let Err(write_error) = failure.write(io::stderr()) {
            eprintln!("error: failed to write command failure: {write_error}");
        }

        ExitCode::FAILURE
    }

    fn write(&self, mut writer: impl io::Write) -> io::Result<()> {
        writeln!(writer, "error: {:#}", self.error)
    }
}

#[cfg(test)]
mod tests {
    use super::CommandFailure;

    mod command_failure {
        use super::*;

        #[test]
        fn writes_error() {
            let error = anyhow::anyhow!("failed");
            let failure = CommandFailure { error };
            let mut output = Vec::new();

            failure.write(&mut output).unwrap();

            assert_eq!(String::from_utf8(output).unwrap(), "error: failed\n");
        }
    }
}
