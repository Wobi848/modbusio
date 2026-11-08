---
name: register-map
description: "Transcribe a vendor's Modbus register list (PDF, Excel or CSV manual of a meter, drive, heat pump or controller) into a modbusio profile exactly: by machine, the address convention named, types, word order, scaling and scale registers checked against the document and against the device in the explorer, each point with its page or row in the source. Use when the maintainer brings a device's manual or register list, asks for a profile of a device, or a profile reads wrong values."
---

# A vendor's register list into a profile

A profile that reads a wrong register looks right and is wrong: every point must be traceable to
the vendor's document and checked against the device where one is at hand.

## Steps

1. **Read the document, do not recall it.** `pdftotext -layout` for a PDF; an Excel or CSV list
   goes through the import in the interface or `profile_import::import`.
2. **Transcribe by machine** into CSV with the columns of an exported profile (`Name; Group; Area;
   Address; Type; Order; Factor; Value offset; Scale register; Unit; Description; States; Access;
   Min; Max`), never type a table by hand. Keep the source's address in `source_address`.
3. **Name the convention** the document uses: from 0 on the wire, from 1, or 40001 with the area in
   front; hex addresses. Read the sentence of the document that says it; where it says nothing, the
   import's guess and the device decide (step 5).
4. **Types and scaling** as the document states them: signed or unsigned, 32 bits as two registers
   in which order, a factor (`× 0.1`, `/10`) or a scale register (`SF`, `10^n`). State texts from
   the document's tables.
5. **Check against the device** where one is at hand: the import's preview reads the first points;
   the explorer shows the registers with every word order. Wrong values mean another convention or
   order, not a wrong device. Never write to check.
6. **Writable** only what the document marks writable, with its limits.
7. Profiles are kept apart from this repo until the library of profiles exists (concept section
   14, answer 4); names and numbers of a real site never enter the repo.

Answer the maintainer in German, with the document's page or row for each claim.
