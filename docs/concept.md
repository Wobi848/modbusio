# Concept: modbusio (Modbus browser)

Updated: 2026-11-08 · Status: **approved on 2026-11-08; M0–M2 built** (state in section 12,
answers in section 14)

Nothing here is fixed. Every choice is a proposal with its reason. modbusio is bacnetio's sibling
for Modbus: same stack, same interface, same rules for writing to plants. Where bacnetio has solved
something, modbusio takes the pattern over (section 11); this concept describes what is different
because Modbus is different.

## 1. Why

Modbus is everywhere in building services: heat and electricity meters, drives, heat pumps, small
controllers, gateways. The free tools are thin (QModMaster, mbpoll, Simply Modbus), the good one
(Modbus Poll) costs money and still leaves the hard part to the technician. The causes of a failed
job are almost always the same, and no tool names them:

| Cause | What you see |
| --- | --- |
| wrong serial settings (baud, parity, stop bits) | no answer, or garbage, CRC errors |
| wrong unit ID, or a gateway that needs one | timeout, or exception 0x0B "gateway target failed to respond" |
| address offset: the list says `40001`, the device wants `0` | values shifted by one register, or exception 02 |
| word and byte order of 32-bit values | absurd numbers (`-1.2e-38`, `4.29e9`) |
| scaling missing (×0.1, ×0.01, a scale register) | `2315` instead of `23.15 °C` |
| reading a block that spans a gap in the register map | exception 02 for the whole block, although every point exists |
| more registers per request than the device takes | exception 03 or silence |
| polling faster than an RTU device answers, or a second master on the trunk | sporadic timeouts, CRC errors |
| A/B swapped, no termination, no bias | nothing, or errors under load |
| TCP device allows one connection, another tool holds it | connection refused or reset |

And Modbus describes nothing about itself: no device list, no names, no units. Without the
vendor's register list a device is 65,536 numbers.

## 2. Goal

**A Modbus browser that anyone can use without explanation: it finds devices, makes their registers
readable as named values with units, and when something fails it says why in plain words.**

Users: the same as bacnetio's (service technicians, integrators, laptop or phone on site). Typical
jobs: is the meter on the trunk and what does it say; does the register list match the device;
commission a drive or heat pump; document the points.

## 3. Interface

The interface is bacnetio's (mbusio branch `ui-sidebar` as its origin): sidebar with the site and
the connection, page head with the one next step, tree, table and details side by side, status bar,
light and dark, the system's language, keys `1`–`6`, `/`, `?`. Principles: bacnetio
`docs/concept.md` section 3.1, taken over unchanged.

### 3.1 Layout

```text
┌──────────────┬────────────────────────────────────────────────────────────────┐
│ modbusio     │ Heat meter · unit 12                       [Next step: …]      │
│ [COM3 ▾]     │ RTU · COM3 · 9600 8E1 · profile "Sensus PolluCom" (own)        │
│ ● 9600 8E1   ├────────────────────────────────────────────────────────────────┤
│   [Change]   │ [Search…] [All 24|Faults 1|Changed 0]  [Read again] [CSV ▾]    │
│──────────────│ ▾ Energy                            │ Flow temperature         │
│ ▣ Workspace 1│     Heat energy   HR 0   1234.5 kWh │ 63.2 °C                  │
│ ⌕ Registers 2│     Volume        HR 4   88.21 m³   │ HR 40011 (offset 10)     │
│ ≡ Recording 3│ ▾ Temperatures                      │ float32, ABCD, ×1        │
│ ◷ Watch     4│     Flow          HR 10  63.2 °C    │ raw 0x427C 0xCCCD        │
│ ✎ Changed   5│     Return        HR 12  41.0 °C    │ [Record] [Watch]         │
│ ⚙ Settings  6│                                     │                          │
├──────────────┴────────────────────────────────────────────────────────────────┤
│ ● Ready · COM3 9600 8E1 · 3 devices · last answer 0.2 s ago                   │
└───────────────────────────────────────────────────────────────────────────────┘
```

