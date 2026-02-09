---
linkTitle: '{0}'
weight: 1
toc: false
---

# Anmerkungen
- [generelle Hinweise](/#noch-mehr-details) zu den Metriken (`H [bit]` und `I [bit]`).
  Letztlich ist `I` aber einfach nur eine Größe wieviel neue Informationen das gebracht hat und `H` nur eine andere Schreibweise für die Anzahl an übrigen Möglichkeiten.
- durch einen einfachen Klick in der Legende kann man einzelne Linien ausblenden
- durch einen Doppelklick in der Legende kann man alle Linien, außer der ausgewählten ausblenden
- ansonsten sind die Plots (bzgl Zoom/Verschieben) eigentlich ziemlich straight forward

# Regeln je Staffel
{1}

# Zusammenfassung
{2}

# Plots
<div class="plot-container plot-light">
{3}
</div>
<div class="plot-container plot-dark">
{4}
</div>
<script>
document.addEventListener("DOMContentLoaded", () => {{
    document.querySelectorAll('.hextra-tabs-toggle').forEach(tabButton => {{
        tabButton.addEventListener("click", () => {{
            window.dispatchEvent(new Event('resize'));
        }});
    }});
}});
</script>
