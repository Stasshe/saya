use anyhow::Result;

use crate::backend::Backend;
use crate::manifest::Manifest;

pub fn run(manifest: &Manifest, backend: &dyn Backend) -> Result<()> {
    let statuses = super::compute_status(manifest, backend)?;
    let missing: Vec<String> = statuses
        .into_iter()
        .filter(|status| !status.installed)
        .map(|status| status.real_name)
        .collect();

    if missing.is_empty() {
        println!("already up to date");
        return Ok(());
    }

    println!("installing: {}", missing.join(", "));
    backend.install(&missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::backend::BackendKind;

    struct FakeBackend {
        installed: Vec<String>,
        expected: Vec<String>,
    }

    impl Backend for FakeBackend {
        fn kind(&self) -> BackendKind {
            BackendKind::Apt
        }

        fn update(&self) -> Result<()> {
            Ok(())
        }

        fn upgrade(&self) -> Result<()> {
            Ok(())
        }

        fn is_installed(&self, real_pkg_name: &str) -> Result<bool> {
            Ok(self.installed.iter().any(|name| name == real_pkg_name))
        }

        fn install(&self, real_pkg_names: &[String]) -> Result<()> {
            assert_eq!(real_pkg_names, self.expected);
            Ok(())
        }

        fn list_manually_installed(&self) -> Result<Vec<String>> {
            Ok(Vec::new())
        }
    }

    #[test]
    fn installs_only_missing_manifest_packages() {
        let mut manifest = Manifest::default();
        manifest.record("git", "git", BackendKind::Apt);
        manifest.record("curl", "curl", BackendKind::Apt);
        let backend = FakeBackend {
            installed: vec!["git".to_string()],
            expected: vec!["curl".to_string()],
        };

        run(&manifest, &backend).unwrap();
    }
}
