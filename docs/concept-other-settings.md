# Concept: devices at other serial settings

Status: **built on 2026-11-14** (branch `concept-other-settings`). The maintainer asked for the concept
and its building in one go; the open questions were decided by their recommendations (at the end).

## Why

The maintainer asked (2026-11-14): "kann man auf dem Bus scannen, ob irgendwelche Geräte falsch
eingestellt sind … ob fälschlicherweise Baud, Parität oder Stoppbit falsch ist?" and then: "mach ein
sauberes Konzept und dann implementieren".

A device set to another speed, parity or stop bits than its trunk does not understand the
master's requests and stays silent. Today:

- The bus analyser (`docs/concept-analyser.md`) hears that the master asks a unit that never
  answers, but cannot say why: missing, switched off, another unit ID, or other settings all look
  the same from outside. It never sends, so it cannot find out.
- *Find speed and frame* (`scan::find_settings`) stops at the first setting where any of units 1–10
  answers. It finds the trunk's setting, not the one device that differs.
- *Search the units of a serial line* asks every unit at one setting only.

Only asking can tell: every unit at every setting, and naming the devices that answer only at
settings other than the trunk's.

## What can be found, and what cannot

- **Another speed:** found. The device answers at its own speed only.
- **Other stop bits or parity:** found where the device is strict; it then answers only at its own
  frame. Many devices are lenient (8N2 against 8N1 often works); such a device answers at both and
  is not wrong in practice. A device that answers but whose answers arrive **damaged** at the
  trunk's setting is named too: parity or stop bits may differ (or the wiring).
- **Another unit ID:** found indirectly. A unit answers at the trunk's setting that the master
  never asks, while the master asks a unit that never answers (known from the bus analyser on the
  same port): "is unit 19 the device meant as 9?"
- **Not found:** a device that is missing, switched off, or behind broken wiring; a device at a
  speed outside the list asked.
- Reading the bits again at another setting from what an adapter already received is not possible:
  a USB adapter hands over bytes decoded at its own setting.

## Proposal: what the user sees

### Where

*Search settings*, *Serial line*, beside *Listen first* and *Find speed and frame*, a third tool:
**Find devices at other settings**. The fields there already hold the port, the speed and the frame;
here the speed and frame name **the trunk's setting** (what the master uses), and *From* / *To* /
*Wait per answer* apply as for the unit search.

```text
┌ Search settings ─────────────────────────────────────────────────────┐
│ Way          [ Network ] [ Gateway ] [ Serial line ]                 │
│ Port         [ COM3 · USB Serial (FTDI)  ▾ ]                         │
│ Speed        [ 19200 ▾ ]   Frame [ 8E1 ▾ ]     ← the trunk's setting │
│ First unit   [ 1 ]   Last unit [ 247 ]   Wait per answer [ auto ]    │
│ ☐ Also slow and rare speeds (1200, 2400, 4800, 57600)                │
│                                                                      │
│ ┌ Find devices at other settings ──────────────────────────────────┐ │
│ │ modbusio asks units 1–247 at 16 settings, about 7 minutes. It     │ │
│ │ sends read requests only. The plant's master must be stopped or   │ │
│ │ disconnected: modbusio listens first and does not send while      │ │
│ │ another master talks.                    [Cancel] [Search now]    │ │
│ └──────────────────────────────────────────────────────────────────┘ │
│ [Cancel] [Listen first] [Find speed and frame] [Find devices at      │
│                                                 other settings] [Search]
└──────────────────────────────────────────────────────────────────────┘
```

The first click asks in the dialog (as writing asks at the button), with the time it will take;
*Search now* starts it.

### What it does

1. **Listen first** (`listen.rs`), sending nothing. Another master heard: stop with the note
   *Another master is active … stop it first*, nothing sent. This check is in the server, not the
   page.
2. **The trunk's setting first**, then every other setting asked: speeds 9600, 19200, 38400,
   115200 and the trunk's own, each with the frames 8E1, 8N1, 8N2, 8O1; with *Also slow and rare
   speeds* 1200, 2400, 4800 and 57600 as well. Read Holding Register 0, one register, as every
   search asks; an exception counts as an answer.
3. **Wait per answer** left empty: long enough for 30 characters at that speed and 50 ms for the
   device, at least 100 ms (100 ms at 9600 and up, 325 ms at 1200). The page says the time before.