- **Tree:** site → connections (one TCP network, one serial port, one gateway) → devices
  (TCP address + unit ID, or serial port + unit ID) → groups of points.
- **Workspace:** with the site chosen, the device list; with a device chosen, its points from its
  profile (section 6); with a point chosen, its details.
- **Registers:** the explorer for a device without a profile (section 5.3).
- **Recording, Watch list:** as in bacnetio.
- **Changed by you:** every write, with the value before it and *Restore* (section 7).
- **Settings:** language, theme, capture for Wireshark, version check, help.

### 3.2 Functions and where they live

The list for "never lose a function" (AGENTS.md): a change that moves or replaces one updates its
row. Rust tests in `tests/` and next to the code, interface logic in `tests/web/`, browser tests in
`tests/ui/`.

| Function | Place | Test |
| --- | --- | --- |
| Search a subnet for port 502, then units 1, 255, 0; one device for every unit; gateways named | *Search settings*, *Network (TCP)* | `scan.rs` (subnet), `ui/01-search` |
| Search the units behind a gateway, Modbus TCP or RTU over TCP | *Search settings*, *Gateway* | `api.rs` (search adds to site), `ui/01-search` |
| Listen to a serial line without sending | *Search settings*, *Serial line*, *Listen first* | `scan.rs` (listening), `listen.rs`, `ui/05-serial` |
| Find speed and frame of a serial line | *Search settings*, *Find speed and frame* | `scan.rs` (settings), `ui/05-serial` |
| Search the units of a serial line, with retries and an estimate | *Search settings*, *Serial line* | `scan.rs`, `link.rs` (pseudo-terminal), `ui/05-serial` |
| Stop a running search | status bar | `api.rs` (one search at a time) |
| Diagnosis in words: notes of a search, of reading, of the explorer | strips above the tables | `diagnosis.rs`, `tests/web/ui.test.js` (German notes) |
| Read Device Identification (43/14) | search; device details, *Ask who it is* | `link.rs` (identification), `scan.rs` |
| Add a device by hand by its line and unit; chosen, and asked once whether it answers (TCP only) | workspace, *Add device …* | `api.rs` (asked whether it answers), `ui/01-search` |
| Device names, profile chosen, removed from the site | device details | `api.rs`, `ui/03-write` |
| What a device refused learnt and kept (registers per request, no holes, 06) | device details, *Learnt* | `read.rs` (gaps, limits, silence), `api.rs` |
| Read a device's points by its profile, grouped, filtered, live | workspace | `api.rs` (read), `ui/02-explore`, `tests/web/ui.test.js` |
| A point's value, address, type, order, scaling, raw registers, states | point details | `profile.rs`, `ui/03-write` |
| Register explorer: addresses found by halving, readings in every order, plausible order | *Registers* | `explore.rs`, `api.rs` (explorer), `ui/02-explore` |
| A register into the device's profile, a new profile where it has none | *Registers*, *Add to profile …* | `ui/02-explore`, `tests/web/ui.test.js` |
| Profiles: list, edit points, save, delete, CSV and Excel | *Profiles* | `api.rs`, `export.rs` (export imports back) |
| Import a vendor's list (CSV, Excel), convention and columns recognised, preview from the device | *Profiles*, device details, *Import register list …* | `profile_import.rs`, `api.rs` (import), `ui/04-import` |
| Write a point behind the unlock (checked by the server), value, confirmation | point details | `api.rs` (writing), `ui/03-write` |
| Write a raw register after its own confirmation | *Registers*, *Write …* | `api.rs` (raw write) |
| Old value read and logged first, read back; log on disk | *Changed by you* | `write.rs`, `storage.rs`, `ui/03-write` |
| Put back one or all old values, also after a restart, the confirmation naming value and place; leaving asks first | *Changed by you* | `write.rs`, `api.rs`, `ui/03-write` |
| A write without confirmation read back: taken or not | *Changed by you* | `tests/write.rs` |
| Log as CSV and Excel | *Changed by you* | `export.rs` |
| Watch list across devices | *Watch list* | `ui/06-live` |
| Recording with charts, CSV and Excel, backed up | *Recording* | `record.rs`, `ui/06-live`, `tests/web/ui.test.js` (charts) |
| Capture for Wireshark (TCP as IPv4, RTU as user link type 147) | *Settings* | `pcap.rs` |
| Sites: several, named, opened, deleted, as a file | sidebar, *Settings* | `site.rs`, `ui/07-settings` |
| Exports: device list, a device's points, the whole site | workspace, *Export* | `export.rs`, `workbook.rs` |
| Language and theme, following the system | *Settings* | `tests/web/ui.test.js`, `ui/07-settings` |
| Phone width: details over the list, the way back, no sideways scrolling | every page | `ui/07-settings` |
| Keys `1`–`6`, `7`, `/`, Enter, Escape | everywhere | `ui/06-live` (keys 4, 5) |
| Only pages of this modbusio may use the API | server | `api.rs` (`foreign_pages_are_refused`) |
| Command line: `interfaces`, `ports`, `scan`, `listen`, `read`, `explore`, `write`, `changed`, `restore`; exit code 1 when nothing could be read | terminal | `cli.rs` (output and exit code), tried against the simulators; the logic below it is tested above |

