//! What the server holds: the open lines, the site, the profiles, the write log, the running
//! search, the recording. Loaded from the data folder at the start and saved on every change.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use super::ApiError;
use crate::access::Limits;
use crate::capture::Capture;
use crate::endpoint::Target;
use crate::links::Links;
use crate::profile::{self, Profile};
use crate::record::Recording;
use crate::scan::Search;
use crate::site::{Site, SiteDevice};
use crate::storage;
use crate::write::Journal;

pub type Shared = Arc<AppState>;

#[derive(Default)]
pub struct AppState {
    pub links: Arc<Links>,
    pub capture: Arc<Capture>,
    /// The last capture stopped, for the download.
    pub captured: Mutex<Option<Vec<crate::capture::Frame>>>,
    /// Where files are kept; none means memory only (tests).
    pub data_dir: Option<PathBuf>,
    /// The loopback interface counts as an adapter (browser tests).
    pub loopback: bool,
    /// Where the web server listens; requests must name it (guard.rs).
    pub web_address: Option<SocketAddr>,
    /// The running or last search.
    pub search: Mutex<Option<Arc<Search>>>,
    pub site: Mutex<Site>,
    pub profiles: Mutex<Vec<Profile>>,
    pub journal: Mutex<Journal>,
    /// Writing is allowed in this session (concept section 7).
    pub unlocked: AtomicBool,
    pub recording: Mutex<Recording>,
    /// The bus analyser, running or the last one ended.
    pub analyser: Mutex<Option<super::analysing::Listening>>,
    /// What listening to a serial port heard last, by port.
    pub listened: Mutex<HashMap<String, crate::listen::Listened>>,
    /// Problems with files at the start, shown once in the interface.
    pub load_problems: Mutex<Vec<String>>,
}

/// What survives a restart besides sites and profiles: which site was open.
#[derive(Default, Serialize, Deserialize)]
struct Settings {
    site: Option<String>,
}

impl AppState {
    pub fn new(data_dir: Option<PathBuf>) -> anyhow::Result<AppState> {
        let capture = Arc::new(Capture::default());
        let state = AppState {
            links: Arc::new(Links::new(capture.clone())),
            capture,
            site: Mutex::new(new_site("Site", &[])),
            data_dir,
            ..AppState::default()
        };
        if let Some(dir) = state.data_dir.clone() {
            state.load(&dir)?;
        }
        Ok(state)
    }

    fn load(&self, dir: &Path) -> anyhow::Result<()> {
        let mut problems = Vec::new();
        let (entries, skipped) = storage::load_entries(&dir.join(storage::JOURNAL))
            .context("cannot read the write log")?;
        if skipped > 0 {
            problems.push(format!(
                "{skipped} lines of the write log could not be read"
            ));
        }
        self.journal.lock().expect("journal lock").entries = entries;
        let (profiles, broken) = storage::load_all::<Profile>(&dir.join(storage::PROFILES));
        problems.extend(broken);
        *self.profiles.lock().expect("profiles lock") = profiles;
        let (sites, broken) = storage::load_all::<Site>(&dir.join(storage::SITES));
        problems.extend(broken);
        let settings: Settings = storage::load_json(&dir.join(storage::SETTINGS))
            .ok()
            .flatten()
            .unwrap_or_default();
        let open = settings
            .site
            .and_then(|id| sites.iter().find(|site| site.id == id).cloned())
            .or_else(|| sites.first().cloned());
        if let Some(site) = open {
            *self.site.lock().expect("site lock") = site;
        }
        if let Ok(Some(recording)) = storage::load_json::<Recording>(&dir.join(storage::RECORDING))
        {
            *self.recording.lock().expect("recording lock") = recording.stopped();
        }
        *self.load_problems.lock().expect("problems lock") = problems;
        Ok(())
    }

    pub fn unlocked(&self) -> bool {
        self.unlocked.load(Ordering::SeqCst)
    }

    pub fn journal_path(&self) -> Option<PathBuf> {
        self.data_dir.as_ref().map(|dir| dir.join(storage::JOURNAL))
    }

