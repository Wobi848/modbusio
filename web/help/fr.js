// The help's guides in French (a draft, like the interface's French); see en.js.

export default {
  find: {
    title: "Trouver les appareils sur le réseau",
    intro: "modbusio interroge chaque adresse du sous-réseau de l’installation sur le port Modbus et liste ce qui répond. Modbus n’a pas d’appel auquel tout appareil répond, chaque adresse est donc essayée.",
    steps: [
      "Reliez cet ordinateur au réseau de l’installation.",
      "Ouvrez {btn:side.search} dans la barre latérale et choisissez {btn:search.tcp}.",
      "Choisissez {btn:search.adapter} qui mène à l’installation ; {btn:search.subnet} se remplit d’après lui.",
      "Appuyez sur {btn:search.start}. Les appareils apparaissent dans la liste dès qu’ils répondent.",
      "Choisissez un appareil. {btn:dev.read} lit ses points dès qu’il a un profil ; {btn:dev.explore} montre ce qu’il contient sans profil.",
    ],
    causes: [
      "Rien trouvé : vérifiez l’adaptateur réseau. Un portable avec Wi-Fi et câble en a deux.",
      "L’installation utilise un autre port que 502 : saisissez-le sous {btn:search.port}.",
      "Un appareil qui répond à chaque identifiant d’unité est une passerelle : voir {guide:gateway}.",
      "Un seul appareil manque : {btn:dev.addByHand} l’ajoute par son adresse.",
    ],
  },
  gateway: {
    title: "Trouver les appareils derrière une passerelle",
    intro: "Une passerelle transmet les requêtes du réseau à une ligne RS-485. Les appareils derrière elle partagent son adresse et se distinguent par leur identifiant d’unité.",
    steps: [
      "Ouvrez {btn:side.search} et choisissez {btn:search.gateway}.",
      "Saisissez {btn:search.gatewayAddress}, avec le port s’il n’est pas 502.",
      "Réglez {btn:search.from} et {btn:search.to} sur les identifiants utilisés ; 1 à 247 les interroge tous et dure plus longtemps.",
      "Appuyez sur {btn:search.start}.",
    ],
    causes: [
      "Aucune unité ne répond : certaines passerelles transmettent les trames RTU telles quelles. Activez {btn:search.rtuOverTcp}.",
      "Les unités répondent lentement ou parfois seulement : augmentez {btn:search.timeout} ; la ligne derrière la passerelle est plus lente que le réseau.",
      "Beaucoup de passerelles n’acceptent qu’une connexion : fermez les autres programmes Modbus.",
    ],
  },
  trunk: {
    title: "Parcourir une ligne RS-485",
    intro: "Avec un adaptateur USB pour RS-485, modbusio est le maître d’une ligne et interroge les identifiants d’unité un par un. Une ligne n’a qu’un maître : ne cherchez jamais pendant que le maître de l’installation parle.",
    steps: [
      "Raccordez l’adaptateur à la ligne (A sur A, B sur B et le fil commun) et branchez-le.",
      "Ouvrez {btn:side.search}, choisissez {btn:search.serial}, puis {btn:search.serialPort}.",
      "Appuyez sur {btn:search.listen} : cela n’envoie rien et dit si un autre maître parle et quelles unités on entend.",
      "Réglez {btn:search.baud} et {btn:search.frame} comme les appareils. Inconnus ? {btn:search.findSettings} les essaie.",
      "Appuyez sur {btn:search.start}.",
    ],
    causes: [
      "Aucun port série dans la liste : l’adaptateur est-il branché ? Sous Linux, l’utilisateur doit être dans le groupe dialout.",
      "Rien ne répond à aucun réglage : le plus souvent A et B sont inversés. Inversez-les sur l’adaptateur.",
      "Certains appareils manquent : voir {guide:othersettings}.",
      "Un autre maître parle : utilisez plutôt {guide:analyser}, qui ne fait qu’écouter.",
    ],
  },
  othersettings: {
    title: "Trouver les appareils réglés sur une autre vitesse ou trame",
    intro: "Un appareil réglé sur une autre vitesse, parité ou d’autres bits d’arrêt que sa ligne n’y répond jamais et peut gêner les autres. modbusio interroge chaque unité à chaque réglage et nomme celles qui répondent ailleurs.",
    steps: [
      "Arrêtez le maître de l’installation ou débranchez-le : cette recherche émet à de nombreux réglages.",
      "Ouvrez {btn:side.search}, choisissez {btn:search.serial} et réglez {btn:search.baud} et {btn:search.frame} de la ligne.",
      "Appuyez sur {btn:search.otherSettings}, lisez ce qui sera fait et combien de temps, puis {btn:search.otherSettingsNow}.",
      "Le résultat nomme chaque unité et le réglage auquel elle répond. Ces appareils rejoignent le site à leur propre réglage, marqués dans l’arbre.",
    ],
    causes: [
      "La recherche s’arrête aussitôt : elle a d’abord écouté et entendu un autre maître. Arrêtez-le.",
      "Une unité répond de façon endommagée au réglage de la ligne : sa parité ou ses bits d’arrêt diffèrent, ou la ligne n’est pas terminée.",
    ],
  },
  profile: {
    title: "Utiliser la liste de registres d’un fabricant",
    intro: "Un profil dit ce que signifient les registres d’un appareil : noms, types, mise à l’échelle, unités. La liste du fabricant en CSV ou Excel en devient un.",
    steps: [
      "Ouvrez {btn:tab.profiles} et appuyez sur {btn:prof.import} ; choisissez le fichier.",
      "Choisissez sous {btn:imp.device} un appareil de ce type : l’aperçu lit ses valeurs maintenant.",
      "Vérifiez {btn:imp.convention} : les listes comptent à partir de 0, de 1, ou écrivent 40001 pour le premier registre de maintien. Des valeurs plausibles dans l’aperçu montrent la bonne.",
      "Appuyez sur {btn:imp.save}, puis donnez ce profil à l’appareil dans ses détails dans l’espace de travail.",
    ],
    causes: [
      "Chaque valeur est décalée d’un registre : l’autre convention est la bonne.",
      "Les grands nombres n’ont aucun sens : l’ordre des mots diffère, voir {guide:explore}.",
      "Une liste en PDF seulement : copiez son tableau dans un tableur et enregistrez-le en CSV.",
    ],
  },
  explore: {
    title: "Regarder les registres sans liste",
    intro: "L’explorateur de registres lit une plage d’adresses et montre chaque registre non signé, signé, en hexadécimal, en texte et, avec son voisin, comme nombre de 32 bits.",
    steps: [
      "Choisissez l’appareil dans l’espace de travail et appuyez sur {btn:dev.explore}.",
      "Réglez {btn:ex.area}, {btn:ex.start} et {btn:ex.count}, puis {btn:ex.go}.",
      "Choisissez un registre qui change avec quelque chose de connu (une température, un compteur).",
      "Essayez {btn:ex.order} jusqu’à ce qu’une valeur de 32 bits soit plausible ; les valeurs sans sens sont atténuées.",
      "{btn:ex.addPoint} garde le registre comme point d’un profil.",
    ],
    causes: [
      "Des plages entières sans données : les appareils refusent les adresses qu’ils n’ont pas. L’explorateur lit autour des trous.",
      "L’appareil refuse les grands blocs : l’explorateur apprend sa limite et en lit de plus petits.",
    ],
  },
  write: {
    title: "Écrire une valeur",
    intro: "Écrire modifie une installation en marche. modbusio demande avant chaque écriture, lit et consigne d’abord l’ancienne valeur, relit ensuite et tient la liste de ce que vous avez modifié.",
    steps: [
      "Choisissez un point inscriptible et appuyez sur {btn:write.button}.",
      "La première écriture d’une session demande le déverrouillage : {btn:unlock.allow}.",
      "Saisissez la valeur et appuyez sur {btn:write.next}. modbusio montre l’ancienne et la nouvelle valeur.",
      "Confirmez avec {btn:write.do}. La valeur relue figure près du point.",
      "{btn:tab.changed} liste chaque valeur écrite, avec la valeur d’avant, pour la remettre.",
    ],
    causes: [
      "Refusé avec l’exception 01 ou 02 : l’appareil ne permet pas d’écrire ce registre.",
      "Refusé avec l’exception 03 : la valeur est hors de ce que l’appareil accepte.",
      "Écrit, mais la relecture diffère : l’appareil ou son automate l’a réécrite.",
    ],
  },
  record: {
    title: "Surveiller et enregistrer des valeurs",
    intro: "La liste de surveillance montre ensemble des points choisis de plusieurs appareils ; l’enregistrement garde leurs valeurs dans le temps, en graphiques et en CSV ou Excel.",
    steps: [
      "Choisissez un point dans l’espace de travail et activez la surveillance ou l’enregistrement dans ses détails.",
      "{btn:tab.watch} montre les points surveillés avec leurs valeurs actuelles.",
      "Ouvrez {btn:tab.recording}, choisissez l’intervalle et appuyez sur {btn:rec.start}.",
      "Dans un graphique, le pointeur nomme les valeurs à un instant ; une plage tracée avec lui est montrée de plus près.",
    ],
    causes: [
      "Des trous dans une ligne : l’appareil n’a pas répondu à ce moment.",
      "Une ligne plate : la valeur n’a pas changé. La légende donne le minimum, le maximum et la moyenne de chaque ligne.",
    ],
  },
  analyser: {
    title: "Écouter une ligne avec l’analyseur de bus",
    intro: "L’analyseur de bus ne fait qu’écouter : branché en parallèle sur une ligne en service, il montre ce que le maître demande, ce que chaque unité répond et en combien de temps, et nomme ce qui ne va pas. Il n’émet jamais.",
    steps: [
      "Raccordez l’adaptateur USB en parallèle à la ligne (A sur A, B sur B), sa terminaison et sa polarisation coupées.",
      "Ouvrez {btn:tab.analyser}, choisissez le port, la vitesse et la trame de la ligne et appuyez sur {btn:an.start}.",
      "Les unités apparaissent avec leurs requêtes, réponses, exceptions et temps de réponse. Choisissez-en une pour ses détails.",
      "Les notes au-dessus de la liste disent en clair ce qui ressort : une unité qui ne répond jamais, des trames endommagées, un second maître.",
      "{btn:an.stop} y met fin. {btn:an.openFile} lit une capture enregistrée plus tôt.",
    ],
    causes: [
      "Seulement des trames endommagées : la vitesse ou la trame est fausse, ou A et B sont inversés.",
      "Rien du tout : aucun maître ne parle, ou l’adaptateur est sur une autre ligne.",
    ],
  },
  exports: {
    title: "Emporter les données",
    intro: "Les listes et les valeurs quittent modbusio en fichiers CSV et Excel ; le trafic lui-même en capture pour Wireshark.",
    steps: [
      "{btn:export} dans l’espace de travail : la liste des appareils, tout le site en Excel, ou le site en fichier à ouvrir sur un autre ordinateur.",
      "Avec un appareil ouvert, {btn:export} propose ses points en CSV et Excel.",
      "{btn:tab.recording} propose ce qui a été enregistré en CSV et Excel.",
      "Sous {btn:tab.settings}, {btn:cap.start} garde le trafic de modbusio en fichier pcapng.",
    ],
    causes: [
      "Wireshark ne décode pas les trames RTU : réglez DLT_USER 147 sur mbrtu, comme l’indique la page des réglages.",
    ],
  },
  keys: {
    title: "Touches",
    intro: "Tout est accessible au clavier.",
  },
};
