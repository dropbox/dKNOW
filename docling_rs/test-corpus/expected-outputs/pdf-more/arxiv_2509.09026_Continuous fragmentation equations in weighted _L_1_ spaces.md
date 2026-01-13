## Continuous Fragmentation Equations in Weighted L 1 Spaces

Dedicated to Rainer Picard on the occasion of his 80th birthday

Lyndsay Kerr, Wilson Lamb and Matthias Langer

## Abstract

We investigate an integro-differential equation that models the evolution of fragmenting clusters. We assume cluster size to be a continuous variable and allow for situations in which mass is not necessarily conserved during each fragmentation event. We formulate the initial-value problem as an abstract Cauchy problem (ACP) in an appropriate weighted L 1 space, and apply perturbation results to prove that a unique, physically relevant classical solution of the ACP is given by a strongly continuous semigroup for a wide class of initial conditions. Moreover, we show that it is often possible to identify a weighted L 1 space in which this semigroup is analytic, leading to the existence of a unique, physically relevant classical solution for all initial conditions belonging to that space. For some specific fragmentation coefficients, we provide examples of weighted L 1 spaces where our results can be applied.

Mathematics Subject Classification ( 2020 ) : 47D06, 80A30, 46B42

Keywords: continuous fragmentation, strongly continuous semigroup, analytic semigroup

## 1 Introduction

There are many processes that can be described in terms of clusters of particles that can merge together (i.e. coagulate ) to produce larger clusters and can break apart (i.e. fragment ) to produce smaller clusters. As highlighted in [7, Section 2.1], such processes arising in nature include the grouping of animals and fish, the formation of preplanetisimals and blood clotting. Coagulation and fragmentation processes also feature in a number of industrial applications, and play an important role in the powder production industry [20,22], aerosols [9] and the formation and degradation of polymers [1,23,24]. When clusters are comprised of identical particles (monomers), then the evolution of clusters can be described in terms of an infinite system of ordinary differential equations. We have previously examined such discrete models in the case of pure fragmentation, both time-dependent and time-independent, [11, 12] and in the case of coagulationfragmentation with time-dependent coagulation [13].

In this article, we turn our attention to continuous models where cluster mass, which we use interchangeably with cluster size, can take any positive value. We focus on the case where no coagulation occurs, i.e. the breakdown of clusters is non-reversible. In this case, the evolution of clusters can be described in terms of the integro-differential

equation

<!-- formula-not-decoded -->

where u ( x, t ) is the density of clusters of size x &gt; 0 at time t ≥ 0, a ( x ) is the rate of fragmentation of clusters of size x , b ( x, y ) gives the average number of clusters of size x that are produced when a cluster of size y fragments and ˚ u ( x ) is the initial density of clusters of size x at time t = 0. We adopt the natural physical assumption that a ( x ) and b ( x, y ) are non-negative for all x, y &gt; 0. We also assume that b ( x, y ) = 0 whenever y ≤ x so that a cluster cannot fragment to produce clusters larger than itself. Note that, since u represents a density, any physically relevant solution to this system must be non-negative. It is also common to impose the assumption

<!-- formula-not-decoded -->

which ensures that mass is conserved during each fragmentation event; however, in most of our results we do not assume that (1.2) holds.

In this article, we use an operator semigroup approach to examine (1.1). Operator semigroups were first used, in [1], to study a binary fragmentation version of the continuous coagulation-fragmentation system, and have since been deployed in numerous analyses of both discrete and continuous coagulation-fragmentation systems; see [3, 6, 11-16, 18] and also [7]. The approach that we use involves formulating (1.1) as a linear abstract Cauchy problem (ACP) in an appropriate Banach space. We then use perturbation theorems for operator semigroups to show that, under mild conditions on the fragmentation coefficients, there exists a unique solution of the ACP, expressible in terms of a substochastic semigroup (i.e. a positive C 0 -semigroup of contractions, often referred to as the fragmentation semigroup) for a particular class of initial conditions.

For a non-negative solution, u , of the continuous fragmentation equation (1.1), the total number of clusters and the total mass of clusters at time t ≥ 0 are given, respectively, by

<!-- formula-not-decoded -->

To control both the number of particles and the mass of the system, a natural Banach space to work in is therefore the Banach space X [1] , where X [1] is the weighted L 1 space,

<!-- formula-not-decoded -->

consisting of real-valued, measurable functions defined (almost everywhere) on (0 , ∞ ), and satisfying

<!-- formula-not-decoded -->

However, while the majority of previous investigations into (1.1) have utilised X [1] , some have been conducted in other weighted L 1 spaces, primarily higher moment spaces of the form

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

with associated norm

for p ≥ 1 [5,6]. Motivated by our work in [11-13], here we choose to analyse (1.1) in the following, more general, weighted L 1 space

<!-- formula-not-decoded -->

where ω is a non-negative measurable function on [0 , ∞ ). It follows that the space X ω is a Banach space with norm

<!-- formula-not-decoded -->

and consists of real-valued, measurable functions defined (almost everywhere) on (0 , ∞ ), and satisfying ∥ f ∥ ω &lt; ∞ . As in [11], working with a more flexible weight allows us to prove results regarding the analyticity of the fragmentation semigroup. This, in turn, allows us to yield results regarding the existence and uniqueness of solutions to the fragmentation ACP under weaker conditions on the fragmentation coefficients than can be obtained if we restrict ourselves to working in X [ p ] for p ≥ 1. In addition, we do not impose the usual mass-conservation assumption (1.2) but instead impose a weaker assumption that allows, for example, mass to be lost during a fragmentation event. In our main result we prove that, under very mild assumptions on the fragmentation coefficients, it is always possible to find a weight ω such that the fragmentation semigroup in analytic on X ω . This result leads to the existence and uniqueness of solutions to the fragmentation ACP for all ˚ u ∈ X ω .

