use std::fs::File;
use std::io::Write;
use std::path::Path;

use directories::ProjectDirs;
use sha2::{Digest, Sha256};
use sled::{Db, IVec};
use std::time::{SystemTime, UNIX_EPOCH};
use zil_errors::LocalStorageError;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Data<ST> {
    pub payload: ST,
    // Storage verions
    pub version: u16,
    // last update for sync with server
    pub last_update: u64,
    // hash sum for compare with server
    pub hashsum: String,
}

pub struct LocalStorage {
    tree: Db,
    version: u16,
}

impl LocalStorage {
    pub const VERSION: u16 = 0;

    pub fn from<P: AsRef<Path>>(path: P) -> Result<Self, LocalStorageError> {
        let tree =
            sled::open(path).map_err(|e| LocalStorageError::StorageAccessError(e.to_string()))?;
        let version = Self::VERSION;

        Ok(LocalStorage { tree, version })
    }

    pub fn new(
        qualifier: &str,
        organization: &str,
        application: &str,
    ) -> Result<Self, LocalStorageError> {
        let path = ProjectDirs::from(qualifier, organization, application)
            .ok_or(LocalStorageError::StoragePathError)?;
        let tree = sled::open(path.data_dir())
            .map_err(|e| LocalStorageError::StorageAccessError(e.to_string()))?;
        let version = Self::VERSION;

        Ok(LocalStorage { tree, version })
    }

    pub fn save_as_file(&self, path: &Path) -> Result<(), LocalStorageError> {
        let export = self.tree.export();

        for (_, _, collection_iter) in export {
            for mut kv in collection_iter {
                let bytes = kv.pop().ok_or(LocalStorageError::FailToloadBytesTree)?;
                let mut file = File::create(path).or(Err(LocalStorageError::FailToCreateFile))?;

                file.write_all(&bytes)
                    .or(Err(LocalStorageError::FailToWriteFile))?;
            }
        }

        Ok(())
    }

    pub fn get_db_size(&self) -> u64 {
        self.tree.size_on_disk().unwrap_or(0)
    }

    pub fn get<ST>(&self, key: &str) -> Result<ST, LocalStorageError>
    where
        ST: for<'a> Deserialize<'a> + Serialize,
    {
        let some_value = self
            .tree
            .get(key)
            .map_err(|e| LocalStorageError::StorageAccessError(e.to_string()))?;
        let value = some_value.ok_or(LocalStorageError::StorageDataNotFound)?;
        let json = String::from_utf8_lossy(&value);

        let data: Data<ST> =
            serde_json::from_str(&json).or(Err(LocalStorageError::StorageDataBroken))?;
        let json_payload =
            serde_json::to_string(&data.payload).or(Err(LocalStorageError::StorageDataBroken))?;
        let hashsum = self.hash(json_payload.as_bytes());

        if hashsum != data.hashsum {
            return Err(LocalStorageError::StorageHashsumError);
        }

        Ok(data.payload)
    }

    pub fn set<ST>(&self, key: &str, payload: ST) -> Result<(), LocalStorageError>
    where
        ST: Serialize,
    {
        let last_update = self.get_unix_time()?;
        let json_payload =
            serde_json::to_string(&payload).or(Err(LocalStorageError::StorageDataBroken))?;
        let hashsum = self.hash(json_payload.as_bytes());
        let data = Data {
            payload,
            hashsum,
            last_update,
            version: self.version,
        };
        let json = serde_json::to_string(&data).or(Err(LocalStorageError::StorageDataBroken))?;
        let vec = IVec::from(json.as_bytes());

        self.tree
            .insert(key, vec)
            .or(Err(LocalStorageError::StorageWriteError))?;

        Ok(())
    }

    fn hash(&self, bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hashsum = hasher.finalize();

        hex::encode(hashsum)
    }

    fn get_unix_time(&self) -> Result<u64, LocalStorageError> {
        let now = SystemTime::now();
        let since_epoch = now
            .duration_since(UNIX_EPOCH)
            .or(Err(LocalStorageError::StorageTimeWentBackwards))?;
        let u64_time = since_epoch.as_secs();

        Ok(u64_time)
    }
}

#[cfg(test)]
mod storage_tests {
    use super::*;

    #[test]
    fn test_read_write() {
        const KEY: &str = "TEST_KEY_FOR_STORAGE";

        let db = LocalStorage::new("com.test_write", "WriteTest Corp", "WriteTest App").unwrap();
        let payload = vec!["test1", "test2", "test3"];

        db.set(KEY, &payload).unwrap();

        let out = db.get::<Vec<String>>(KEY).unwrap();

        assert_eq!(out, payload);
    }
}
