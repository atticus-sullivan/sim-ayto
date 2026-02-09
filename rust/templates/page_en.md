linkTitle: '{0}'
weight: 1
toc: false
---

# Remarks
- [general information](/en/#more-details) regarding the metrics (`H [bit]` and `I [bit]`).
  In the end `I` is just a measure for how much new information was gained and `H` just a different notation for the amount of left possibilities.
- with a single-click on items in the legend you can hide that line in the plot
- with a double-click on an item in the legend you can hide all other lines in the plot
- other things like zooming or panning of the plots should be pretty straight forward

# Ruleset per Season
{1}

# Summary
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
