- [ ] work on the split up TODOs
- [ ] work on the output TODOs
  - ideally functions currently having output work like this
    1. call function for actual task returning data
    2. call function to print the data (could also be done on its own)
  - but not everyting has to fit this pattern. In some cases printing data
    during the actual task just makes more sense
- [ ] adjust the tests
- [ ] write more tests (the refactor should have come up with functions which
      can be tested more easily as the data is returned first instead of printed
      directly)
- [ ] licensing headers. Better: switch to "reuse" project
- [ ] Ergibt es noch Sinn die Statistiken als json auszugeben? Das kommt ja ursprünglich nur daher, dass die CSVs notwendig waren um die Plots (via LaTeX) zu generieren. Jetzt mit Plotly ist das eigentlich nicht mehr der Fall
