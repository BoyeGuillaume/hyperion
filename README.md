# HyFormal
This is a formal, symbolic engine for hyperion. It aims to provide a framework for automatic theorem proving and reasoning, using a formal system based on first-order logic.

## Formal-system syntax

We define the syntax of the formal system as a group of 3 elements: expressions, propositions and types. We can intuitively
think of types as a *constrained* set of values, expressions as *terms* and propositions as *formulas*.

$$
\begin{aligned}
  \mathcal{P} &:=\;& \text{true} \mid \text{false} \mid \mathcal{P} \land \mathcal{P} | \mathcal{P} \lor \mathcal{P} \mid \\
  &&\neg \mathcal{P} \mid \mathcal{P} \Rightarrow \mathcal{P}
  \mid \mathcal{P} \Leftrightarrow \mathcal{P} \mid \forall x: T.\; \mathcal{P} \mid \\
  &&\exists x: T.\; \mathcal{P} \mid \mathcal{E}_1 = \mathcal{E}_2
\end{aligned}
$$

$$
\begin{aligned}
  \mathcal{E} &:=\;& \mathcal{P} \mid x \mid \dagger \mid x(\mathcal{E}) \mid \lambda x: T.\; \mathcal{E} \mid \\
  && x := \mathcal{E};\; \mathcal{E} \mid \text{if } \mathcal{P} \text{ then } \mathcal{E}_1 \text{ else } \mathcal{E}_2 \mid \\
  && \mathcal{E}_1, \mathcal{E}_2
\end{aligned}
$$

$$
\begin{aligned}
  T &:=\;& \text{Bool} \mid \Omega \mid \dagger \mid T_1 \rightarrow T_2 \mid T_1 \times T_2 \mid \\
  && 
\end{aligned}
$$

Where $x$ is a variable, $T$ is a type, $\mathcal{P}$ is a proposition and $\mathcal{E}$ is an expression. The term $\dagger$ symbolizes a never (crash or hang).
