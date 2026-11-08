---
name: standards
description: "Take rules from the Modbus specifications (Application Protocol 1.1b3, Serial Line 1.02, Messaging on TCP/IP 1.0b) or another standard into modbusio exactly: read from the source, each rule with its chapter, nothing stricter than the source. Use whenever a limit, a frame format, a timing or an exception comes from a specification, when the maintainer says a rule must be correct (\"muss 100% stimmen\"), or asks where a rule comes from."
---

# Rules from the specifications, exactly

A check stricter than the source refuses devices that work. Every rule must be traceable to a
sentence or table of the source.

## Sources

- The Modbus specifications are free at modbus.org (*Specifications*). Keep the PDFs outside Git
  (`reference/`, listed in `.git/info/exclude`).
- Search them before asking the maintainer anything a source answers.

## Steps

1. **Read the source, do not recall it.** `pdftotext -layout` for PDFs.
2. **Name the chapter** beside the rule in the code (`spec 6.3`, `Serial Line 2.5.1.1`), as
   `src/protocol/` does.
3. **Test the source's own examples**: the spec's request and answer bytes (`pdu.rs`), the CRC
   example (`rtu.rs`).
4. **Only what the source demands**: devices deviate (8N1 instead of 8E1, fewer registers per
   request); modbusio meets them where they are and says so in words, it does not refuse them.
