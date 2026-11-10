//! Writing to a device (concept section 7). Modbus has no priorities and no release, so every
//! write reads the old value first and logs it, reads back afterwards, and can be undone by
//! writing the old value again. *Changed by you* is derived from the log, not kept apart.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::access::{self, Limits};
use crate::endpoint::Target;
use crate::link::LinkError;
use crate::links::Links;
use crate::protocol::Area;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// Unique and rising from 1: the number the user names to put a write back.
    pub id: u64,
    /// Seconds since 1970-01-01 UTC.
    pub time: u64,
    /// The device's key (`tcp:10.0.0.5:502/1`).
    pub device: String,
    /// The point's name, where the write came from a profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub point: Option<String>,
    pub area: Area,
    pub address: u16,
    /// What the device held before, as read; `None` when the write was refused before sending.
    pub before: Option<Vec<u16>>,
    pub written: Vec<u16>,
    /// What the device held after, as read back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<Vec<u16>>,
    /// The new value in words (`25 Hz`), and the old one where known.
    #[serde(default)]
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_text: Option<String>,
    /// This write put back the old value of that entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restores: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Entry {
    pub fn succeeded(&self) -> bool {
        self.error.is_none()
    }

    /// The device confirmed but holds another value than written.
    pub fn kept_other(&self) -> bool {
        self.succeeded()
            && self
                .after
                .as_ref()
                .is_some_and(|after| *after != self.written)
    }

    fn place(&self) -> (String, Area, u16) {
        (self.device.clone(), self.area, self.address)
    }
}

/// Why nothing is written to unit 0.
pub const BROADCAST: &str =
    "unit 0 is the broadcast: every device on the line would take the write";

/// What one write did on the device.
#[derive(Debug, PartialEq)]
pub struct Outcome {
    pub before: Option<Vec<u16>>,
    pub after: Option<Vec<u16>>,
    pub error: Option<LinkError>,
}

/// Reads the old value, writes `values` at `address` of `area`, reads back. Refuses to write when
/// the old value cannot be read: without it, nothing could be put back.
pub async fn write(
    links: &Links,
    target: &Target,
    area: Area,
    address: u16,
    values: &[u16],
    limits: &mut Limits,
    timeout: Duration,
) -> Outcome {
    if target.unit == 0 {
        // unit 0 is the broadcast: every device on the line would take the write, none answers
        return Outcome {
            before: None,
            after: None,
            error: Some(LinkError::NotSent(BROADCAST.into())),
        };
    }
    let count = values.len() as u16;
    let before = match access::read(links, target, area, address, count, timeout).await {
        Ok(before) => before,
        Err(error) => {
            return Outcome {
                before: None,
                after: None,
                error: Some(error),
            };
        }
    };
    if let Err(error) = access::write(links, target, area, address, values, limits, timeout).await {
        // only an exception is a refusal: without any answer the device may hold the new value
        let after = match error {
            LinkError::Exception(_) => None,
            _ => access::read(links, target, area, address, count, timeout)
                .await
                .ok(),
        };
        let taken = before != values && after.as_deref() == Some(values);
        return Outcome {
            before: Some(before),
            after,
            error: (!taken).then_some(error),
        };
    }
    let after = access::read(links, target, area, address, count, timeout)
        .await
        .ok();
    Outcome {
        before: Some(before),
        after,
        error: None,
    }
}

/// A write as the user asked for it, before it is done.
pub struct Planned {
    pub target: Target,
    pub point: Option<String>,
    pub area: Area,
    pub address: u16,
    pub values: Vec<u16>,
    /// The new value in words.
    pub text: String,
    pub restores: Option<u64>,
}