4. Progress in the status bar as for every search; *Stop* ends it, what was found stays.
5. Devices found join the open site **at the setting they answer at**
   (`serial:COM3:9600:8N1/9`), so they can be read and their settings register written right away
   (through a profile, behind the unlock as every write).

### What it says

Notes in words, as every search's (strips above the device list, the command line's end):

| Seen | Note |
| --- | --- |
| a unit answers only at other settings | "Unit 9 answers only at 9600 8N1; the trunk runs 19200 8E1: set unit 9 to 19200 8E1, or the master cannot reach it." |
| a unit answers at the trunk's setting, but its answers arrive damaged there | "Answers of unit 5 arrive damaged at 19200 8E1: its parity or stop bits may differ, or its wiring." |
| a unit answers that the master never asks, while it asks units that never answer (bus analyser on this port) | "Unit 19 answers at the trunk's setting, but the master never asks it; it asks unit 9, which never answers: is unit 19 the device meant as 9?" |
| every unit found answers at the trunk's setting | "All 4 units answer at 19200 8E1, the trunk's setting." |
| nothing answers at any setting | the note of today: *No unit answered on COM3 …* |

**The trunk's setting** is the one given in the dialog. A unit that answers there is right, even if
it answers at others too (lenient devices).

### The bus analyser points to it

The analyser's notes *never answers* and *damaged from one unit* name other settings as a cause and
point to *Search settings → Serial line → Find devices at other settings*.

### Command line

```text
modbusio scan --serial COM3 --baud 19200 --frame 8E1 --other-settings
modbusio scan --serial COM3 --baud 19200 --frame 8E1 --other-settings --all-speeds --to 32
```

## Writing to plants

Nothing is written. Only read requests (03, one register) are sent; but they are sent, at many
settings, so:

- **Listen first, in the server:** another master heard → nothing sent, the note says why.
- The **confirmation in the dialog** names the time and that the plant's master must be stopped.
- **Refused** (`409`) while the bus analyser listens on the port or another search runs.
- At a wrong setting the bytes are noise to the devices set right; one would have to take noise for
  a frame with a valid CRC of a function it knows to act on it. A read request is the only thing
  modbusio sends, so even then nothing changes in a device.

## Functions and where they live

New rows for `docs/concept.md` section 3.2; nothing moves.

| Function | Place | Test |
| --- | --- | --- |
| Find devices at other serial settings: listen first, every unit at every setting, devices join the site at their setting | *Search settings*, *Serial line*, *Find devices at other settings* | `other_settings.rs` (verdict, one test per note), `scan.rs` (simulated line with devices at their own settings), `api.rs` (refused while another master talks), `ui/05-serial` |
| `scan --other-settings` | terminal | `cli.rs` |
| The analyser's notes point to it | *Bus analyser* | `analyser/notes.rs` |

`simulated_rtu --settings` gives its devices settings of their own: the drive at 9600, the room
controller with two stop bits; a pseudo-terminal carries speed and stop bits to the simulator, not
parity, so parity is tested on the verdict without I/O.

## Compatibility

Nothing stored changes. The search report gains `other_settings` (per unit, where it answered);
new route `POST /api/scan/other-settings`. `CHANGELOG.md`: the new search.

## Order of building

1. Verdict without I/O (`src/other_settings.rs`): from what each setting heard, the trunk's setting
   and the units the master asks, the notes. One test per note, parity cases included.
2. The search itself on `Links`, listening first; `simulated_rtu --settings`; Rust tests on the
   pseudo-terminal.
3. Server route, command line.
4. Dialog with its question, notes, the analyser's pointer; texts; browser test.

## Decided

The maintainer asked to build it right after the concept; each recommendation was taken.
The question in the dialog asks the server for the time (`check: true`), so the page does not
repeat the server's arithmetic.

1. **Speeds asked by default:** 9600, 19200, 38400, 115200 and the trunk's, slow and rare ones
   behind a checkbox? *Recommendation:* yes; all eight speeds take about three times as long
   (about 20 minutes for 247 units).
2. **Frames:** 8E1, 8N1, 8N2, 8O1 (as *Find speed and frame*)? *Recommendation:* yes; 8E2 and 8O2
   are rare.
3. **Devices found at other settings into the site?** *Recommendation:* yes, at their setting, so
   their settings register can be read and written at once.
4. **The bus analyser's units:** take the units the master asks from an analysis of the same port
   for the "meant as" note? *Recommendation:* yes, when one is kept; nothing is sent for it.
