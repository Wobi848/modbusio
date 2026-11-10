# Web API

modbusio's interface talks to this JSON API. It listens on `127.0.0.1` only and answers only
pages it served itself: a request whose `Host` or `Origin` names another site gets `403`, so no
web page elsewhere can write to the plant through it.

Errors come as `{"error": "text in words"}` with the status:

| Status | Meaning |
| --- | --- |
| 400 | the request is wrong (a field out of range, not a number, a name missing) |
| 403 | a foreign page |
| 404 | nothing of that name (device, profile, site, write) |
| 409 | not possible now: writing locked, a search running, no profile yet |
| 422 | the body is not JSON of the expected form |
| 502 | the line or the device did not do what was asked |

A **device key** names a device everywhere: its line and unit ID, `tcp:10.0.0.5:502/1`,
`rtu-tcp:10.0.0.9:4001/2`, `serial:COM3:9600:8E1/5`. Areas are `coil`, `discrete_input`,
`input_register`, `holding_register`. Types are `bool u16 s16 u32 s32 u64 s64 f32 f64`, `textN`
(N characters) and `bitN` (bit N of a register). Word orders are `ABCD CDAB BADC DCBA`.

A reading is `{value, text, raw, error}`: the number after scaling (or `null`), the value in words
with its unit or state text, the registers as hex, and an error code (`timeout`, `damaged`, `crc`,
`malformed`, `other_unit`, `closed`, `open`, `io`, `exception_XX`).

A note is `{code, ...values, text}`; the codes are those of `src/diagnosis.rs`, the interface
translates them by code and shows `text` where it has none.

## General

`GET /api/info` → `{program: "modbusio", version, serial, data_dir, problems}`. `serial` is false on
Android; `problems` lists files that could not be read at the start.

## Searching

`GET /api/adapters` → `[{name, ip, prefix_len, subnet, description, kind, default}]`; `kind` is
`cable`, `wireless` or `virtual`.

`GET /api/serial-ports` → `[{name, description, usb}]`, USB adapters first.

`POST /api/scan/tcp` `{subnet?, interface?, port?, timeout_ms?}` starts a search of a subnet (the
adapter's when `subnet` is left out; up to /22). `400` for a subnet too large or wrong, `409` while a
search runs.

`POST /api/scan/units` `{endpoint, from?, to?, timeout_ms?, retries?}` asks units `from`–`to` (1–247)
on one line: `tcp:IP:PORT` (a gateway), `rtu-tcp:IP:PORT`, `serial:PORT:SPEED:FRAME`.

`POST /api/scan/settings` `{port, units?, timeout_ms?}` tries the usual speeds and frames on a
serial port with units 1–10 (or the given ones) until one answers.

`GET /api/scan` → the report of the running or last search: `{phase, done, total, devices: [{key,
vendor, product, revision}], hosts: [{address, gateway, every_unit, answered}], notes, finished,
settings, estimate_seconds}`. When a search ends, its devices join the open site.

`POST /api/scan/stop` ends the running search; what it found so far stays.

`POST /api/listen` `{port, seconds?}` listens to a serial port at each usual setting without
sending → `{listened: {trials: [{settings, frames, stray_bytes, units}], active, quiet}, note}`.
`409` while a search runs.

## Sites and devices

`GET /api/site` → `{id, name, devices: [{key, name, kind, line, unit, profile, profile_name, points,
vendor, product, revision, limits}]}`. `limits` is what modbusio learnt: `{most_registers,
most_bits, no_holes, single_register_write}`.

`POST /api/site/rename` `{name}` → the site.

`GET /api/sites` → `[{id, name, devices, open}]`.

`POST /api/sites/new` `{name}` creates a site and opens it → the site.

`POST /api/sites/open` `{id}` → the site. `POST /api/sites/delete` `{id}` → the list of sites.

`GET /api/site.json` downloads the open site with the profiles its devices use.
`POST /api/site.json` with such a file opens it as a new site; its profiles join unless one of the
same id exists. The file is checked whole before anything of it is kept: `400` for a device key
that is none, a name over 80 characters, or a profile that could not be saved by
`POST /api/profiles` (its points, or an id that is no file name). A device named twice is there
once.

Opening another site (`sites/new`, `sites/open`, `site.json`, deleting the open one) answers `409`
while a search runs: what a search finds joins the open site.

`POST /api/devices/add` `{key, name?}` → the site with `check`: on Modbus TCP the device is asked
once whether it answers, `{answered, error}` with the error code when not; `null` on a serial line or
RTU over TCP, where another master may be active and nothing is sent unasked. The device is added
either way. `409` when the device is on it already.

`POST /api/devices/update` `{key, name?, profile?}` → the site; `profile: ""` removes the profile.

`POST /api/devices/remove` `{key}` → the site. Nothing changes in the device.

`POST /api/devices/identify` `{key}` asks Read Device Identification (43/14) →
`{objects: [{id, text}]}` and keeps vendor, product and version; `502` when the device does not
offer it.

## Reading

`POST /api/devices/read` `{key, points?}` reads the points of the device's profile (only the named
ones with `points`) → `{profile: {id, name, order}, points: [point fields, effective_order, count,
reading], failed, notes}`. `409` when the device has no profile. What the device refused is learnt
and kept (`notes`).

