---
linkTitle: 'DE'
weight: 1
toc: false

---
# Anmerkungen
- [generelle Hinweise](/sim-ayto#noch-mehr-details) zu den Metriken (`H [bit]` und `I [bit]`).
  Letztlich ist `I` aber einfach nur eine Größe wieviel neue Informationen das gebracht hat und `H` nur eine andere Schreibweise für die Anzahl an übrigen Möglichkeiten.
- durch einen einfachen Klick in der Legende kann man einzelne Linien ausblenden
- durch einen Doppelklick in der Legende kann man alle Linien, außer der ausgewählten ausblenden
- ansonsten sind die Plots (bzgl Zoom/Verschieben) eigentlich ziemlich straight forward

# Regeln je Staffel
| {{< i18n "season" >}} | {{< i18n "players" >}} | {{< i18n "rulesetShort" >}} | {{< i18n "rulesetDesc" >}} |
| --- | --- | --- | --- |
| de01 | 10/11 | ?0=1 | {{< i18n "rs-XTimesDup-1-0" >}} |
| de01r | 10/11 | ?0=1 | {{< i18n "rs-XTimesDup-1-0" >}} |
| de02 | 10/11 | ?0=1 | {{< i18n "rs-XTimesDup-1-0" >}} |
| de02r | 10/11 | ?0=1 | {{< i18n "rs-XTimesDup-1-0" >}} |
| de03 | 10/11 | ?0=1 | {{< i18n "rs-XTimesDup-1-0" >}} |
| de03r | 10/11 | ?0=1 | {{< i18n "rs-XTimesDup-1-0" >}} |
| de04 | 10/11 | ?1=0 | {{< i18n "rs-XTimesDup-0-1" >}} |
| de04r | 10/11 | ?0=1 | {{< i18n "rs-XTimesDup-1-0" >}} |
| de05 | 10/12 | =3 | {{< i18n "rs-FixedTrip" >}} |
| de05r | 10/12 | ?1=1 | {{< i18n "rs-XTimesDup-1-1" >}} |
| de06 | 10/11 | ?1=0 | {{< i18n "rs-XTimesDup-0-1" >}} |
| de07 | 10/11 | ?1=0 | {{< i18n "rs-XTimesDup-0-1" >}} |
    

# Plots
<div class="plot-container plot-light">
<script src="https://cdn.plot.ly/plotly-3.3.1.min.js"></script>
{{< tabs items="MN/MC,MB/TB,Combined,#Lights MB/TB,#Lights MN/MC,#Lights-known MN/MC" >}}
{{% tab %}}<div id="D9C9pS2Z5xefyIO2xdmE" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("D9C9pS2Z5xefyIO2xdmE", {
  "data": [
    {
      "type": "scatter",
      "name": "de01- W",
      "mode": "lines",
      "x": [
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
        8.0,
        9.0,
        10.0
      ],
      "y": [
        1.4426960309452606,
        2.4181608390932303,
        0.1886180433233922,
        1.4766506896860045,
        3.313890775821944,
        0.9004643264490856,
        2.0703893278913976,
        3.3219280948873626,
        0.0,
        0.0
      ],
      "text": [
        "E03",
        "E04",
        "E06",
        "E08 -- Blackout",
        "E10",
        "E12",
        "E14",
        "E16",
        "E18",
        "E20"
      ]
    }
  ],
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
</script>{{% /tab %}}{{% tab %}}<div id="NkfQb2rFH7GkDRZHDAfl" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("NkfQb2rFH7GkDRZHDAfl", {
  "data": [
    {
      "type": "scatter",
      "name": "de01- W",
      "mode": "lines",
      "x": [
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
        8.0,
        9.0,
        10.0
      ],
      "y": [
        0.15200309344504995,
        0.14755718841385784,
        3.5680243699622696,
        1.9913585001720526,
        0.24410467729894278,
        3.040641984497346,
        0.25153876699596445,
        0.5849625007211561,
        0.0,
        0.0
      ],
      "text": [
        "E02",
        "E04",
        "E06",
        "E08",
        "E10",
        "E12",
        "E14",
        "E16",
        "E18",
        "E20"
      ]
    }
  ],
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
</script>{{% /tab %}}{{% tab %}}<div id="awFYPklRAakmYhgdfWSS" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("awFYPklRAakmYhgdfWSS", {
  "data": [
    {
      "type": "scatter",
      "name": "de01- W",
      "mode": "lines",
      "x": [
        0.0,
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
        8.0,
        9.0,
        10.0,
        11.0,
        12.0,
        13.0,
        14.0,
        15.0,
        16.0,
        17.0,
        18.0,
        19.0,
        20.0
      ],
      "y": [
        25.112989209604315,
        24.960986116159265,
        23.518290085214005,
        23.37073289680015,
        20.952572057706917,
        17.384547687744647,
        17.195929644421255,
        15.204571144249204,
        13.727920454563199,
        13.483815777264256,
        10.169925001442312,
        7.129283016944966,
        6.22881869049588,
        5.977279923499917,
        3.9068905956085187,
        3.321928094887362,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0
      ],
      "text": [
        "initial",
        "MB#1-E02",
        "MN#1-E03",
        "MB#2-E04",
        "MN#2-E04",
        "MB#3-E06",
        "MN#3-E06",
        "MB#4-E08",
        "MN#4-E08 -- Blackout",
        "MB#5-E10",
        "MN#5-E10",
        "MB#6-E12",
        "MN#6-E12",
        "MB#7-E14",
        "MN#7-E14",
        "MB#8-E16",
        "MN#8-E16",
        "MB#9-E18",
        "MN#9-E18",
        "MB#10-E20",
        "MN#10-E20"
      ]
    }
  ],
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
</script>{{% /tab %}}{{% tab %}}<div id="N74GqMOiyeI28qlxgHr5" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("N74GqMOiyeI28qlxgHr5", {
  "data": [
    {
      "type": "scatter",
      "name": "de01- W",
      "mode": "lines+markers",
      "x": [
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
        8.0,
        9.0,
        10.0
      ],
      "y": [
        0,
        0,
        1,
        1,
        0,
        1,
        0,
        0,
        0,
        1
      ],
      "text": [
        "E02",
        "E04",
        "E06",
        "E08",
        "E10",
        "E12",
        "E14",
        "E16",
        "E18",
        "E20"
      ]
    }
  ],
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
</script>{{% /tab %}}{{% tab %}}<div id="ZkN3i63jf2hoKV1Qj8YY" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("ZkN3i63jf2hoKV1Qj8YY", {
  "data": [
    {
      "type": "scatter",
      "name": "de01- W",
      "mode": "lines+markers",
      "x": [
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
        8.0,
        9.0,
        10.0
      ],
      "y": [
        1,
        2,
        3,
        2,
        5,
        5,
        5,
        6,
        7,
        10
      ],
      "text": [
        "E03",
        "E04",
        "E06",
        "E08 -- Blackout",
        "E10",
        "E12",
        "E14",
        "E16",
        "E18",
        "E20"
      ]
    }
  ],
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
</script>{{% /tab %}}{{% tab %}}<div id="JcShw9TchtFtfBnzDjet" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("JcShw9TchtFtfBnzDjet", {
  "data": [
    {
      "type": "scatter",
      "name": "de01- W",
      "mode": "lines+markers",
      "x": [
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
        8.0,
        9.0,
        10.0
      ],
      "y": [
        1,
        2,
        2,
        0,
        3,
        2,
        2,
        3,
        4,
        6
      ],
      "text": [
        "E03",
        "E04",
        "E06",
        "E08 -- Blackout",
        "E10",
        "E12",
        "E14",
        "E16",
        "E18",
        "E20"
      ]
    }
  ],
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
{{% tab %}}<div id="05OLH8JTy2hNaxrSeCrE" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("05OLH8JTy2hNaxrSeCrE", {
  "data": [
    {
      "type": "scatter",
      "name": "de01- W",
      "mode": "lines",
      "x": [
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
        8.0,
        9.0,
        10.0
      ],
      "y": [
        1.4426960309452606,
        2.4181608390932303,
        0.1886180433233922,
        1.4766506896860045,
        3.313890775821944,
        0.9004643264490856,
        2.0703893278913976,
        3.3219280948873626,
        0.0,
        0.0
      ],
      "text": [
        "E03",
        "E04",
        "E06",
        "E08 -- Blackout",
        "E10",
        "E12",
        "E14",
        "E16",
        "E18",
        "E20"
      ]
    }
  ],
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
</script>{{% /tab %}}{{% tab %}}<div id="kLFTzuFVFcHu1v9P2mtn" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("kLFTzuFVFcHu1v9P2mtn", {
  "data": [
    {
      "type": "scatter",
      "name": "de01- W",
      "mode": "lines",
      "x": [
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
        8.0,
        9.0,
        10.0
      ],
      "y": [
        0.15200309344504995,
        0.14755718841385784,
        3.5680243699622696,
        1.9913585001720526,
        0.24410467729894278,
        3.040641984497346,
        0.25153876699596445,
        0.5849625007211561,
        0.0,
        0.0
      ],
      "text": [
        "E02",
        "E04",
        "E06",
        "E08",
        "E10",
        "E12",
        "E14",
        "E16",
        "E18",
        "E20"
      ]
    }
  ],
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
</script>{{% /tab %}}{{% tab %}}<div id="fj9OLMluLAOs7Kc2wlvt" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("fj9OLMluLAOs7Kc2wlvt", {
  "data": [
    {
      "type": "scatter",
      "name": "de01- W",
      "mode": "lines",
      "x": [
        0.0,
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
        8.0,
        9.0,
        10.0,
        11.0,
        12.0,
        13.0,
        14.0,
        15.0,
        16.0,
        17.0,
        18.0,
        19.0,
        20.0
      ],
      "y": [
        25.112989209604315,
        24.960986116159265,
        23.518290085214005,
        23.37073289680015,
        20.952572057706917,
        17.384547687744647,
        17.195929644421255,
        15.204571144249204,
        13.727920454563199,
        13.483815777264256,
        10.169925001442312,
        7.129283016944966,
        6.22881869049588,
        5.977279923499917,
        3.9068905956085187,
        3.321928094887362,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0
      ],
      "text": [
        "initial",
        "MB#1-E02",
        "MN#1-E03",
        "MB#2-E04",
        "MN#2-E04",
        "MB#3-E06",
        "MN#3-E06",
        "MB#4-E08",
        "MN#4-E08 -- Blackout",
        "MB#5-E10",
        "MN#5-E10",
        "MB#6-E12",
        "MN#6-E12",
        "MB#7-E14",
        "MN#7-E14",
        "MB#8-E16",
        "MN#8-E16",
        "MB#9-E18",
        "MN#9-E18",
        "MB#10-E20",
        "MN#10-E20"
      ]
    }
  ],
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
</script>{{% /tab %}}{{% tab %}}<div id="zT0L3RPA0BZjDrzVueVJ" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("zT0L3RPA0BZjDrzVueVJ", {
  "data": [
    {
      "type": "scatter",
      "name": "de01- W",
      "mode": "lines+markers",
      "x": [
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
        8.0,
        9.0,
        10.0
      ],
      "y": [
        0,
        0,
        1,
        1,
        0,
        1,
        0,
        0,
        0,
        1
      ],
      "text": [
        "E02",
        "E04",
        "E06",
        "E08",
        "E10",
        "E12",
        "E14",
        "E16",
        "E18",
        "E20"
      ]
    }
  ],
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
</script>{{% /tab %}}{{% tab %}}<div id="w6OsZ3iyk1jAKblEzcGh" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("w6OsZ3iyk1jAKblEzcGh", {
  "data": [
    {
      "type": "scatter",
      "name": "de01- W",
      "mode": "lines+markers",
      "x": [
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
        8.0,
        9.0,
        10.0
      ],
      "y": [
        1,
        2,
        3,
        2,
        5,
        5,
        5,
        6,
        7,
        10
      ],
      "text": [
        "E03",
        "E04",
        "E06",
        "E08 -- Blackout",
        "E10",
        "E12",
        "E14",
        "E16",
        "E18",
        "E20"
      ]
    }
  ],
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
</script>{{% /tab %}}{{% tab %}}<div id="UYm42ekL7u8UaZBHpfnT" class="plotly-graph-div" style="height:100%; width:100%;"></div>
<script type="text/javascript">
    Plotly.newPlot("UYm42ekL7u8UaZBHpfnT", {
  "data": [
    {
      "type": "scatter",
      "name": "de01- W",
      "mode": "lines+markers",
      "x": [
        1.0,
        2.0,
        3.0,
        4.0,
        5.0,
        6.0,
        7.0,
        8.0,
        9.0,
        10.0
      ],
      "y": [
        1,
        2,
        2,
        0,
        3,
        2,
        2,
        3,
        4,
        6
      ],
      "text": [
        "E03",
        "E04",
        "E06",
        "E08 -- Blackout",
        "E10",
        "E12",
        "E14",
        "E16",
        "E18",
        "E20"
      ]
    }
  ],
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