The article is structured as follows. In Section 2 we provide some definitions and preliminary results that we will need in our treatment of (1.1). In particular, we provide two perturbation theorems that we will apply to an ACP formulation of (1.1). In Section 3 we formulate (1.1) as an ACP in X ω . Under very mild assumptions on the fragmentation coefficients we use the first perturbation theorem from Section 2 to show that, for a particular class of initial conditions, a unique solution of the fragmentation ACP is provided by a C 0 -semigroup. In Section 4 we apply the second perturbation result, under slightly stronger assumptions on the fragmentation coefficients, to show that this semigroup is analytic and provides a unique solution to the fragmentation ACP for all initial conditions in X ω . We then demonstrate in Section 5 that for a wide class of fragmentation coefficients it is possible to find a weight such that the results in Section 4 hold. Finally, in Section 6 we consider two specific examples of fragmentation coefficients, and provide particular weights which can be used in each of these cases.

## 2 Preliminaries

In this section, we supply preliminary material that we will require in our investigation into (1.1). In the first subsection, we begin by presenting some concepts and a result relating to AL -spaces and positive semigroups. In the following subsection, we then provide two important perturbation theorems that we will later apply to the fragmentation ACP to obtain results regarding the existence and uniqueness of solutions.

## 2.1 AL -Spaces and Positive Semigroups

Let us recall some definitions from the theory of Banach lattices and positive C 0 -semigroups, which will be crucial for our investigations. For a detailed theory of general C 0 -semigroups and positive semigroups, in particular, we refer the reader to [4,8,10].

Let X be a real ordered vector space . We denote by X + the positive cone , i.e. the set of all non-negative elements in X . Likewise, for a subspace D of X , the set of non-negative elements in D is denoted by D + .

Now suppose that X is a vector lattice (or Riesz space ), i.e. it is an ordered vector space and sup { f, g } exists in X for all f, g ∈ X . Let f ∈ X . Then f ± := sup {± f, 0 } ∈ X + and | f | := sup { f, -f } ∈ X + are well defined and satisfy f = f + -f -and | f | = f + + f -. A subspace Y of X is called a sublattice if it is closed under the mappings f ↦→ f + , f ↦→ f -. A vector lattice is a Banach lattice if it is a Banach space with norm ∥ · ∥ and | x | ≤ | y | implies ∥ x ∥ ≤ ∥ y ∥ for all x, y ∈ X . Note that in a Banach lattice the relation ∥| x |∥ = ∥ x ∥ is valid for all x ∈ X ; see [4, (2.74)].

If X is a Banach lattice and satisfies ∥ f + g ∥ = ∥ f ∥ + ∥ g ∥ for all f, g ∈ X + , then X is called an AL-space . It follows from [4, Theorems 2.64 and 2.65] that, when X is an AL -space, there is a unique bounded linear functional, ϕ , that extends ∥ · ∥ from X + to X .

Note that X ω , equipped with the norm ∥ · ∥ ω , is an AL -space (where f ≤ g ⇔ f ( x ) ≤ g ( x ) for almost all x &gt; 0); see [7, p. 100]. The unique, bounded linear functional, ϕ ω , that extends ∥ · ∥ from ( X ω ) + to X ω is given by

<!-- formula-not-decoded -->

Let us recall also some notions about operators and semigroups on ordered Banach spaces. A linear operator A on an ordered Banach space X is called positive if Af ≥ 0 for all f ∈ X + . A C 0 -semigroup ( S ( t )) t ≥ 0 on an ordered Banach space is called a positive semigroup if S ( t ) ≥ 0 for all t ≥ 0. A positive semigroup is called substochastic if ∥ S ( t ) ∥ ≤ 1 for all t ≥ 0 and stochastic if ∥ S ( t ) f ∥ = ∥ f ∥ for all f ∈ X + and t ≥ 0.

For later use we prove the following simple lemma.

Lemma 2.1. Let U be a positive linear operator on a vector lattice, X , with lattice norm ∥ · ∥ . Then ∥ Uf ∥ ≤ ∥ U | f |∥ for all f ∈ X .

Proof. Let f ∈ X . Then ± f ≤ | f | and hence ± Uf ≤ U | f | by the positivity of U , which, in turn, implies that | Uf | ≤ U | f | . Since ∥ · ∥ is a lattice norm, it follows that

<!-- formula-not-decoded -->

When we deal with analytic semigroups, we also need the complexification of a real Banach lattice. Let us recall the following notions (see, e.g. [4, Section 2.2.5] or [7, Section 3.2.5]). The complexification X C of a real vector lattice X is the set of pairs ( x, y ) ∈ X × X , where we write ( x, y ) =: x + iy . We refer to x , y and x -iy respectively as the real part , imaginary part and complex adjoint of x + iy . Vector operations are extended in an obvious way, and the partial order in X C is defined by

<!-- formula-not-decoded -->

Note that x + iy is a non-negative element in X C if and only if x ∈ X + and y = 0. Motivated by the scalar case, we define a modulus by

<!-- formula-not-decoded -->

which exists (see [4, p. 62]), and a norm by

<!-- formula-not-decoded -->

Whenever we deal with analytic semigroups we consider them on the complexification X C . Note that the complexification of X ω is the weighted L 1 space of complexvalued functions.

## 2.2 Two Perturbation Theorems

