use clap::Parser;

/// Open Rojo's documentation in your browser.
#[derive(Debug, Parser)]
pub struct DocCommand {}

impl DocCommand {
    pub fn run(self) -> anyhow::Result<()> {
        opener::open("https://rojo.space/docs")?;
        Ok(())
    }
}
