// The help's guides in Italian (a draft, like the interface's Italian); see en.js.

export default {
  find: {
    title: "Trovare i dispositivi in rete",
    intro: "modbusio interroga ogni indirizzo della sottorete dell’impianto sulla porta Modbus ed elenca ciò che risponde. Modbus non ha una chiamata a cui ogni dispositivo risponde, perciò si prova ogni indirizzo.",
    steps: [
      "Collega questo computer alla rete dell’impianto.",
      "Apri {btn:side.search} nella barra laterale e scegli {btn:search.tcp}.",
      "Scegli {btn:search.adapter} che porta all’impianto; {btn:search.subnet} si compila da esso.",
      "Premi {btn:search.start}. I dispositivi compaiono nell’elenco appena rispondono.",
      "Scegli un dispositivo. {btn:dev.read} legge i suoi punti quando ha un profilo; {btn:dev.explore} mostra cosa contiene senza profilo.",
    ],
    causes: [
      "Niente trovato: controlla la scheda di rete. Un portatile con Wi-Fi e cavo ne ha due.",
      "L’impianto usa una porta diversa da 502: inseriscila in {btn:search.port}.",
      "Un dispositivo che risponde a ogni ID di unità è un gateway: vedi {guide:gateway}.",
      "Manca un solo dispositivo: {btn:dev.addByHand} lo aggiunge con il suo indirizzo.",
    ],
  },
  gateway: {
    title: "Trovare i dispositivi dietro un gateway",
    intro: "Un gateway inoltra le richieste dalla rete a una linea RS-485. I dispositivi dietro di esso condividono il suo indirizzo e si distinguono per l’ID di unità.",
    steps: [
      "Apri {btn:side.search} e scegli {btn:search.gateway}.",
      "Inserisci {btn:search.gatewayAddress}, con la porta se non è 502.",
      "Imposta {btn:search.from} e {btn:search.to} sugli ID in uso; da 1 a 247 li interroga tutti e richiede più tempo.",
      "Premi {btn:search.start}.",
    ],
    causes: [
      "Nessuna unità risponde: alcuni gateway inoltrano i frame RTU invariati. Attiva {btn:search.rtuOverTcp}.",
      "Le unità rispondono lentamente o solo a volte: aumenta {btn:search.timeout}; la linea dietro il gateway è più lenta della rete.",
      "Molti gateway accettano una sola connessione: chiudi gli altri programmi Modbus.",
    ],
  },
  trunk: {
    title: "Cercare su una linea RS-485",
    intro: "Con un adattatore USB per RS-485, modbusio è il master di una linea e interroga gli ID di unità uno a uno. Una linea ha un solo master: non cercare mai mentre parla il master dell’impianto.",
    steps: [
      "Collega l’adattatore alla linea (A con A, B con B e il filo comune) e inseriscilo.",
      "Apri {btn:side.search}, scegli {btn:search.serial}, poi {btn:search.serialPort}.",
      "Premi {btn:search.listen}: non invia nulla e dice se parla un altro master e quali unità si sentono.",
      "Imposta {btn:search.baud} e {btn:search.frame} come i dispositivi. Sconosciuti? {btn:search.findSettings} li prova.",
      "Premi {btn:search.start}.",
    ],
    causes: [
      "Nessuna porta seriale nell’elenco: l’adattatore è inserito? Su Linux l’utente deve essere nel gruppo dialout.",
      "Niente risponde con nessuna impostazione: di solito A e B sono scambiati. Scambiali sull’adattatore.",
      "Mancano alcuni dispositivi: vedi {guide:othersettings}.",
      "Parla un altro master: usa invece {guide:analyser}, che ascolta soltanto.",
    ],
  },
  othersettings: {
    title: "Trovare dispositivi impostati su altra velocità o frame",
    intro: "Un dispositivo impostato su velocità, parità o bit di stop diversi dalla sua linea lì non risponde mai e può disturbare gli altri. modbusio interroga ogni unità con ogni impostazione e nomina quelle che rispondono altrove.",
    steps: [
      "Ferma il master dell’impianto o scollegalo: questa ricerca trasmette con molte impostazioni.",
      "Apri {btn:side.search}, scegli {btn:search.serial} e imposta {btn:search.baud} e {btn:search.frame} della linea.",
      "Premi {btn:search.otherSettings}, leggi cosa farà e quanto dura, poi {btn:search.otherSettingsNow}.",
      "Il risultato nomina ogni unità e l’impostazione a cui risponde. Questi dispositivi entrano nel sito con la propria impostazione, segnati nell’albero.",
    ],
    causes: [
      "La ricerca si ferma subito: ha prima ascoltato e sentito un altro master. Fermalo.",
      "Un’unità risponde in modo danneggiato con l’impostazione della linea: parità o bit di stop differiscono, o la linea non è terminata.",
    ],
  },
  profile: {
    title: "Usare l’elenco dei registri di un produttore",
    intro: "Un profilo dice cosa significano i registri di un dispositivo: nomi, tipi, scala, unità. L’elenco del produttore in CSV o Excel ne diventa uno.",
    steps: [
      "Apri {btn:tab.profiles} e premi {btn:prof.import}; scegli il file.",
      "Scegli in {btn:imp.device} un dispositivo di quel tipo: l’anteprima legge ora i suoi valori.",
      "Controlla {btn:imp.convention}: gli elenchi contano da 0, da 1, o scrivono 40001 per il primo holding register. Valori plausibili nell’anteprima mostrano quella giusta.",
      "Premi {btn:imp.save}, poi assegna questo profilo al dispositivo nei suoi dettagli nell’area di lavoro.",
    ],
    causes: [
      "Ogni valore è spostato di un registro: la convenzione giusta è l’altra.",
      "I numeri grandi non hanno senso: l’ordine delle parole è diverso, vedi {guide:explore}.",
      "Un elenco solo in PDF: copia la tabella in un foglio di calcolo e salvala come CSV.",
    ],
  },
  explore: {
    title: "Guardare i registri senza elenco",
    intro: "L’esploratore dei registri legge un intervallo di indirizzi e mostra ogni registro senza segno, con segno, in esadecimale, come testo e, con il vicino, come numero a 32 bit.",
    steps: [
      "Scegli il dispositivo nell’area di lavoro e premi {btn:dev.explore}.",
      "Imposta {btn:ex.area}, {btn:ex.start} e {btn:ex.count}, poi {btn:ex.go}.",
      "Scegli un registro che cambia con qualcosa di noto (una temperatura, un contatore).",
      "Prova {btn:ex.order} finché un valore a 32 bit è plausibile; i valori senza senso sono attenuati.",
      "{btn:ex.addPoint} conserva il registro come punto di un profilo.",
    ],
    causes: [
      "Interi intervalli senza dati: i dispositivi rifiutano gli indirizzi che non hanno. L’esploratore legge attorno ai buchi.",
      "Il dispositivo rifiuta blocchi grandi: l’esploratore impara il limite e ne legge di più piccoli.",
    ],
  },
  write: {
    title: "Scrivere un valore",
    intro: "Scrivere modifica un impianto in funzione. modbusio chiede prima di ogni scrittura, legge e registra prima il valore vecchio, rilegge dopo e tiene l’elenco di ciò che hai modificato.",
    steps: [
      "Scegli un punto scrivibile e premi {btn:write.button}.",
      "La prima scrittura di una sessione chiede lo sblocco: {btn:unlock.allow}.",
      "Inserisci il valore e premi {btn:write.next}. modbusio mostra il valore vecchio e quello nuovo.",
      "Conferma con {btn:write.do}. Il valore riletto sta accanto al punto.",
      "{btn:tab.changed} elenca ogni valore scritto, con quello precedente, per ripristinarlo.",
    ],
    causes: [
      "Rifiutato con eccezione 01 o 02: il dispositivo non permette di scrivere questo registro.",
      "Rifiutato con eccezione 03: il valore è fuori da ciò che il dispositivo accetta.",
      "Scritto, ma riletto è diverso: il dispositivo o il suo controllore lo ha sovrascritto.",
    ],
  },
  record: {
    title: "Osservare e registrare valori",
    intro: "L’elenco di osservazione mostra insieme punti scelti di più dispositivi; la registrazione conserva i loro valori nel tempo, come grafici e come CSV o Excel.",
    steps: [
      "Scegli un punto nell’area di lavoro e attiva l’osservazione o la registrazione nei suoi dettagli.",
      "{btn:tab.watch} mostra i punti osservati con i valori attuali.",
      "Apri {btn:tab.recording}, scegli l’intervallo e premi {btn:rec.start}.",
      "In un grafico il puntatore nomina i valori in un istante; un intervallo tracciato con esso è mostrato più da vicino.",
    ],
    causes: [
      "Buchi in una linea: il dispositivo non ha risposto in quel momento.",
      "Una linea piatta: il valore non è cambiato. La legenda dà minimo, massimo e media di ogni linea.",
    ],
  },
  analyser: {
    title: "Ascoltare una linea con l’analizzatore di bus",
    intro: "L’analizzatore di bus ascolta soltanto: collegato in parallelo a una linea in funzione, mostra cosa chiede il master, cosa risponde ogni unità e in quanto tempo, e nomina ciò che non va. Non trasmette mai.",
    steps: [
      "Collega l’adattatore USB in parallelo alla linea (A con A, B con B), con terminazione e polarizzazione spente.",
      "Apri {btn:tab.analyser}, scegli porta, velocità e frame della linea e premi {btn:an.start}.",
      "Le unità compaiono con richieste, risposte, eccezioni e tempi di risposta. Scegline una per i dettagli.",
      "Le note sopra l’elenco dicono in parole chiare cosa spicca: un’unità che non risponde mai, frame danneggiati, un secondo master.",
      "{btn:an.stop} termina. {btn:an.openFile} legge una cattura salvata prima.",
    ],
    causes: [
      "Solo frame danneggiati: velocità o frame sono sbagliati, o A e B sono scambiati.",
      "Proprio niente: nessun master parla, o l’adattatore è su un’altra linea.",
    ],
  },
  exports: {
    title: "Portare via i dati",
    intro: "Elenchi e valori lasciano modbusio come file CSV ed Excel; il traffico stesso come cattura per Wireshark.",
    steps: [
      "{btn:export} nell’area di lavoro: l’elenco dei dispositivi, l’intero sito in Excel, o il sito come file da aprire su un altro computer.",
      "Con un dispositivo aperto, {btn:export} offre i suoi punti in CSV ed Excel.",
      "{btn:tab.recording} offre ciò che ha registrato in CSV ed Excel.",
      "In {btn:tab.settings}, {btn:cap.start} conserva il traffico di modbusio come file pcapng.",
    ],
    causes: [
      "Wireshark non decodifica i frame RTU: imposta DLT_USER 147 su mbrtu, come dice la pagina delle impostazioni.",
    ],
  },
  keys: {
    title: "Tasti",
    intro: "Tutto è raggiungibile con la tastiera.",
  },
};
