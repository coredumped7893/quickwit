// Copyright (C) 2024 Quickwit, Inc.
//
// Quickwit is offered under the AGPL v3.0 and as commercial software.
// For commercial licensing, contact us at hello@quickwit.io.
//
// AGPL:
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use itertools::Itertools;
use quickwit_common::uri::Uri;
use quickwit_config::{IndexTemplate, IndexTemplateId, TestableForRegression};
use quickwit_proto::metastore::{serde_utils, MetastoreError, MetastoreResult};
use quickwit_proto::types::IndexId;
use quickwit_storage::{OwnedBytes, Storage, StorageError, StorageErrorKind, StorageResult};
use serde::{Deserialize, Serialize};
use tracing::error;

pub(super) const MANIFEST_FILE_NAME: &str = "manifest.json";

// The legacy manifest file was deprecated in 0.8.0, we can drop support for it in 0.10.0 or 0.11.0.
const LEGACY_MANIFEST_FILE_NAME: &str = "indexes_states.json";

#[derive(Clone, Debug, Deserialize)]
struct LegacyManifest {
    #[serde(default, flatten)]
    indexes: BTreeMap<IndexId, IndexStatus>,
}

impl LegacyManifest {
    fn into_manifest(self) -> Manifest {
        Manifest {
            indexes: self.indexes,
            templates: HashMap::new(),
        }
    }
}

// TODO: Remove the aliases once we drop support for the legacy manifest file.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IndexStatus {
    #[serde(alias = "Creating")]
    Creating,
    #[serde(alias = "Alive")]
    Active,
    #[serde(alias = "Deleting")]
    Deleting,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(into = "VersionedManifest")]
#[serde(from = "VersionedManifest")]
pub(crate) struct Manifest {
    pub indexes: BTreeMap<IndexId, IndexStatus>,
    // The templates are serialized as a sorted `Vec<IndexTemplate>` so the btree map is
    // unnecessary here and we can pass the hash map as is to the `MetastoreState`
    pub templates: HashMap<IndexTemplateId, IndexTemplate>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "version")]
enum VersionedManifest {
    #[serde(rename = "0.7")]
    V0_7(ManifestV0_7),
}

impl From<Manifest> for VersionedManifest {
    fn from(manifest: Manifest) -> Self {
        VersionedManifest::V0_7(manifest.into())
    }
}

