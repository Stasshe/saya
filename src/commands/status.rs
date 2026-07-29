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
        let mark = match (s.desired_present, s.installed) {
            (true, true) => "installed",
            (true, false) => "missing",
            (false, true) => "pending removal",
            (false, false) => "absent",
        };
        println!("{:<24} {}", s.name, mark);
    }
    Ok(())
}
