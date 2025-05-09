# sim-ayto
Berechnet die noch verfügbaren Möglichkeiten

<img src="img/example.png" width="800">

# Ergebnisse

Auf [atticus-sullivan.github.io/sim-ayto/](https://atticus-sullivan.github.io/sim-ayto/) findet ihr die Ergebnisse der Berechnungen.

# Selbst rumprobieren
Da die Ergebnisse automatisch gebaut werden, könnt ihr auch ein wenig rumspielen
(halt nicht richtig interaktiv, aber mehr als die Website (+ Account) braucht ihr nicht)

<details><summary>Anleitung</summary>

1. Github Account erstellen

2. Projekt `fork`en sodass ihr euere eigene Kopie von dem Projekt habt an dem ihr arbeiten könnt.
   <br>
   <img src="img/fork.png" width="800">
   
4. Hier kommt noch eine Seite dazwischen, da könnt ihr z.B. dem Projekt nen andren Namen geben unter dem es bei euch laufen soll. Wichtig dabei ist, dass ihr den Haken bei `copy main branch only` **wegmacht**.
   <br>
   <img src="img/fork2.png" width="400">
   
6. Github actions aktivieren (das ist der Mechanismus, der die Ergebnisse automatisch generiert)
   <br>
   <img src="img/enable-actions.png" width="800">

7. Github pages aktiviern und github actions als Quelle auswählen

8. Ab jetzt ist alles fertig eingerichtet und sobald ihr eine Datei in dem Projekt ändert, werden die entsprechenden Ergebnisse automatisch generiert.

9. Eingabedateien ändern. Hierbei ist es wichtig, dass ihr das auf dem `main` Branch macht. Das wechseln funktioniert genauso wie auf den `build` Branch wechseln beim Ergebnisse anschauen.
    <br>
    <img src="img/edit1.png" width="800">
    <br>
    <img src="img/edit2.png" width="800">
    <br>
    <img src="img/edit3.png" width="800"> 

11. Datei speichern
    <br>
    <img src="img/save1.png" width="800">
    <br>
    <img src="img/save2.png" width="400">

13. Status vom Ergebnisse generieren anschaun (nicht unbedingt notwendig)
    <br>
    <img src="img/status1.png" width="800">
    <br>
    Hier seht ihr ein Beispiel von einem erfolgreiehn durchlauf. Normalerweise wird bei euch der Schritt 1 länger dauern (das hängt ganz von den Änderungen ab die ihr gemacht habt). Die Schritte unter 2 brauchen aber nur beim ersten Durchlauf so lange und sind in allen weiteren Durchläufen deutlich schneller.
    <br>
    <img src="img/status2.png" width="800">
    <br>
    Bei Problemen sollte idR Schritt 1 fehlgeschlagen haben. Wenn ihr das aufklappt, findet ihr evtl raus was genau das Problem war (ihr könnt mir aber auch gerne [hier]([https://github.com/atticus-sullivan/sim-ayto/issues](https://github.com/atticus-sullivan/sim-ayto/discussions/categories/q-a)) schreiben in dem Fall.
   
11. Nachdem die Ergebnisse erfolgreich generiert wurden, sollte es in etwa so aussehen:
    <br>
    <img src="img/finished.png" width="800">
    <br>
    Wenn anstelle des grünen Hakens ein rotes X ist, ist beim generieren etwas schief gelaufen und ihr könnt wie in 8. beschrieben nachschaunen was genau das Problem war.
    <br>
    Die Ergebnisse könnt ihr euch jetzt wie oben unter [Ergebnisse](#Ergebnisse) beschrieben anschauen.

</details>

## Eingabe-Dateien
Zusätzlich zu der folgenden "Dokumentation" ist es sinnvoll (evtl reicht es sogar aus) sich die Eingabedateien `*.yaml` vergangener Staffeln anzuschauen.

<details><summary>Beschreibung des Dateiformats staffel.yaml </summary>

Allgemein gilt: Alles hinter einem `#` ist ein Kommentar und wird später ignoriert.

### Allgemeine Eingaben
#### setA/setB
Diese beiden Keys (`setA` und `setB`) geben die Teilnehmer der beiden Gruppen an.

#### renameA/renameB
Hier kann mit den Namen aus `setA`/`setB` als Keys ein anderer Name für die Person im Output angegeben werden. Beim Angeben der Maps der Constraints etc werden weiterhin die "alten" Namen verwendet, bei der Ausgabe der Tabellen (und Constraints) werden die "neuen" Namen verwendet. Idee ist es, einerseits falls im Laufe der Staffel Spitznamen entstehen, Personen umbenennen zu können (wobei man in diesem Fall besser "richtig" umbenennt). Andererseits kann man so längere Namen in der Ausgabe haben aber beim Definieren der Maps der Constraints mit den kürzeren Namen arbeiten.

#### rule_set
Mittels `rule_set` kann angegeben werden mit welchen Regeln die Sendung verläuft.

Mögliche Regeln:
- `rule_set: !Eq`: Kein Doppelmatch (bisher) enthalten

- `rule_set: !FixedDup <dup>`: Es gibt ein Doppelmatch. Eine Person aus dem Doppelmatch (`<dup>`) ist bereits bekannt. Diese muss aus `setB` kommen (bei Bedarf müssen die beiden sets leider vertauscht werden).

- `rule_set: !SomeoneIsDup`: Es gibt ein Doppelmatch. Weiter ist über dieses Doppelmatch jedoch nichts bekannt. Die beiden Doppelmatch personen müssen aus `setB` kommen (bei Bedarf müssen die beiden sets leider vertauscht werden).

- `rule_set: !FixedTrip <tripA>`: Gleich wie `FixedDup` nur, dass eine Person aus `setA` drei Matches aus `setB` hat von denen eine Person (`tripA`) bekannt ist.

- `rule_set: !SomeoneIsTrip`: Analog wie `FixedTrip` ist dies das pendant zu `SomeoneIsDup`.

- `rule_set: !NToN`: Jeder kann mit jedem ein Match sein. Hier gibt es ein paar Besonderheiten zu beachten.
  - `setA` und `setB` müssen in diesem Fall genau identisch sein.
  - Achtung: Abhängig von der Anzahl an Personen dauert die Berechnung hier deutlich länger

### Matchboxen und Matchingnights
Matchboxen und Matchingnights werden beide als `constraint` eingegeben

#### type
Dieses Feld bestimmt ob dieser constraint eine Matchbox oder Matchingnight ist.
- `type: !Box {num: 9, comment: "E11"}`: `num` kann auch `9.1` sein (sollte aber nicht größer oder gleich `x.5` sein, sonst macht das die Statistik mit anderen Staffeln kaputt). `comment` kann ein beliebiger string sein.

- `type: !Night {num: 9, comment: "E11"}`: Analog zur `Box`

#### check
Gibt an auf welche Art hier vergleichen wird.
- `check: !Lights [6]`: `6` Lichter waren an

- `check: !Eq`: Die Personen die in `map` als *values* angegeben sind haben dasselbe Match, welches das ist, ist bleibt aber unbekannt (der *key* ist dabei egal).

#### map
Hier wird angegeben wer mit wem in die Matchbox gegangen ist bzw wer mit wem in der Matchingnight saß.
Angegeben wird das ganze als *key-value* pair bei dem der *key* aus `setA` und der *value* aus `setB` kommt.

Beispiel:
```yaml
Sabrina: Mike
Paulina: Danilo
Kim: Paco
Alicia: Steffen
Jenny: Marvin
Steffi: Teezy
Darya: Emanuell
Marie: Fabio
Shakira: Peter
Sandra: Elia
```

#### weitere optionale Felder
<details><summary>Auflistung</summary>

##### hidden
- `hidden: true`: verhindert die Ausgabe einer Tabelle für diese Entscheidung.
- wenn nicht angegeben, ist `hidden: false` der standard.

##### noExclude
Mit dem Hinzufügen von Doppelten Matches, kann ein Perfect-Match auch bedeuten, dass andere Matches explizit nicht mehr stattfinden.
Daher werden bei einer Matchbox mit "einem Licht" (aka Perfect-Match) über den `exclude` key automatisch Paare ausgeschlossen werden.

Sollte dies unerwünscht sein, kann der entwerder `noExclude: true` gesetzt werden oder der `exclude` manuell gesetzt werden (bei letzterem ist ist die syntax aber etwas komplizierter).

Wenn nichts angegeben wird ist `noExclude: false` der Standard.

##### buildTree
- `buildTree: true`: generiert für diesen Constraint eine `.dot` Datei mit einer
  Baumdarstellung der verbliebenen Matches (wenn `hidden=true` ist, wird NIE ein
  Baum gebaut).
- wenn nicht angegeben, ist `buildTree: false` der standard.

##### resultUnknown
- `resultUnknown: true`: Sorgt dafür, dieser Constraint quasi ignoriert wird und
  auch keine Tabelle erstellt wird. Die Statistiken bzgl dem `check` Parameter
  (zB Erwartungswert und Wahrscheinlichkeitsverteilung der verschiedenen
  Ausgänge) werden weiterhin ausgegeben.
- wenn nicht angegeben, ist `resultUnknown: false` der standard.

</details>
</details>

## Anmerkungen
- Damit die Statistik stimmt, darauf achten, dass falls mehrere Einträge zusammen gehören die ersten die sind, die geskippt werden (mit `hidden: true`) beim Zählen

- Beim Eingeben von neuen Nights, vergisst man gerne die schon fest bekannten Matches. Einfach am Ende nochmal schaun ob es wirklich 10 Zeilen sind ;)

- An die Git(hub) Kenner, die Actions laufen nur mit dem `main` und `build` Branch, diese also nicht umbenennen (und nicht wundern wenns nicht klappt für neue Branches).

## Hinweise zur Funktionisweise
Prinzipiell funktioniert das Tool wie folgt:
- Es durchläuft alle Möglichkeiten
- Dabei werden alle Möglichkeiten eliminiert, die nicht mit einem Constraint (Matchingnight/Matchbox) vereinbar sind (dafür wird im Normalfall berechnet gegeben einem Matching, wieviele Lichter müssten an sein und dann überprüft ob das mit der Realität übereinstimmt)
- Basierend auf den noch übrigen Möglichkeiten könnte man dann die abschließende Tabelle generieren (in der Realität berechnet das Tool die Tabelle ein wenig anders, da auch de ganzen alten Tabellen unterwegs generiert werden sollen und auf die Art und Weise auch der benötigte Speicher (RAM) reduziert werden kann)

> [!NOTE]
> Eine Matchbox wird intern genauso wie eine Matchingnight gehandhabt (mit x Lichtern), nur dass es andere Checks beim Einlesen der Eingabe gibt um Fehler zu vermeiden

Wie alle Möglichkeiten aufgezählt werden können unterscheidet sich von Ruleset zu Ruleset. Der hierfür relevante Code findet sich in `rust > src > ruleset.rs > iter_perms()`. In der Regel wird folgendes Vorgehen angewandt:
1. Generiere alle Möglichkeiten die Personen aus set_b anzuordnen
2. Rechne jede dieser hierbei generierten Möglichkeiten auf keine, eine oder mehrere "echte" Möglichkeiten (je nach Ruleset) um
- (durch 2. kann die Fortschrittsanzeige teils night ganz korrekt sein. Extrem fällt das aktuell beim `NToN` ruleset auf.)

# Kontakt
Falls irgendwas nicht passen sollte, ihr was nicht versteht oder andere Anmerkungen habt, könnt ihr mir oben unter [Discussions](https://github.com/atticus-sullivan/sim-ayto/discussions/categories/q-a) hier auf Github eine Nachricht schreiben (wenn ihr auch einen Github Account habt).
