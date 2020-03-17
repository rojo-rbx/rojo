pub fn doc() -> Result<(), anyhow::Error> {
    opener::open("https://rojo.space/docs")?;
    Ok(())
}
