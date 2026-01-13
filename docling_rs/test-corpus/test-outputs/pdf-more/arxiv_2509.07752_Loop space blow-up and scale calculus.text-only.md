## Loop space blow-up and scale calculus

Urs Frauenfelder Universit¨ at Augsburg

September 10, 2025

## Abstract

In this note we show that the Barutello-Ortega-Verzini regularization map is scale smooth.

## Introduction

Regularization of two-body collisions is an important topic in celestial mechanics and the dynamics of electrons in atoms. Most classical regularizations blow up the energy hypersurfaces to regularize collisions. Recently Barutello, Ortega, and Verzini [BOV21] discovered a new regularization technique which does not blow up the energy hypersurface, but instead of that the loop space. The discovery is explained in detail in [FW21a, § 2.2] in case of the free fall.

This new regularization technique is in particular useful for non-autonomous systems for which there is no preserved energy and therefore no energy hypersurface which can be blown up.

## Setup and main result

Let C × := C \ { 0 } be the punctured complex plane. We abbreviate by

<!-- formula-not-decoded -->

the space of smooth loops in the punctured plane. Barutello-Ortega-Verzini regularization is carried out using a map R : L C × → L C × defined as follows. For z ∈ L C × define the map

<!-- formula-not-decoded -->

Indeed it takes values in R / Z since t z (0) = 0 and t z (1) = 1 coincide modulo 1 which, by continuity of t z , implies surjectivity. The derivative

<!-- formula-not-decoded -->

∗ Email: urs.frauenfelder@math.uni-augsburg.de

∗

Joa Weber UNICAMP

joa@unicamp.br

depends continuously on τ and is anywhere strictly positive. So t z is also injective. Therefore the inverse τ z := t z -1 exists and it is C 1 ; see [FW21a, § 2.2]. This shows that t z and τ z are elements of the circle's diffeomorphism group Diff( S 1 ). Using this notion the Barutello-Ortega-Verzini map or, alternatively, the rescale-square operation is defined by

<!-- formula-not-decoded -->

The loop space L C × is an open subset of the Fr´ echet space L C = C ∞ ( S 1 , C ). This Fr´ echet space arises as the smooth level of the scale Hilbert space

<!-- formula-not-decoded -->

Indeed Λ C = ∩ k ∈ N 0 Λ C k . For details of scale Hilbert spaces and scale smoothness (sc ∞ ) see [HWZ21]; for an introduction see [Web19], for a summary of what we need here see [FW21b].

The scale Hilbert space Λ C = W 2 , 2 ( S 1 , C ) contains the open subset

<!-- formula-not-decoded -->

which inherits the levels

<!-- formula-not-decoded -->

Observe that L C × = C ∞ ( S 1 , C × ) = ∩ k ∈ N 0 Λ C × k is the smooth level of Λ C × . The map t z , hence R , is well defined for z ∈ Λ C × . In this note we prove

Theorem A. The map R : Λ C × → Λ C × is scale smooth.

## Scale smoothness

## Neumeisters theorem

The proof of Theorem A is based on a result of Neumeister which tells that the action on the free loop space of the diffeomorphism group of the circle

<!-- formula-not-decoded -->

with levels D k := D∩ W 2 , 2+ k ( S 1 , S 1 ), for k ∈ N 0 , is scale smooth.

Theorem 0.1 ([Neu21, Prop. 3.2]) . The reparametrization map

<!-- formula-not-decoded -->

is scale smooth.

Remark 0.2 (Why the zero level is chosen W 2 , 2 and not W 1 , 2 ) . In case ( ψ, z ) ∈ W 1 , 2 ( S 1 , S 1 ) × W 1 , 2 ( S 1 , C ), the derivative

<!-- formula-not-decoded -->

is not necessarily in L 2 . But in case ( ψ, z ) ∈ W 2 , 2 × W 2 , 2 the derivative lies in W 1 , 2 since both factor do and on one of them we can use that W 1 , 2 ⊂ C 0 . Then the second weak derivative exists as well

<!-- formula-not-decoded -->

and lies in L 2 as desired.

## Time rescaling

Lemma 0.3. The map is scale smooth.

Proof. We show that the map t is strongly scale smooth (ssc ∞ ). By definition, this means that the map t is on each level k ∈ N 0 smooth as a map Λ C × k →D k . But strongly scale smooth implies scale smooth (sc ∞ ). To this end we decompose the map t as the composition t = M◦ ( I , ι ◦ N ) of several maps each of which is obviously smooth. These maps are

