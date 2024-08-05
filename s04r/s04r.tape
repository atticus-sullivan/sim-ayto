Output .not_needed.gif

Require awk
Require xargs

Set Theme "Catppuccin Frappe"
Set Shell "zsh"
Set FontSize 22
Set Width 1800
Set Height 860

Type@0ms "awk -v RS='' 'END {print NR-2}' s04r.col.out | xargs -I{} awk -v RS="" 'NR == {} {print $0}' s04r.col.out" Enter
Sleep 1s
Screenshot s04r_ctab.png
