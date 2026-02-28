- [ ] licensing headers. Better: switch to "reuse" project
- [ ] split into different crates/binaries? (ree roughly the subcommands). Maybe this way the amount of full re-builds can be reduced (e.g. when just changing something regarding the comparisons)
  - simulation
  - comparison
  - solver
- [ ] search for all uses of `u8` and use a type alias like `LightsCnt`
- [ ] test cases aufräumen, aktuell häufig eine große Funktion mit mehreren
cases -> sollten mehrere kleine funktionen (mit aussagekräftigem Namen) sein mit
je nur einem case
- [ ] nach folgenden begriffen suchen und dann aufräumen:
    - existing
    - code
    - legacy

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
- [x] testing
- [x] Write doc-comments for files
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
- [x] (re-)organization of functions and files
- [x] write tests
- [x] Write doc-comments for files

## iterstate.rs
- [x] (re-)organization of functions and files
- [x] write tests
- [x] Write doc-comments for files

## tree.rs
- [x] (re-)organization of functions and files
- [x] write tests
- [x] Write doc-comments for files

## matching_repr
- [x] (re-)organization of functions and files
- [x] write tests
- [x] Write doc-comments for files


## ruleset_data
- [x] (re-)organization of functions and files
- [x] write tests
- [x] Write doc-comments for files

## ruleset
- [x] (re-)organization of functions and files
- [x] write tests
- [x] Write doc-comments for files



---


I have the following module. Please review the code. Focus on whether the functions are nice for testing. Options to consider are measures like splitting up functions, making functions generic over newly defined traits etc.

---

Please review the following unit-tests. Please focus on the following:
- Are there cases/functions which are untesed?
- Should unit-tests be split up / merged?
- Are there duplicate test-cases which can be deleted?
- Are there tests for functions which are not defined in this module?
- Note I have the following convention: Test-cases must have the following naming: <original function-name>_<"simple" or a short description of the case>


Also note that MaskedMatching::from() is not available anymore. These occurrences usually are replaced by MaskedMatching::from_matching_ref() now.

Do NOT write any tests yet.

---

Alright then please write tests (with the guidelines I've given). Use your suite as template. You may extend if you find yet uncovered cases/functions.

---

Ok no. I'd like to keep the modules as is. Now let us start with this module. Please write me testcases for every function which should be tested (just plugging code does not need to be tested).

Note I have the following convention: Test-cases must have the following naming: &lt;original function name&gt;\_&lt;"simple" or a short description of the case&gt;

Do not add comments.

---

Alright. This is the final module: 

Please write me an internal unit-test suite for it. You may rename, edit, remove the existing tests. Only write tests for functions defined in this module!

Note I have the following convention: Test-cases must have the following naming: &lt;original function name&gt;\_&lt;"simple" or a short description of the case&gt;

Do not add comments. 

---

Alright. This is the next module to review. Start with writing a review focused on the testability. But do NOT write tests yet.
