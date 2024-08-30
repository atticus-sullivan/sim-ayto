# Berechnete Ergebnisse

In den verlinkten Dateien ist primär der Verlauf der Staffeln zu sehen. Damit
ist gemeint welche Informationen gesammelt wurden und was das für die noch
verbleibenden Möglichkeiten bedeutet.

1. Dieser Verlauf ist einmal als großes Bild (um die Ausgabe auf einfache Art und
Weise einfärben zu können) und als reiner Text verlinkt.
<br>
Da der gesamte Verlauf enthalten ist und man eh nach unten scrollen muss, ist die
Spoilergefahr hier vielleicht nicht ganz so hoch.
2. (in 1. enthalten)
3. Zusätzlich gibt es jeweils noch die *aktuelle* Tabelle (**Achtung SPOILERGEFAHR!!!**).
4. Außerdem noch der aktuelle Baum mit den *noch verbleibenden* Möglichkeiten (Achtung
**SPOILERGEFAHR!!!**), sofern im aktuellen Stadium sinnvoll.

## AYTO

| Staffel 1 | [mit Farbe](data/de01/de01.col.png) | [nur Text](data/de01/de01.txt) | [aktuelle Tabelle](data/de01/de01_tab.png) | [Zusammenfassung](data/de01/de01_sum.png) | [aktueller Baum](data/de01/de01.pdf) |
| Staffel 2 | [mit Farbe](data/de02/de02.col.png) | [nur Text](data/de02/de02.txt) | [aktuelle Tabelle](data/de02/de02_tab.png) | [Zusammenfassung](data/de02/de02_sum.png) | [aktueller Baum](data/de02/de02.pdf) |
| Staffel 3 | [mit Farbe](data/de03/de03.col.png) | [nur Text](data/de03/de03.txt) | [aktuelle Tabelle](data/de03/de03_tab.png) | [Zusammenfassung](data/de03/de03_sum.png) | [aktueller Baum](data/de03/de03.pdf) |
| Staffel 4 | [mit Farbe](data/de04/de04.col.png) | [nur Text](data/de04/de04.txt) | [aktuelle Tabelle](data/de04/de04_tab.png) | [Zusammenfassung](data/de04/de04_sum.png) | [aktueller Baum](data/de04/de04.pdf) |
| Staffel 5 | [mit Farbe](data/de05/de05.col.png) | [nur Text](data/de05/de05.txt) | [aktuelle Tabelle](data/de05/de05_tab.png) | [Zusammenfassung](data/de05/de05_sum.png) | [aktueller Baum](data/de05/de05.pdf) |

## AYTO - RSIL

| Staffel 1 | [mit Farbe](data/de01r/de01r.col.png) | [nur Text](data/de01r/de01r.txt) | [aktuelle Tabelle](data/de01r/de01r_tab.png) | [Zusammenfassung](data/de01r/de01r_sum.png) | [aktueller Baum](data/de01r/de01r.pdf) |
| Staffel 2 | [mit Farbe](data/de02r/de02r.col.png) | [nur Text](data/de02r/de02r.txt) | [aktuelle Tabelle](data/de02r/de02r_tab.png) | [Zusammenfassung](data/de02r/de02r_sum.png) | [aktueller Baum](data/de02r/de02r.pdf) |
| Staffel 3 | [mit Farbe](data/de03r/de03r.col.png) | [nur Text](data/de03r/de03r.txt) | [aktuelle Tabelle](data/de03r/de03r_tab.png) | [Zusammenfassung](data/de03r/de03r_sum.png) | [aktueller Baum](data/de03r/de03r.pdf) |
| Staffel 4 | [mit Farbe](data/de04r/de04r.col.png) | [nur Text](data/de04r/de04r.txt) | [aktuelle Tabelle](data/de04r/de04r_tab.png) | [Zusammenfassung](data/de04r/de04r_sum.png) | |

## AYTO - US

| Staffel 1 | [mit Farbe](data/us01/us01.col.png) | [nur Text](data/us01/us01.txt) | [aktuelle Tabelle](data/us01/us01_tab.png) | [Zusammenfassung](data/us01/us01_sum.png) | [aktueller Baum](data/us01/us01.pdf) |
| Staffel 8 | [mit Farbe](data/us08/us08.col.png) | [nur Text](data/us08/us08.txt) | [aktuelle Tabelle](data/us08/us08_tab.png) | [Zusammenfassung](data/us08/us08_sum.png) | [aktueller Baum](data/us08/us08.pdf) |

# Vergleich der Staffeln untereinander

- [deutsche staffeln](stats_de.html).
- [us staffeln](stats_us.html).

# Weitere Erklärungen zu den Ausgaben

Im Anschluss sind ein paar Erklärungen zu den Ausgaben zu finden. Für noch mehr
infos (u.A. zu den Eingabedateien oder wie man das Tool mit eigenen Daten
füttern kann) findet ihr [hier das komplette Repository](https://github.com/atticus-sullivan/sim-ayto)
inklusive dem ganzen Code und den Daten.

Ebenso findet ihr dort auch eine Möglichkeit
[Fragen zu stellen](https://github.com/atticus-sullivan/sim-ayto/discussions/categories/q-a),
[Ideen und Anregungen zu teilen](https://github.com/atticus-sullivan/sim-ayto/discussions/categories/ideas)
und auf
[Bugs/Fehler hinzuweisen](https://github.com/atticus-sullivan/sim-ayto/issues).

## Normale Ausgaben
Vor der jeweiligen Tabelle kommt immer nochmal was genau als Einschränkung/Constraint dazu kam. Die genannte Episode bezieht sich dabei immer auf die Episode in der das ganze aufgelöst wurde. Bei den MNs steht vor dem jeweiligen Match zusätzlich wie oft diese bereits in einer MN zusammensaßen.

Das ganze `I` (Informationsgehalt) / `H` (Entropie, steht hinter wie viele Möglichkeiten noch übrig sind) ist der Versuch einzuschätzen wie viel eine Entscheidung gebracht hat und wie weit sie noch vom Ziel entfernt sind. Das ganze kommt aus der Informationstheorie.

`I[l/bits]`: Zeigt an wieviel Information mit dieser Entscheidung gewonnen wird angenommen die jeweilige Anzahl an Lichtern leuchten. Mittels $2^{-I}$ kann man falls gewünscht auf die Wahrscheinlichkeit zurückrechnen.

`E[I]/bits`: Ist der Erwartungswert, des Informationsgewinns.

Ganz am Ende wird eine Zusammenfassung über alle Constraints ausgegeben. Ein Stern in dieser Tabelle bedeutet, dass das Match das erste mal so zusammensaß. Eine kleine Übersicht über die nicht so intuitiven Spalten:
- `L` die Anzahl der Lichter
- `I` siehe oben
- `new` zählt wie viele Matches so in noch keiner MN zusammensaßen
- `min dist` als distanz wird die Anzahl unterschiedlicher Matches betrachtet, diese Spalte zeigt welche ander MN am ähnlichsten dieser ist (und wie ähnlich sie ist). In der erste MN kann dies natürlich noch nicht bestimmt werden.

## Baum
Im Baum ist die erste Zeile (entspricht der Person aus Set A) auf einer Ebene
immer fest. Somit steht jede Ebene für due Zuweisung einer (oder mehreren)
Person aus SetB zu der fixen Person aus SetA.

Bereits sicher feststehende Matches (sei es durch eine Matchingnight oder durch
Ausschlussverfahren) werden in die oberen Ebenen geschoben. Auch sonst werden
die Ebenen so sortiert, dass die Anzahl der *verschiedenen* Matches von oben
nach unten ansteigt.
