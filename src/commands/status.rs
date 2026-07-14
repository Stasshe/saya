use anyhow::Result;

use crate::backend::Backend;
use crate::manifest::Manifest;

pub fn run(manifest: &Manifest, backend: &dyn Backend) -> Result<()> {
    let statuses = super::compute_status(manifest, backend)?;
    if statuses.is_empty() {
        println!("manifest is empty");
        return Ok(());
    }
    for s in statuses {
        let mark = if s.installed { "installed" } else { "missing" };
        println!("{:<24} {}", s.name, mark);
    }
    Ok(())
}
