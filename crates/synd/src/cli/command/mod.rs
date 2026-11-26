pub mod clean;
pub mod config;
pub mod daemon;
pub mod doctor;
pub mod export;
pub mod feed;
pub mod import;
pub mod mcp;
pub mod term;

fn print_json_schema<T: schemars::JsonSchema>() -> anyhow::Result<()> {
    serde_json::to_writer_pretty(std::io::stdout().lock(), &schemars::schema_for!(T))?;
    Ok(())
}
