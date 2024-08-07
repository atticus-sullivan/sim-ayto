# sim-ayto
Berechnet die noch verfügbaren Möglichkeiten

# Ergebnisse
![Auf `build` Branch wechseln](img/09_build1_dark.png)
Um zu den Ergebnissen zu kommen, muss man zunächst auf den `build` Branch
wechseln).

Dann sind die Tabellen/Bäume unter `<Staffel>/<Staffel>*.{png,pdf,out}` zu finden
(bei Problemen hilft wahrscheinlich entweder Seite neu laden oder auf `Download`
(-> Datei wird je nach Browser direkt im Browser geöffnet) klicken)

## Ausgabe-Dateien
- `<Staffel>.out`: Hier findet man die meisten Informationen (u.A. den
  bisherigen Verlauf).<br>
  Vor der jeweiligen Tabelle kommt immer nochmal was genau
  als Einschränkung/Constraint dazu kam (die Zahl die da im Leeren steht ist die
  Anzahl der Lichter). Die Episode bezieht sich immer auf die Episode in der das
  ganze aufgelöst wurde.<br>
  Bei den Boxes wird immer der optimale Case rausgesucht
  (sollte immer der Case sein, der am nächsten zu den 100% ist) auch wenn das
  rein aus Informationstheoretischer Sicht ist (in der die Zahlen wirklich als
  Wahrscheinlichkeiten betrachtet werden).<br>
  Das ganze `I` (Informationsgehalt) /
  `H` (Entropie) geht auch in die informationstheoretische-Richtung
  (die Bits Information addieren sich auf, `x bits left` sage wieviel
  Information noch fehlt um das Ergebnis sicher zu wissen). Informationsgehalt
  ist vielleicht ganz interessant, weil sie ja immer davon reden, dass sie neue
  Erkenntnisse haben, auch wenn sie nix gefunden haben. Der Informationsgehalt
  sagt ob das wirklich so ist (aber eigentlich auch nur eine Umrechnung wie viel
  Möglichkeiten rausgeflogen sind im Verhältnis dern übrigen Möglichkeiten). Die
  Entropie wird derzeit glaube ich noch **falsch berechnet** (ist aber eh nicht
  so gut intuitiv zu erklären).<br>
  Neue Zeile -> neue Box/Night
- `<Staffel>.{pdf,png}`: Baum mit den noch übrigen Möglichkeiten. Die erste
  Zeile im Knoten ist immer der "Key" der auf der Ebene angeschaut wird
- `<Staffel>_tab.{pdf,png}`: Tabelle mit den noch übrigen Möglichkeiten
- `stats.pdf`: Ein paar Statistiken wann in den bisherigen Saffeln wieviel
  Information gewonnen wurde (Informationsgehalt). (Gedacht um vergleichen zu
  können wie sie sich so schlagen, aber Achtung: die Synchronisation an dieser
  Stelle ist nicht ganz einfach, und stimmt daher manchmal nicht ganz)<br>
  Der vertikale orangene Strich zeigt an, wann die Staffel vorbei war/ist
  (dahinter idR nur noch das richtige Ergebnis eingegeben).Der horizontale zeigt
  an, ab wieviel unbekannten Bits gewonnen werden kann (10 Möglichkeiten über
  normal)<br>
  Achtung: Die Punkte beziehen sich immer auf nach der MB/MN (d.h. wenn nach
  MN#10 nur noch 0 bit übrig sind wüssten sie das Ergebnis danach sicher, können
  es aber quasi nicht mehr einloggen)

# Selbst rumprobieren
Da die Ergebnisse automatisch gebaut werden, könnt ihr auch ein wenig rumspielen
(halt nicht richtig interaktiv, aber mehr als die Website (+ Account) braucht ihr nicht)

1. Github Account erstellen
2. Projekt `fork`en
   ![forken](./img/01_fork_dark.png)
4. Hier kommt noch eine Seite dazwischen, da könnt ihr z.B. dem Projekt nen andren Namen geben
   unter dem es bei euch laufen soll
3. Github actions aktivieren (automatisches Ergebnisse bauen)<br>
   Ab hier ist das alles in eurem eigenen Repository
   ![GH actions aktivieren](./img/02_enable-actions_dark.png)
