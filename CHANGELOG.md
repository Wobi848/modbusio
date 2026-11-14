# Changelog

## Unreleased

First version.

### Added

- Search a TCP subnet for Modbus devices (port 502, units 1, 255 and 0), naming devices that answer
  every unit and gateways; the units behind a gateway, over Modbus TCP or RTU over TCP; the units
  on an RS-485 line over a USB adapter, with listening first (it names the unit IDs heard) and
  finding speed and frame.
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
  and the exchanges, filters, the bytes of each frame, and units added to the site; on the
  command line `analyse`. The keys count up: the bus analyser is `7`, *Settings* now `8`.
- Interface in English and German; French, Italian and Spanish as drafts for review (`?lang=`).
- A note in the sidebar when a newer release is out, looked for once a day; can be switched off.
- Android app with the same interface, Modbus TCP only.
