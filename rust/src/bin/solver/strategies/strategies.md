# Explanation of the game
I have this game where I have two sets of size 10. Initially one matching of the elements from set_a to set_b is randomly selected. Now I have a sequence of trials where I can log in a full-matching and get told how many of my choices are correct (I call this a Night). In between, I can select one 1:1 match and get told if this is correct (I call this a Box).
The order is Box, Night, Box, Night, ...
In total I have 10 Nights to find the correct (full) matching.

Now I came up with a solver. It generates all permutations and comes up with guesses for both, the Boxes and the Nights.
For the Boxes, I select the 1:1 match which currently is closes to 50% of the remaining possible solutions.
For the nights, I iterate over all remaining possible solutions and determine which is best if I choose it. To determine "best", I calculate the "entropy", so I calculate the probability distribution over all possible outcomes (x matches are correct) and then calculate sum(- Pr * log2(Pr)). Then, I select the matching with the highest "entropy".


I don't want to change my entire solver. But maybe we can optimize the strategy for solving.

# Mathematical view
Generally for the MNs the $N$ (remaining) solutions are partitioned into buckets $S_k$ (with $k$ the number of lights produced) by choosing one matching.

Let
- $n_i = |S_i|$ be the size of the buckets ($\sum_i n_i = N$)
- $p_i = \frac{n_i}{N}$

# Metrics
## Entropy
$$
H = - \sum_i p_i \log_2 p_i
$$

Notes:
- max entropy is $\log_2(\text{num outcomes}) = \log_2(11)$ (because $|[0,10]| = 11$ lights are possible)

### Objective
- minimizes the expected log of the bucket size
- rewards spreading probability mass over many moderate buckets rather than
concentrating it
- maximizes the expected information
- non-empty buckets drive entropy up

## Expected remaining
$$
E[\text{remaining}] = \sum_i p_i n_i = \frac{\sum_i n_i^2}{N}
$$

### Objective
- penalizes large squared bucket sizes
- minimizes the expected number of candidates left directly
- basically uses norm $L_2$
- penalizes single large buckets more (quadratically)

## Max bucket
$$
M = \max_i n_i
$$

### Objective
- tries to minimize the largest possible bucket
- basically uses norm $L_\infty$
- is only about the largest bucket (worst-case)


# Comparison of the Metrics
$A: n = [5,5,1,1,1,1]$ \
$B: n = [6,2,2,2,2,0]$

- *Entropy* favors $A$ because it spreads the probability more
- *Expected remaining* favors $B$ because the mean is smaller
- *Max bucket* favors $A$ because of the better worst-case

# Compare matchings
## Scoring
1. normalize everything between 0 and 1
2. calculate a weighted sum of all scores for a single score to maximize
    1. max `entropy_norm`
    2. min `e_norm` (expected survivors)
    3. min `max_frac` (worst-case survivors)
    4. reward `singleton` (chance to uniquely identify now -> win in the next round)
    5. reward `exact` (chance the guess is exactly the solution)
3. in the sum scores to minimize are negative, scores to maximize/reward are
   positive

Notes:
- The optimal weights should be determined by simulations
- It might make sense to come up with a schedule for the weights
  - start with prioritizing *entropy*
  - later priorizie singleton and exact probs (only later they become realistic)
  - very small -> maybe raw extact prob

## Lexicographic
compare metrics one by another
- pro: no weights needed
- pro: robust

Order:
1. high entropy (primary comparison) -> only continue if there is a tie
2. low expected remaining
3. large p_singleton
4. large p_exact
5. small max_frac

# Sampling
## Poisson approximation
1. use the `Rem` table to calculate `Pr[k lights]` for each matching `m` with poisson
  1. sees each position in the matching as a binomial with the probability
     density as specified in `Rem`
  2. Assumes the positions are independent, but for large amounts of solutions
     it is basically this way
  3. can be calculated with dynamic programming
    1. 
2. then
  - `approx_entropy = entropy(Pr[k lights])`
  - `approx_p_exact = dp[10]`
  - `approx_max_frac = max_k dp[k]`
  - `approx_expected_remaining = N * sum(dp[k]^2)`
  - `singleton_prob` cannot really be approximated this way
3. can use `approx_entropy` and `p_exact` to sample the top-k from the remaining
   possibilities for which to compute the exact score

Notes:
- still add 5-10 random candidates to hedge against approximation failures
- https://en.wikipedia.org/wiki/Poisson_binomial_distribution

### Poisson calculation
Definitions:
- $n = 10$ positions in the candidate
- $q_0, ..., q_n$ probability for a match on that position
- $q_i = rem[i][m(i)] / N$
- We search for $Pr[X = k]$ for $k = 0..n$
- $X = \sum_{i=1}^n B_i$ with $B_i$ the Bernoulli $q_i$ (either it is a match or not) -> poisson-binomial (not a binomial because $p_i \neq p_j$)

The probability for $k$ success is
$$
Pr[K = k] = \sum_{A \in F_k} \prod_{i \in A} p_i \prod_{j \in A^c} (1 - p_j)
$$
with
- $F_k$ the set of all subsets of $k$ which can be selected from ${1, ..., n}$
- $A^c$ is the complement of $A$

One way to compute this is by the *direct convolution (DC) algorithm* (Wikipedia)
```
// PMF and nextPMF begin at index 0
function DC(p_1, ..., p_n) is
     declare new PMF array of size 1
     PMF[0] = [1]
     for i = 1 to n do
          declare new nextPMF array of size i + 1
          nextPMF[0] = (1 - p_i) * PMF[0]
          nextPMF[i] = p_i * PMF[i - 1]
          for k = 1 to i - 1 do
               nextPMF[k] = p_i * PMF[k - 1] + (1 - p_i) * PMF[k]
          repeat
          PMF = nextPMF
     repeat
     return PMF
end function
```
$Pr[K = k]$ is found in `PMF[k]`

Notes:
- stable
- exact
- fast for n < 2000

TODO: how does this compare to ChatGPTs answer?

# Lookahead
TODO

# Initials
TODO


# Full decision tree
- maybe in the late-game
- yes can be done but is quite complex

Note:
- you can use dynamic programming here with `(set of solutions left, #events)`
as key.

# Might it make sense to evaluate impossible solutions as candidates as well?
Yes it does. But the not anymore possible solutions which get a good score are
very specific, so just randomly sampling the eliminated ones probably won't do
it. Instead, we'd need smart ways of generating good candidates (max hamming,
based on `1:1` probability matrix, etc) which can be quite complicated.

# What if...
## we want to maximize the probability of solving the game in 10 Nights
Currently we aim for `min E[#events needed]`

No change in the early-game

Later the goal should be `min DecisionTree-depth`

We can use `score = entropy + \lambda * elimination_rate` with increasing lambda
over time and `elimination_rate = expected fraction eliminated`

## we change the Night/Box schedule

