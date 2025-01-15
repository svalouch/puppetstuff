// SPDX-License-Identifier: GPL-3.0-only

use attohttpc::Session;
use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::{debug, instrument, warn};

#[derive(Debug, Deserialize)]
struct ForgeResponse {
    current_release: ForgeCurrentRelease,
    deprecated_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Deserialize)]
struct ForgeCurrentRelease {
    version: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CacheEntry {
    pub version: String,
    pub is_deprecated: bool,
    pub time_fetched: u64,
}

#[derive(Debug)]
pub struct ForgeApi {
    session: Session,
    cache: HashMap<String, CacheEntry>,
}

/// Wrapper around the Forge-API with a crude cache implementation
impl ForgeApi {
    pub fn new(cache_file: Option<String>) -> Self {
        let mut session = Session::new();
        session.follow_redirects(false);
        Self {
            session,
            cache: match cache_file {
                Some(f) => Self::load_cache(f),
                None => HashMap::new(),
            },
        }
    }

    fn load_cache<P: AsRef<Path> + std::fmt::Debug>(cache_file: P) -> HashMap<String, CacheEntry> {
        debug!("Loading cache from {cache_file:?}");
        let data = match std::fs::read_to_string(&cache_file) {
            Ok(d) => d,
            Err(_) => {
                warn!("No cache found or not readable");
                return HashMap::new();
            }
        };
        match serde_json::from_str(&data) {
            Ok(d) => d,
            Err(e) => {
                warn!("Cache parsing failed: {e}");
                return HashMap::new();
            }
        }
    }
    pub fn store_cache<P: AsRef<Path> + std::fmt::Debug>(&self, cache_file: P) {
        debug!("Writing cache to {cache_file:?}");
        std::fs::write(cache_file, serde_json::to_string(&self.cache).unwrap()).unwrap();
    }

    #[instrument(skip(self))]
    pub fn get_version(&mut self, name: &str) -> Result<Version, String> {
        self.get_data(name)?;
        Ok(Version::parse(&self.cache.get(name).unwrap().version).unwrap())
    }

    #[instrument(skip(self))]
    pub fn is_deprecated(&mut self, name: &str) -> Result<bool, String> {
        self.get_data(name)?;
        Ok(self.cache.get(name).unwrap().is_deprecated)
    }

    fn get_data(&mut self, name: &str) -> Result<(), String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if let Some(e) = self.cache.get(name) {
            if e.time_fetched < now - 60 * 60 {
                debug!("Value in cache and outdated");
                let (version, deprecated_at) = self.fetch_data(name)?;
                let e = self.cache.get_mut(name).unwrap();
                e.version = version.to_string();
                e.is_deprecated = deprecated_at.is_some();
                e.time_fetched = now;
            } else {
                debug!("Value in cache");
            }
        } else {
            debug!("Value not in cache");
            let (version, deprecated_at) = self.fetch_data(name)?;
            self.cache.insert(
                name.to_owned(),
                CacheEntry {
                    version: version.to_string(),
                    is_deprecated: deprecated_at.is_some(),
                    time_fetched: now,
                },
            );
        }
        Ok(())
    }

    fn fetch_data(&self, name: &str) -> Result<(Version, Option<DateTime<Utc>>), String> {
        let name = name.replace("/", "-");
        let url = &format!("https://forgeapi.puppet.com/v3/modules/{}?exclude_fields=readme,changelog,license,reference,tasks,plans,metadata,tags", name);
        debug!("Fetching {url}");

        let res: ForgeResponse = self
            .session
            .get(url)
            .send()
            .map_err(|e| format!("Failure in communication with forge: {e}"))?
            .json()
            .map_err(|_| "Failed to parse forge json")?;
        let version = Version::parse(&res.current_release.version)
            .map_err(|e| format!("Returned version is not semver-compatible: {e}"))?;
        Ok((version, res.deprecated_at))
    }
}