`POST /api/values` `{items: [{key, point}]}` reads points across devices, each device once →
`[{key, point, unit, reading}]` or `[{key, point, error}]`.

`POST /api/explore` `{key, area, start, count}` reads up to 2000 addresses, halving blocks the
device refuses → `{rows: [{address, value, error, hex, signed, text, pairs: [{order, u32, s32,
float, plausible}]}], order, found, stopped, limits, most_per_read, notes}`. `order` is the word order
that makes the floats plausible, where one does.

## Profiles

`GET /api/profiles` → `[{id, name, vendor, model, origin, points, used_by}]`.

`GET /api/profiles/get?id=` → `{profile, points}`.

`POST /api/profiles` with a whole profile `{id, name, vendor, model, origin, source, order,
points}` saves it; an empty `id` makes a new one from the name. A point is `{name, group, area,
address, type, order?, factor, offset, scale_register?, unit, description, states: [[number,
text]], writable, min?, max?, source_address}`. `400` for a point that cannot be (a coil of type
`f32`, an input register marked writable, two points of one name).

`POST /api/profiles/delete` `{id}` → the list; `409` while devices of the open site use it.

`POST /api/profiles/import` `{file_name, content_base64, options?, key?, order?}` reads a register
list (CSV, or the first sheet of an Excel or OpenDocument file) → `{imported: {header_row,
headers, columns, convention, convention_reason, points, skipped}, preview, rows, source}`. With
`key`, `preview` holds the readings of the first 40 points from that device. `options` may set
`header_row`, `columns` (role → column), `convention` (`zero_based`, `one_based`, `reference`) and
`area`. Nothing is saved.

`GET /api/profile.csv?id=`, `GET /api/profile.xlsx?id=`: a profile as a list that imports back the
same.

## Writing

`GET /api/session` → `{unlocked}`. `POST /api/session` `{unlocked}` unlocks or locks writing for
this session. Every write and restore is refused with `409` while locked.

`POST /api/write` `{key, point, value}` writes a point of the device's profile that is marked
writable; `{key, raw: {area, address, type, order?, factor?}, confirm_raw: true, value}` writes
without one. `value` is a number (decimal point or comma), a state text, or on/off. The old value is
read first: without it nothing is written. → the log entry `{id, time, device, point, area,
address, before, written, after, text, before_text, restores, error}`; a device that refused has
`error` set (status 200, it is logged). `400` for a value the point does not take, `409` for a
point not writable, `400` for unit 0, the broadcast. With `check: true` only the value is checked,
as the confirmation is asked: → `{checked: true, text}`, the value as it would be written; nothing
is read, written or logged.

`GET /api/journal` → `{entries: [entry, standing, kept_other, original, original_text], standing}`.
`standing` marks writes not put back yet; `original` is what restoring writes, in registers, and
`original_text` the same in words.

`POST /api/restore` `{ids}` or `{all: true}` writes the old values of standing writes again, each
logged as a write with `restores` → `{results: [{id, entry}|{id, error}]}`. The newest is put back
first, as an undo: a write over part of an earlier one goes before it.

`GET /api/journal.csv?offset=`, `GET /api/journal.xlsx?offset=`: the log; `offset` is the viewer's
time zone in minutes east of UTC.

## Recording

`GET /api/recording` → `{series: [{device, point, label, unit, samples: [[ms, value]]}], interval,
running}`.

`POST /api/recording/add` `{key, point}`, `POST /api/recording/remove` `{key, point}` → the
recording. At most 50 points.

`POST /api/recording/start` `{interval}` (5, 10, 15, 30 or 60 s), `POST /api/recording/stop`,
`POST /api/recording/clear` → the recording.

`GET /api/recording.csv?offset=`, `GET /api/recording.xlsx?offset=`: one row per time.

## Capture for Wireshark

`GET /api/capture` → `{running, frames, kept}`. `POST /api/capture/start`, `POST
/api/capture/stop` → the same. `GET /api/capture.pcapng` → the last capture stopped; `409` before
the first.

## Exports

`GET /api/devices.csv`: the site's devices. `GET /api/points.csv?key=`, `GET /api/points.xlsx?key=`:
a device's points, read now. `GET /api/site.xlsx`: the device list and a sheet per device with a
profile, each read now.

## New versions

`GET /api/update` → `{current, latest: {version, url} | null, newer}`: the newest release on GitHub,
read from the redirect of `…/releases/latest`; `502` without an answer. The page asks at most once a
day and only while *Look for new versions* is on; without that, modbusio never goes online.