impl From<VersionedManifest> for Manifest {
    fn from(versioned_manifest: VersionedManifest) -> Self {
        match versioned_manifest {
            VersionedManifest::V0_7(manifest) => manifest.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ManifestV0_7 {
    indexes: BTreeMap<IndexId, IndexStatus>,
    templates: Vec<IndexTemplate>,
}

impl From<Manifest> for ManifestV0_7 {
    fn from(manifest: Manifest) -> Self {
        let templates = manifest
            .templates
            .into_values()
            .sorted_unstable_by(|left, right| left.template_id.cmp(&right.template_id))
            .collect();
        ManifestV0_7 {
            indexes: manifest.indexes,
            templates,
        }
    }
}

impl From<ManifestV0_7> for Manifest {
    fn from(manifest: ManifestV0_7) -> Self {
        let indexes = manifest.indexes.into_iter().collect();
        let templates = manifest
            .templates
            .into_iter()
            .map(|template| (template.template_id.clone(), template))
            .collect();
        Manifest { indexes, templates }
    }
}

impl TestableForRegression for Manifest {
    fn sample_for_regression() -> Self {
        let mut indexes = BTreeMap::new();
        indexes.insert("test-index-1".to_string(), IndexStatus::Creating);
        indexes.insert("test-index-2".to_string(), IndexStatus::Active);
        indexes.insert("test-index-3".to_string(), IndexStatus::Deleting);

        let mut templates = HashMap::new();
        templates.insert(
            "test-template-1".to_string(),
            IndexTemplate::sample_for_regression(),
        );
        Manifest { indexes, templates }
    }

    fn assert_equality(&self, other: &Self) {
        assert_eq!(self.indexes, other.indexes);
        assert_eq!(self.templates, other.templates);
    }
}

pub(super) async fn load_or_create_manifest(storage: &dyn Storage) -> MetastoreResult<Manifest> {
    if file_exists(storage, MANIFEST_FILE_NAME).await? {
        let manifest_json = get_bytes(storage, MANIFEST_FILE_NAME).await?;
        let manifest: Manifest = serde_utils::from_json_bytes(&manifest_json)?;
        return Ok(manifest);
    }
    if file_exists(storage, LEGACY_MANIFEST_FILE_NAME).await? {
        let legacy_manifest_json = get_bytes(storage, LEGACY_MANIFEST_FILE_NAME).await?;
        let legacy_manifest: LegacyManifest = serde_utils::from_json_bytes(&legacy_manifest_json)?;
        let manifest = legacy_manifest.into_manifest();
        save_manifest(storage, &manifest).await?;

        if let Err(storage_error) = delete_file(storage, LEGACY_MANIFEST_FILE_NAME).await {
            error!(
                error=%storage_error,
                "failed to delete legacy manifest file located at `{}/{LEGACY_MANIFEST_FILE_NAME}`", storage.uri()
            );
        }
        return Ok(manifest);
    }
    let manifest = Manifest::default();
    save_manifest(storage, &manifest).await?;
    Ok(manifest)
}

pub(super) async fn save_manifest(
    storage: &dyn Storage,
    manifest: &Manifest,
) -> MetastoreResult<()> {
    let manifest_json_bytes = serde_utils::to_json_bytes_pretty(manifest)?;
    put_bytes(storage, MANIFEST_FILE_NAME, manifest_json_bytes).await?;
    Ok(())
}

async fn delete_file(storage: &dyn Storage, path: &str) -> StorageResult<()> {
    storage.delete(Path::new(path)).await?;
    Ok(())
}

async fn file_exists(storage: &dyn Storage, path_str: &str) -> MetastoreResult<bool> {
    let path = Path::new(path_str);
    let exists = storage.exists(path).await.map_err(|storage_error| {
        into_metastore_error(storage_error, storage.uri(), path, "list")
    })?;
    Ok(exists)
}

async fn get_bytes(storage: &dyn Storage, path_str: &str) -> MetastoreResult<OwnedBytes> {
    let path = Path::new(path_str);
    let bytes = storage.get_all(path).await.map_err(|storage_error| {
        into_metastore_error(storage_error, storage.uri(), path, "load")
    })?;
    Ok(bytes)
}

async fn put_bytes(storage: &dyn Storage, path_str: &str, content: Vec<u8>) -> MetastoreResult<()> {
    let path = Path::new(path_str);
    storage
        .put(path, Box::new(content))
        .await
        .map_err(|storage_error| {
            into_metastore_error(storage_error, storage.uri(), path, "save")
        })?;
    Ok(())
}

fn into_metastore_error(
    storage_error: StorageError,
    uri: &Uri,
    path: &Path,
    operation_name: &str,
) -> MetastoreError {
    match storage_error.kind() {
        StorageErrorKind::Unauthorized => MetastoreError::Forbidden {
            message: format!(
                "failed to access manifest file located at `{uri}/{}`: unauthorized",
                path.display()
            ),
        },
        _ => MetastoreError::Internal {
            message: format!(
                "failed to {operation_name} manifest file located at `{uri}/{}`",
                path.display()
            ),
            cause: storage_error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_legacy_manifest_deserialization() {
        let legacy_manifest_json = r#"{
            "test-index-1": "Creating",
            "test-index-2": "Alive",
            "test-index-3": "Deleting"
        }
        "#;
        let legacy_manifest: LegacyManifest = serde_json::from_str(legacy_manifest_json).unwrap();
        assert_eq!(legacy_manifest.indexes.len(), 3);

        assert_eq!(
            legacy_manifest.indexes.get("test-index-1").unwrap(),
            &IndexStatus::Creating
        );
        assert_eq!(
            legacy_manifest.indexes.get("test-index-2").unwrap(),
            &IndexStatus::Active
        );
        assert_eq!(
            legacy_manifest.indexes.get("test-index-3").unwrap(),
            &IndexStatus::Deleting
        );
    }

    #[test]
    fn test_legacy_manifest_into_manifest() {
        let legacy_manifest = LegacyManifest {
            indexes: vec![
                ("test-index-1".to_string(), IndexStatus::Creating),
                ("test-index-2".to_string(), IndexStatus::Active),
                ("test-index-3".to_string(), IndexStatus::Deleting),
            ]
            .into_iter()
            .collect(),
        };
        let manifest = legacy_manifest.into_manifest();

        assert_eq!(manifest.indexes.len(), 3);
        assert_eq!(manifest.templates.len(), 0);

        assert_eq!(
            manifest.indexes.get("test-index-1").unwrap(),
            &IndexStatus::Creating
        );
        assert_eq!(
            manifest.indexes.get("test-index-2").unwrap(),
            &IndexStatus::Active
        );
        assert_eq!(
            manifest.indexes.get("test-index-3").unwrap(),
            &IndexStatus::Deleting
        );
    }

    #[test]
    fn test_manifest_serde() {
        let indexes = BTreeMap::from_iter([
            ("test-index-1".to_string(), IndexStatus::Creating),
            ("test-index-2".to_string(), IndexStatus::Active),
            ("test-index-3".to_string(), IndexStatus::Deleting),
        ]);
        let templates = HashMap::from_iter([
            (
                "test-template-1".to_string(),
                IndexTemplate::for_test("test-template-1", &["test-index-foo*"], 100),
            ),
            (
                "test-template-2".to_string(),
                IndexTemplate::for_test("test-template-2", &["test-index-bar*"], 200),
            ),
        ]);
        let manifest = Manifest { indexes, templates };
        let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
        let manifest_deserialized: Manifest = serde_json::from_str(&manifest_json).unwrap();
        assert_eq!(manifest, manifest_deserialized);
    }

    #[tokio::test]
    async fn test_create_mutate_save_load_manifest() {
        let storage = quickwit_storage::storage_for_test();
        let mut manifest = load_or_create_manifest(&*storage).await.unwrap();

        assert_eq!(manifest.indexes.len(), 0);
        assert_eq!(manifest.templates.len(), 0);

        let empty_manifest_size = storage
            .get_all(Path::new(MANIFEST_FILE_NAME))
            .await
            .unwrap()
            .len();
        assert!(empty_manifest_size > 0);

        manifest
            .indexes
            .insert("test-index".to_string(), IndexStatus::Creating);
        manifest.templates.insert(
            "test-template".to_string(),
            IndexTemplate::for_test("test-template", &["test-index-*"], 100),
        );

        save_manifest(&*storage, &manifest).await.unwrap();

        let populated_manifest_size = storage
            .get_all(Path::new(MANIFEST_FILE_NAME))
            .await
            .unwrap()
            .len();
        assert!(populated_manifest_size > empty_manifest_size);

        let manifest = load_or_create_manifest(&*storage).await.unwrap();
        assert_eq!(manifest.indexes.len(), 1);
        assert_eq!(
            manifest.indexes.get("test-index").unwrap(),
            &IndexStatus::Creating
        );

        assert_eq!(manifest.templates.len(), 1);

        let template = manifest.templates.get("test-template").unwrap();
        assert_eq!(template.template_id, "test-template");
        assert_eq!(template.index_id_patterns, ["test-index-*"]);
        assert_eq!(template.priority, 100);
    }

    #[tokio::test]
    async fn test_legacy_manifest_migration() {
        let storage = quickwit_storage::storage_for_test();
        let legacy_manifest_json = json!(
            {
                "test-index-1": "Creating",
                "test-index-2": "Alive",
                "test-index-3": "Deleting"
            }
        );
        let legacy_manifest_json_bytes = serde_json::to_vec(&legacy_manifest_json).unwrap();

        put_bytes(
            &*storage,
            LEGACY_MANIFEST_FILE_NAME,
            legacy_manifest_json_bytes,
        )
        .await
        .unwrap();

        let manifest = load_or_create_manifest(&*storage).await.unwrap();
        assert_eq!(manifest.indexes.len(), 3);
        assert_eq!(manifest.templates.len(), 0);

        assert_eq!(
            manifest.indexes.get("test-index-1").unwrap(),
            &IndexStatus::Creating
        );
        assert_eq!(
            manifest.indexes.get("test-index-2").unwrap(),
            &IndexStatus::Active
        );
        assert_eq!(
            manifest.indexes.get("test-index-3").unwrap(),
            &IndexStatus::Deleting
        );

        let legacy_manifest_exists = file_exists(&*storage, LEGACY_MANIFEST_FILE_NAME)
            .await
            .unwrap();
        assert!(!legacy_manifest_exists);

        let manifest_exists = file_exists(&*storage, MANIFEST_FILE_NAME).await.unwrap();
        assert!(manifest_exists);
    }
}
