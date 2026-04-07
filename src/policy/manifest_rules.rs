use anyhow::Result;

use crate::core::models::TestManifest;

/// Validate that every test in the manifest references a real feature and
/// outcome that exist in the provided sets.
#[allow(dead_code)]
pub fn validate_manifest_references(
    manifest: &TestManifest,
    known_feature_ids: &[String],
    known_outcome_ids: &[(String, String)], // (feature_id, outcome_id)
) -> Result<()> {
    for test in &manifest.tests {
        if !known_feature_ids.contains(&test.feature_id) {
            anyhow::bail!(
                "test '{}' references unknown feature '{}'",
                test.id,
                test.feature_id
            );
        }

        let pair = (test.feature_id.clone(), test.outcome_id.clone());
        if !known_outcome_ids.contains(&pair) {
            anyhow::bail!(
                "test '{}' references unknown outcome '{}' for feature '{}'",
                test.id,
                test.outcome_id,
                test.feature_id
            );
        }
    }

    // Check for duplicate test IDs
    let mut ids: Vec<&str> = manifest.tests.iter().map(|t| t.id.as_str()).collect();
    ids.sort_unstable();
    let orig_len = ids.len();
    ids.dedup();
    if ids.len() != orig_len {
        anyhow::bail!("manifest contains duplicate test IDs");
    }

    Ok(())
}
