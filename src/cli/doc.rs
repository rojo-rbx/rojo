use opener::{open, OpenError};
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub struct DocError(Error);

#[derive(Debug, Snafu)]
enum Error {
    Open { source: OpenError },
}

impl From<OpenError> for Error {
    fn from(source: OpenError) -> Self {
        Error::Open { source }
    }
}

pub fn doc() -> Result<(), DocError> {
    doc_inner()?;
    Ok(())
}

fn doc_inner() -> Result<(), Error> {
    open("https://rojo.space/docs")?;
    Ok(())
}
