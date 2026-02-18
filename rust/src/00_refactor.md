- [ ] licensing headers. Better: switch to "reuse" project
- [ ] re-create `build` branch (`.gitignore` changed)
- [ ] split into different crates/binaries? (ree roughly the subcommands). Maybe this way the amount of full re-builds can be reduced (e.g. when just changing something regarding the comparisons)
  - simulation
  - comparison
  - solver
- [ ] check all access modifiers
  1. comment out everything
  2. start with the two crates
  3. comment in step-by step and try to keep access modifiers as low as possible

# LOCs (sorted)
- instead of splitting it is also ok to simplify the code
## unproblematic
3 ./comparison/plotly.rs
25 ./comparison/ruleset.rs
25 ./comparison/theme.rs
27 ./ruleset/parse.rs
42 ./ruleset_data/dummy.rs
46 ./lib.rs
64 ./ruleset_data.rs
66 ./comparison/information.rs
76 ./constraint/utils.rs
82 ./comparison/plotly/layout.rs
91 ./comparison/plotly/scatter.rs
91 ./constraint/eval_compute.rs
100 ./constraint/parse_helpers.rs
106 ./comparison/summary.rs
118 ./comparison/lights.rs
120 ./bin/ayto.rs
130 ./game.rs
137 ./tree.rs
189 ./matching_repr.rs

## can something be done?
204 ./ruleset_data/dup.rs
226 ./ruleset/utils.rs
227 ./constraint/eval_types.rs
257 ./ruleset_data/dup_x.rs
271 ./game/output.rs
280 ./comparison.rs
285 ./iterstate.rs
## try to split
304 ./constraint/eval_predicates.rs
311 ./comparison/plotly/heatmap.rs
327 ./constraint/eval_report.rs
341 ./matching_repr/bitset.rs
346 ./game/parse.rs
348 ./matching_repr/iter.rs
386 ./matching_repr/conversions.rs
392 ./game/eval.rs
## must split
453 ./constraint/parse.rs
656 ./ruleset/generators.rs
784 ./ruleset.rs
1124 ./constraint.rs

# Iterate over files manually
## bin
### bin/ayto.rs
- [x] (re-)organization of functions and files
- [x] write tests
- [x] Write doc-comments for files

### bin/solver/*
TODO (ausgelassen erstmal)
- [ ] testing
- [ ] Write doc-comments for files
- [x] (re-)organization of functions and files

## comparison
- [x] (re-)organization of functions and files
- [x] write tests
- [x] Write doc-comments for files

## constraint
- [x] (re-)organization of functions and files
- [ ] write tests
- [ ] Write doc-comments for files

### constraint.rs
### constraint/eval_compute.rs
### constraint/eval_predicates.rs
### constraint/eval_report.rs
### constraint/eval_types.rs
### constraint/parse_helpers.rs
### constraint/parse.rs
### constraint/utils.rs

## game
- [ ] (re-)organization of functions and files
- [ ] write tests
- [ ] Write doc-comments for files

### game.rs
### game/eval.rs
### game/output.rs
### game/parse.rs

## iterstate.rs
- [ ] (re-)organization of functions and files
- [ ] write tests
- [ ] Write doc-comments for files

## tree.rs
- [ ] (re-)organization of functions and files
- [ ] write tests
- [ ] Write doc-comments for files


## matching_repr
- [ ] (re-)organization of functions and files
- [ ] write tests
- [ ] Write doc-comments for files

### matching_repr.rs
### matching_repr/bitset.rs
### matching_repr/conversions.rs
### matching_repr/iter.rs

## ruleset_data
- [ ] (re-)organization of functions and files
- [ ] write tests
- [ ] Write doc-comments for files

### ruleset_data.rs
### ruleset_data/dummy.rs
### ruleset_data/dup.rs
### ruleset_data/dup_x.rs

## ruleset
- [ ] (re-)organization of functions and files
- [ ] write tests
- [ ] Write doc-comments for files

### ruleset.rs
### ruleset/generators.rs
### ruleset/parse.rs
### ruleset/utils.rs
