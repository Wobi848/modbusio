<img src="web/icon.svg" alt="" width="96" align="right">

# modbusio

Modbus browser for service and commissioning: finds devices on a network, behind a gateway or on an
RS-485 line, makes their registers readable as named values with units, and says in plain words
why when something fails. bacnetio's sibling for Modbus; see [docs/concept.md](docs/concept.md).

## Download

[Releases](https://github.com/Wobi848/modbusio/releases/latest): `modbusio-windows-x86_64.exe`,
`modbusio-linux-x86_64` and `modbusio-android.apk`. Nothing to install: start the program, and the
browser opens. On Android, allow installing from this source.

## Usage

`modbusio` without a command starts the local web server on http://127.0.0.1:8082/ and opens the
browser (`--web-port`, `--no-browser`, `--data-dir` for the write log, profiles, sites and the
recording backup). The interface is bacnetio's: a sidebar with the site and the search settings,
a page head with the one next step, list and details side by side, a status bar, light and dark,
the system's language (English or German; French, Italian and Spanish as drafts for review
with `?lang=fr|it|es`), keys `1`–`6` for the areas, `7` for the settings, `/`
to search the points, Escape to close.

- **Search** (*Search settings* in the sidebar): a **network** (every address of the subnet asked
  for port 502, then units 1, 255 and 0 on each open one; a device answering every unit is shown
  once, a gateway is named), the units **behind a gateway** (Modbus TCP, or RTU over TCP for serial
  gateways in transparent mode), or a **serial line** over a USB RS-485 adapter (Windows and Linux):
  *Listen first* hears without sending whether another master is active and at which settings;
  *Find speed and frame* tries the usual ones; the search asks units 1–247 one by one. Devices found
  join the open site with what they say about themselves (43/14 Read Device Identification).
- **Workspace**: the site's lines and devices as a tree, the device list, a device's points from
  its profile grouped by plant part, live every two seconds; the chosen point with its value, raw
  registers, type, word order and scaling. What a device refused is learnt and kept (registers per
  request, no reading across gaps, 06 for single registers).
- **Registers** (explorer): which addresses of an area exist (blocks the device refuses are halved
  until each address is known) and what each register could mean: u16, s16, hex, text, and with
  the next register u32, s32 and float32 in each word order, the plausible order suggested. What
  you recognise goes into the device's profile with one click.
- **Profiles**: what the registers of a kind of device mean: area, address, type, word order,
  factor and offset or a scale register, unit, state texts, writable with limits. **Import** the
  vendor's register list as CSV or Excel: the columns and the address convention (`40001`,
  `30001`, from 1, from 0, hex) are recognised, and the preview reads the first points from the
  device, so a wrong offset or word order shows at once. Export as CSV and Excel, which import back
  the same.
- **Writing** behind the session unlock and one more confirmation each: the old value is read and
  logged first (without it nothing is written), the new one read back. **Changed by you** lists
  every write still standing, with *Put back* one or all, also after a restart; leaving the page
  while values stand asks first. Raw registers can be written from the explorer after a
  confirmation naming the address.
- **Watch list** across devices, **recording** every 5 s to 1 min as charts, CSV and Excel.
- **Capture for Wireshark** (*Settings*): modbusio's own traffic as pcapng. Modbus TCP opens as it
  is; RTU frames use the user link type 147: in Wireshark map *DLT_USER* User 0 (147) to `mbrtu`.
- **Sites**: several kept, named, opened again, saved to and read from a file with their profiles.
- **Exports**: device list, a device's points, the whole site in one Excel file.

```text
modbusio interfaces                          list network adapters (* = used by default)
modbusio ports                               list serial ports, USB adapters first
modbusio scan                                search the adapter's subnet on port 502
modbusio scan --subnet 192.168.10.0/24       search a plant's subnet, e.g. behind a VPN
modbusio scan --gateway 192.168.10.50 --to 32          the units behind a gateway
modbusio scan --gateway 192.168.10.50:4001 --rtu       … passing RTU over TCP
modbusio scan --serial /dev/ttyUSB0 --baud 19200 --frame 8N1
modbusio scan --serial COM3 --find           find speed and frame first, then the units
modbusio listen COM3                         listen without sending: another master?
modbusio analyse COM3                        bus analyser: each exchange of the trunk's master
modbusio analyse COM3 --baud 19200 --frame 8E1 --seconds 600 --pcap trunk.pcapng
modbusio read 192.168.10.20/1 ir 0 3 --type f32        three float32 from input register 0
modbusio read serial:COM3:9600:8E1/5 hr 10 --factor 0.1 --csv values.csv
modbusio explore 192.168.10.20/1 hr 0 200    which addresses exist, what they could mean
modbusio write 192.168.10.20/2 hr 1 3000     reads the old value, writes, reads back, logs
modbusio changed                             values written and not put back yet
modbusio restore 4                           put back write number 4 (or: restore all)
```

