// The help's guides in Spanish (a draft, like the interface's Spanish); see en.js.

export default {
  find: {
    title: "Encontrar los dispositivos en la red",
    intro: "modbusio pregunta a cada dirección de la subred de la instalación en el puerto Modbus y lista lo que responde. Modbus no tiene una llamada a la que responda todo dispositivo, por eso se prueba cada dirección.",
    steps: [
      "Conecta este ordenador a la red de la instalación.",
      "Abre {btn:side.search} en la barra lateral y elige {btn:search.tcp}.",
      "Elige {btn:search.adapter} que lleva a la instalación; {btn:search.subnet} se rellena a partir de él.",
      "Pulsa {btn:search.start}. Los dispositivos aparecen en la lista en cuanto responden.",
      "Elige un dispositivo. {btn:dev.read} lee sus puntos cuando tiene un perfil; {btn:dev.explore} muestra lo que contiene sin perfil.",
    ],
    causes: [
      "No se encontró nada: comprueba el adaptador de red. Un portátil con wifi y cable tiene dos.",
      "La instalación usa otro puerto que el 502: introdúcelo en {btn:search.port}.",
      "Un dispositivo que responde a cada ID de unidad es una pasarela: ver {guide:gateway}.",
      "Falta un solo dispositivo: {btn:dev.addByHand} lo añade por su dirección.",
    ],
  },
  gateway: {
    title: "Encontrar los dispositivos tras una pasarela",
    intro: "Una pasarela pasa las peticiones de la red a una línea RS-485. Los dispositivos tras ella comparten su dirección y se distinguen por su ID de unidad.",
    steps: [
      "Abre {btn:side.search} y elige {btn:search.gateway}.",
      "Introduce {btn:search.gatewayAddress}, con el puerto si no es 502.",
      "Ajusta {btn:search.from} y {btn:search.to} a los ID en uso; de 1 a 247 pregunta a todos y tarda más.",
      "Pulsa {btn:search.start}.",
    ],
    causes: [
      "Ninguna unidad responde: algunas pasarelas pasan las tramas RTU sin cambios. Activa {btn:search.rtuOverTcp}.",
      "Las unidades responden despacio o solo a veces: aumenta {btn:search.timeout}; la línea tras la pasarela es más lenta que la red.",
      "Muchas pasarelas admiten una sola conexión: cierra otros programas Modbus.",
    ],
  },
  trunk: {
    title: "Buscar en una línea RS-485",
    intro: "Con un adaptador USB para RS-485, modbusio es el maestro de una línea y pregunta los ID de unidad uno a uno. Una línea tiene un solo maestro: nunca busques mientras habla el maestro de la instalación.",
    steps: [
      "Conecta el adaptador a la línea (A con A, B con B y el hilo común) y enchúfalo.",
      "Abre {btn:side.search}, elige {btn:search.serial} y luego {btn:search.serialPort}.",
      "Pulsa {btn:search.listen}: no envía nada y dice si habla otro maestro y qué unidades se oyen.",
      "Ajusta {btn:search.baud} y {btn:search.frame} como los dispositivos. ¿Desconocidos? {btn:search.findSettings} los prueba.",
      "Pulsa {btn:search.start}.",
    ],
    causes: [
      "Ningún puerto serie en la lista: ¿está enchufado el adaptador? En Linux el usuario debe estar en el grupo dialout.",
      "Nada responde con ningún ajuste: lo más frecuente es que A y B estén cruzados. Cámbialos en el adaptador.",
      "Faltan algunos dispositivos: ver {guide:othersettings}.",
      "Habla otro maestro: usa en su lugar {guide:analyser}, que solo escucha.",
    ],
  },
  othersettings: {
    title: "Encontrar dispositivos con otra velocidad o trama",
    intro: "Un dispositivo ajustado a otra velocidad, paridad o bits de parada que su línea nunca responde allí y puede molestar a los demás. modbusio pregunta a cada unidad con cada ajuste y nombra las que responden en otro.",
    steps: [
      "Detén el maestro de la instalación o desconéctalo: esta búsqueda envía con muchos ajustes.",
      "Abre {btn:side.search}, elige {btn:search.serial} y ajusta {btn:search.baud} y {btn:search.frame} de la línea.",
      "Pulsa {btn:search.otherSettings}, lee lo que hará y cuánto dura, y luego {btn:search.otherSettingsNow}.",
      "El resultado nombra cada unidad y el ajuste con el que responde. Esos dispositivos entran en el sitio con su propio ajuste, marcados en el árbol.",
    ],
    causes: [
      "La búsqueda se detiene enseguida: escuchó primero y oyó a otro maestro. Detenlo.",
      "Una unidad responde dañada con el ajuste de la línea: su paridad o bits de parada difieren, o la línea no está terminada.",
    ],
  },
  profile: {
    title: "Usar la lista de registros de un fabricante",
    intro: "Un perfil dice qué significan los registros de un dispositivo: nombres, tipos, escala, unidades. La lista del fabricante en CSV o Excel se convierte en uno.",
    steps: [
      "Abre {btn:tab.profiles} y pulsa {btn:prof.import}; elige el archivo.",
      "Elige en {btn:imp.device} un dispositivo de ese tipo: la vista previa lee ahora sus valores.",
      "Comprueba {btn:imp.convention}: las listas cuentan desde 0, desde 1, o escriben 40001 para el primer registro de retención. Valores plausibles en la vista previa muestran la correcta.",
      "Pulsa {btn:imp.save} y después asigna este perfil al dispositivo en sus detalles en el área de trabajo.",
    ],
    causes: [
      "Cada valor está desplazado un registro: la otra convención es la correcta.",
      "Los números grandes no tienen sentido: el orden de palabras es otro, ver {guide:explore}.",
      "Una lista solo en PDF: copia su tabla a una hoja de cálculo y guárdala como CSV.",
    ],
  },
  explore: {
    title: "Mirar los registros sin lista",
    intro: "El explorador de registros lee un rango de direcciones y muestra cada registro sin signo, con signo, en hexadecimal, como texto y, con su vecino, como número de 32 bits.",
    steps: [
      "Elige el dispositivo en el área de trabajo y pulsa {btn:dev.explore}.",
      "Ajusta {btn:ex.area}, {btn:ex.start} y {btn:ex.count}, y luego {btn:ex.go}.",
      "Elige un registro que cambie con algo conocido (una temperatura, un contador).",
      "Prueba {btn:ex.order} hasta que un valor de 32 bits sea plausible; los valores sin sentido se muestran atenuados.",
      "{btn:ex.addPoint} conserva el registro como punto de un perfil.",
    ],
    causes: [
      "Rangos enteros sin datos: los dispositivos rechazan las direcciones que no tienen. El explorador lee alrededor de los huecos.",
      "El dispositivo rechaza bloques grandes: el explorador aprende su límite y lee otros más pequeños.",
    ],
  },
  write: {
    title: "Escribir un valor",
    intro: "Escribir cambia una instalación en marcha. modbusio pregunta antes de cada escritura, lee y registra primero el valor anterior, vuelve a leer después y lleva la lista de lo que has cambiado.",
    steps: [
      "Elige un punto escribible y pulsa {btn:write.button}.",
      "La primera escritura de una sesión pide desbloquear: {btn:unlock.allow}.",
      "Introduce el valor y pulsa {btn:write.next}. modbusio muestra el valor anterior y el nuevo.",
      "Confirma con {btn:write.do}. El valor releído queda junto al punto.",
      "{btn:tab.changed} lista cada valor escrito, con el anterior, para restaurarlo.",
    ],
    causes: [
      "Rechazado con la excepción 01 o 02: el dispositivo no permite escribir este registro.",
      "Rechazado con la excepción 03: el valor está fuera de lo que el dispositivo acepta.",
      "Escrito, pero al releer es otro: el dispositivo o su controlador lo sobrescribió.",
    ],
  },
  record: {
    title: "Observar y registrar valores",
    intro: "La lista de observación muestra juntos puntos elegidos de varios dispositivos; el registro guarda sus valores en el tiempo, como gráficos y como CSV o Excel.",
    steps: [
      "Elige un punto en el área de trabajo y activa la observación o el registro en sus detalles.",
      "{btn:tab.watch} muestra los puntos observados con sus valores actuales.",
      "Abre {btn:tab.recording}, elige el intervalo y pulsa {btn:rec.start}.",
      "En un gráfico el puntero nombra los valores en un instante; un intervalo trazado con él se muestra más de cerca.",
    ],
    causes: [
      "Huecos en una línea: el dispositivo no respondió en ese momento.",
      "Una línea plana: el valor no cambió. La leyenda da el mínimo, el máximo y la media de cada línea.",
    ],
  },
  analyser: {
    title: "Escuchar una línea con el analizador de bus",
    intro: "El analizador de bus solo escucha: conectado en paralelo a una línea en marcha, muestra qué pregunta el maestro, qué responde cada unidad y en cuánto tiempo, y nombra lo que va mal. Nunca envía.",
    steps: [
      "Conecta el adaptador USB en paralelo a la línea (A con A, B con B), con su terminación y polarización apagadas.",
      "Abre {btn:tab.analyser}, elige el puerto, la velocidad y la trama de la línea y pulsa {btn:an.start}.",
      "Las unidades aparecen con sus peticiones, respuestas, excepciones y tiempos de respuesta. Elige una para ver sus detalles.",
      "Las notas sobre la lista dicen con palabras claras lo que destaca: una unidad que nunca responde, tramas dañadas, un segundo maestro.",
      "{btn:an.stop} lo termina. {btn:an.openFile} lee una captura guardada antes.",
    ],
    causes: [
      "Solo tramas dañadas: la velocidad o la trama es incorrecta, o A y B están cruzados.",
      "Nada en absoluto: ningún maestro habla, o el adaptador está en otra línea.",
    ],
  },
  exports: {
    title: "Llevarse los datos",
    intro: "Las listas y los valores salen de modbusio como archivos CSV y Excel; el tráfico mismo como captura para Wireshark.",
    steps: [
      "{btn:export} en el área de trabajo: la lista de dispositivos, todo el sitio en Excel, o el sitio como archivo para abrir en otro ordenador.",
      "Con un dispositivo abierto, {btn:export} ofrece sus puntos en CSV y Excel.",
      "{btn:tab.recording} ofrece lo registrado en CSV y Excel.",
      "En {btn:tab.settings}, {btn:cap.start} guarda el tráfico de modbusio como archivo pcapng.",
    ],
    causes: [
      "Wireshark no decodifica las tramas RTU: ajusta DLT_USER 147 a mbrtu, como dice la página de ajustes.",
    ],
  },
  keys: {
    title: "Teclas",
    intro: "Todo se alcanza con el teclado.",
  },
};
