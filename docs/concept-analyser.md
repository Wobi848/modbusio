# Concept: bus analyser for Modbus RTU

Status: **approved on 2026-11-14** (answers at the end). Steps 1 to 3 of the building order are
built (branches `concept-analyser`, `analyser-server`). Mockup:
[mockup/analyser.html](mockup/analyser.html).

## Why

The maintainer asked whether a USB adapter can hang on an RTU trunk in parallel to analyse it, and
then (2026-11-14): "ich denke wir sollten einen sehr ausgereiften Analyser bauen, der das kann".

On a running plant the question is rarely "what does this device say" but "why does the plant's
own master have trouble": a meter that drops out every few minutes, a controller that shows old
values, a second master nobody knew about, a vendor's integration that reads the wrong register.
modbusio cannot answer that today:

- *Listen first* (`src/listen.rs`) hears a few seconds per setting and answers three things:
  another master is active, at which settings, which unit IDs talk. It counts frames and stray
  bytes. Nothing more is kept or shown.
- The capture for Wireshark (`capture.rs`, `pcap.rs`) holds only modbusio's own requests and
  answers.
- Taking part as a second master to find out is no option: it collides with the plant's master
  (`docs/concept.md` section 5.2, and AGENTS.md: never search a trunk while another master is
  active).

Other tools: Wireshark decodes RTU from a file with the right link type, but does not capture a
serial port by itself. Serial sniffers show a frame list in hex; they leave the reading of it to
the technician. Saying in words what is wrong is where modbusio is different (concept section 8).

## What already exists to build on

- RTU framing by form and CRC, requests and answers alike (`listen::frames`,
  `pdu::request_length`, `pdu::response_length`), working on bytes that USB adapters deliver in
  chunks.
- Finding speed and frame by listening (`listen::listen`).
- Decoding answers (`pdu::decode_response`), exceptions in words (`pdu::Exception`), every reading
  of a register and the plausible word order (`values.rs`), profiles that name registers.
- pcapng with RTU as user link type 147 (`pcap.rs`), charts (`web/chart.js`), CSV and Excel
  (`export.rs`, `workbook.rs`).
- The simulated trunk on a pseudo-terminal (`examples/simulated_rtu.rs`).

## Proposal: what the user sees

### Rule one: the analyser never sends

The port is opened and only read. The code that analyses gets a stream that can only be read (a
type without `write`), so sending is impossible by construction, not by care. An RS-485 adapter
with automatic direction only drives the line when bytes are written, so it stays silent.

The sidebar, the page head and the status bar say "Listening only · sends nothing" while it runs.

### Connecting (in words on the empty page)

The empty page explains, with a small sketch:

- A, B and GND of the adapter to the trunk, anywhere (at the master or at a device).
- **Termination and bias on the adapter off**: the trunk has them already. A third terminating
  resistor in the middle of a trunk can cause errors on longer lines.
- Nothing readable: A and B are often swapped, because vendors label them differently.

### A new area: *Bus analyser*

Its own area in the sidebar, after *Changed by you* (key 7; *Settings* moves to 8). Not on Android
(no serial port there, concept section 10). The page head's next step is *Start listening*.