    /// The write log, with what another modbusio wrote into the same data folder meanwhile.
    pub fn journal(&self) -> MutexGuard<'_, Journal> {
        crate::write::up_to_date(&self.journal, self.journal_path().as_deref())
    }

    /// Saves the open site and notes it as the one to open next time.
    pub fn save_site(&self) -> Result<(), ApiError> {
        let Some(dir) = &self.data_dir else {
            return Ok(());
        };
        let site = self.site.lock().expect("site lock").clone();
        storage::save_json(
            &dir.join(storage::SITES).join(format!("{}.json", site.id)),
            &site,
        )
        .and_then(|()| {
            storage::save_json(
                &dir.join(storage::SETTINGS),
                &Settings {
                    site: Some(site.id.clone()),
                },
            )
        })
        .map_err(|error| ApiError::internal(format!("cannot save the site: {error}")))
    }

    pub fn save_profile(&self, profile: &Profile) -> Result<(), ApiError> {
        let Some(dir) = &self.data_dir else {
            return Ok(());
        };
        storage::save_json(
            &dir.join(storage::PROFILES)
                .join(format!("{}.json", profile.id)),
            profile,
        )
        .map_err(|error| ApiError::internal(format!("cannot save the profile: {error}")))
    }

    pub fn remove_profile_file(&self, id: &str) {
        if let Some(dir) = &self.data_dir {
            let _ = std::fs::remove_file(dir.join(storage::PROFILES).join(format!("{id}.json")));
        }
    }

    /// Every site in the data folder, with the open one as it is now.
    pub fn sites(&self) -> Vec<Site> {
        let open = self.site.lock().expect("site lock").clone();
        let mut sites: Vec<Site> = match &self.data_dir {
            Some(dir) => storage::load_all::<Site>(&dir.join(storage::SITES)).0,
            None => Vec::new(),
        };
        sites.retain(|site| site.id != open.id);
        sites.push(open);
        sites.sort_by_key(|site| site.name.to_lowercase());
        sites
    }

    pub fn remove_site_file(&self, id: &str) {
        if let Some(dir) = &self.data_dir {
            let _ = std::fs::remove_file(dir.join(storage::SITES).join(format!("{id}.json")));
        }
    }

    /// The device with this key on the open site, and its target.
    pub fn device(&self, key: &str) -> Result<(SiteDevice, Target), ApiError> {
        let target = Target::parse_key(key).map_err(ApiError::bad_request)?;
        let device = self
            .site
            .lock()
            .expect("site lock")
            .device(key)
            .cloned()
            .ok_or_else(|| ApiError::not_found(format!("no device {key} on this site")))?;
        Ok((device, target))
    }

    pub fn profile(&self, id: &str) -> Option<Profile> {
        self.profiles
            .lock()
            .expect("profiles lock")
            .iter()
            .find(|profile| profile.id == id)
            .cloned()
    }

    /// A profile id not taken yet, from a name.
    pub fn free_profile_id(&self, name: &str) -> String {
        let base = profile::slug(name);
        let profiles = self.profiles.lock().expect("profiles lock");
        let taken = |id: &str| profiles.iter().any(|profile| profile.id == id);
        if !taken(&base) {
            return base;
        }
        (2..)
            .map(|number| format!("{base}-{number}"))
            .find(|id| !taken(id))
            .expect("some number is free")
    }

    /// Keeps the limits a read, a look or a write learnt about a device.
    pub fn keep_limits(&self, key: &str, before: Limits, learnt: Limits) -> Result<(), ApiError> {
        if learnt == before {
            return Ok(());
        }
        self.keep_learnt(key, |device| device.limits = learnt)
    }

    /// Keeps what a request learnt about a device (limits, identity). A device taken off the
    /// site meanwhile keeps nothing: what was read is still answered.
    pub fn keep_learnt(
        &self,
        key: &str,
        change: impl FnOnce(&mut SiteDevice),
    ) -> Result<(), ApiError> {
        {
            let mut site = self.site.lock().expect("site lock");
            let Some(device) = site.device_mut(key) else {
                return Ok(());
            };
            change(device);
        }
        self.save_site()
    }

    /// Changes a device as the user asked and saves the site.
    pub fn update_device(
        &self,
        key: &str,
        change: impl FnOnce(&mut SiteDevice),
    ) -> Result<(), ApiError> {
        {
            let mut site = self.site.lock().expect("site lock");
            let Some(device) = site.device_mut(key) else {
                return Err(ApiError::not_found(format!("no device {key} on this site")));
            };
            change(device);
        }
        self.save_site()
    }
}

/// A new site with an id not used by `taken`.
pub fn new_site(name: &str, taken: &[Site]) -> Site {
    let base = profile::slug(name);
    let id = std::iter::once(base.clone())
        .chain((2..).map(|number| format!("{base}-{number}")))
        .find(|id| !taken.iter().any(|site| &site.id == id))
        .expect("some number is free");
    Site {
        id,
        name: name.into(),
        devices: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_are_kept_for_a_device_on_the_site_and_dropped_for_one_taken_off() {
        let state = AppState::new(None).expect("state");
        let key = "tcp:10.0.0.5:502/1";
        let on_site = || state.device(key).ok().map(|(device, _)| device.limits);
        let learnt = Limits {
            most_registers: 10,
            ..Limits::default()
        };
        state
            .site
            .lock()
            .expect("site lock")
            .add(SiteDevice::new(key.into()));
        assert!(state.keep_limits(key, Limits::default(), learnt).is_ok());
        assert_eq!(on_site(), Some(learnt));
        // taken off the site while it was read: what the read learnt has no place, and is no error
        state.site.lock().expect("site lock").devices.clear();
        assert!(state.keep_limits(key, learnt, Limits::default()).is_ok());
        let identified = |device: &mut SiteDevice| device.vendor = Some("Acme".into());
        assert!(state.keep_learnt(key, identified).is_ok());
        assert_eq!(on_site(), None);
    }
}
