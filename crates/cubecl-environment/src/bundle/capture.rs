//! The active environment as one flat bundle, in memory.
//!
//! [`export`](super::export) reads environment files and writes a file, so
//! it needs a file system at both ends. A browser has neither: its
//! environment lives in the origin's private file system, reachable only
//! through the storage. This is the export it can run — what a visit tuned,
//! captured as the bytes another device seeds with through
//! [`import`](super::import) — and it reads through the storage on every
//! target, so a capture holds the same entries wherever it is taken.

use alloc::string::String;
use alloc::vec::Vec;

use super::flat;
use super::{BundleError, EnvironmentInfo};
use crate::bytes::Bytes;
use crate::persistence::{self, storage};

/// What [`capture`] describes the bundle as, and which namespaces it takes.
#[derive(Debug, Clone, Default)]
pub struct CaptureOptions {
    /// Human-chosen bundle name, e.g. "Intel Xe-LPG in Chromium".
    pub name: String,
    /// The environments the bundle was captured on. `os` and `arch` are
    /// auto-filled from the build target when left empty.
    pub environments: Vec<EnvironmentInfo>,
    /// Only capture namespaces under one of these prefixes, e.g. `autotune`.
    /// A prefix matches whole segments, so it selects the namespace itself
    /// and everything below it. No prefix, or an empty one, means every
    /// namespace.
    pub namespaces: Vec<String>,
}

/// The active environment's entries as one flat bundle.
///
/// Every namespace the storage holds is read in full, so the bytes are the
/// whole capture: restrict the namespaces when the compiled kernels are not
/// wanted, which is the usual case for what a browser publishes.
pub async fn capture(options: &CaptureOptions) -> Result<Bytes, BundleError> {
    let manifest = super::BundleManifest::stamped(&options.name, &options.environments);
    let mut entries = flat::Entries::new();

    for namespace in persistence::namespaces().await {
        if !selected(&namespace, &options.namespaces) {
            continue;
        }
        let storage = storage::open(&namespace).await;
        for (key, value) in storage.scan().await {
            entries.insert((namespace.clone(), key.to_vec()), value);
        }
    }

    flat::encode(&entries, &manifest)
}

/// Whether `namespace` is under one of `prefixes`, on whole segments, or
/// under no restriction at all.
fn selected(namespace: &str, prefixes: &[String]) -> bool {
    let unrestricted = prefixes.is_empty() || prefixes.iter().any(String::is_empty);
    unrestricted
        || prefixes.iter().any(|prefix| {
            namespace == prefix
                || namespace
                    .strip_prefix(prefix.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prefix_selects_whole_segments() {
        let prefixes = [String::from("autotune")];
        assert!(selected("autotune", &prefixes));
        assert!(selected("autotune/0.11/cuda", &prefixes));
        assert!(!selected("autotunes/0.11/cuda", &prefixes));
        assert!(!selected("cuda/0.11/ptx", &prefixes));
    }

    #[test]
    fn no_prefix_selects_everything() {
        assert!(selected("cuda/0.11/ptx", &[]));
        assert!(selected("cuda/0.11/ptx", &[String::new()]));
    }
}