## 4. Scope

### Version 1

- **Modbus TCP** (port 502, configurable), IPv4
- **Modbus RTU** over a USB RS-485 adapter (Windows and Linux)
- **RTU over TCP** (serial gateways that pass RTU frames through, e.g. Moxa, USR, Waveshare in
  "transparent" mode), because many sites have them
- Function codes: read 01, 02, 03, 04; write 05, 06, 15, 16; 43/14 Read Device Identification
  where the device offers it; 08/00 echo for the trunk test
- Find devices (section 5), register explorer, profiles (section 6), writing behind the unlock
  (section 7), diagnosis in words (section 8)
- Recording, watch list, CSV and Excel export, as in bacnetio
- UI in English and German from the start; French, Italian, Spanish as drafts like bacnetio
- Windows, Linux, without administrator rights; Android with TCP (section 10)

### Later or never

- Modbus ASCII: only if a field case asks for it
- Acting as a Modbus server or gateway: not planned (mbusio already serves its meters as Modbus)
- Function codes 20/21 (file records), 22, 23: on request
- Shared online library of profiles: later, kept apart from this repo like bacnetio's plugins

## 5. Finding devices

Modbus has no Who-Is. Finding means asking, carefully.

### 5.1 TCP

1. **Hosts:** TCP connect to port 502 on every address of the chosen subnet (up to /22), a few
   dozen at a time, short timeout. A connect needs no administrator rights and passes the Windows
   firewall because we open it. Over a VPN: the plant's subnet as in bacnetio.
2. **Unit IDs:** on each open port, ask unit 1, 255 and 0 first (the usual ones for a direct
   device), then, when the host answers 0x0B/0x0A or is a known gateway, 1–247 to find the devices
   behind it.
3. **Who is it:** 43/14 Read Device Identification (vendor, product, version) where offered;
   otherwise "Modbus device, no identification".
4. A connection refused while the port showed open, or reset after the first request: "the device
   takes only one connection; another program holds it".

### 5.2 RTU

1. **Listen first, sending nothing** (as bacnetio does on MS/TP): a few seconds on the trunk. Valid
   frames at some speed mean another master is active: say so, and that searching would collide.
2. **Speed and frame:** the user's setting, or *Find out*: try the usual ones (9600 8E1, 19200 8E1,
   9600 8N1, 19200 8N1, 38400, 4800, 2400, 115200 …) with a probe to the first answering unit.
3. **Unit IDs 1–247:** one request each (03 at register 0, then 04, 01 as fallback; *any* answer,
   including an exception, means a device). Turn-around delay after a broadcast and the 3.5-character
   gap respected; the time per address is estimated beforehand ("about 4 minutes at 9600").
