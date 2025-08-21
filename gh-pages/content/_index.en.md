---
title: 'Home'
weight: 1
# bookFlatSection: false
# bookToc: true
# bookHidden: false
# bookCollapseSection: false
# bookComments: false
# bookSearchExclude: false
---

## Overview

| Format     | AYTO DE                           | AYTO DE RSIL                           | AYTO US                           | AYTO UK                           |
| ----       | :--:                              | :--:                                   | :---:                             | :---:                             |
| Season  1 | [:white_check_mark:](ayto/de/01/) | [:white_check_mark:](ayto/de-rsil/01/) | [:white_check_mark:](ayto/us/01/) | [:white_check_mark:](ayto/uk/01/) |
| Season  2 | [:white_check_mark:](ayto/de/02/) | [:white_check_mark:](ayto/de-rsil/02/) | [:white_check_mark:](ayto/us/02/) |                                   |
| Season  3 | [:white_check_mark:](ayto/de/03/) | [:white_check_mark:](ayto/de-rsil/03/) | [:white_check_mark:](ayto/us/03/) |                                   |
| Season  4 | [:white_check_mark:](ayto/de/04/) | [:white_check_mark:](ayto/de-rsil/04/) | [:white_check_mark:](ayto/us/04/) |                                   |
| Staffel  5 | [:white_check_mark:](ayto/de/05/) | [:hourglass:       ](ayto/de-rsil/05/) | [:white_check_mark:](ayto/us/05/) |                                   |
| Staffel  6 | [:white_check_mark:](ayto/de/06/) |                                        | [:white_check_mark:](ayto/us/06/) |                                   |
| Season  7 |                                   |                                        | [:white_check_mark:](ayto/us/07/) |                                   |
| Season  8 |                                   |                                        | [:white_check_mark:](ayto/us/08/) |                                   |
| Season  9 |                                   |                                        | [:white_check_mark:](ayto/us/09/) |                                   |
| Season 10 |                                   |                                        | [                  ]()            |                                   |
<!-- :x: -->

Since there only was one *UK* season so far, this seson is also included in the
comparison with the *US* seasons.

{{% hint info %}}
In german the "matching ceremonies" are called "Matchingnights" (MN) and the
"truth booths" are called "Matchboses" (MB).
So you'll probably find some places where these terms / acronyms are used
instead of the ones you're accustomed to (I did not localize everything).
{{% /hint %}}

## Information regarding the Illustration

For each seson you can see the whole process of the season. This means what
information was collected and what this means for the remaining possibilities.

*Tree* precisely shows the remaining matchings that are still possible. This
only makes sense when there are not too many possibilities left.

### Information regarding Spoilers
The pages are constructed in a way that you explicitly need to expand what you
want to look up. Only the "Events" (matching ceremony / truth booth) in an
episode is shown by default on the page.

The episode mentioned always referrs to when the **outcome of the event was
revealed**.

Expandable sections with higher spoiler potential (current state and complete
history until now) are explicitly marked with :warning:.

What the current state is can always be looked up by checking under *Single
Tables* what the most recent entry is.

### Regarding the comparisons
- the `- W` / `- L` at the end of the entry in the legend referrs to whether the
  cast of that season won (*win* `- W`) or lost (*loose* `- L`)

## More details

{{% details "Click here for more details" %}}
In the following you can find more explanations regarding the outputs. For even
more details (e.g. how the input files work or how to manually add more data)
see the [complete repository](https://github.com/atticus-sullivan/sim-ayto)
including the code and data (note that currently some of the explanations are
only available in german).

Also you find a way of [asking questions](https://github.com/atticus-sullivan/sim-ayto/discussions/categories/q-a),
[share ideas](https://github.com/atticus-sullivan/sim-ayto/discussions/categories/ideas)
and
[note bugs/errors](https://github.com/atticus-sullivan/sim-ayto/issues).

### Normal outputs
In front of each table you'll find precisely what constraint was added here. The
mentioned episode always referrs to when the outcome of the constraint was
**revealed**. In case of matching ceremonies there is also a note before the
match how often they sat toegther in the past.

Everything related to `I` (Information content) / `H` (Entropy, after the note
how many possibilities are still left) is the attempt to get a feeling for how
much this decision was worth and how far they are from the finish line. This is
related to information theory.

`I[l/bits]`: Represents how much information was gained with this decision
assuming the given amount of beams are lit. Via
\( 2^{-I} \)
you can calculate back to the probabilities if you can relate more with that.

`E[l]/bits`: Is the expected value of the gain of information

#### Regular tables
The **font**color is an indicator for how high the probability for this match is
(below 1% red, higher than 45% yello, higher than 55% cyan and more than 80%
green).

The **background**color shows which people have the highest probability with which
other people.
- **green** background: Match is for both persons the most likely one.
- **red/light gray** backgroung: Match is for the person whose column/row this
is the most likely one.

#### Summary table in the end
In the end there is a summary of all constraints. A star in this tale means this
match was formed the first time here. A small overview over the columns which
might not be clear on first sight:
- `L` the amount of lights
- `I` refer to above
- `new` counts how many matches never sat together a matching ceremony
- `min dist` the distance is denoted with the amount of different matches, this
  column shows which other matching ceremony is most alike this one (and how
much alike they are). This cannot be calculated for the first ceremony
obviously.

### Tree
In the tree the first row (correlated with the person out of setA) at a level
is always fixd. Thus, each level represents the matching of one (or multiple)
people of setB to this fixed person of setA.

Already known matches (be it from truth booth or throug elimination) are put on
the topmost levels. For the remaining levels the ordering is sorted based on the
amount of *different* matches for the fixed person.
{{% /details %}}
