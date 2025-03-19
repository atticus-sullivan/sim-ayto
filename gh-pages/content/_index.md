---
title: 'Home'
weight: 1
# bookFlatSection: false
# bookToc: true
# bookHidden: false
# bookCollapseSection: false
# bookComments: false
# bookSearchExclude: false
---

## Übersicht

| Format     | AYTO DE                           | AYTO DE RSIL                           | AYTO US                           | AYTO UK                           |
| ----       | :--:                              | :--:                                   | :---:                             | :---:                             |
| Staffel  1 | [:white_check_mark:](ayto/de/01/) | [:white_check_mark:](ayto/de-rsil/01/) | [:white_check_mark:](ayto/us/01/) | [:white_check_mark:](ayto/uk/01/) |
| Staffel  2 | [:white_check_mark:](ayto/de/02/) | [:white_check_mark:](ayto/de-rsil/02/) | [:white_check_mark:](ayto/us/02/) |                                   |
| Staffel  3 | [:white_check_mark:](ayto/de/03/) | [:white_check_mark:](ayto/de-rsil/03/) | [:white_check_mark:](ayto/us/03/) |                                   |
| Staffel  4 | [:white_check_mark:](ayto/de/04/) | [:white_check_mark:](ayto/de-rsil/04/) | [:white_check_mark:](ayto/us/04/) |                                   |
| Staffel  5 | [:white_check_mark:](ayto/de/05/) |                                        | [:white_check_mark:](ayto/us/05/) |                                   |
| Staffel  6 | [:hourglass:       ](ayto/de/06/) |                                        | [:white_check_mark:](ayto/us/06/) |                                   |
| Staffel  7 |                                   |                                        | [:white_check_mark:](ayto/us/07/) |                                   |
| Staffel  8 |                                   |                                        | [:white_check_mark:](ayto/us/08/) |                                   |
| Staffel  9 |                                   |                                        | [:white_check_mark:](ayto/us/09/) |                                   |
| Staffel 10 |                                   |                                        | [                  ]()            |                                   |
<!-- :x: -->

Da es nur eine *UK* Staffel gibt bislang, ist diese Staffel bei den *US*
Staffeln im Vergleich mit enthalten.

## Informationen zur Darstellung

Für jede Staffel ist der gesamte Verlauf der Staffeln zu sehen. Damit
ist gemeint welche Informationen gesammelt wurden und was das für die noch
verbleibenden Möglichkeiten bedeutet.

*Baum* zeigt die genauen Matchings an, die noch möglich sind. Dies ist aber erst
sinnvoll wenn es nicht mehr all zu viele Möglichkeiten gibt.

### Infos zu Spoilern
Die Seiten sind immer so aufgebaut, dass man explizit ausklappen muss was man
sich ansehen möchte. Einzig was es für "Events" in der Folge gab ist
standardmäßig auf der Seite zu sehen (also ob/wieviele Matchboxen /
Matchingnights es in welcher Folge gab).

Die genannte Folge bezieht sich dabei immer darauf wann das ganze **aufgelöst
wurde**.

Ausklappbare Abschnitte mit besonderer Spoilergefahr (aktueller Stand und
gesamter Verlauf bis hier her) sind explizit mit einem :warning: gekennzeichnet.

Was der aktuellste Stand gerade ist kann man immer nachschauen, indem man bei
*Einzelne Tabellen* schaut was der letzte Eintrag gerade ist.

### Zu den Vergleichen
- das `- W` bzgl `- L` am Ende in der Legende steht dafür, ob der Cast in der
Staffel gewonnen (*win* `- W`) oder verloren hat (*loose* `- L`)

## Noch mehr Details

{{% details "Klicke hier für mehr" %}}
Im Anschluss sind noch ein paar mehr Erklärungen zu den Ausgaben zu finden. Für
noch mehr infos (u.A. zu den Eingabedateien oder wie man das Tool mit eigenen
Daten füttern kann) findet ihr [hier das komplette
Repository](https://github.com/atticus-sullivan/sim-ayto) inklusive dem ganzen
Code und den Daten.

Ebenso findet ihr dort auch eine Möglichkeit
[Fragen zu stellen](https://github.com/atticus-sullivan/sim-ayto/discussions/categories/q-a),
[Ideen und Anregungen zu teilen](https://github.com/atticus-sullivan/sim-ayto/discussions/categories/ideas)
und auf
[Bugs/Fehler hinzuweisen](https://github.com/atticus-sullivan/sim-ayto/issues).

### Normale Ausgaben
Vor der jeweiligen Tabelle kommt immer nochmal was genau als
Einschränkung/Constraint dazu kam. Die genannte Episode bezieht sich dabei immer
auf die **Episode in der das ganze aufgelöst wurde**. Bei den MNs steht vor dem
jeweiligen Match zusätzlich wie oft diese bereits in einer MN zusammensaßen.

Das ganze `I` (Informationsgehalt) / `H` (Entropie, steht hinter wie viele
Möglichkeiten noch übrig sind) ist der Versuch einzuschätzen wie viel eine
Entscheidung gebracht hat und wie weit sie noch vom Ziel entfernt sind. Das
ganze kommt aus der Informationstheorie.

`I[l/bits]`: Zeigt an wieviel Information mit dieser Entscheidung gewonnen wird
angenommen die jeweilige Anzahl an Lichtern leuchten. Mittels
\( 2^{-I} \)
kann man
falls gewünscht auf die Wahrscheinlichkeit zurückrechnen.

`E[I]/bits`: Ist der Erwartungswert, des Informationsgewinns.

#### Reguläre Tabellen
Die **Schrift**farbe ist ein Indikator dafür wie hoch die Wahrscheinlichkeit für
dieses Match ist (unter 1% rot, ab 45% gelb, ab 55% cyan, ab 80% grün).

Die **Hintergrund**farbe zeigt an welche Person(en) für eine andere Person am
wahrscheinlichsten ist.
- leicht **grüner** Hintergrund: Match ist für beide Personen am wahrscheinlichsten
- **leicht roter**/**hellgrauer** Hintergrund: Match ist für die Person deren
Spalte/Zeile das ist am wahrscheinlichsten.


#### Zusammenfassende Tabelle am Ende
Ganz am Ende wird eine Zusammenfassung über alle Constraints ausgegeben. Ein
Stern in dieser Tabelle bedeutet, dass das Match das erste mal so zusammensaß.
Eine kleine Übersicht über die nicht so intuitiven Spalten:
- `L` die Anzahl der Lichter
- `I` siehe oben
- `new` zählt wie viele Matches so in noch keiner MN zusammensaßen
- `min dist` als distanz wird die Anzahl unterschiedlicher Matches betrachtet,
diese Spalte zeigt welche ander MN am ähnlichsten dieser ist (und wie ähnlich
sie ist). In der erste MN kann dies natürlich noch nicht bestimmt werden.

### Baum
Im Baum ist die erste Zeile (entspricht der Person aus SetA) auf einer Ebene
immer fest. Somit steht jede Ebene für due Zuweisung einer (oder mehreren)
Person aus SetB zu der fixen Person aus SetA.

Bereits sicher feststehende Matches (sei es durch eine Matchbox oder durch
Ausschlussverfahren) werden in die oberen Ebenen geschoben. Auch sonst werden
die Ebenen so sortiert, dass die Anzahl der *verschiedenen* Matches von oben
nach unten ansteigt.
{{% /details %}}