/// Does `planned` and logs it, refused or not: in `journal`, and in the file at `path` where
/// there is a data folder. `describe` puts the old registers in words.
#[allow(clippy::too_many_arguments)]
pub async fn journaled(
    links: &Links,
    planned: Planned,
    limits: &mut Limits,
    timeout: Duration,
    journal: &std::sync::Mutex<Journal>,
    path: Option<&std::path::Path>,
    describe: impl Fn(&[u16]) -> String,
    now: std::time::SystemTime,
) -> Result<Entry, String> {
    let outcome = write(
        links,
        &planned.target,
        planned.area,
        planned.address,
        &planned.values,
        limits,
        timeout,
    )
    .await;
    let since = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let mut journal = up_to_date(journal, path);
    let entry = Entry {
        id: journal.next_id(),
        time: since.as_secs(),
        device: planned.target.key(),
        point: planned.point,
        area: planned.area,
        address: planned.address,
        before_text: outcome.before.as_deref().map(&describe),
        before: outcome.before,
        written: planned.values,
        after: outcome.after,
        text: planned.text,
        restores: planned.restores,
        error: outcome.error.map(|error| error.to_string()),
    };
    journal.entries.push(entry.clone());
    if let Some(path) = path {
        crate::storage::append_entry(path, &entry)
            .map_err(|error| format!("done, but the log file could not be saved: {error}"))?;
    }
    Ok(entry)
}

/// The log of every write, in the order written.
#[derive(Default)]
pub struct Journal {
    pub entries: Vec<Entry>,
}

/// The journal with what the log file at `path` holds beyond it: the command line beside a
/// running server writes into the same data folder, and a number must not be given twice.
pub fn up_to_date<'journal>(
    journal: &'journal std::sync::Mutex<Journal>,
    path: Option<&std::path::Path>,
) -> std::sync::MutexGuard<'journal, Journal> {
    let mut journal = journal.lock().expect("journal lock");
    // a file that cannot be read leaves the log as it is known; saving to it says what is wrong
    if let Some((stored, _)) = path.and_then(|path| crate::storage::load_entries(path).ok()) {
        journal.take_up(stored);
    }
    journal
}

impl Journal {
    /// Adds the entries of `stored` this journal does not hold yet.
    fn take_up(&mut self, stored: Vec<Entry>) {
        let known: HashSet<u64> = self.entries.iter().map(|entry| entry.id).collect();
        self.entries.extend(
            stored
                .into_iter()
                .filter(|entry| !known.contains(&entry.id)),
        );
    }

    /// The number after the highest one in the log.
    pub fn next_id(&self) -> u64 {
        self.entries.iter().map(|entry| entry.id).max().unwrap_or(0) + 1
    }

    pub fn find(&self, id: u64) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    /// Writes still standing: per place the last successful write, unless it put the old value
    /// back, by restoring or by writing it once more. Oldest first.
    pub fn changed(&self) -> Vec<&Entry> {
        let mut last: HashMap<(String, Area, u16), &Entry> = HashMap::new();
        for entry in self.entries.iter().filter(|entry| entry.succeeded()) {
            last.insert(entry.place(), entry);
        }
        let mut standing: Vec<&Entry> = last
            .into_values()
            .filter(|entry| entry.restores.is_none())
            .filter(|entry| self.original(entry).as_ref() != Some(&entry.written))
            .collect();
        standing.sort_by_key(|entry| entry.id);
        standing
    }

    /// The first write of the run that led to `entry`: what the device held before it is what
    /// putting back restores, so restoring after two writes gives the value before the first.
    fn first_of_run<'journal>(&'journal self, entry: &'journal Entry) -> &'journal Entry {
        let place = entry.place();
        self.entries
            .iter()
            .rev()
            .filter(|earlier| {
                earlier.id < entry.id && earlier.succeeded() && earlier.place() == place
            })
            .take_while(|earlier| earlier.restores.is_none())
            .last()
            .unwrap_or(entry)
    }

    /// The value to put back for a standing write.
    pub fn original(&self, entry: &Entry) -> Option<Vec<u16>> {
        self.first_of_run(entry).before.clone()
    }