<!-- formula-not-decoded -->

and and

<!-- formula-not-decoded -->

This proves Lemma 0.3.

## Inverse

Proposition 0.4. The map I : D → D , ψ ↦→ ψ -1 , is scale smooth.

Proof. We compute the scale differentials of the inversion map I . By definition of the inverse, for every ψ ∈ D we have the identity ψ ◦ I ( ψ ) = id. Differentiating this identity we obtain for a tangent vector ˆ ψ ∈ T ψ D = W 2 , 2 ( S 1 , R ) that

<!-- formula-not-decoded -->

Therefore we obtain the formula

<!-- formula-not-decoded -->

Note that ψ ′ ( t ) = 0, for every t , since ψ : S 1 → S 1 is a diffeomorphism.

<!-- formula-not-decoded -->

̸

<!-- formula-not-decoded -->

Let ψ ∈ D and ˆ ψ 1 , ˆ ψ 2 ∈ T ψ D . Note that ψ appears three times in the formula for DI | ψ ˆ ψ . Hence the second derivative is a sum of three terms, namely

<!-- formula-not-decoded -->

Note that D 2 I | ψ ( ˆ ψ 1 , ˆ ψ 2 ) is a polynomial in the six variables

<!-- formula-not-decoded -->

If ψ is in W k +4 , 2 and ˆ ψ 1 , ˆ ψ 2 are in W k +3 , 2 , then all these variables are in W k +2 , 2 . Since multiplication W k +2 , 2 × W k +2 , 2 → W k +2 , 2 is continuous, we conclude that the map

<!-- formula-not-decoded -->

is continuous. Therefore, by the criterium in [FW21b, Le. 4.8] the inversion map I is of class sc 2 .

Differentiating further by induction we obtain that for every n ∈ N D n I | ψ ( ˆ ψ 1 , . . . , ˆ ψ n ) is a polynomial in the ( n +1) n variables

<!-- formula-not-decoded -->

Hence the map

<!-- formula-not-decoded -->

is continuous. Therefore the map I is sc n for every n ∈ N , hence sc ∞ . This finishes the proof of Proposition 0.4.

Remark 0.5. Together with Neumeisters Theorem 0.1, Proposition 0.4 shows that the diffeomorphism group of the circle is a scale Lie group.

## Proof of main result

Proof of Theorem A. The map R can be written as the composition R ( z ) = ρ ( σ ( z ) , I ◦ t ( z )) of scale smooth maps and is therefore itself scale smooth by the scale calculus chain rule [HWZ21, Thm. 1.3.1].

We abbreviate by σ : Λ C × → Λ C × the squaring map z ↦→ z 2 . The squaring map is obviously smooth on every level, hence ssc ∞ , thus sc ∞ . The map I is sc ∞ by Proposition 0.4. The map t is sc ∞ by Lemma 0.3. The map ρ is sc ∞ by Theorem 0.1. This concludes the proof of Theorem A.

## References

- [BOV21] Vivina Barutello, Rafael Ortega, and Gianmaria Verzini. Regularized variational principles for the perturbed Kepler problem. Adv. Math. , 383:Paper No. 107694, 64, 2021. arXiv:2003.09383.
- [FW21a] Urs Frauenfelder and Joa Weber. The regularized free fall I - Index computations. Russian Journal of Mathematical Physics , 28(4):464487, 2021. SharedIt.
- [FW21b] Urs Frauenfelder and Joa Weber. The shift map on Floer trajectory spaces. J. Symplectic Geom. , 19(2):351-397, 2021. arXiv:1803.03826.
- [HWZ21] Helmut Hofer, Krzysztof Wysocki, and Eduard Zehnder. Polyfold and Fredholm theory , volume 72 of Ergebnisse der Mathematik und ihrer Grenzgebiete. 3. Folge. A Series of Modern Surveys in Mathematics . Springer, Cham, 2021. Preliminary version on arXiv:1707.08941.
- [Neu21] Oliver Neumeister. The curve shrinking flow, compactness and its relation to scale manifolds. arXiv e-prints , 2021. arXiv:2104.12906.
- [Web19] Joa Weber. Scale Calculus and M-Polyfolds - An Introduction . Publica¸ c˜ oes Matem´ aticas do IMPA. [IMPA Mathematical Publications]. Instituto Nacional de Matem´ atica Pura e Aplicada (IMPA), Rio de Janeiro, 2019. 32 o Col´ oquio Brasileiro de Matem´ atica. Access pdf. Extended version in preparation.