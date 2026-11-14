# Concept: catching up with bacnetio's interface

Status: **approved on 2026-11-14** (the maintainer: "ok gogog"); step 1 built on branch
`bacnetio-panels`.

## Why

The maintainer, trying modbusio (2026-11-14): "die Dinger sind nicht schiebbar. In bacnetio ist
alles anpassbar. Ich sehe das Problem, dass wir Dinge nicht abgleichen. Die Tools sind ja eigentlich
fast gleich vom Aufbau her."

modbusio took bacnetio's interface on 8 November (`docs/concept.md` section 11) and has not looked
back since. bacnetio grew meanwhile: 73 modules in `web/` against modbusio's 23. `docs/concept.md`
section 14, answer 6 planned to compare both after M2; that comparison was never made. Some of what
is missing was even half taken over: `list.css` and `layout.css` carry the styles of the resize
borders, but no page draws them.

## What bacnetio has, and what modbusio does with it

Each of bacnetio's modules that is not about BACnet itself, with what modbusio has today and what
it should do. Copied parts are marked in their head ("Copied from bacnetio") for the comparison
after M2 (AGENTS.md).

| bacnetio | What it does | modbusio today | Proposal | Step |
| --- | --- | --- | --- | --- |
| `panels.js` | sidebar, tree and details resized by dragging or arrow keys, double-click back to automatic, kept per browser | styles only | take over | 1 |
| `columns.js` | table columns resized by dragging their head's border, double-click back, kept per table | none | take over, with modbusio's tables | 2 |
| `sorting.js` | lists sorted by a click on a column head: numbers as numbers, addresses in natural order, empty last | none | take over: devices, points, profiles, registers, the analyser's units and exchanges | 3 |
| `keys.js` | arrows, Page Up/Down, Home/End move through lists and menus | digits, `/`, Escape | take over the arrows | 4 |
| `menus.js` | a toolbar menu placed against the window, never cut off | menus can be cut off at the work area's edge | take over | 4 |
| `chart.js`, `chart-pointer.js`, `analysis.js` | crosshair with the values at a time, a window of time, key figures (min, max, mean), fewer samples where more than the chart can show, zoom by drawing a span | plain lines with two axis labels | take over for the recording and the analyser's charts | 5 |
| `help.js` | guides for the usual tasks in the interface's language (`concept-help.md`) | README only | take over the frame; write Modbus guides (search a trunk, a gateway, a vendor's list, word order, writing, the bus analyser) | 6 |
| `excel.js` | Excel files with column heads and a title in the reader's language | English heads | take over | 7 |
| `print.js` | the open device's points as a page to print or save as PDF | `print.css` only | take over | 7 |
| `site-search.js` | one search over the whole site from the sidebar | the point filter of one device | take over: devices, points of the devices read | 8 |
| `compare.js` | two devices side by side, their points matched by name | none | take over: two devices with the same profile | 8 |
| `drag-toggle.js` | watch and record switched on many points by dragging over them | switches in the details only | first switches in the point list, then this | 9 |
| `problems.js` | everything that needs a look across the devices read | the *Faults* filter of one device | take over: points that fail, devices that do not answer, values you hold | 9 |
| `adapter-actions.js` | network adapters read again while the page is open | read when the search settings open | take over | 4 |
| `theme.js` accents | accent colour and background of each theme | light and dark | take over | 7 |
| `change-many.js` | one value written to many points at once | none | **a concept of its own**: it writes to plants | later |
| `memory.js`, `live-list.js`, `log.js`, `watch.js`, `recording.js`, `capture.js`, `sites.js`, `update.js`, `i18n.js` | the same jobs | done in modbusio's own modules | compare after M2 for a shared crate | – |
| `bbmd*`, `mstp*`, `sc*`, `structure*`, `properties.js`, `file-actions.js`, `device-commands.js`, `grouping*.js`, `point-query.js`, `plugin*` | BACnet's own (plugins: `concept.md` section 13, not now) | – | not taken | – |

## Proposal: what the user sees, step 1

As bacnetio (`docs/concept-hierarchy.md` there): on a wide window, the border right of the
sidebar, between tree and list, and between list and details can be dragged. The arrow keys move a
focused border by 16 px; a double-click hands the width back to the window. The widths are kept in
this browser. The list in the middle never gets narrower than 360 px. On a phone and where the
details slide over the list, there are no borders to drag.

## Writing to plants

None of the steps writes, except *change many*, which gets a concept of its own.

## Functions and where they live

Each step adds its rows to `docs/concept.md` section 3.2 when built; nothing moves.

| Function | Place | Test |
| --- | --- | --- |
| Sidebar, tree and details resized, kept, back to automatic | every page with them | `ui/07-settings` |

## Compatibility

Nothing stored on the server changes; widths and sorting live in the browser. `CHANGELOG.md`
names each step.

## Order of building

The steps of the table, each its own pull request: 1 panels, 2 columns, 3 sorting, 4 keyboard,
menus and adapters, 5 charts, 6 help, 7 Excel, print and accents, 8 site search and compare, 9 list
switches with dragging and problems across the site.

## Open for the decision

1. **The order.** *Recommendation:* as above: first what the maintainer missed (panels, columns,
   sorting), then the daily work (keyboard, charts), then what helps others (help, print).
2. **Change many** gets its own concept before anything is built. *Recommendation:* yes; it writes.