    /// The write that puts back the standing write `entry`, with the old value and its words
    /// from the same log entry. `None` where the old value is not known.
    pub fn restoring(&self, entry: &Entry) -> Result<Option<Planned>, String> {
        let first = self.first_of_run(entry);
        let Some(original) = first.before.clone() else {
            return Ok(None);
        };
        Ok(Some(Planned {
            target: Target::parse_key(&entry.device)?,
            point: entry.point.clone(),
            area: entry.area,
            address: entry.address,
            text: first.before_text.clone().unwrap_or_else(|| {
                let registers: Vec<String> = original.iter().map(u16::to_string).collect();
                registers.join(" ")
            }),
            values: original,
            restores: Some(entry.id),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u64, before: u16, written: u16, restores: Option<u64>) -> Entry {
        Entry {
            id,
            time: id,
            device: "tcp:10.0.0.5:502/2".into(),
            point: None,
            area: Area::HoldingRegister,
            address: 1,
            before: Some(vec![before]),
            written: vec![written],
            after: Some(vec![written]),
            text: written.to_string(),
            before_text: None,
            restores,
            error: None,
        }
    }

    #[test]
    fn a_write_stands_until_restored() {
        let mut journal = Journal::default();
        journal.entries.push(entry(1, 2500, 3000, None));
        assert_eq!(journal.changed().len(), 1);
        journal.entries.push(entry(2, 3000, 2500, Some(1)));
        assert!(journal.changed().is_empty());
    }

    #[test]
    fn restoring_after_two_writes_gives_the_first_old_value() {
        let mut journal = Journal::default();
        journal.entries.push(entry(1, 2500, 3000, None));
        journal.entries.push(entry(2, 3000, 3500, None));
        let standing = journal.changed();
        assert_eq!(standing.len(), 1);
        assert_eq!(journal.original(standing[0]), Some(vec![2500]));
    }

    #[test]
    fn putting_back_names_the_value_it_writes() {
        let mut journal = Journal::default();
        let mut first = entry(1, 2500, 3000, None);
        first.before_text = Some("25 Hz".into());
        let mut second = entry(2, 3000, 3500, None);
        second.before_text = Some("30 Hz".into());
        journal.entries.extend([first, second]);
        let standing = journal.changed()[0];
        let planned = journal
            .restoring(standing)
            .expect("a device key")
            .expect("the old value");
        assert_eq!((planned.values, planned.text), (vec![2500], "25 Hz".into()));
        assert_eq!(planned.restores, Some(2));
    }

    #[test]
    fn a_value_written_back_by_hand_stands_for_nothing() {
        let mut journal = Journal::default();
        journal.entries.push(entry(1, 2500, 3000, None));
        journal.entries.push(entry(2, 3000, 2500, None));
        assert!(journal.changed().is_empty());
        // a new run after it starts from the value the device holds then
        journal.entries.push(entry(3, 2500, 4000, None));
        let standing = journal.changed();
        assert_eq!(standing.len(), 1);
        assert_eq!(journal.original(standing[0]), Some(vec![2500]));
    }

    #[test]
    fn a_failed_write_changes_nothing_and_writing_the_same_value_stands_for_nothing() {
        let mut journal = Journal::default();
        let mut failed = entry(1, 2500, 3000, None);
        failed.error = Some("exception_03".into());
        journal.entries.push(failed);
        journal.entries.push(entry(2, 2500, 2500, None));
        assert!(journal.changed().is_empty());
    }

    #[tokio::test]
    async fn nothing_is_sent_to_unit_0() {
        let links = Links::default();
        let target = Target {
            endpoint: "tcp:127.0.0.1:9".parse().expect("endpoint"),
            unit: 0,
        };
        let outcome = write(
            &links,
            &target,
            Area::HoldingRegister,
            0,
            &[1],
            &mut Limits::default(),
            Duration::from_millis(10),
        )
        .await;
        assert_eq!(outcome.error, Some(LinkError::NotSent(BROADCAST.into())));
        assert_eq!(outcome.before, None);
    }

    #[test]
    fn numbers_rise_from_one() {
        let mut journal = Journal::default();
        assert_eq!(journal.next_id(), 1);
        journal.entries.push(entry(7, 0, 1, None));
        assert_eq!(journal.next_id(), 8);
    }

    #[test]
    fn a_device_holding_another_value_than_written_is_seen() {
        let mut kept = entry(1, 2500, 3000, None);
        kept.after = Some(vec![2500]);
        assert!(kept.kept_other());
    }
}