4. Range, retries and timeout under *Search settings*, as in mbusio's primary scan.
5. CRC errors are counted per address; many of them point to wiring (section 8), not to the device.

### 5.3 Register explorer

For a device without a profile, or to check a profile against the device:

- Read an area (coils, discrete inputs, input registers, holding registers) over a range, in blocks
  sized to what the device takes; a block answered 02 is halved until the single addresses that
  exist are known (gaps are remembered per device).
- Each register with its raw value and the readings of it: u16, s16, hex, bits; with its neighbour
  as u32, s32, float32 in all four orders (ABCD, CDAB, BADC, DCBA); as text (two characters per
  register).
- **Plausibility:** for 32-bit values the order that gives plausible numbers across the range is
  suggested, not chosen silently. A single pair is marked plausible only in the chosen order: with
  swapped bytes two unrelated registers often make a float of a likely size.
- Live: the rows on screen read again every 1–2 s, changes marked.
- Anything found becomes a point of the device's profile with one click (name, type, unit, scale).

## 6. Profiles (register maps)

The heart of modbusio. A **profile** describes a kind of device; a device on the site uses one.

| Field | Content |
| --- | --- |
| area | coil, discrete input, input register, holding register |
| address | as the device wants it on the wire (0-based); the convention of the source is kept beside it |
| type | bool, u16, s16, u32, s32, u64, s64, float32, float64, text(n), bits of a register |
| order | word and byte order for multi-register types (default per profile, per point overridable) |
| scale | factor and offset, or a scale register (`value × 10^SF`, as SunSpec and many meters do) |
| unit, name, group, description | what the user reads; groups as the tree's plant parts |
| state texts | numbers to words (`0 = off, 1 = heating, 2 = cooling`) |
| writable | whether a write is offered at all, with limits (min, max) where the source names them |

- **Import** from the vendor's list as CSV or Excel, with a preview: which column is what, which
  address convention (`40001`, `30001`, `0`-based, hex), and the first values read from the device
  beside each row to see at once whether offset and order are right.
- **Export** as CSV and Excel, the same columns, so a profile can be edited outside and loaded
  again.
- **Kept as files** in the data folder, one per profile, plain JSON; marked *own* or *imported*.
  A library of profiles shipped with modbusio comes later (open question 4).
- **Reading follows the profile:** points close together are read in one request, never across a
  known gap, never more than the device takes (125 registers, 2000 coils, or the device's own
  smaller limit learnt from exception 03 or a timeout).

## 7. Writing

Same rules as bacnetio: unlocked per session, confirmation in the page (never a browser dialog),
every write logged, refused or not, input checked at the server.

Modbus has no priority array and no relinquish. So:

- **Before every write the old value is read and logged.** *Changed by you* lists every point
  still differing from its old value, with *Restore* (one or all), also after a restart.
- **Read back after the write**; a device that answers OK and keeps the old value is shown so.
  Also after a write that got no confirmation (no answer, a damaged one): only an exception is a
  refusal. A device holding the new value then has taken the write, and it stands like any other.
- **Putting back** writes the value from before the first write of a run to a place, and the
  confirmation names that value and the place. A value written back by hand stands for nothing.
- **Leaving the site** lists what was changed and not restored, as bacnetio does for held values.
- Writes go only to points the profile marks writable, or in the explorer to a register after one
  more confirmation naming area and address.
- Coils and multi-register values: 05/15 and 16 by default; 06 for single registers where the
  device refuses 16 (remembered per device).
- Broadcast writes (unit 0) are not offered.

## 8. Diagnosis in words

Shown where the problem happens, as in bacnetio.

