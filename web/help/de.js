// The help's guides in German; see en.js.

export default {
  find: {
    title: "Geräte im Netzwerk finden",
    intro: "modbusio fragt jede Adresse im Subnetz der Anlage auf dem Modbus-Port an und listet auf, was antwortet. Modbus kennt keinen Aufruf, auf den jedes Gerät antwortet, deshalb wird jede Adresse probiert.",
    steps: [
      "Diesen Computer mit dem Netzwerk der Anlage verbinden.",
      "In der Seitenleiste {btn:side.search} öffnen und {btn:search.tcp} wählen.",
      "Den {btn:search.adapter} wählen, der zur Anlage führt; {btn:search.subnet} wird daraus ausgefüllt.",
      "{btn:search.start} drücken. Die Geräte erscheinen in der Liste, sobald sie antworten.",
      "Ein Gerät wählen. {btn:dev.read} liest seine Datenpunkte, sobald es ein Profil hat; {btn:dev.explore} zeigt ohne Profil, was es enthält.",
    ],
    causes: [
      "Nichts gefunden: den Netzwerkadapter prüfen. Ein Laptop mit WLAN und Kabel hat zwei.",
      "Die Anlage nutzt einen anderen Port als 502: unter {btn:search.port} eintragen.",
      "Ein Gerät, das auf jede Unit-ID antwortet, ist ein Gateway: siehe {guide:gateway}.",
      "Ein einzelnes Gerät fehlt: {btn:dev.addByHand} fügt es mit seiner Adresse hinzu.",
    ],
  },
  gateway: {
    title: "Geräte hinter einem Gateway finden",
    intro: "Ein Gateway reicht Anfragen aus dem Netzwerk an eine RS-485-Leitung weiter. Die Geräte dahinter teilen seine Adresse und unterscheiden sich durch ihre Unit-ID.",
    steps: [
      "{btn:side.search} öffnen und {btn:search.gateway} wählen.",
      "Die {btn:search.gatewayAddress} eintragen, mit Port, wenn er nicht 502 ist.",
      "{btn:search.from} und {btn:search.to} auf die benutzten Unit-IDs setzen; 1 bis 247 fragt alle und dauert länger.",
      "{btn:search.start} drücken.",
    ],
    causes: [
      "Keine Unit antwortet: Manche Gateways reichen RTU-Frames unverändert durch. {btn:search.rtuOverTcp} einschalten.",
      "Units antworten langsam oder nur manchmal: {btn:search.timeout} erhöhen; die Leitung hinter dem Gateway ist langsamer als das Netzwerk.",
      "Viele Gateways erlauben nur eine Verbindung: andere Modbus-Programme schliessen.",
    ],
  },
  trunk: {
    title: "Eine RS-485-Leitung durchsuchen",
    intro: "Mit einem USB-Adapter für RS-485 ist modbusio der Master einer Leitung und fragt die Unit-IDs der Reihe nach. Eine Leitung hat nur einen Master: nie suchen, während der Master der Anlage spricht.",
    steps: [
      "Den Adapter an die Leitung anschliessen (A an A, B an B und die gemeinsame Ader) und einstecken.",
      "{btn:side.search} öffnen, {btn:search.serial} wählen, dann unter {btn:search.serialPort} den Adapter.",
      "{btn:search.listen} drücken: Es sendet nichts und sagt, ob ein anderer Master spricht und welche Units zu hören sind.",
      "{btn:search.baud} und {btn:search.frame} so einstellen wie die Geräte. Unbekannt? {btn:search.findSettings} probiert sie durch.",
      "{btn:search.start} drücken.",
    ],
    causes: [
      "Kein serieller Port in der Liste: Ist der Adapter eingesteckt? Unter Linux muss der Benutzer in der Gruppe dialout sein.",
      "Bei keiner Einstellung antwortet etwas: Meist sind A und B vertauscht. Am Adapter tauschen.",
      "Einzelne Geräte fehlen: siehe {guide:othersettings}.",
      "Ein anderer Master spricht: stattdessen {guide:analyser} benutzen, die hört nur zu.",
    ],
  },
  othersettings: {
    title: "Geräte mit anderer Geschwindigkeit oder anderem Frame finden",
    intro: "Ein Gerät mit anderer Geschwindigkeit, Parität oder anderen Stoppbits als seine Leitung antwortet dort nie und kann die anderen stören. modbusio fragt jede Unit bei jeder Einstellung und nennt die, die anderswo antworten.",
    steps: [
      "Den Master der Anlage anhalten oder abklemmen: Diese Suche sendet bei vielen Einstellungen.",
      "{btn:side.search} öffnen, {btn:search.serial} wählen und {btn:search.baud} und {btn:search.frame} der Leitung einstellen.",
      "{btn:search.otherSettings} drücken, lesen, was passiert und wie lange es dauert, dann {btn:search.otherSettingsNow}.",
      "Das Ergebnis nennt jede Unit und die Einstellung, bei der sie antwortet. Solche Geräte kommen mit ihrer eigenen Einstellung in die Anlage und sind im Baum markiert.",
    ],
    causes: [
      "Die Suche bricht sofort ab: Sie hat zuerst zugehört und einen anderen Master gehört. Diesen anhalten.",
      "Eine Unit antwortet bei der Einstellung der Leitung beschädigt: Parität oder Stoppbits weichen ab, oder die Leitung ist nicht abgeschlossen.",
    ],
  },
  profile: {
    title: "Die Registerliste eines Herstellers verwenden",
    intro: "Ein Profil sagt, was die Register eines Geräts bedeuten: Namen, Typen, Skalierung, Einheiten. Aus der Liste des Herstellers als CSV oder Excel wird eines.",
    steps: [
      "{btn:tab.profiles} öffnen und {btn:prof.import} drücken; die Datei wählen.",
      "Unter {btn:imp.device} ein Gerät dieser Art wählen: Die Vorschau liest seine Werte jetzt.",
      "{btn:imp.convention} prüfen: Listen zählen ab 0, ab 1 oder schreiben 40001 für das erste Holding-Register. Plausible Werte in der Vorschau zeigen die richtige Zählweise.",
      "{btn:imp.save} drücken, dann dem Gerät im Arbeitsbereich in seinen Details dieses Profil geben.",
    ],
    causes: [
      "Jeder Wert liegt ein Register daneben: Die andere Zählweise ist die richtige.",
      "Grosse Zahlen ergeben keinen Sinn: Die Wortreihenfolge weicht ab, siehe {guide:explore}.",
      "Die Liste gibt es nur als PDF: Die Tabelle in eine Tabellenkalkulation kopieren und als CSV speichern.",
    ],
  },
  explore: {
    title: "Register ohne Liste ansehen",
    intro: "Der Register-Explorer liest einen Adressbereich und zeigt jedes Register vorzeichenlos, mit Vorzeichen, hexadezimal, als Text und zusammen mit dem Nachbarn als 32-Bit-Zahl.",
    steps: [
      "Das Gerät im Arbeitsbereich wählen und {btn:dev.explore} drücken.",
      "{btn:ex.area}, {btn:ex.start} und {btn:ex.count} einstellen, dann {btn:ex.go}.",
      "Ein Register wählen, das sich mit etwas Bekanntem ändert (eine Temperatur, ein Zähler).",
      "{btn:ex.order} durchprobieren, bis ein 32-Bit-Wert plausibel ist; unsinnige Werte sind abgeschwächt dargestellt.",
      "{btn:ex.addPoint} übernimmt das Register als Datenpunkt in ein Profil.",
    ],
    causes: [
      "Ganze Bereiche ohne Daten: Geräte lehnen Adressen ab, die sie nicht haben. Der Explorer liest um die Lücken herum.",
      "Das Gerät lehnt grosse Blöcke ab: Der Explorer lernt die Grenze und liest kleinere.",
    ],
  },
  write: {
    title: "Einen Wert schreiben",
    intro: "Schreiben verändert eine laufende Anlage. modbusio fragt vor jedem Schreiben, liest und protokolliert zuerst den alten Wert, liest danach zurück und führt eine Liste dessen, was du geändert hast.",
    steps: [
      "Einen schreibbaren Datenpunkt wählen und {btn:write.button} drücken.",
      "Das erste Schreiben einer Sitzung fragt nach der Freigabe: {btn:unlock.allow}.",
      "Den Wert eingeben und {btn:write.next} drücken. modbusio zeigt den alten und den neuen Wert.",
      "Mit {btn:write.do} bestätigen. Der zurückgelesene Wert steht beim Datenpunkt.",
      "{btn:tab.changed} listet jeden geschriebenen Wert mit dem Wert davor, um ihn zurückzusetzen.",
    ],
    causes: [
      "Abgelehnt mit Ausnahme 01 oder 02: Das Gerät erlaubt das Schreiben dieses Registers nicht.",
      "Abgelehnt mit Ausnahme 03: Der Wert liegt ausserhalb dessen, was das Gerät annimmt.",
      "Geschrieben, aber zurückgelesen steht etwas anderes: Das Gerät oder seine Steuerung hat den Wert überschrieben.",
    ],
  },
  record: {
    title: "Werte beobachten und aufzeichnen",
    intro: "Die Liste Beobachten zeigt gewählte Datenpunkte mehrerer Geräte zusammen; die Aufzeichnung hält ihre Werte über die Zeit fest, als Diagramme und als CSV oder Excel.",
    steps: [
      "Einen Datenpunkt im Arbeitsbereich wählen und in seinen Details Beobachten oder Aufzeichnen einschalten.",
      "{btn:tab.watch} zeigt die beobachteten Datenpunkte mit ihren Werten jetzt.",
      "{btn:tab.recording} öffnen, das Intervall wählen und {btn:rec.start} drücken.",
      "Im Diagramm nennt der Zeiger die Werte zu einem Zeitpunkt; ein damit aufgezogener Bereich wird genauer gezeigt.",
    ],
    causes: [
      "Lücken in einer Linie: Das Gerät hat zu dieser Zeit nicht geantwortet.",
      "Eine flache Linie: Der Wert hat sich nicht geändert. Die Legende nennt Minimum, Maximum und Mittel jeder Linie.",
    ],
  },
  analyser: {
    title: "Mit der Busanalyse an einer Leitung mithören",
    intro: "Die Busanalyse hört nur zu: Parallel an eine laufende Leitung angeschlossen zeigt sie, was der Master fragt, was jede Unit antwortet und wie schnell, und nennt, was nicht stimmt. Sie sendet nie.",
    steps: [
      "Den USB-Adapter parallel an die Leitung anschliessen (A an A, B an B), Abschluss und Bias am Adapter aus.",
      "{btn:tab.analyser} öffnen, Port, Geschwindigkeit und Frame der Leitung wählen und {btn:an.start} drücken.",
      "Die Units erscheinen mit Anfragen, Antworten, Ausnahmen und Antwortzeiten. Eine wählen für ihre Details.",
      "Die Hinweise über der Liste sagen in klaren Worten, was auffällt: eine Unit, die nie antwortet, beschädigte Frames, ein zweiter Master.",
      "{btn:an.stop} beendet es. {btn:an.openFile} liest einen früher gespeicherten Mitschnitt.",
    ],
    causes: [
      "Nur beschädigte Frames: Geschwindigkeit oder Frame stimmen nicht, oder A und B sind vertauscht.",
      "Gar nichts: Kein Master spricht, oder der Adapter hängt an einer anderen Leitung.",
    ],
  },
  exports: {
    title: "Daten mitnehmen",
    intro: "Listen und Werte verlassen modbusio als CSV- und Excel-Dateien; der Verkehr selbst als Mitschnitt für Wireshark.",
    steps: [
      "{btn:export} im Arbeitsbereich: die Geräteliste, die ganze Anlage als Excel oder die Anlage als Datei zum Öffnen auf einem anderen Computer.",
      "Ist ein Gerät geöffnet, bietet {btn:export} seine Datenpunkte als CSV und Excel.",
      "{btn:tab.recording} bietet das Aufgezeichnete als CSV und Excel.",
      "Unter {btn:tab.settings} hält {btn:cap.start} den eigenen Verkehr von modbusio als pcapng-Datei fest.",
    ],
    causes: [
      "Wireshark dekodiert RTU-Frames nicht: DLT_USER 147 auf mbrtu setzen, wie es auf der Seite Einstellungen steht.",
    ],
  },
  keys: {
    title: "Tasten",
    intro: "Alles ist mit der Tastatur erreichbar.",
  },
};
