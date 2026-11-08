---
name: field-data
description: "Evaluate what the maintainer brings back from a real Modbus plant (screenshots, a register dump from the explorer, a point list CSV, a vendor's register list, a capture, console output, exception codes like \"exception 02\" or \"no answer\") and turn each problem into a fix with a simulator test. Use whenever the maintainer sends results from a site, a VPN session, a serial line or another Modbus tool (Modbus Poll, QModMaster, a vendor's tool), or says something is slow, missing or wrong on a real device."
---

# Evaluating results from a real plant

The data may hold real addresses and names: never copy them into the repo, tests, issues or
commits. Use made-up names and the simulators instead.

## Steps

1. Read the evidence before guessing:
   - **Explorer** (*Registers*): which addresses answered, which exception, the learnt limit
   - **Capture** (*Settings → Capture for Wireshark*): requests and answers byte by byte; on RTU
     whether answers came at all, damaged (CRC) or from another unit
   - **Device details** (*Learnt*): registers per request, no reading across gaps, 06 for single
     registers
   - **The vendor's list**: the address convention (40001 or 0), word order, scale registers,
     signed or unsigned
   - **Console window** of modbusio: a port in use, no permission, files that could not be saved
2. Name the cause in one sentence, with the code that explains it (file and line).
3. Ask only for what is missing, specifically: which screen, which file, which button.
4. Turn each cause into a fix with a test against a simulator (skill simulator) that shows the
   same behaviour.
5. Write what was learnt as a known cause below when it will come again.

## Known causes

- Values shifted by one register: the list counts from 40001 or from 1 (import: *The list counts*).
- Absurd floats (`1.2e-38`, `4.2e9`): another word order; the explorer suggests the plausible one.
- Exception 02 for a block although every point exists: the device refuses reading across gaps;
  modbusio learns it and reads only the points.
- Exception 0B from a gateway: the unit behind it does not answer (unit ID, its serial side).
- Timeouts on RTU at one setting and damaged answers at another: speed or parity; *Find speed and
  frame*, then wiring (A/B, termination, bias).
- A TCP device closing the second connection: another program holds its only connection.

Answer the maintainer in German.