| Observation | Message (roughly) |
| --- | --- |
| serial port cannot be opened | "COM3 is used by another program" / "No permission: add the user to `dialout` once" |
| frames from another master while listening | "Another master is active on this trunk: searching now would collide." |
| CRC errors, answers at one speed only partly | "Answers arrive damaged: check A/B, termination, bias, cable." |
| answers only with another parity | "The devices use 8N1, not 8E1." |
| exception 01 / 02 / 03 / 04 | in words, with the address and a hint (wrong area, gap in the map, too many registers, device fault) |
| exception 0A / 0B | "The gateway has no path / the device behind the gateway does not answer: unit ID or its serial side." |
| values in the list look shifted by one | "The list may count from 40001: try offset −1." (suggested, not changed silently) |
| float values absurd in one order, plausible in another | "Word order looks like CDAB." |
| TCP connection reset after the first request | "The device takes one connection; close the other program." |
| timeout on a block, answer on single reads | "The device takes at most N registers per request; reading in smaller blocks." |

## 9. Stack

Taken over from bacnetio (its section 5), the reasons are the same:

- **Rust**, one binary, local web server with the UI embedded, Android as a WebView shell.
- **UI** in `web/`: plain HTML, CSS and JavaScript modules, no build step, bacnetio's tokens and
  components copied.
- **Protocol (decided in M0, 2026-11-08):** modbusio's own framing in `src/protocol/` (PDU, MBAP,
  RTU with CRC), without I/O, about 600 lines with the spec's examples as tests. tokio-modbus
  0.17 would have hidden what modbusio needs: the raw bytes for the capture, CRC errors to count,
  frames heard while listening, RTU over TCP on one code path with the serial line, late answers
  told apart. tokio-modbus serves as the **other side** in the simulators and tests instead, an
  implementation independent of modbusio's. Serial ports through `tokio-serial` as in bacnetio.
- **Layers:** server (HTTP) → logic (find, read, profiles, write, diagnosis) → transport (TCP,
  serial). The logic knows no HTTP, the server knows no Modbus.
- **Connections (`link.rs`, `links.rs`):** one per line, shared by everything.
  - A TCP connection is closed after 10 seconds without a request (`IDLE_CLOSE`): many devices
    take one or a few connections, and the plant's own master must get in again after a search.
    A serial port stays open; *Listen* closes it.
  - A read that finds the connection closed by the device is sent once more on a new one. A
    write never: the device may have taken the first.
  - What arrived of a TCP answer stays with the connection when the wait runs out, so a late
    answer is skipped by its transaction number instead of shifting every answer after it. A port
    answering something that is no Modbus TCP closes the connection (code `io`).
  - Two-wire adapters without direction control give back what they send. The echo is taken off
    before the answer, also when it arrives in pieces. The answer to 05, 06 and 08 repeats the
    request: whether a line echoes is learnt from the reads before (every write reads first), and
    where it is not known yet, the request coming back once with nothing after it is the answer.

## 10. Android

As bacnetio: Kotlin app, Rust core as a process, WebView on `127.0.0.1`. Modbus TCP works
directly. RTU on Android needs a USB serial bridge in Kotlin (mbusio-android has one for M-Bus);
open question 5.

## 11. Taken from bacnetio

Copied, no shared crate until after M2 (section 14, answer 6):

