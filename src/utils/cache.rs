use std::future::Future;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};

use crate::utils::config::Config;

#[derive(Debug, Clone)]
pub struct Cache {
    root: PathBuf,
}

impl Default for Cache {
    fn default() -> Self {
        Self {
            root: PathBuf::from(".cache"),
        }
    }
}

impl Cache {
    pub fn configured() -> Self {
        let config = Config::load().unwrap_or_default();
        Self {
            root: config.cache_dir(),
        }
    }

    #[cfg(test)]
    pub fn with_root(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn key(
        &self,
        namespace: &str,
        paths: &[&Path],
        body: &str,
        provider: &str,
    ) -> Result<String> {
        let mut hasher = Sha256::new();
        hasher.update(namespace.as_bytes());
        // Include the provider so Ollama and OpenAI responses are never mixed in cache.
        hasher.update(provider.as_bytes());
        for path in paths {
            hasher.update(path.to_string_lossy().as_bytes());
            if let Ok(bytes) = std::fs::read(path) {
                hasher.update(bytes);
            }
        }
        hasher.update(body.as_bytes());
        Ok(format!("{}-{:x}", namespace, hasher.finalize()))
    }

    pub async fn get_or_insert_json<T, F, Fut>(&self, key: &str, make: F) -> Result<T>
    where
        T: DeserializeOwned + Serialize,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        std::fs::create_dir_all(&self.root)?;
        let path = self.root.join(format!("{key}.json"));
        if path.exists() {
            let text = std::fs::read_to_string(path)?;
            return Ok(serde_json::from_str(&text)?);
        }

        let value = make().await?;
        std::fs::write(path, serde_json::to_vec_pretty(&value)?)?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct CachedValue {
        value: String,
    }

    #[tokio::test]
    async fn caches_json_value_after_first_insert() {
        let root = std::env::temp_dir().join(format!("hirelens-cache-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let cache = Cache::with_root(root.clone());
        let calls = Arc::new(AtomicUsize::new(0));

        let first: CachedValue = cache
            .get_or_insert_json("sample", || {
                let calls = Arc::clone(&calls);
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(CachedValue {
                        value: "fresh".into(),
                    })
                }
            })
            .await
            .expect("first cache write should work");

        let second: CachedValue = cache
            .get_or_insert_json("sample", || {
                let calls = Arc::clone(&calls);
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(CachedValue {
                        value: "stale".into(),
                    })
                }
            })
            .await
            .expect("second cache read should work");

        assert_eq!(first, second);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let _ = std::fs::remove_dir_all(root);
    }
}
