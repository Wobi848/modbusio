# Security

modbusio talks to building automation: it can change values and settings in a running plant. A
flaw that lets a page, a file or another program make it write, or that exposes its web interface
beyond the computer it runs on, is a security issue.

## Reporting

Please do not open a public issue for a security problem. Use GitHub's private vulnerability
reporting (*Security → Report a vulnerability* on the repository) and describe what you found and
how to reproduce it. The maintainer answers as soon as possible and fixes it before it is made
public.

## What modbusio does to keep a plant safe

- The web server listens on `127.0.0.1` only and refuses requests that do not name it
  (`src/server/guard.rs`), so other pages in the browser cannot use it.
- Every write needs the unlock of the session, checked by the server, and a confirmation in the
  page. The old value is read and logged first; without it nothing is written.
- Values written and not put back are listed until they are, also after a restart, and leaving
  the page asks first.

Modbus itself has no authentication. Anyone on the building network or the RS-485 line can do what
modbusio does; keep that network separate from others.