- server with Host/Origin guard, no CORS, `--web-port`, `--no-browser`, `--data-dir`
- the interface: shell, tokens, tree, tables, details, keys, help, languages, theme
- write journal and unlock, leaving check; recording, watch list, CSV and Excel export
- sites kept and named; capture for Wireshark (Modbus TCP as Ethernet/IP/TCP frames; RTU in the
  user link type Wireshark's `mbrtu` dissector reads)
- update check, builds, CI, release flow, Android shell
- the project rules (`AGENTS.md`), adapted (section 13)

Default web port **8082**, so bacnetio (8081) and mbusio can run beside it.

## 12. Testing and milestones

Testing as bacnetio (section 11 there): simulators in `examples/` built on tokio-modbus' server, on
loopback for TCP and on a pseudo-terminal for RTU (as `simulated_mstp`); a simulator for each field
behaviour (gaps, small block limit, CDAB floats, scale registers, a gateway with units behind it,
one connection only, slow answers). Never a real device in a test. Browser tests in Chromium,
Firefox and WebKit, in a network namespace.

| | Goal | Done when |
| --- | --- | --- |
| M0 | check tokio-modbus and serial timing | TCP and RTU read against a simulator and one real device; listening without sending works |
| M1 | CLI | `modbusio scan` (TCP subnet, RTU unit IDs), `read` (area, range, type, order), diagnosis as text |
| M2 | web UI | workspace, explorer, profiles with import, write with unlock and restore |
| M3 | field-ready | field test on a real trunk and a TCP site; first public release |
| M4 | Android | TCP on the phone; RTU per open question 5 |

**State 2026-11-08:** M0 done (own framing against tokio-modbus over TCP, RTU over TCP and a
pseudo-terminal), M1 done (the command line), M2 done (web interface with every function of
section 3.2, English and German, tests in Rust, `node --test` and the browsers in Chromium,
Firefox and WebKit). M4 built: the Android app (TCP), not yet tried on a phone. The check for a
new version (section 11) is built. French, Italian and Spanish are drafts (section 4), opened with
`?lang=` until someone from the trade has reviewed them. Open: M3 (a field test on a real line and
a real TCP site, the first release).

## 13. Skills and project rules

`AGENTS.md` as bacnetio's (English in the repo, German with the maintainer, branches and PRs,
"never lose a function", languages, real plants), with Modbus words. Project skills in
`.claude/skills/`, each created when the code it names exists, so none describes paths that are not
there. Built on 2026-11-08: all below except `plugin`.

| Skill | From bacnetio | What changes for Modbus | When |
| --- | --- | --- | --- |
| `concept` | same | — | now |
| `pull-request` | same | repo name | with the GitHub repo |
| `builds`, `release` | same, only on explicit request | file names | with CI |
| `review` | adapted | writing safety without priorities: old value logged, read back, restore, leaving check; block reads across gaps | with the first write |
| `field-data` | adapted | evidence is a register dump, serial settings, exception codes, a capture; known causes from section 1 | with M0 |
| `simulator` | adapted | tokio-modbus servers on loopback and pseudo-terminal; behaviours from section 12 | with the first simulator |
| `browser-tests` | adapted | other simulators and ports | with the first UI test |
| `try-it` | adapted | port 18082, simulators | with the UI |
| `api-route` | same pattern | Modbus logic instead of BACnet | with the server |
| `new-text` | same | glossary with Modbus terms (Holding-Register, Unit-ID, Wortreihenfolge) | with the UI |
| `standards` | adapted | Modbus Application Protocol 1.1b3 and Serial Line 1.02 (modbus.org, free) | when a rule is taken from them |
| **`register-map`** | new | transcribe a vendor's register list into a profile exactly: by machine, address convention named, types and scale checked against the document and against the device in the explorer, each point with its page in the source | with profiles (M2) |
| `plugin` | not now | only if modbusio gets bacnetio's plugin system | later |

## 14. Answers

Answered by the maintainer on 2026-11-08:

1. **RTU and TCP both in version 1**: RTU is the more common variant in building services and
   where the technician needs help most.
2. **Writing in version 1**, with unlock, old value and restore (section 7).
3. **Repository:** public on GitHub as `Wobi848/modbusio`, MIT, like bacnetio.
4. **No profiles shipped in version 1**: first own and imported profiles; a library later and
   apart from the repo, because each profile must be checked against the vendor's document (skill
   `register-map`).
5. **RTU on Android** through a USB serial bridge as in mbusio-android: after M4, only with a phone
   and an adapter to test with.
6. **Copy from bacnetio, no shared crate yet.** After M2, compare both projects and move into a
   shared crate only what stayed the same in both (likely: server guard, write journal, UI shell
   and tokens, languages). A shared crate now would couple two release cycles before it is known
   what really means the same.

Open: nothing yet.
