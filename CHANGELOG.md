# Changelog

## Unreleased

First version.

### Added

- Search a TCP subnet for Modbus devices (port 502, units 1, 255 and 0), naming devices that answer
  every unit and gateways; the units behind a gateway, over Modbus TCP or RTU over TCP; the units
  on an RS-485 line over a USB adapter, with listening first (it names the unit IDs heard) and
  finding speed and frame.
- Devices set to another speed, parity or stop bits than their trunk found by asking every unit
  at every setting, after listening first and only while the plant's master is stopped; named in
  plain words and added to the site at the setting they answer at (also `scan --other-settings`).
- Read Device Identification (43/14) for vendor, product and version.
- Profiles: what a device's registers mean, with types, word orders, scaling, scale registers,
  units, state texts and limits; import of vendor register lists from CSV and Excel with the
  address convention recognised, the access column deciding what may be written, and a preview read
  from the device; export as CSV and Excel.
  Closing a profile with changes not saved asks first.
- Register explorer: existing addresses found by halving refused blocks, every register's readings,
  the plausible word order suggested, points taken into the profile.
- Reading by profile in blocks sized to what each device takes, learnt and kept.
- Writing behind the session unlock with a confirmation, the old value read and logged first, read
  back; *Changed by you* with putting old values back, the newest first, also after a restart. A write the device
  did not confirm is read back too, and putting back names the value it writes.
- Watch list, recording with charts, CSV and Excel exports, capture for Wireshark, several sites.
- Connections kept out of the plant's way: a TCP connection is closed after 10 seconds without a
  request, so a device taking only one lets its own master in again; RS-485 adapters that give
  back what they send are recognised.
- Command line: `interfaces`, `ports`, `scan`, `listen`, `read`, `explore`, `write`, `changed`,
  `restore`.
- Bus analyser for RS-485 trunks, listening without ever sending through an adapter hung on the
  trunk in parallel: each request of the trunk's master paired with its answer, answer times, poll
  cycle, no answers, damaged answers and exceptions per unit, the frames for Wireshark. While it
  listens, modbusio sends nothing on that port. A page of its own (*Bus analyser*) with the units
  and the exchanges, filters, the bytes of each frame, and units added to the site; the registers
  the master uses with every reading, the profile's points and their changes as a chart; the
  master's writes with the value before them; the raw frames; answer times as a chart; what is
  wrong in plain words (a silent unit, answers missed after a neighbour, a block refused every
  time, slow answers, damage at one unit, a second master, a busy trunk); units,
  exchanges and registers as CSV and Excel; a draft profile from the registers the master uses;
  capture files (pcapng or pcap, also from other tools) opened into the same views; on the
  command line `analyse`, and `analyse --file` for a saved capture. The keys count up: the bus analyser is `7`, *Settings* now `8`.
- The tree groups devices by their wire (serial port or TCP address), folds open and shut, and marks a
  device set to other serial settings than most on its wire; a wire whose devices are not all set alike
  shows it as a fault, in the tree and above its device list.
- Tables show all their columns while their list has room and fold to the few that matter below
  that, by the width of the list instead of the window; between 1000 and 1200 px of window the
  list uses the room the hidden details left empty.
- A line in the tree can be chosen: the list shows its devices, and the whole line can be removed
  from the site at once, after a question.
- Serial ports without a chip behind them (Linux lists `ttyS0` to `ttyS3` on every PC) are left
  out of the port lists.
- Sidebar, tree and details as wide as you drag them (or move their border with the arrow keys),
  kept in this browser; a double-click gives the width back to the window.
- Lists sorted by a click on a column head, and their columns as wide as you drag them: devices,
  points, profiles, the watch list and the bus analyser's tables.
- The arrow keys, Page Up/Down, Home and End move through the list on screen and through an open
  menu; Enter opens a marked device or profile; Tab stays inside a dialog.
- Toolbar menus stay inside the window; network adapters and serial ports are read again while
  the search settings are open.
- Charts name every line's value at the time under the pointer, show a span drawn with the pointer
  closer (*Show everything* goes back) and give each line's lowest, highest and mean.
- Help (sidebar or the key `?`): guides for the usual tasks, from searching a network, a gateway or
  an RS-485 line to register lists, writing, recording and the bus analyser, and the keys.
- Accent colour (blue, crimson, violet, teal) and a background for the light and for the dark theme,
  chosen in the settings and kept in this browser.
- Interface in English and German; French, Italian and Spanish as drafts for review (`?lang=`).
- A note in the sidebar when a newer release is out, looked for once a day; can be switched off.
- Android app with the same interface, Modbus TCP only.