We now present two perturbation theorems, which we use later to prove the existence of semigroups associated with the fragmentation equation. We apply these two theorems in two different situations depending on the assumptions on the fragmentation kernel. First, we recall a perturbation result [11, Proposition 2.4], which is based on [19, Theorem 2.7]. The following theorem is essentially [11, Proposition 2.4] but is reformulated slightly to make the assumptions clearer.

Theorem 2.2. Let ( X, ∥ · ∥ ) and ( Z, ∥ · ∥ Z ) be AL-spaces, such that

- (i) Z is a sublattice of X ;
- (ii) Z is dense in X ;
- (iii) ( Z, ∥ · ∥ Z ) is continuously embedded in ( X, ∥ · ∥ ) .

Also, let ϕ and ϕ Z be the linear extensions of ∥ · ∥ from X + to X and of ∥ · ∥ Z from Z + to Z respectively. Let A : D ( A ) → X , B : D ( B ) → X be linear operators in X such that D ( A ) ⊆ D ( B ) . Assume that the following conditions are satisfied:

- (a) -A is positive;
- (b) A generates a positive C 0 -semigroup, ( T ( t )) t ≥ 0 , on X ;
- (c) the semigroup ( T ( t )) t ≥ 0 leaves Z invariant and its restriction to Z is a ( necessarily positive ) C 0 -semigroup on ( Z, ∥ · ∥ Z ) ( in which case, the corresponding generator ˜ A is given by

<!-- formula-not-decoded -->

- (d) B | D ( A ) is a positive linear operator;
- (e) ϕ (( A + B ) f ) ≤ 0 for all f ∈ D ( A ) + ;
- (f) ( A + B ) f ∈ Z and ϕ Z (( A + B ) f ) ≤ 0 for all f ∈ D ( ˜ A ) + ;
- (g) ∥ Af ∥ ≤ ∥ f ∥ Z for all f ∈ D ( ˜ A ) + .

Then G := A + B generates a substochastic C 0 -semigroup, ( S ( t )) t ≥ 0 , on X , and no other extension of A + B generates a C 0 -semigroup. Moreover, the semigroup ( S ( t )) t ≥ 0 leaves Z invariant. If ϕ (( A + B ) f ) = 0 for all f ∈ D ( A ) + , then ( S ( t )) t ≥ 0 is stochastic.

Before we state the second perturbation theorem, let us recall the notion of relative boundedness.

Definition 2.3. [10, Definition III.2.1] Let X be a Banach space and let A : D ( A ) → X and B : D ( B ) → X be linear operators with D ( A ) ⊆ D ( B ). Then B is A -bounded if there exist α, β ≥ 0 such that

<!-- formula-not-decoded -->

If B is A -bounded, then the A -bound , or relative bound , is

<!-- formula-not-decoded -->

♢

We also need the definition of a Miyadera perturbation.

Definition 2.4. [4, Section 4.4] Let X be a Banach space and let A : D ( A ) → X and B : D ( B ) → X be linear operators, where D ( A ) ⊆ D ( B ) ⊆ X . Moreover, let A be the generator of a C 0 -semigroup, ( T ( t )) t ≥ 0 , on X . Then B is a Miyadera perturbation of A if B is A -bounded and there exist numbers ζ &gt; 0 and γ ∈ (0 , 1) such that

<!-- formula-not-decoded -->

♢

The Miyadera perturbation theorem [4, Theorem 4.16] states that A + B generates a C 0 -semigroup if A is the generator of a C 0 -semigroup and B is a Miyadera perturbation of A . This result is used in the proof of the following perturbation theorem.

Theorem 2.5. Let A be the generator of a positive C 0 -semigroup on an AL-space, X , such that -A is positive. Moreover, let B be an A -bounded linear operator, with A -bound strictly less than 1 . Then the following statements hold.

- (i) The operator A + B is the generator of a C 0 -semigroup on X .
- (ii) If B is positive, then the semigroup generated by A + B is positive.
- (iii) If the semigroup generated by A is analytic and B is positive, then the semigroup generated by A + B is analytic.

Proof. (i) We show that B is a Miyadera perturbation of A . Let ϕ be the unique linear extension of ∥ · ∥ from X + to X and let ( T ( t )) t ≥ 0 be the semigroup generated by A . For ζ &gt; 0 and f ∈ D ( A ) + we have

<!-- formula-not-decoded -->

The aim is to extend this inequality to all f ∈ D ( A ). To this end, define, for each δ &gt; 0, the positive operator T δ by T δ f := 1 δ ∫ δ 0 T ( t ) f d t , f ∈ X . If follows from [10, Lemma II.1.3 (iii)] and [7, Proposition 4.2.4 (a)] that T δ maps X into D ( A ) and that T δ f → f as δ → 0 + for all f ∈ X . Hence T δ | f | ∈ D ( A ) + and T δ | f | →| f | as δ → 0 + for all f ∈ X . Since, -A , T ( t ) and T δ are positive operators for t ≥ 0 and δ &gt; 0, it follows from Theorem 2.1 that

<!-- formula-not-decoded -->

for all f ∈ X . Hence, for f ∈ X , we obtain from (2.4) that

<!-- formula-not-decoded -->

If we now let f ∈ D ( A ), then with the help of [10, Lemmas II.1.3 (ii) and (iv)] we obtain, for t ≥ 0,

<!-- formula-not-decoded -->

which implies that

<!-- formula-not-decoded -->

uniformly in t on [0 , ζ ] for every ζ &gt; 0. This shows that the left-hand side of (2.5) converges to ∫ ζ 0 ∥ AT ( t ) f ∥ d t as δ → 0 + . Taking the limit as δ → 0 + in (2.5) we obtain

<!-- formula-not-decoded -->

Let M ≥ 1 and ω ≥ 0 be such that ∥ T ( t ) ∥ ≤ Me ωt for all t ≥ 0. By assumption there exist α ∈ [0 , 1) and β ≥ 0 such that (2.1) holds. This relation, together with (2.6), implies that

<!-- formula-not-decoded -->

for all f ∈ D ( A ). Since α &lt; 1, we can choose ζ &gt; 0 such that α + βMζe ωζ &lt; 1 and hence (2.3) holds with γ &lt; 1. Now the Miyadera perturbation theorem implies that A + B generates a C 0 -semigroup.

(ii) Let ( T ( t )) t ≥ 0 and ( S ( t )) t ≥ 0 be the semigroups generated by A and A + B respectively, and let ω 0 be the growth bound of ( T ( t )) t ≥ 0 . Choose λ &gt; ω 0 ; then A -λI generates the semigroup ( e -λt T ( t )) t ≥ 0 , which has a negative growth bound, and A -λI + B generates the semigroup ( e -λt S ( t )) t ≥ 0 . It follows from [4, Lemma 4.15] that B is a Miyadera Perturbation of A -λI . Further, the proof of [4, Theorem 4.16] yields that S ( t ) can be written as a series,

<!-- formula-not-decoded -->

where S j ( t ), j ∈ N 0 , t ∈ [0 , ∞ ), are bounded operators satisfying

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

The positivity of ( T ( t )) t ≥ 0 and B imply that S j ( t ) ≥ 0. Hence the representation (2.7) yields that ( S ( t )) t ≥ 0 is a positive semigroup.

(iii) Since ( S ( t )) t ≥ 0 is positive, the operator A + B is resolvent positive; see [7, p. 128]. It follows from [2, Theorem 1.1] that ( S ( t )) t ≥ 0 is analytic.

Note that Theorem 2.5 could also be proved using [21, Theorem 0.1].

## 3 Fragmentation Semigroup on Xω

In this section, we formulate (1.1) as an ACP in X ω . We begin by introducing some natural non-negativity assumptions on the fragmentation coefficients that we require throughout our investigation.

Assumption 3.1. Let a ∈ L ∞ loc ([0 , ∞ )) be such that a ( x ) ≥ 0 for almost all x &gt; 0. Further, let b : (0 , ∞ ) 2 → [0 , ∞ ) be measurable such that b ( x, y ) = 0 for almost all x, y with x &gt; y . ♢

Let Theorem 3.1 hold, let ω : [0 , ∞ ) → [0 , ∞ ) be measurable, and let X ω be as in (1.3). Motivated by (1.1), we define two operators, A ( ω ) and B ( ω ) , in X ω by

<!-- formula-not-decoded -->

and

<!-- formula-not-decoded -->

This allows us to pose (1.1) as the ACP

<!-- formula-not-decoded -->

It follows from [10, Propositions I.4.10 (i) and I.4.11] that the operator A ( ω ) is closed, densely defined and generates the C 0 -semigroup ( T ( ω ) ( t )) t ≥ 0 on X ω given by

<!-- formula-not-decoded -->

This semigroup is clearly substochastic. For later use, note that, for any λ &gt; 0, the resolvent operator of A ( ω ) is given by

<!-- formula-not-decoded -->

and so

<!-- formula-not-decoded -->

We will also require the following additional assumption on the fragmentation coefficient b .

Assumption 3.2. Let ω : [0 , ∞ ) → [0 , ∞ ) and assume that there exists κ ∈ (0 , 1] such that

<!-- formula-not-decoded -->

♢

For example, suppose that

<!-- formula-not-decoded -->

and also that γ ( x ) := ω ( x ) /x is non-decreasing on (0 , ∞ ). Then, for all y &gt; 0, we have

<!-- formula-not-decoded -->

showing that (3.3) is satisfied with κ = 1. In particular, if (3.4) holds, then Theorem 3.2 is automatically satisfied when ω ( x ) = x p for any p ≥ 1. We note that (3.4) allows for fragmentation events in which mass is conserved or in which mass is lost. It follows that Assumption 3.2 is weaker than the mass-conservation assumption (1.2).

In order to be able to perturb A ( ω ) with B ( ω ) , we consider the following estimate. It follows from Theorem 3.2 that, for any f ∈ D ( A ( ω ) ) + ,

<!-- formula-not-decoded -->

Consequently, for all f ∈ D ( A ( ω ) ),

<!-- formula-not-decoded -->

from which it follows that

<!-- formula-not-decoded -->

To apply Theorem 2.2 to the operators A ( ω ) and B ( ω ) , we require a suitable subspace of X ω . We define such a subspace in terms of a function c that is non-decreasing on [0 , ∞ ) and satisfies

<!-- formula-not-decoded -->

Note that such a function can always be found when a ∈ L ∞ loc ([0 , ∞ )) as we can take

<!-- formula-not-decoded -->

see [7, Remark 5.1.38]. Let C ( ω ) be the multiplication operator defined by

<!-- formula-not-decoded -->

and equip D ( C ( ω ) ) with the graph norm

<!-- formula-not-decoded -->

Note that ( D ( C ( ω ) ) , ∥ · ∥ C ( ω ) ) is identical to the space L 1 ((0 , ∞ ) , ˜ ω ( x ) d x ) = X ˜ ω , where

<!-- formula-not-decoded -->

We now apply Theorem 2.2 to A ( ω ) and B ( ω ) .

Theorem 3.3. Let Theorems 3.1 and 3.2 hold. Then G ( ω ) = A ( ω ) + B ( ω ) is the generator of a substochastic C 0 -semigroup, ( S ( ω ) ( t )) t ≥ 0 , on X ω . Moreover, if c is nondecreasing and satisfies (3.8) and ˜ ω is as in (3.9) , then ( S ( ω ) ( t )) t ≥ 0 leaves X ˜ ω invariant.

Proof. We show that the conditions of Theorem 2.2 are all satisfied when A = A ( ω ) , B = B ( ω ) and the AL -spaces ( X, ∥ · ∥ ) and ( Z, ∥ · ∥ Z ) are, respectively, X ω and X ˜ ω .

Clearly, X ˜ ω is a sublattice of X ω and, furthermore, is dense and continuously embedded in X ω . Moreover, conditions (a)-(c) are all satisfied by A ( ω ) .

It is also clear that B ( ω ) is positive, and, for f ∈ D ( A ( ω ) ) + , (3.5) leads to

<!-- formula-not-decoded -->

Hence (d) and (e) hold. Moreover, the monotonicity of the function c and Theorem 3.2 imply that

<!-- formula-not-decoded -->

This means that Theorem 3.2 also holds for the weight ˜ ω . Therefore we obtain from (3.7) and (3.10) that D ( A ( ˜ ω ) ) ⊆ D ( B ( ˜ ω ) ) and ϕ ˜ ω (( A ( ˜ ω ) + B ( ˜ ω ) ) f ) ≤ 0 for all f ∈ D ( A ( ˜ ω ) ) + , and so (f) is also satisfied. Finally, we use (3.8) to obtain

<!-- formula-not-decoded -->

for f ∈ D ( ˜ A ( ω ) ) + , which shows that (g) holds.

Thus, the conditions of Theorem 2.2 are all satisfied and therefore G ( ω ) = A ( ω ) + B ( ω ) is the generator of a substochastic C 0 -semigroup, ( S ( ω ) ( t )) t ≥ 0 , on X ω , which leaves X ˜ ω invariant.

Remark 3.4 . Note that the assumption a ∈ L ∞ loc ([0 , ∞ )) prevents the occurrence of the phenomenon called 'shattering'; see [7, Section 2.3.1]. ♢

Theorem 3.3 allows us to deduce the solution of an ACP that is related to, but distinct from, the fragmentation ACP (3.1). In particular, under the conditions of Theorem 3.3, u ( t ) = S ( ω ) ( t )˚ u is the unique classical solution of the ACP

<!-- formula-not-decoded -->

for all ˚ u ∈ D ( G ( ω ) ). Moreover, if ˚ u ∈ D ( G ( ω ) ) + , then u ( t ) is non-negative for all t ≥ 0. The invariance result in Theorem 3.3 also allows us to obtain a solution of the fragmentation ACP (3.1) for a certain class of initial conditions.

Corollary 3.5. Let Theorems 3.1 and 3.2 hold. Then u ( t ) = S ( ω ) ( t )˚ u is the unique classical solution of (3.1) for all ˚ u ∈ X ˜ ω . If ˚ u ∈ ( X ˜ ω ) + then this solution is nonnegative.

Proof. Let ˚ u ∈ X ˜ ω . From Theorem 3.3, G ( ω ) = A ( ω ) + B ( ω ) is the generator of the substochastic C 0 -semigroup, ( S ( ω ) ( t )) t ≥ 0 that leaves X ˜ ω invariant. Hence, from (3.8), S ( ω ) ( t )˚ u ∈ X ˜ ω = D ( C ( ω ) ) ⊆ D ( A ( ω ) ). It is clear that G ( ω ) and A ( ω ) + B ( ω ) coincide on D ( A ( ω ) ), and we know that u ( t ) = S ( ω ) ( t )˚ u is the unique classical solution of (3.11). It follows that u ( t ) = S ( ω ) ( t )˚ u is also the unique classical solution of (3.1). The nonnegativity result follows from the positivity of ( S ( ω ) ( t )) t ≥ 0 .

Remark 3.6 . Note that X ω is a space of type L , and hence there exists a measurable representation u ( x, t ) of the classical solution that is absolutely continuous with respect to t for almost all x and satisfies (1.1) almost everywhere; see [7, Theorem 5.1.1]. ♢

## 4 Analyticity of the Fragmentation Semigroup

In this section we show that, under a slightly stronger assumption than Theorem 3.2, the operator A ( ω ) + B ( ω ) is the generator of an analytic C 0 -semigroup. This, in turn, allows us to deduce the existence and uniqueness of classical solutions to the fragmentation ACP (3.1) for a wider class of initial conditions than in Theorem 3.5.

Assumption 4.1. Let the weight ω : [0 , ∞ ) → [0 , ∞ ) be such that there exist κ 1 &gt; 0, κ 2 ∈ (0 , 1) and η 0 &gt; 0 so that

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

♢

We now apply Theorem 2.5 to the operators A ( ω ) and B ( ω ) .

Theorem 4.2. Let Theorems 3.1 and 4.1 be satisfied. Then the operator G ( ω ) = A ( w ) + B ( ω ) ( with domain D ( A ( ω ) )) is the generator of an analytic, positive C 0 -semigroup, ( S ( ω ) ( t )) t ≥ 0 on X ω . If κ 1 ≤ 1 , then ( S ( ω ) ( t )) t ≥ 0 is also substochastic.

Proof. We know that A ( ω ) is the generator of a positive C 0 -semigroup ( e -ta () ) t ≥ 0 on X ω , and it is clear that -A ( ω ) is a positive operator. Further, (3.2) and a routine calculation shows that, for f ∈ X ω , λ ∈ C \ R with Re λ &gt; 0,

<!-- formula-not-decoded -->

and hence ( e -ta () ) t ≥ 0 is an analytic semigroup by [10, Theorem II.4.6].

We need to show that B ( ω ) is A ( ω ) -bounded with A ( ω ) -bound strictly less than one. Let f ∈ D ( A ( ω ) ). Then

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

which shows that B ( ω ) is A ( ω ) -bounded with A ( ω ) -bound at most κ 2 &lt; 1. Since B ( ω ) is positive, it follows from Theorem 2.5 that G ( ω ) = A ( ω ) + B ( ω ) generates a positive, analytic C 0 -semigroup, ( S ( ω ) ( t )) t ≥ 0 , on X ω . Moreover, if κ 1 ≤ 1, then the semigroup ( S ( ω ) ( t )) t ≥ 0 is substochastic by Theorem 3.3.

This theorem allows us to deduce the following result regarding solutions of the fragmentation ACP.

Corollary 4.3. Let Theorems 3.1 and 4.1 be satisfied. Then u ( t ) = S ( ω ) ( t )˚ u is the unique classical solution of (3.1) for all ˚ u ∈ X ω . If ˚ u ∈ ( X ω ) + then this solution is non-negative.

Proof. The existence and uniqueness follows from [17, Theorem 1.2.4 and Corollary 4.1.5]. The non-negativity is a consequence of the non-negativity of the semigroup ( S ( ω ) ( t )) t ≥ 0 .

Remark 4.4 . The fact that there exist η 0 &gt; 0 and κ 2 &lt; 1 such that (4.2) holds is equivalent to

<!-- formula-not-decoded -->

♢

Remark 4.5 . The case when ω ( x ) = x p with p &gt; 1 and b satisfies the mass conservation condition (1.2) is considered in [7, Theorem 5.1.47]. ♢

In the following example we consider homogeneous fragmentation kernels; these are also treated in [7, Example 5.1.51].

## Example 4.6. Let

Then

<!-- formula-not-decoded -->

In particular, if (4.3) holds with ω = ω 1 , then it holds also with ω = ω 2 .

Proof. Set g i ( x ) := log ω i ( x ), x &gt; 0, i ∈ { 1 , 2 } . For 0 &lt; x &lt; y and i ∈ { 1 , 2 } we can then write

<!-- formula-not-decoded -->

which, together with (4.4), yields

<!-- formula-not-decoded -->

This, in turn, implies (4.5).

<!-- formula-not-decoded -->

with a non-negative, measurable function h such that ∫ 1 0 h ( ξ ) ξ d ξ = 1. For instance, we can choose h ( z ) = ( ν +2) z ν with ν ∈ ( -2 , 0). Then, for ω ( x ) = x p with p ≥ 1, we have

<!-- formula-not-decoded -->

This shows that (1.2) is satisfied, i.e. we have mass conservation. Moreover, for every p &gt; 1, (4.2) holds with some κ 2 &lt; 1 and any η 0 &gt; 0, and hence also (4.3) is satisfied for every p &gt; 1. ♢

In the following proposition we compare the validity of (4.3) for two different weights.

Proposition 4.7. Let ω 1 and ω 2 be differentiable and increasing functions such that ω i ( x ) &gt; 0 for x &gt; 0 , i ∈ { 1 , 2 } . Moreover, assume that

<!-- formula-not-decoded -->

## 5 Existence of a weight

In this section we prove (in Theorem 5.3) that, under the following mild assumption on the fragmentation kernel b , there exists a weight ω such that (4.1) and (4.2) are satisfied.

Assumption 5.1. Assume that one of the following two statements is true:

- (i) b is bounded on [0 , η ] 2 for every η &gt; 0;
- (ii) there exists η 0 &gt; 0 such that b is bounded on [ η 0 , η ] 2 for every η &gt; η 0 and there exists ω 0 ∈ L ∞ (0 , η 0 ) such that ω 0 ( x ) ≥ 0 for almost every x ∈ [0 , η 0 ] and y ↦→ ∫ η 0 0 b ( x, y ) ω 0 ( x ) d x is locally bounded on [ η 0 , ∞ ).

In case (i) we set η

<!-- formula-not-decoded -->

Remark 5.2 . The assumption about the local boundedness of y ↦→ ∫ η 0 0 b ( x, y ) ω 0 ( x ) d x is satisfied in many cases. For instance, if ∫ y 0 b ( x, y ) x d x ≤ y for all y , we can choose ω 0 ( x ) = x , which, for y ∈ [ η 0 , ∞ ), yields

<!-- formula-not-decoded -->

which is locally bounded.

♢

Theorem 5.3. Suppose that Theorems 3.1 and 5.1 hold and let κ &gt; 0 . Then there exists a function ω : [0 , ∞ ) → [0 , ∞ ) such that ω ( x ) = ω 0 ( x ) for x ∈ [0 , η 0 ) , ω is continuous on [ η 0 , ∞ ) , ω ( x ) &gt; 0 for x &gt; η 0 , and

<!-- formula-not-decoded -->

Before we prove Theorem 5.3, we need two lemmas.

Lemma 5.4. Let η 0 , b and ω 0 be as in Theorem 5.3. Then there exist continuous functions h : [ η 0 , ∞ ) → [0 , ∞ ) and ˜ b : [ η 0 , ∞ ) 2 → [0 , ∞ ) such that

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Proof. Let us start with the proof of (5.1). By assumption, if we set

<!-- formula-not-decoded -->

then it follows that g is bounded on each interval [ η 0 , η ] with η &gt; η 0 , and hence the numbers

<!-- formula-not-decoded -->

are well defined. We construct h as a piecewise linear function:

<!-- formula-not-decoded -->

It is easy to see that h is continuous, and that, for y ∈ [ η 0 + n, η 0 + n + 1), we have h ( y ) ≥ h n ≥ g ( y ), which proves (5.1).

The proof of (5.2) is similar, but we construct a piecewise linear function ˜ b which is constant along diagonals. Set

<!-- formula-not-decoded -->

which is well defined since b is bounded on [ η 0 , η 0 + n +1] 2 by assumption. It is easy to see that the piecewise linear function ˜ b , defined by

<!-- formula-not-decoded -->

is continuous on [ η 0 , ∞ ) 2 . Moreover, if x, y ∈ [ η 0 , ∞ ) with x + y -2 η 0 ∈ [ n, n +1], then ˜ b ( x, y ) ≥ b n ≥ b ( x, y ).

The following lemma is based on standard ideas. For the convenience of the reader we present the proof.

Lemma 5.5. Let η 0 ≥ 0 , let ˜ b : [ η 0 , ∞ ) 2 → [0 , ∞ ) and f : [ η 0 , ∞ ) → [0 , ∞ ) be continuous functions, and let κ &gt; 0 . Then there exists a unique continuous function ω : [ η 0 , ∞ ) → [0 , ∞ ) such that

<!-- formula-not-decoded -->

Proof. Let ℓ &gt; 0 be arbitrary. We construct a unique function ω [ ℓ ] on [ η 0 , η 0 + ℓ ] such that

<!-- formula-not-decoded -->

In the space C ([ η 0 , η 0 + ℓ ]) consider the positive Volterra operator T ℓ defined by

<!-- formula-not-decoded -->

Set

It follows that

<!-- formula-not-decoded -->

By induction we show that, for u ∈ C ([ η 0 , η 0 + ℓ ]),

<!-- formula-not-decoded -->

where ∥ u ∥ := max x ∈ [ η 0 ,η 0 + ℓ ] | u ( x ) | . For n = 0 this is trivial. Now assume that (5.5) holds for some n ; then, for y ∈ [ η 0 , η 0 + ℓ ],

<!-- formula-not-decoded -->

Hence (5.5) is true for all n ∈ N 0 , which implies that

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

where the series converges in the operator norm, and therefore R ( κ, T ℓ ) is a positive operator. This shows that (5.4) has a unique positive solution. Since ℓ was arbitrary and ω [ ℓ 2 ] is an extension of ω [ ℓ 1 ] if ℓ 1 &lt; ℓ 2 , we obtain a unique positive solution of (5.3).

Proof of Theorem 5.3. By Theorem 5.4 there exist h and ˜ b such that (5.1) and (5.2) hold. Without loss of generality, h can be chosen such that h ( y ) &gt; 0 for y &gt; η 0 . Let ω be the unique solution of (5.3) with f := h and set ω ( x ) := ω 0 ( x ) for x ∈ [0 , η 0 ). Then, for y ∈ [ η 0 , ∞ ),

<!-- formula-not-decoded -->

Since h ( y ) &gt; 0 for y &gt; η 0 , we have ω ( y ) &gt; 0 for y &gt; η 0 .

The next theorem shows that, under certain assumptions, condition (4.3) is satisfied with an exponential weight.

Theorem 5.6. Assume that

<!-- formula-not-decoded -->

and that there exist δ 1 , δ 2 &gt; 0 , d &gt; 1 and b m &gt; 0 such that

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

for large enough y . Then there exists c &gt; 1 such that (4.3) is satisfied with ω ( x ) = c x .

Proof. Let c &gt; d be such that 1 log c &lt; δ 1 but otherwise arbitrary, and let δ ∈ (0 , δ 2 ]. Note that the function x ↦→ c x x is strictly increasing on the interval [ 1 log c , ∞ ) , which, together with (5.6), (5.8) and (5.7), implies that

<!-- formula-not-decoded -->

Since c &gt; d , we arrive at

<!-- formula-not-decoded -->

If we first choose δ small enough so that δb m &lt; 1 2 and then c large enough so that c -δ &lt; 1 2 , then the right-hand side of (5.9) is strictly less than 1.

The theorem can be applied to show the existence of an exponential weight in Theorem 6.1.

## 6 Examples

Let us consider two examples. The first one shows that (4.3) can be satisfied for an exponential weight although it is not satisfied for any power weight. It yields binary fragmentation and is similar to the Becker-D¨ oring model in the discrete case in the sense that particles of size y fragment into small particles (of size at most 1) and large particles (of size at least y -1).

## Example 6.1. Let

<!-- formula-not-decoded -->

when y &gt; 2 and b ( x, y ) = 2 y when y ≤ 2. Then we have mass conservation: e.g. for y &gt; 2 we have

<!-- formula-not-decoded -->

For the power weight ω ( x ) = x p with p ≥ 1 we obtain from the Mean Value Theorem that, for y &gt; 2,

<!-- formula-not-decoded -->

as y →∞ . Hence (4.3) is not satisfied. On the other hand, we obtain from Theorem 5.6 that there exists an exponential weight so that (4.3) is fulfilled. We can also give a direct and explicit construction, namely, for the exponential weight ω ( x ) = e x , we have, for y &gt; 2,

<!-- formula-not-decoded -->

as y →∞ , and hence (4.3) is satisfied.

♢

The second example, which is from [7, Example 5.1.51], shows that (4.3) is not always satisfied for an exponential weight. This is different from the situation in the discrete case; for the latter see [11, Theorem 5.5].

## Example 6.2. Let

<!-- formula-not-decoded -->

when y &gt; √ 2 and b ( x, y ) = 2 y when y ≤ √ 2. For ω ( x ) = e x we have, for y &gt; √ 2, (big

O notation for y →∞ )

<!-- formula-not-decoded -->

Hence (4.3) is not satisfied. A similar-but slightly lengthier-calculation can be done for an arbitrary exponential weight ω ( x ) = c x with c &gt; 1. As shown in [7, Example 5.1.51], condition (4.3) is not satisfied for power weights either. However, we can find a faster growing weight so that (4.3) holds, namely, choose ω ( x ) = xe x 2 . Then, for y &gt; √ 2,

<!-- formula-not-decoded -->

as y →∞ , which shows that (4.3) is satisfied.

## References

- [1] M. Aizenman and T.A. Bak, Convergence to equilibrium in a system of reacting polymers, Comm. Math. Phys. 65 (1979), 203-230.
- [2] W. Arendt and A. Rhandi, Perturbation of positive semigroups, Arch. Math. (Basel) 56 (1991), 107-119.
- [3] J. Banasiak, On an extension of the Kato-Voigt perturbation theorem for substochastic semigroups and its applications, Taiwanese J. Math 5 (2001), 169-191.
- [4] J. Banasiak and L. Arlotti, Perturbations of Positive Semigroups with Applications , Springer Monographs in Mathematics, Springer-Verlag London, Ltd., London, 2006.
- [5] J. Banasiak and W. Lamb, Analytic fragmentation semigroups and continuous coagulation-fragmentation equations with unbounded rates, J. Math. Anal. Appl. 391 (2012), 312-322.
- [6] J. Banasiak, W. Lamb and M. Langer, Strong fragmentation and coagulation with power-law rates, J. Engrg. Math. 82 (2013), 199-215.
- [7] J. Banasiak, W. Lamb and P. Lauren¸ cot, Analytic Methods for CoagulationFragmentation Models. Vol. I , Monographs and Research Notes in Mathematics, CRC Press, Boca Raton, FL, 2020.

♢

- [8] A. Batkai, M. Kramar Fijavˇ z and A. Rhandi, Positive Operator Semigroups , Operator Theory: Advances and Applications (vol. 257), Birkh¨ auser/Springer, Cham, 2017.
- [9] R.I. Drake, A general mathematical survey of the coagulation equation, in: Topics in Current Aerosol Research (Part 2) , ed. by G.M. Hidy and J.R. Brock, International Reviews in Aerosol Physics and Chemistry 3, Pergamom Press, 1972, pp. 201-376.
- [10] K.-J. Engel and R. Nagel, One-Parameter Semigroups for Linear Evolution Equations , Graduate Texts in Mathematics (vol. 194), Springer-Verlag, New York, 2000.
- [11] L. Kerr, W. Lamb and M. Langer, Discrete fragmentation systems in weighted ℓ 1 spaces, J. Evol. Equ. 20 (2020), 1419-1451.
- [12] L. Kerr, W. Lamb and M. Langer, Discrete fragmentation equations with timedependent coefficients, Discrete Contin. Dyn. Syst. Ser. S 17 (2024), 1947-1965.
- [13] L. Kerr and M. Langer, Discrete coagulation-fragmentation systems in weighted ℓ 1 spaces, arXiv:2504.21665 (2025).
- [14] A.C. McBride, A.L. Smith and W. Lamb, Strongly differentiable solutions of the discrete coagulation-fragmentation equation, Phys. D 239 (2010), 1436-1445.
- [15] D.J. McLaughlin, W. Lamb and A.C. McBride, A semigroup approach to fragmentation models, SIAM J. Math. Anal. 28 (1997), 1158-1172.
- [16] D.J. McLaughlin, W. Lamb and A.C. McBride, An existence and uniqueness result for a coagulation and multiple-fragmentation equation, SIAM J. Math. Anal. 28 (1997), 1173-1190.
- [17] A. Pazy, Semigroups of Linear Operators and Applications to Partial Differential Equations , Applied Mathematical Sciences (vol. 44), Springer-Verlag, New York, 1983.
- [18] L. Smith, W. Lamb, M. Langer and A. McBride, Discrete fragmentation with mass loss, J. Evol. Equ. 12 (2012), 191-201.
- [19] H.R. Thieme and J. Voigt, Stochastic semigroups: their construction by perturbation and approximation, in: Positivity IV-Theory and Applications , T.U. Dresden, Dresden, 2006, pp. 135-146.
- [20] R.E.M. Verdurmen, P. Menn, J. Ritzert, S. Blei, G.C.S. Nhumaio, S.T. Sonne, M. Gunsing, J. Straatsma, M. Verschueren, M. Sibeijn, G. Schulte, U. Fritsching, K. Bauckhage, C. Tropea, M. Sommerfeld, A.P. Watkins, A.J. Yule and H. Schønfeldt, Simulation of agglomeration in spray drying installations: the EDECAD project, Drying Technology 22 (2004), 1403-1461.
- [21] J. Voigt, On resolvent positive operators and positive C 0 -semigroups on AL -spaces, Semigroup Forum 38 (1989), 263-266.
- [22] J. Wells, 'Modelling coagulation in industrial spray drying: an efficient onedimensional population balance approach', PhD thesis, University of Strathclyde, Department of Mathematics and Statistics, 2018.
- [23] R.M. Ziff, Kinetics of polymerization, J. Statist. Phys. 23 (1980), 241-263.

```
[24] R.M. Ziff and E.D. McGrady, The kinetics of cluster fragmentation and depolymerisation, J. Phys. A 18 (1985), 3027-3037. Lyndsay Kerr, Wilson Lamb, Matthias Langer Department of Mathematics and Statistics University of Strathclyde 26 Richmond Street Glasgow G1 1XH United Kingdom Email addresses: lyndsay.kerr@strath.ac.uk w.lamb@strath.ac.uk m.langer@strath.ac.uk ORCID: 0000-0002-6667-7175 (L.K.) 0000-0001-8084-6054 (W.L.) 0000-0001-8813-7914 (M.L.)
```