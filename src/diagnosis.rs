//! Notes in plain words (concept section 8). Each has a stable code and its values, so the
//! interface can translate it; the English text is the fallback and the command line's output.

use std::fmt;

use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum Note {
    /// No address of the subnet has the port open.
    NoHostOpen { subnet: String, port: u16 },
    /// The subnet is too large to ask address by address.
    SubnetTooLarge { subnet: String },
    /// The port is open but nothing answers Modbus TCP.
    PortOpenNoAnswer { address: String },
    /// One device that answers every unit ID.
    AnswersEveryUnit { address: String },
    /// A gateway: units behind it are searched separately.
    Gateway { address: String },
    /// Answers came back damaged.
    Damaged { line: String, count: usize },
    /// Another unit answered than the one asked.
    OtherUnit {
        line: String,
        asked: u8,
        answered: u8,
    },
    /// Nothing answered on a line.
    NothingAnswered { line: String },
    /// Frames of another master were heard while listening.
    OtherMaster { port: String, settings: String },
    /// The line was quiet while listening: no other master.
    Quiet { port: String },
    /// The settings at which devices answered.
    SettingsFound { settings: String },
    /// Floats read plausibly only in another word order.
    WordOrder { order: String },
    /// A device took fewer registers per request than asked.
    BlockLimit { count: u16 },
    /// Reading across unused addresses fails; only named points are read.
    NoHoles,
    /// A device wants 06 for single registers.
    SingleWrites,
    /// Bus analyser: a unit leaves some of the master's requests unanswered.
    MissesAnswers {
        unit: u8,
        missing: u64,
        requests: u64,
        percent: u32,
    },
    /// Bus analyser: … and nearly always right after a request to another unit.
    MissesAfter {
        unit: u8,
        missing: u64,
        requests: u64,
        percent: u32,
        after: u8,
    },
    /// Bus analyser: the master asks a unit that never answers.
    NeverAnswers { unit: u8, requests: u64 },
    /// Bus analyser: the same request is refused with the same exception every time.
    RefusedEveryTime {
        unit: u8,
        area: String,
        first: u16,
        last: u16,
        exception: String,
    },
    /// Bus analyser: a unit answers slowly at times.
    SlowAnswers {
        unit: u8,
        max_ms: u64,
        median_ms: u64,
    },
    /// Bus analyser: damaged answers come from one unit only.
    DamagedFromOne { unit: u8, count: u64 },
    /// Bus analyser: the master asks, nothing answers at all.
    MasterUnanswered { line: String },
    /// Bus analyser: answers without their request: another master on the trunk.
    TwoMasters { line: String, count: u64 },
    /// Bus analyser: the master writes to unit 0, which every unit takes and none answers.
    Broadcasts { count: u64 },
    /// Bus analyser: the trunk carries bytes most of the time.
    BusBusy { percent: u32 },
}

impl fmt::Display for Note {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Note::NoHostOpen { subnet, port } => write!(
                f,
                "No address in {subnet} has port {port} open: wrong subnet or port, or a firewall in between."
            ),
            Note::SubnetTooLarge { subnet } => write!(
                f,
                "{subnet} is too large to ask address by address (up to /22): give the plant's subnet."
            ),
            Note::PortOpenNoAnswer { address } => write!(
                f,
                "{address} has the port open but answers no Modbus TCP: a serial gateway in transparent mode? Add it as RTU over TCP."
            ),
            Note::AnswersEveryUnit { address } => write!(
                f,
                "{address} answers every unit ID: one device, shown as unit 1."
            ),
            Note::Gateway { address } => write!(
                f,
                "{address} is a gateway: search the units behind it."
            ),
            Note::Damaged { line, count } => write!(
                f,
                "{count} answers on {line} arrived damaged: check speed and parity, A/B, termination and bias."
            ),
            Note::OtherUnit { line, asked, answered } => write!(
                f,
                "On {line} unit {answered} answered when {asked} was asked: another master, or two devices with one address."
            ),
            Note::NothingAnswered { line } => write!(
                f,
                "No unit answered on {line}: check unit IDs, speed and parity, A/B and the power of the devices."
            ),
            Note::OtherMaster { port, settings } => write!(
                f,
                "Another master is active on {port} ({settings}): searching now would collide. Stop it first."
            ),
            Note::Quiet { port } => write!(f, "{port} is quiet: no other master is active."),
            Note::SettingsFound { settings } => write!(f, "Devices answer at {settings}."),
            Note::WordOrder { order } => write!(f, "The values read plausibly only in word order {order}."),
            Note::BlockLimit { count } => write!(
                f,
                "The device refused larger blocks: reading at most {count} registers per request."
            ),
            Note::NoHoles => f.write_str(
                "The device refuses reading across addresses it does not have; only the points are read.",
            ),
            Note::SingleWrites => {
                f.write_str("The device wants function 06 for single registers; writing so.")
            }
            Note::MissesAnswers {
                unit,
                missing,
                requests,
                percent,
            } => write!(
                f,
                "Unit {unit} does not answer {missing} of {requests} requests ({percent} %): check its wiring and power, and whether the master waits long enough."
            ),
            Note::MissesAfter {
                unit,
                missing,
                requests,
                percent,
                after,
            } => write!(
                f,
                "Unit {unit} does not answer {missing} of {requests} requests ({percent} %), nearly always right after a request to unit {after}: unit {unit} may need more time after its neighbour."
            ),
            Note::NeverAnswers { unit, requests } => write!(
                f,
                "The master asks unit {unit}, which never answered ({requests} requests): a device missing, switched off or at another address."
            ),
            Note::RefusedEveryTime {
                unit,
                area,
                first,
                last,
                exception,
            } => write!(
                f,
                "The master reads {area} {first}–{last} of unit {unit}; the device answers exception {exception} every time: the master's register list does not match the device."
            ),
            Note::SlowAnswers {
                unit,
                max_ms,
                median_ms,
            } => write!(
                f,
                "Unit {unit} answers after up to {max_ms} ms (usually {median_ms} ms): a master waiting less counts these as failures."
            ),
            Note::DamagedFromOne { unit, count } => write!(
                f,
                "{count} answers of unit {unit} arrived damaged, the master's requests did not: check the wiring at unit {unit}."
            ),
            Note::MasterUnanswered { line } => write!(
                f,
                "The master asks on {line}, but no unit answers: the master's speed or parity, or the trunk behind this point."
            ),
            Note::TwoMasters { line, count } => write!(
                f,
                "{count} answers on {line} came without their request: a second master seems to share this trunk, and their requests collide."
            ),
            Note::Broadcasts { count } => write!(
                f,
                "The master wrote {count} times to all units at once (unit 0): every unit takes it, none answers."
            ),
            Note::BusBusy { percent } => write!(
                f,
                "The trunk is busy {percent} % of the time: polling slower or a higher speed leaves room."
            ),
        }
    }
}

/// A note with its English text, as the API and the command line give it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct NoteView {
    #[serde(flatten)]
    pub note: Note,
    pub text: String,
}

impl From<Note> for NoteView {
    fn from(note: Note) -> Self {
        NoteView {
            text: note.to_string(),
            note,
        }
    }
}
