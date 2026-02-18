- [ ] licensing headers. Better: switch to "reuse" project
- [ ] re-create `build` branch (`.gitignore` changed)
- [ ] split into different crates/binaries? (ree roughly the subcommands). Maybe this way the amount of full re-builds can be reduced (e.g. when just changing something regarding the comparisons)
  - simulation
  - comparison
  - solver

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
559 ./bin/solver.rs
656 ./ruleset/generators.rs
784 ./ruleset.rs
1124 ./constraint.rs

# Iterate over files manually
## bin
### bin/ayto.rs
DONE
### bin/solver.rs
DONE

## comparison
### comparison.rs
### comparison/plotly/heatmap.rs
### comparison/plotly/layout.rs
### comparison/plotly.rs
### comparison/plotly/scatter.rs
### comparison/theme.rs
### comparison/ruleset.rs
### comparison/summary.rs
### comparison/information.rs
### comparison/lights.rs

## constraint
### constraint.rs
### constraint/eval_compute.rs
### constraint/eval_predicates.rs
### constraint/eval_report.rs
### constraint/eval_types.rs
### constraint/parse_helpers.rs
### constraint/parse.rs
### constraint/utils.rs

## game
### game.rs
### game/eval.rs
### game/output.rs
### game/parse.rs

## iterstate.rs
## tree.rs

## matching_repr
### matching_repr.rs
### matching_repr/bitset.rs
### matching_repr/conversions.rs
### matching_repr/iter.rs

## ruleset_data
### ruleset_data.rs
### ruleset_data/dummy.rs
### ruleset_data/dup.rs
### ruleset_data/dup_x.rs

## ruleset
### ruleset.rs
### ruleset/generators.rs
### ruleset/parse.rs
### ruleset/utils.rs
