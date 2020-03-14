use std::{error::Error, fmt};

/// Wrapper type to print errors with source-chasing.
pub struct ErrorDisplay<E>(pub E);

impl<E: Error> fmt::Display for ErrorDisplay<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        writeln!(formatter, "{}", self.0)?;

        let mut current_err: &dyn Error = &self.0;
        while let Some(source) = current_err.source() {
            writeln!(formatter, "  caused by {}", source)?;
            current_err = &*source;
        }

        Ok(())
    }
}
