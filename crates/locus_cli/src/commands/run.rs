//! `locus run` command (stub for future implementation).

use anyhow::Result;

use crate::output;

pub async fn handle(model: Option<String>, provider: Option<String>) -> Result<()> {
    output::warning("`locus run` is not yet implemented.");
    output::dim("The interactive agent session will be available when locus_runtime is ready.");

    if let Some(m) = model {
        output::dim(&format!("  Requested model: {}", m));
    }
    if let Some(p) = provider {
        output::dim(&format!("  Requested provider: {}", p));
    }

    Ok(())
}
