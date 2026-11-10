# Changelog

## Unreleased

First version.

### Added

- Search a TCP subnet for Modbus devices (port 502, units 1, 255 and 0), naming devices that answer
  every unit and gateways; the units behind a gateway, over Modbus TCP or RTU over TCP; the units
  on an RS-485 line over a USB adapter, with listening first and finding speed and frame.
- Read Device Identification (43/14) for vendor, product and version.
- Profiles: what a device's registers mean, with types, word orders, scaling, scale registers,
  units, state texts and limits; import of vendor register lists from CSV and Excel with the
  address convention recognised and a preview read from the device; export as CSV and Excel.
- Register explorer: existing addresses found by halving refused blocks, every register's readings,
  the plausible word order suggested, points taken into the profile.
- Reading by profile in blocks sized to what each device takes, learnt and kept.
- Writing behind the session unlock with a confirmation, the old value read and logged first, read
  back; *Changed by you* with putting old values back, also after a restart. A write the device
  did not confirm is read back too, and putting back names the value it writes.
- Watch list, recording with charts, CSV and Excel exports, capture for Wireshark, several sites.
- Connections kept out of the plant's way: a TCP connection is closed after 10 seconds without a
  request, so a device taking only one lets its own master in again; RS-485 adapters that give
  back what they send are recognised.
- Command line: `interfaces`, `ports`, `scan`, `listen`, `read`, `explore`, `write`, `changed`,
  `restore`.
- Interface in English and German; French, Italian and Spanish as drafts for review (`?lang=`).
- A note in the sidebar when a newer release is out, looked for once a day; can be switched off.
- Android app with the same interface, Modbus TCP only.
