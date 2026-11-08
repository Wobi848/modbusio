---
name: builds
description: "Build modbusio for Windows (exe from Linux with zig), only when the maintainer explicitly asks for that build in the current request (\"bau mir die exe\", \"Windows-Version\"). Never offer it on its own initiative and never publish anything."
disable-model-invocation: true
---

# Windows build of modbusio

Only on the maintainer's explicit request in this conversation. Building is not publishing: no
tag, release, upload or push of the files.

```sh
pip install ziglang            # or a zig from the package manager; a venv in the scratchpad works
cargo install cargo-zigbuild
cargo zigbuild --release --target x86_64-pc-windows-gnu
```

The exe is `target/x86_64-pc-windows-gnu/release/modbusio.exe`. Copy it where the maintainer asks
(usually `~/Downloads`). Say that SmartScreen warns about an unsigned file. Say which commit and
branch it was built from, and that it is untested on Windows unless the maintainer tried it.
