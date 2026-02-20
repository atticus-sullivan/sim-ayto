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
- [ ] search for all uses of `u8` and use a type alias like `LightsCnt`

# LOCs (sorted)
- instead of splitting it is also ok to simplify the code
- TODO re-generate the report

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
- [x] write tests
- [x] Write doc-comments for files

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