4. Datei mit den Daten (`.dat`) editieren (falls Ergebnisse schon angeschaut, muss hier
   wieder auf den `main` Branch gewechselt werden, funktioniert genauso wie auf
   den `build` Branch wechseln)
   ![edit](./img/03_edit1_dark.png)
5. Datei speichern
   ![save](./img/04_commit_dark.png)
6. Status vom Ergebnisse baun anschaun (nicht unbedingt notwendig)
   ![status](./img/05_actions1_dark.png)
   ![status](./img/06_actions2_dark.png)
   ![status](./img/07_actions3_dark.png)
   Das kann ein wenig dauern (~1 min + paar min die ganzen Sachen immer neu
   installieren)
   ![status fin](./img/08_actions4_dark.png)
   Fertig (war erfolgreich, falls hier n rotes `x` ist, ist was schief gelaufen, wenn 
   ihr auf `build` klickt und `build results files` aufmacht seht ihr vielleicht wo der
   Fehler ist)
7. Ergebnisse anschaun, hierzu wieder branch wechseln
   ![zu `build` wechseln](./img/09_build1_dark.png)
   Ergebnisse liegen jetzt unter `<Staffel>/<Staffel>*.{png,pdf,out}`

## Eingabe-Dateien
Zusätzlich zu der folgenden "Dokumentation" ist es sinnvoll (evtl reicht es sogar aus) sich die Eingabedateien `*.yaml` vergangener Staffeln anzuschauen.

- `<Staffel>.yaml`:
  - Alles hinter einem `#` ist ein Kommentar und wird später
    ignoriert
  - Keys `setA` und `setB` geben die zu Anfang bekannten Teilnehmer an. `setB`
  muss dabei das (um eins) größere sein.
  - Mittels `rule_set` kann angegeben werden mit welchen Regeln die Sendung verläuft.
  Gibt es (noch) kein Doppelmatch (`!Eq`), ist die zusätzliche Person bekannt
  (`!FixedDup <name aus setB>`) oder ist irgendjemand (aus `setB`) die zusätzliche
  Person (`!SomeoneIsDup`).
  - Matchboxen und Matchingnights werden beide als `constraint` eingegeben mit
  der jeweiligen Anzahl an Lichtern.
      - Wenn für eine Modellierung eine Entscheidung mehrere Einschränkungen
      herbeiführt, kann ein Constraint als `hidden` markiert werden.
      - Mit dem hinzufügen von Doppelten Matches, kann ein Perfect-Match auch
      bedeuten, dass andere Matches explizit nicht mehr stattfinden. Daher werden bei
      einer Matchbox mit "einem Licht" (aka Perfect-Match) über den `exclude` key
      automatisch Paare ausgeschlossen werden. Sollte dies unerwünscht sein, kann
      der entwerder `noExclude: true` gesetzt werden oder der `exclude` manuell
      gesetzt werden (bei letzterem ist ist die syntax aber etwas komplizierter).
      - `type`: `MB` oder `MN` (wird für die Graphen benutzt)
      - `num`: Laufnummer der MBs/MNs (für die Graphen). Achtung, um globale
      Nummern zu erzeugen, wird `MB*2-1` und `MN*2` gerechnet => (`x <= num < x+0.5`
      um dopplungen zu vermeiden)
      - `comment`: Kommentar (zB in welcher Episode die Information zu finden
      ist), wird nur im text-output `.out` ausgegeben
      - `map`: Wer mit wem wird hier als `dict` (`x: y`) angegeben

## Anmerkungen
- Damit die Statistik stimmt, darauf achten, dass falls mehrere Einträge
  zusammen gehören die ersten die sind, die geskippt werden beim zählen
  <!-- (Informationsgehalt etc werden dem folgenden Eintrag zugeschrieben) -->
- Beim Eingeben von neuen Nights, vergisst man gerne die schon fest bekannten
  Matches. Einfach am Ende nochmal schaun ob es wirklich 10 Zeilen sind ;)
- An die Git(hub) Kenner, die Actions laufen nur mit dem `main` und `build`
  Branches, diese also nicht umbenennen (und nicht wundern wenns nicht klappt für
  neue Branches).

# "Kontakt"
Falls irgendwas nicht passen sollte, ihr was nicht versteht oder andere Anmerkungen habt, könnt ihr mir oben unter `Issues` hier auf Github eine Nachricht schreiben (wenn ihr auch einen Github Account habt).
