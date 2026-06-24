use anyhow::Result;

use crate::backend::Backend;
use crate::manifest::Manifest;

pub fn run(manifest: &Manifest, backend: &dyn Backend) -> Result<()> {
    let statuses = super::compute_status(manifest, backend)?;
    let missing: Vec<String> = statuses
        .into_iter()
        .filter(|s| !s.installed)
        .map(|s| s.real_name)
        .collect();

    if missing.is_empty() {
        println!("already up to date");
        return Ok(());
    }

    println!("installing: {}", missing.join(", "));
    backend.install(&missing)
}