```text
┌──────────────┬──────────────────────────────────────────────────────────────────────┐
│ modbusio     │ Bus analyser                                  [Stop listening]       │
│              │ COM3 · 19200 8E1 · listening only, sends nothing · 12:04 running     │
│ ▣ Workspace 1├──────────────────────────────────────────────────────────────────────┤
│ ⌕ Registers 2│ ⚠ Unit 12 does not answer 14 of 380 requests (4 %), always after a   │
│ ▤ Profiles  3│   request to unit 5: unit 12 may need more time after a neighbour.   │
│ ◉ Watch     4│ ⚠ The master reads HR 100–120 of unit 7 every cycle; the device      │
│ ≡ Recording 5│   answers exception 02 each time: the master's register list is off. │
│ ✎ Changed   6├──────────────────────────────────────────────────────────────────────┤
│ ≈ Analyser ●7│ [Units 6] [Exchanges] [Registers] [Frames]     [Pause] [Export ▾]    │
│ ⚙ Settings  8│ Unit  Requests  Answers  No answer  Exceptions  Answer ms  Cycle s   │
│              │    1      412      412          0           0     18–24       2.0    │
│              │    5      412      410          2           0     22–95       2.0    │
│              │    7      824      412          0   412 × 02     19–22       2.0    │
│              │   12      380      366         14           0     40–310      2.2    │
├──────────────┴──────────────────────────────────────────────────────────────────────┤
│ ● Listening · COM3 19200 8E1 · 31,204 frames · 0 damaged · bus load 38 %            │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

**Start:** port (USB first, as in the search), speed and frame or *Find out by listening* (the
default, `listen::listen`), *Start listening*. A port modbusio itself holds for reading is closed
first, as *Listen first* does today; while the analyser runs, modbusio's own reads, searches and
writes on that port are refused (`409`), so modbusio never becomes a second master by accident.
Another port (a second adapter) stays usable.

The analysis runs on when the page is left, like a recording; the sidebar shows a dot.

### The four views

1. **Units** (the start): one row per unit ID heard. Requests, answers, no answer, exceptions by
   code, damaged frames in answers, answer time (min, median, 95 %, max), poll cycle (time between
   two requests of the master to the same unit), last seen. A unit with a profile in the open site
   (same port, same unit ID) shows its name. Details: the same numbers per function and address
   range, the answer time as a chart over time, *Add to site* for a unit not in the site yet.
2. **Exchanges**: request and answer paired, one row each: time, unit, function, area and range
   ("HR 100, 20 registers"), result (values, *written*, *exception 02 in words*, *no answer*),
   answer time. Filter: unit, function, only problems (no answer, exceptions, damaged), only
   writes. Details: the bytes of both frames with their parts named (unit, function, address,
   count, data, CRC) and the values decoded.
3. **Registers**: what the master uses, per unit and area: each address read or written, how often,
   the last value, how often it changed, min and max, and with the next register every reading as
   in the explorer (u16, s16, hex, float and 32-bit in the plausible word order). With a profile:
   the point's name, value and unit. Details: the value over time as a chart (from the answers seen,
   no request of our own), and *Into a profile …* (see below).
4. **Frames**: the raw list, as Wireshark would show it, for the case nothing else explains:
   time, gap to the frame before, bytes in hex, request or answer, CRC good or damaged, stray bytes
   between frames.

**Writes by the master** are marked in every view (a pencil, as in *Changed by you*): which unit,
which register, old value (the last one read before, where seen) and new value. This answers "who
changed the setpoint" without touching the plant.

**Pause** freezes the view; listening and counting go on. **Clear** starts the counts again.

### Diagnosis in words

As concept section 8: a strip per finding above the views, with the unit and what to check. First
set (thresholds are proposals, each with a test):

| Seen | Sentence (short) |
| --- | --- |
| a unit answers some requests, not others | "Unit 12 does not answer 4 % of requests." Plus the pattern, when one holds: always after a request to unit N, only for function X, only at the shortest poll interval |
| a unit is asked but never answers | "The master asks unit 9, which never answers: a device missing, switched off or at another address." |
| the same exception every cycle | "The master reads HR 100–120 of unit 7; the device answers exception 02 every time: the master's register list does not match the device." |
| answers late, close to the next request | "Unit 12 answers after up to 310 ms; the master asks again after 2.2 s. If the master's timeout is shorter, it counts these as failures." |
| damaged frames from one unit only | "Answers of unit 5 arrive damaged, the master's requests do not: check the wiring at unit 5." |
| damaged frames everywhere | "Many damaged frames: wrong speed or parity, A/B, termination, bias, cable." (as *Listen first*) |
| requests without any answer at all | "Nothing answers: the master's speed or the trunk behind this point." |
| frames that only fit two masters (a second request while an answer is due, two poll patterns) | "Two masters seem to share this trunk: their requests collide." |
| broadcast writes (unit 0) | "The master writes to all units at once (unit 0): no device answers a broadcast." (information, not a fault) |
| bus load high | "The trunk is busy 85 % of the time: a slower poll or a higher speed leaves room." |

### Into a profile

On *Registers*, for a unit: *Into a profile …* opens the existing profile editor with a draft: one
point per address range the master reads, area and address as seen, type u16 (u32 or float where
the plausible word order says so), name empty. The technician names them. A vendor's list is still
the better source (skill `register-map`); this is for the device without one.

### Saving and opening

- **Save as pcapng** (Wireshark, user link type 147 as today; requests and answers marked).
  modbusio keeps no format of its own for an analysis.
- **Open a pcapng file** of RTU frames (from modbusio or another tool) into the analyser: the same
  four views, offline. So a capture taken on site can be analysed later, at the desk.
- **Export** the units, exchanges and registers as CSV and Excel.

### Command line

```text
modbusio analyse COM3                         find the settings, then show exchanges live
modbusio analyse COM3 --baud 19200 --frame 8E1 --seconds 600 --pcap trunk.pcapng
modbusio analyse --file trunk.pcapng          the units table and the notes of a saved file
```

Ctrl+C or `--seconds` ends it with the units table and the notes.

## What a USB adapter can and cannot measure

Honest limits, said in the page (help) and in the README:

- A USB adapter hands bytes over in chunks. An FTDI chip waits up to 16 ms by default before it
  passes on a small chunk (its *latency timer*). Times are when a chunk arrived, not when the byte
  was on the wire. Answer times are therefore exact to about this latency, good enough for
  "answers in 300 ms instead of 20", not for the 1.5- and 3.5-character gaps of the specification
  (Serial Line 1.02, 2.5.1.1). The analyser does not judge those gaps.
- Where the system says it (Linux: `/sys/bus/usb-serial/devices/<port>/latency_timer`), the page
  names the latency and how to lower it to 1 ms; modbusio does not change it (that needs rights).
- Frames are found by form and CRC, not by timing (as `listen.rs` today). For functions whose length
  modbusio does not know (vendor functions, 0x17, file records), the end is found by searching for a
  matching CRC; such a frame is shown as "function 0x41, 23 bytes, not decoded".
- An answer of 05 or 06 is the request repeated byte for byte. Which is which follows from the
  order: the first is the request, the identical second the answer.

## Writing to plants

None. The analyser sends nothing (rule one). No unlock, no confirmation. The writes it shows are the
plant master's.

## Functions and where they live

New rows for `docs/concept.md` section 3.2 (entered there when built). Nothing existing moves.
*Listen first* stays in the search settings; it is the short form of the same thing.

| Function | Place | Test |
| --- | --- | --- |
| Listen to a trunk continuously, sending nothing; the port refused to modbusio's own requests meanwhile | *Bus analyser*, *Start listening* | `analyser.rs` (read-only stream), `api.rs` (`409` on the held port), `ui/08-analyser` |
| Frames found in chunked bytes, requests and answers, unknown functions by CRC, echoes of 05/06 | *Frames* | `analyser/frames.rs` with byte streams cut at every place |
| Request and answer paired, no answer, exceptions, answer time | *Exchanges* | `analyser/exchanges.rs` |
| Per unit: counts, answer times, poll cycle, damaged frames | *Units* | `analyser/units.rs`, `simulated_trunk` |
| Registers the master uses, values, changes, readings, profile names | *Registers* | `analyser/registers.rs` |
| Writes of the plant's master marked, old and new value | every view | `analyser/exchanges.rs` |
| Value and answer time over time as charts | details | `tests/web/ui.test.js` |
| Notes in words (table above) | strips | `diagnosis.rs`, one test per note |
| Draft profile from the registers used | *Registers*, *Into a profile …* | `ui/08-analyser` |
| Save as pcapng, open a pcapng file | *Export*, *Open file …* | `pcap.rs` (written file read back) |
| Export units, exchanges, registers as CSV and Excel | *Export* | `export.rs` |
| Pause, clear, keeps running when the page is left | page head, sidebar dot | `ui/08-analyser` |
| `modbusio analyse` | terminal | `cli.rs` |

A new simulator **`simulated_trunk`** plays a whole trunk on a pseudo-terminal: a master polling
units 1, 5, 7, 12 in a cycle and their answers, with switches for the faults in the table above
(a unit that misses answers, exceptions, damaged answers from one unit, slow answers, a second
master, broadcasts, writes). Every note and every view is tested against it, never against a plant.

## Compatibility

Nothing stored changes. Sites, profiles, the write log and settings load as before. The key of
*Settings* changes from 7 to 8 (the areas count up as today); `CHANGELOG.md` names it. New API
routes (`/api/analyser/…`) go into `docs/api.md`. pcapng files modbusio wrote before open in the
analyser.

## Order of building

Each step a pull request of its own, everything working after each:

1. **Frames and exchanges, without I/O**: request decoding, the frame finder with state (chunks,
   unknown functions, echoes), pairing, counts per unit. Tests with byte streams only.
2. **`simulated_trunk`** with its faults.
3. **Server and command line**: start, stop, state since a sequence number, the port held, save as
   pcapng; `modbusio analyse`.
4. **Page: Units and Exchanges**, empty page with the connection sketch, Pause, Clear.
5. **Registers and Frames**, charts, marked writes, CSV and Excel.
6. **Notes in words** from the table, one by one with a test each.
7. **Into a profile**, **open a pcapng file**.
8. Field test on a real trunk (M3), then the README and the help.

## Answers

Answered by the maintainer on 2026-11-14; each recommendation taken:

1. **Its own area** *Bus analyser* (key 7; *Settings* moves to 8).
2. **Name:** *Bus analyser*, German *Busanalyse*.
3. **Opening pcapng files**, also from other tools, is part of this concept (step 7).
4. **Modbus TCP and RTU over TCP** stay out: they cannot be heard without a mirror port.
5. **Memory:** frames up to 200,000 in memory, counts and register values without limit; for
   longer, start again. No rolling file on disk.
6. **Draft profile from traffic:** yes, small, in the existing editor.