`analyse` listens to a trunk through a USB RS-485 adapter hung on it in parallel and never sends:
connect A, B and GND anywhere on the trunk, with the adapter's own termination and bias switched
off (the trunk has them already). It finds speed and frame by listening unless they are given,
prints each request of the trunk's master with its answer and answer time, and ends at Ctrl+C or
after `--seconds` with a table per unit: requests, answers, no answer, damaged answers, exceptions,
answer times and poll cycle. Times are when the adapter handed the bytes over: a USB adapter waits
up to 16 ms before it passes on a small chunk (FTDI's default *latency timer*), so answer times are
exact to about that.

A device is named by its line and unit ID: `192.168.10.20/1` (TCP, port 502),
`tcp:192.168.10.20:5020/1`, `rtu-tcp:192.168.10.50:4001/2`, `serial:/dev/ttyUSB0:19200:8N1/5`.
Addresses are as on the wire, from 0: holding register `40001` in a vendor's list is `hr 0`.
`write`, `changed` and `restore` use the same log as the interface. `read` and `explore` end with
exit code 1 when no value could be read, so a script sees it.

On Linux the user must be in the group `dialout` once to open a serial port.

To try it without a plant, start the simulators on loopback addresses:

- `cargo run --example simulated_meter -- 127.0.0.2 5020` an electricity meter on Modbus TCP,
  answering every unit, floats in ABCD, identification; `--one-connection` takes one connection at
  a time.
- `cargo run --example simulated_gateway -- 127.0.0.3 5020` a gateway with a heat meter (unit 1,
  floats CDAB, a scale register), a drive (unit 2, gaps, 16 registers per request, refuses 16 for
  one register) and a room controller (unit 5); other units answer exception 0B. `--rtu` passes
  RTU frames over TCP instead.
- `cargo run --example simulated_rtu` (Linux) the same three devices on a pseudo-terminal; it prints
  the terminal's name (`/dev/pts/7`), which modbusio opens like a USB RS-485 adapter.
- `cargo run --example simulated_trunk` (Linux) a whole trunk for the bus analyser: a master
  polling units 1, 5, 7, 9 and 12 on a pseudo-terminal, with a silent unit, a refused block, slow
  and missing answers, damaged frames, writes and broadcasts; `--clean` without the faults,
  `--second-master`, `--cycle-ms 2000`.

### New versions

Once a day modbusio asks GitHub whether a newer release is out and, if so, shows a link to it in
the sidebar. GitHub sees the computer's internet address, nothing else; modbusio never downloads or
installs anything. *Settings → About* switches the check off, and then modbusio does not go online
at all. Without internet, as on most plants, nothing is shown.

## Build

```sh
cargo build --release
cargo test
node --test tests/web/*.test.js   # interface logic, no browser needed
node tests/ui/run.mjs             # the interface in a browser (tests/ui/README.md)
```

The interface lives in `web/` (plain HTML, CSS and JavaScript modules, no build step); the JSON
API it uses is described in [docs/api.md](docs/api.md).

## Android

The app in [android/](android/README.md) runs the same program with the same interface on a phone,
Modbus TCP only (Android gives a program no serial port).

## Protocol

The Modbus framing (PDU, MBAP, RTU with CRC) is modbusio's own, in `src/protocol/` without I/O, so
the capture, the CRC counts, listening to a line and RTU over TCP see the bytes. The simulators and
tests use [tokio-modbus](https://github.com/slowtec/tokio-modbus) (MIT/Apache-2.0) as the other
side, an implementation independent of modbusio's.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md); security problems please as described in
[SECURITY.md](SECURITY.md), not as a public issue.
