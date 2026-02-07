---
linkTitle: 'US + UK'
weight: 1
toc: false

---
# Remarks
- [general information](/sim-ayto/en/#more-details) regarding the metrics (`H [bit]` and `I [bit]`).
  In the end `I` is just a measure for how much new information was gained and `H` just a different notation for the amount of left possibilities.
- with a single-click on items in the legend you can hide that line in the plot
- with a double-click on an item in the legend you can hide all other lines in the plot
- other things like zooming or panning of the plots should be pretty straight forward

# Ruleset per Season
| {{< i18n "season" >}} | {{< i18n "players" >}} | {{< i18n "rulesetShort" >}} | {{< i18n "rulesetDesc" >}} |
| --- | --- | --- | --- |
| uk01 | 10/10 | = | {{< i18n "rs-Eq" >}} |
| us01 | 10/10 | = | {{< i18n "rs-Eq" >}} |
| us02 | 10/11 | ?0=1 | {{< i18n "rs-XTimesDup-1-0" >}} |
| us03 | 10/10 | = | {{< i18n "rs-Eq" >}} |
| us04 | 10/10 | = | {{< i18n "rs-Eq" >}} |
| us05 | 11/11 | = | {{< i18n "rs-Eq" >}} |
| us06 | 11/11 | = | {{< i18n "rs-Eq" >}} |
| us07 | 11/11 | = | {{< i18n "rs-Eq" >}} |
| us08 | 16/16 | N:N | {{< i18n "rs-NToN" >}} |
| us09 | 11/11 | = | {{< i18n "rs-Eq" >}} |
    

# Plots
<div class="plot-container plot-light">
<script src="https://cdn.plot.ly/plotly-3.3.1.min.js"></script>
{{< tabs items="MN/MC,MB/TB,Combined,#Lights MB/TB,#Lights MN/MC,#Lights-known MN/MC" >}}
{{% tab %}}<div id="fNj9TP9jkaKSX9x3SwS8" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("fNj9TP9jkaKSX9x3SwS8", {
  "data": [],
  "layout": {
    "title": {
      "text": "Matchingnight / matching ceremony"
    },
    "margin": {
      "autoexpand": true
    },
    "autosize": true,
    "font": {
      "color": "#4c4f69"
    },
    "paper_bgcolor": "#eff1f5",
    "plot_bgcolor": "#eff1f5",
    "colorway": [
      "#1e66f5",
      "#df8e1d",
      "#40a02b",
      "#d20f39",
      "#8839ef",
      "#dc8a78",
      "#ea76cb",
      "#fe640b",
      "#e64553",
      "#179299",
      "#209fb5"
    ],
    "hovermode": "x",
    "clickmode": "event",
    "dragmode": "pan",
    "xaxis": {
      "title": {
        "text": "#MB"
      },
      "mirror": true,
      "showline": true,
      "linecolor": "#9ca0b0",
      "gridcolor": "#8c8fa1",
      "zerolinecolor": "#7c7f93"
    },
    "yaxis": {
      "title": {
        "text": "I [bit]"
      },
      "linecolor": "#9ca0b0",
      "gridcolor": "#8c8fa1",
      "zerolinecolor": "#7c7f93"
    }
  },
  "config": {
    "scrollZoom": true,
    "displaylogo": false,
    "responsive": true
  }
});
</script>{{% /tab %}}{{% tab %}}<div id="GdKxh6wtq8rqNGvdVUzM" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("GdKxh6wtq8rqNGvdVUzM", {
  "data": [],
  "layout": {
    "title": {
      "text": "Matchbox / truth booth"
    },
    "margin": {
      "autoexpand": true
    },
    "autosize": true,
    "font": {
      "color": "#4c4f69"
    },
    "paper_bgcolor": "#eff1f5",
    "plot_bgcolor": "#eff1f5",
    "colorway": [
      "#1e66f5",
      "#df8e1d",
      "#40a02b",
      "#d20f39",
      "#8839ef",
      "#dc8a78",
      "#ea76cb",
      "#fe640b",
      "#e64553",
      "#179299",
      "#209fb5"
    ],
    "hovermode": "x",
    "clickmode": "event",
    "dragmode": "pan",
    "xaxis": {
      "title": {
        "text": "#MN"
      },
      "mirror": true,
      "showline": true,
      "linecolor": "#9ca0b0",
      "gridcolor": "#8c8fa1",
      "zerolinecolor": "#7c7f93"
    },
    "yaxis": {
      "title": {
        "text": "I [bit]"
      },
      "linecolor": "#9ca0b0",
      "gridcolor": "#8c8fa1",
      "zerolinecolor": "#7c7f93"
    }
  },
  "config": {
    "scrollZoom": true,
    "displaylogo": false,
    "responsive": true
  }
});
</script>{{% /tab %}}{{% tab %}}<div id="oXFBauEFjLD11dt5iY6d" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("oXFBauEFjLD11dt5iY6d", {
  "data": [],
  "layout": {
    "title": {
      "text": "Left possibilities"
    },
    "margin": {
      "autoexpand": true
    },
    "autosize": true,
    "font": {
      "color": "#4c4f69"
    },
    "paper_bgcolor": "#eff1f5",
    "plot_bgcolor": "#eff1f5",
    "colorway": [
      "#1e66f5",
      "#df8e1d",
      "#40a02b",
      "#d20f39",
      "#8839ef",
      "#dc8a78",
      "#ea76cb",
      "#fe640b",
      "#e64553",
      "#179299",
      "#209fb5"
    ],
    "hovermode": "x",
    "clickmode": "event",
    "dragmode": "pan",
    "xaxis": {
      "title": {
        "text": "#MB/#MN"
      },
      "mirror": true,
      "showline": true,
      "linecolor": "#9ca0b0",
      "gridcolor": "#8c8fa1",
      "zerolinecolor": "#7c7f93"
    },
    "yaxis": {
      "title": {
        "text": "H [bit]"
      },
      "linecolor": "#9ca0b0",
      "gridcolor": "#8c8fa1",
      "zerolinecolor": "#7c7f93"
    }
  },
  "config": {
    "scrollZoom": true,
    "displaylogo": false,
    "responsive": true
  }
});
</script>{{% /tab %}}{{% tab %}}<div id="1JR5ft5hmzRK1O1It7k9" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("1JR5ft5hmzRK1O1It7k9", {
  "data": [],
  "layout": {
    "title": {
      "text": "#Lights -- MB"
    },
    "margin": {
      "autoexpand": true
    },
    "autosize": true,
    "font": {
      "color": "#4c4f69"
    },
    "paper_bgcolor": "#eff1f5",
    "plot_bgcolor": "#eff1f5",
    "colorway": [
      "#1e66f5",
      "#df8e1d",
      "#40a02b",
      "#d20f39",
      "#8839ef",
      "#dc8a78",
      "#ea76cb",
      "#fe640b",
      "#e64553",
      "#179299",
      "#209fb5"
    ],
    "hovermode": "x",
    "clickmode": "event",
    "dragmode": "pan",
    "xaxis": {
      "title": {
        "text": "#MB"
      },
      "mirror": true,
      "showline": true,
      "linecolor": "#9ca0b0",
      "gridcolor": "#8c8fa1",
      "zerolinecolor": "#7c7f93"
    },
    "yaxis": {
      "title": {
        "text": "#Lights"
      },
      "linecolor": "#9ca0b0",
      "gridcolor": "#8c8fa1",
      "zerolinecolor": "#7c7f93"
    }
  },
  "config": {
    "scrollZoom": true,
    "displaylogo": false,
    "responsive": true
  }
});
</script>{{% /tab %}}{{% tab %}}<div id="HT9C9ahvroVBzVeSNgl0" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("HT9C9ahvroVBzVeSNgl0", {
  "data": [],
  "layout": {
    "title": {
      "text": "#Lights -- MN"
    },
    "margin": {
      "autoexpand": true
    },
    "autosize": true,
    "font": {
      "color": "#4c4f69"
    },
    "paper_bgcolor": "#eff1f5",
    "plot_bgcolor": "#eff1f5",
    "colorway": [
      "#1e66f5",
      "#df8e1d",
      "#40a02b",
      "#d20f39",
      "#8839ef",
      "#dc8a78",
      "#ea76cb",
      "#fe640b",
      "#e64553",
      "#179299",
      "#209fb5"
    ],
    "hovermode": "x",
    "clickmode": "event",
    "dragmode": "pan",
    "xaxis": {
      "title": {
        "text": "#MN"
      },
      "mirror": true,
      "showline": true,
      "linecolor": "#9ca0b0",
      "gridcolor": "#8c8fa1",
      "zerolinecolor": "#7c7f93"
    },
    "yaxis": {
      "title": {
        "text": "#Lights"
      },
      "linecolor": "#9ca0b0",
      "gridcolor": "#8c8fa1",
      "zerolinecolor": "#7c7f93"
    }
  },
  "config": {
    "scrollZoom": true,
    "displaylogo": false,
    "responsive": true
  }
});
</script>{{% /tab %}}{{% tab %}}<div id="iqqayZYee3tPzo55wfPW" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("iqqayZYee3tPzo55wfPW", {
  "data": [],
  "layout": {
    "title": {
      "text": "#Lights - known_lights -- MN"
    },
    "margin": {
      "autoexpand": true
    },
    "autosize": true,
    "font": {
      "color": "#4c4f69"
    },
    "paper_bgcolor": "#eff1f5",
    "plot_bgcolor": "#eff1f5",
    "colorway": [
      "#1e66f5",
      "#df8e1d",
      "#40a02b",
      "#d20f39",
      "#8839ef",
      "#dc8a78",
      "#ea76cb",
      "#fe640b",
      "#e64553",
      "#179299",
      "#209fb5"
    ],
    "hovermode": "x",
    "clickmode": "event",
    "dragmode": "pan",
    "xaxis": {
      "title": {
        "text": "#MN"
      },
      "mirror": true,
      "showline": true,
      "linecolor": "#9ca0b0",
      "gridcolor": "#8c8fa1",
      "zerolinecolor": "#7c7f93"
    },
    "yaxis": {
      "title": {
        "text": "#Lights - known_lights"
      },
      "linecolor": "#9ca0b0",
      "gridcolor": "#8c8fa1",
      "zerolinecolor": "#7c7f93"
    }
  },
  "config": {
    "scrollZoom": true,
    "displaylogo": false,
    "responsive": true
  }
});
</script>{{% /tab %}}
{{< /tabs >}}
</div>
<div class="plot-container plot-dark">
<script src="https://cdn.plot.ly/plotly-3.3.1.min.js"></script>
{{< tabs items="MN/MC,MB/TB,Combined,#Lights MB/TB,#Lights MN/MC,#Lights-known MN/MC" >}}
{{% tab %}}<div id="xeIIVcSVCRopAa2Ymk4p" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("xeIIVcSVCRopAa2Ymk4p", {
  "data": [],
  "layout": {
    "title": {
      "text": "Matchingnight / matching ceremony"
    },
    "margin": {
      "autoexpand": true
    },
    "autosize": true,
    "font": {
      "color": "#cad3f5"
    },
    "paper_bgcolor": "#24273a",
    "plot_bgcolor": "#24273a",
    "colorway": [
      "#8aadf4",
      "#eed49f",
      "#a6da95",
      "#ed8796",
      "#c6a0f6",
      "#f4dbd6",
      "#f5bde6",
      "#f5a97f",
      "#ee99a0",
      "#8bd5ca",
      "#7dc4e4"
    ],
    "hovermode": "x",
    "clickmode": "event",
    "dragmode": "pan",
    "xaxis": {
      "title": {
        "text": "#MB"
      },
      "mirror": true,
      "showline": true,
      "linecolor": "#6e738d",
      "gridcolor": "#8087a2",
      "zerolinecolor": "#939ab7"
    },
    "yaxis": {
      "title": {
        "text": "I [bit]"
      },
      "linecolor": "#6e738d",
      "gridcolor": "#8087a2",
      "zerolinecolor": "#939ab7"
    }
  },
  "config": {
    "scrollZoom": true,
    "displaylogo": false,
    "responsive": true
  }
});
</script>{{% /tab %}}{{% tab %}}<div id="LABm0E2iSpH0ysDwkJ6O" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("LABm0E2iSpH0ysDwkJ6O", {
  "data": [],
  "layout": {
    "title": {
      "text": "Matchbox / truth booth"
    },
    "margin": {
      "autoexpand": true
    },
    "autosize": true,
    "font": {
      "color": "#cad3f5"
    },
    "paper_bgcolor": "#24273a",
    "plot_bgcolor": "#24273a",
    "colorway": [
      "#8aadf4",
      "#eed49f",
      "#a6da95",
      "#ed8796",
      "#c6a0f6",
      "#f4dbd6",
      "#f5bde6",
      "#f5a97f",
      "#ee99a0",
      "#8bd5ca",
      "#7dc4e4"
    ],
    "hovermode": "x",
    "clickmode": "event",
    "dragmode": "pan",
    "xaxis": {
      "title": {
        "text": "#MN"
      },
      "mirror": true,
      "showline": true,
      "linecolor": "#6e738d",
      "gridcolor": "#8087a2",
      "zerolinecolor": "#939ab7"
    },
    "yaxis": {
      "title": {
        "text": "I [bit]"
      },
      "linecolor": "#6e738d",
      "gridcolor": "#8087a2",
      "zerolinecolor": "#939ab7"
    }
  },
  "config": {
    "scrollZoom": true,
    "displaylogo": false,
    "responsive": true
  }
});
</script>{{% /tab %}}{{% tab %}}<div id="6eqbyrxJHbfOgT1pfa2X" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("6eqbyrxJHbfOgT1pfa2X", {
  "data": [],
  "layout": {
    "title": {
      "text": "Left possibilities"
    },
    "margin": {
      "autoexpand": true
    },
    "autosize": true,
    "font": {
      "color": "#cad3f5"
    },
    "paper_bgcolor": "#24273a",
    "plot_bgcolor": "#24273a",
    "colorway": [
      "#8aadf4",
      "#eed49f",
      "#a6da95",
      "#ed8796",
      "#c6a0f6",
      "#f4dbd6",
      "#f5bde6",
      "#f5a97f",
      "#ee99a0",
      "#8bd5ca",
      "#7dc4e4"
    ],
    "hovermode": "x",
    "clickmode": "event",
    "dragmode": "pan",
    "xaxis": {
      "title": {
        "text": "#MB/#MN"
      },
      "mirror": true,
      "showline": true,
      "linecolor": "#6e738d",
      "gridcolor": "#8087a2",
      "zerolinecolor": "#939ab7"
    },
    "yaxis": {
      "title": {
        "text": "H [bit]"
      },
      "linecolor": "#6e738d",
      "gridcolor": "#8087a2",
      "zerolinecolor": "#939ab7"
    }
  },
  "config": {
    "scrollZoom": true,
    "displaylogo": false,
    "responsive": true
  }
});
</script>{{% /tab %}}{{% tab %}}<div id="lMuFbbPoywhHG3tp39Ga" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("lMuFbbPoywhHG3tp39Ga", {
  "data": [],
  "layout": {
    "title": {
      "text": "#Lights -- MB"
    },
    "margin": {
      "autoexpand": true
    },
    "autosize": true,
    "font": {
      "color": "#cad3f5"
    },
    "paper_bgcolor": "#24273a",
    "plot_bgcolor": "#24273a",
    "colorway": [
      "#8aadf4",
      "#eed49f",
      "#a6da95",
      "#ed8796",
      "#c6a0f6",
      "#f4dbd6",
      "#f5bde6",
      "#f5a97f",
      "#ee99a0",
      "#8bd5ca",
      "#7dc4e4"
    ],
    "hovermode": "x",
    "clickmode": "event",
    "dragmode": "pan",
    "xaxis": {
      "title": {
        "text": "#MB"
      },
      "mirror": true,
      "showline": true,
      "linecolor": "#6e738d",
      "gridcolor": "#8087a2",
      "zerolinecolor": "#939ab7"
    },
    "yaxis": {
      "title": {
        "text": "#Lights"
      },
      "linecolor": "#6e738d",
      "gridcolor": "#8087a2",
      "zerolinecolor": "#939ab7"
    }
  },
  "config": {
    "scrollZoom": true,
    "displaylogo": false,
    "responsive": true
  }
});
</script>{{% /tab %}}{{% tab %}}<div id="IVV0jhyVQ0gxDs4h38xg" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("IVV0jhyVQ0gxDs4h38xg", {
  "data": [],
  "layout": {
    "title": {
      "text": "#Lights -- MN"
    },
    "margin": {
      "autoexpand": true
    },
    "autosize": true,
    "font": {
      "color": "#cad3f5"
    },
    "paper_bgcolor": "#24273a",
    "plot_bgcolor": "#24273a",
    "colorway": [
      "#8aadf4",
      "#eed49f",
      "#a6da95",
      "#ed8796",
      "#c6a0f6",
      "#f4dbd6",
      "#f5bde6",
      "#f5a97f",
      "#ee99a0",
      "#8bd5ca",
      "#7dc4e4"
    ],
    "hovermode": "x",
    "clickmode": "event",
    "dragmode": "pan",
    "xaxis": {
      "title": {
        "text": "#MN"
      },
      "mirror": true,
      "showline": true,
      "linecolor": "#6e738d",
      "gridcolor": "#8087a2",
      "zerolinecolor": "#939ab7"
    },
    "yaxis": {
      "title": {
        "text": "#Lights"
      },
      "linecolor": "#6e738d",
      "gridcolor": "#8087a2",
      "zerolinecolor": "#939ab7"
    }
  },
  "config": {
    "scrollZoom": true,
    "displaylogo": false,
    "responsive": true
  }
});
</script>{{% /tab %}}{{% tab %}}<div id="ILC0DDNJqoGHxWHYkMN8" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("ILC0DDNJqoGHxWHYkMN8", {
  "data": [],
  "layout": {
    "title": {
      "text": "#Lights - known_lights -- MN"
    },
    "margin": {
      "autoexpand": true
    },
    "autosize": true,
    "font": {
      "color": "#cad3f5"
    },
    "paper_bgcolor": "#24273a",
    "plot_bgcolor": "#24273a",
    "colorway": [
      "#8aadf4",
      "#eed49f",
      "#a6da95",
      "#ed8796",
      "#c6a0f6",
      "#f4dbd6",
      "#f5bde6",
      "#f5a97f",
      "#ee99a0",
      "#8bd5ca",
      "#7dc4e4"
    ],
    "hovermode": "x",
    "clickmode": "event",
    "dragmode": "pan",
    "xaxis": {
      "title": {
        "text": "#MN"
      },
      "mirror": true,
      "showline": true,
      "linecolor": "#6e738d",
      "gridcolor": "#8087a2",
      "zerolinecolor": "#939ab7"
    },
    "yaxis": {
      "title": {
        "text": "#Lights - known_lights"
      },
      "linecolor": "#6e738d",
      "gridcolor": "#8087a2",
      "zerolinecolor": "#939ab7"
    }
  },
  "config": {
    "scrollZoom": true,
    "displaylogo": false,
    "responsive": true
  }
});
</script>{{% /tab %}}
{{< /tabs >}}
</div>
<script>
document.addEventListener("DOMContentLoaded", () => {
    document.querySelectorAll('.hextra-tabs-toggle').forEach(tabButton => {
        tabButton.addEventListener("click", () => {
            window.dispatchEvent(new Event('resize'));
        });
    });
});
</script>
