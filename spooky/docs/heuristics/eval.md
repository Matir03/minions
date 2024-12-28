# Evaluation Heuristics
Documents the heuristics used for evaluation, and how they are combined to get the final score.

## List of heuristics
- Board points to win $w_0, w_1$.
- Current tech score $t$.
- Money $m_0, m_1$.
- Board scores $b_1, b_2, \dots, b_n$.

## Final score
$$d = c_w(w_1 - w_0) + c_t t + c_m(m_0 - m_1) + c_b \sum_{i=1}^n b_i$$
$$s = \tanh(c_d d)$$

### Parameters
- $c_w = 25$
- $c_t = 4n$
- $c_m = 1$
- $c_b = 1$
- $c_d = 0.05$

## Compuation of heuristics

### Board points to win
$w_i$ = number of boards $i$ needs to win

### Current tech score
Let $a_i$ be the tech line advancement of each team, and $U_i$ be set of units $i$ has acquired.

$$a = \max(a_0, a_1)$$
$$\gamma = 0.98$$
$$t_i = \sum_{u \in U_i} \gamma ^ {a - u}$$ 
$$t = t_0 - t_1 + a_0 - a_1$$

### Money
$m_i$ = money of team $i$

### Board scores
Let $P_i$ be the set of pieces on board $i$. For each piece $p$, let $s(p)$ be its side ($1$ if $S_0$, $-1$ if $S_1$), and $v(p)$ be its value.

$$b_i = \sum_{p \in P_i} s(p) v(p)$$