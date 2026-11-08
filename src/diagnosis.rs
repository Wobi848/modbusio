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
