// The help's guides in English (docs/concept-bacnetio-sync.md, step 6). {btn:key} names a button or
// field by the interface's own text, {guide:id} another guide by its title; the order and targets
// are in help.js.

export default {
  find: {
    title: "Find devices on the network",
    intro: "modbusio asks every address of the plant's subnet on the Modbus port and lists what answers. Modbus has no call that every device answers, so each address is tried.",
    steps: [
      "Connect this computer to the plant's network.",
      "Open {btn:side.search} in the sidebar and choose {btn:search.tcp}.",
      "Choose the {btn:search.adapter} that leads to the plant; {btn:search.subnet} fills in from it.",
      "Press {btn:search.start}. The devices appear in the list as they answer.",
      "Choose a device. {btn:dev.read} reads its points once it has a profile; {btn:dev.explore} shows what it holds without one.",
    ],
    causes: [
      "Nothing found: check the network adapter. A laptop with Wi-Fi and a cable has two.",
      "The plant uses another port than 502: enter it under {btn:search.port}.",
      "A device that answers every unit ID is a gateway: see {guide:gateway}.",
      "One device is missing: {btn:dev.addByHand} adds it by its address.",
    ],
  },
  gateway: {
    title: "Find the devices behind a gateway",
    intro: "A gateway passes requests from the network on to an RS-485 line. The devices behind it share its address and differ by their unit ID.",
    steps: [
      "Open {btn:side.search} and choose {btn:search.gateway}.",
      "Enter the {btn:search.gatewayAddress}, with the port if it is not 502.",
      "Set {btn:search.from} and {btn:search.to} to the unit IDs in use; 1 to 247 asks them all and takes longer.",
      "Press {btn:search.start}.",
    ],
    causes: [
      "No unit answers: some gateways pass RTU frames on unchanged. Switch on {btn:search.rtuOverTcp}.",
      "Units answer slowly or only sometimes: raise {btn:search.timeout}; the line behind the gateway is slower than the network.",
      "Many gateways allow one connection only: close other Modbus programs.",
    ],
  },
  trunk: {
    title: "Search an RS-485 line",
    intro: "With a USB adapter for RS-485, modbusio is the master of a line and asks the unit IDs one by one. A line has one master only: never search while the plant's master is talking.",
    steps: [
      "Connect the adapter to the line (A to A, B to B, and the common wire) and plug it in.",
      "Open {btn:side.search} and choose {btn:search.serial}, then the {btn:search.serialPort}.",
      "Press {btn:search.listen}: it sends nothing and tells whether another master is talking, and which units it hears.",
      "Set {btn:search.baud} and {btn:search.frame} as the devices are set. Unknown? {btn:search.findSettings} tries them.",
      "Press {btn:search.start}.",
    ],
    causes: [
      "No serial port in the list: is the adapter plugged in? On Linux your user must be in the group dialout.",
      "Nothing answers at any setting: A and B are swapped more often than not. Swap them at the adapter.",
      "Some devices are missing: see {guide:othersettings}.",
      "Another master is talking: use {guide:analyser} instead, it only listens.",
    ],
  },
  othersettings: {
    title: "Find devices set to another speed or frame",
    intro: "A device set to another speed, parity or stop bits than its line never answers there, and it can disturb the others. modbusio asks every unit at every setting and names those that answer elsewhere.",
    steps: [
      "Stop the plant's master or disconnect it: this search sends at many settings.",
      "Open {btn:side.search}, choose {btn:search.serial} and set the line's own {btn:search.baud} and {btn:search.frame}.",
      "Press {btn:search.otherSettings}, read what it will do and how long, then {btn:search.otherSettingsNow}.",
      "The result names each unit and the setting it answers at. Such devices join the site at their own setting, marked in the tree.",
    ],
    causes: [
      "The search stops at once: it listened first and heard another master. Stop that one.",
      "A unit answers damaged at the line's setting: its parity or stop bits differ, or the line is not terminated.",
    ],
  },
  profile: {
    title: "Use a vendor's register list",
    intro: "A profile says what a device's registers mean: names, types, scaling, units. The vendor's list as CSV or Excel becomes one.",
    steps: [
      "Open {btn:tab.profiles} and press {btn:prof.import}; choose the file.",
      "Choose under {btn:imp.device} a device of that kind: the preview reads its values now.",
      "Check {btn:imp.convention}: lists count from 0, from 1, or write 40001 for the first holding register. Plausible values in the preview show the right one.",
      "Press {btn:imp.save}, then give the device this profile in its details in the workspace.",
    ],
    causes: [
      "Every value is one register off: the other convention is the right one.",
      "Large numbers make no sense: the word order differs, see {guide:explore}.",
      "A list as PDF only: copy its table into a spreadsheet and save it as CSV.",
    ],
  },
  explore: {
    title: "Look at registers without a list",
    intro: "The register explorer reads a range of addresses and shows each as unsigned, signed, hex, text and, with its neighbour, as a 32-bit number.",
    steps: [
      "Choose the device in the workspace and press {btn:dev.explore}.",
      "Set {btn:ex.area}, {btn:ex.start} and {btn:ex.count}, then {btn:ex.go}.",
      "Choose a register that changes with something you know (a temperature, a counter).",
      "Try {btn:ex.order} until a 32-bit value reads plausibly; values that make no sense are shown dimmed.",
      "{btn:ex.addPoint} keeps the register as a point of a profile.",
    ],
    causes: [
      "Whole ranges without data: devices refuse addresses they do not have. The explorer reads around the gaps.",
      "The device refuses large blocks: the explorer learns its limit and reads smaller ones.",
    ],
  },
  write: {
    title: "Write a value",
    intro: "Writing changes a running plant. modbusio asks before every write, reads and logs the old value first, reads back afterwards, and keeps a list of what you changed.",
    steps: [
      "Choose a writable point and press {btn:write.button}.",
      "The first write of a session asks to unlock: {btn:unlock.allow}.",
      "Enter the value and press {btn:write.next}. modbusio shows the old and the new value.",
      "Confirm with {btn:write.do}. The value read back stands beside the point.",
      "{btn:tab.changed} lists every value you wrote, with the value before, to put it back.",
    ],
    causes: [
      "Refused with exception 01 or 02: the device does not allow writing this register.",
      "Refused with exception 03: the value is outside what the device accepts.",
      "Written, but it reads back otherwise: the device or its controller overwrote it.",
    ],
  },
  record: {
    title: "Watch and record values",
    intro: "The watch list shows chosen points of several devices together; the recording keeps their values over time, as charts and as CSV or Excel.",
    steps: [
      "Choose a point in the workspace and switch on watching or recording in its details.",
      "{btn:tab.watch} shows the watched points with their values now.",
      "Open {btn:tab.recording}, choose the interval and press {btn:rec.start}.",
      "In a chart the pointer names the values at a time; a span drawn with it is shown closer.",
    ],
    causes: [
      "Gaps in a line: the device did not answer at that time.",
      "A flat line: the value did not change. The legend gives each line's lowest, highest and mean.",
    ],
  },
  analyser: {
    title: "Listen to a line with the bus analyser",
    intro: "The bus analyser only listens: wired in parallel to a running line, it shows what the master asks, what each unit answers and how fast, and names what is wrong. It never sends.",
    steps: [
      "Connect the USB adapter in parallel to the line (A to A, B to B), its termination and bias switched off.",
      "Open {btn:tab.analyser}, choose the port, speed and frame of the line and press {btn:an.start}.",
      "The units appear with their requests, answers, exceptions and answer times. Choose one for its details.",
      "The notes above the list say in plain words what stands out: a unit that never answers, damaged frames, a second master.",
      "{btn:an.stop} ends it. {btn:an.openFile} reads a capture saved earlier.",
    ],
    causes: [
      "Only damaged frames: the speed or frame is wrong, or A and B are swapped.",
      "Nothing at all: no master is talking, or the adapter is on another line.",
    ],
  },
  exports: {
    title: "Take data along",
    intro: "Lists and values leave modbusio as CSV and Excel files; the traffic itself as a capture for Wireshark.",
    steps: [
      "{btn:export} in the workspace: the device list, the whole site as Excel, or the site as a file to open on another computer.",
      "With a device open, {btn:export} offers its points as CSV and Excel.",
      "{btn:tab.recording} offers what it recorded as CSV and Excel.",
      "Under {btn:tab.settings}, {btn:cap.start} keeps modbusio's own traffic as a pcapng file.",
    ],
    causes: [
      "Wireshark does not decode RTU frames: set DLT_USER 147 to mbrtu, as the settings page says.",
    ],
  },
  keys: {
    title: "Keys",
    intro: "Everything can be reached with the keyboard.",
  },
};
