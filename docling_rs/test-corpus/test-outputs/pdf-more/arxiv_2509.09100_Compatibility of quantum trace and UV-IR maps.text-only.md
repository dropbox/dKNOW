## COMPATIBILITY OF QUANTUM TRACE AND UV-IR MAPS

SAMUEL PANITCH AND SUNGHYUK PARK

Abstract. This paper studies the connection between the quantum trace map [BW11, PPar, GY24b] - which maps the sl 2 -skein module to the quantum Teichm¨ uller space for surfaces and to the quantum gluing module for 3-manifolds - and the quantum UV-IR map [NY20] - which maps the gl 2 -skein module to the gl 1 -skein module of the branched double cover. We show that the two maps are compatible in a precise sense, and that the compatibility map is natural under changes of triangulation; for surfaces, this resolves a conjecture of Neitzke and Yan [NY20]. As a corollary, under a mild hypothesis on the 3-manifold, the quantum trace map can be recovered from the quantum UV-IR map, hence providing yet another construction of the 3d quantum trace map recently introduced in [PPar, GY24b].

## Contents

|                                                                  | 2                                             |
|------------------------------------------------------------------|-----------------------------------------------|
| Organization of the paper                                        | 4                                             |
| Acknowledgements                                                 | 5                                             |
| 2. Quantum trace map                                             | 5                                             |
| 2.1. 2d quantum trace map                                        | 5                                             |
| 2.2. 3d quantum trace map                                        | 14                                            |
| 2.3. Naturality with respect to Pachner moves                    | 20                                            |
| 3. Quantum UV-IR map                                             | 26                                            |
| 3.1. 2d quantum UV-IR map                                        | 26                                            |
| 3.2. 3d spectral networks                                        | 32                                            |
| 3.3. gl 1 -skein modules with defects                            | 33                                            |
| 3.4. 3d quantum UV-IR map                                        | 37                                            |
| 3.5. Naturality with respect to Pachner moves                    | 38                                            |
| 3.6. Stated quantum UV-IR map                                    | 41                                            |
| 4. Compatibility for surfaces                                    | 47                                            |
| 4.1. From gl 2 to sl 2                                           | 48                                            |
| 4.2. Sign-twisted products                                       | 52                                            |
| 4.3. Commutative square for a triangle                           | 55                                            |
| 4.4. Gluing the commutative squares                              | 59                                            |
| 4.5. Proof of Neitzke-Yan conjecture                             | 60                                            |
| 4.6. Naturality with respect to flips                            | 64                                            |
| 5. Compatibility for 3-manifolds                                 | 65                                            |
| 5.1. Commutative square for a face suspension                    | 65                                            |
| 5.2. Gluing the commutative squares                              | 71                                            |
|                                                                  | 75                                            |
| 5.3. Naturality with respect to Pachner moves                    | 5.3. Naturality with respect to Pachner moves |
| 5.4. Recovering the quantum trace map from the quantum UV-IR map | 75                                            |
| 6. Examples                                                      | 76                                            |

| Appendix A.                                         | Stated gl 2 -skein modules 80   |
|-----------------------------------------------------|---------------------------------|
| Appendix B. Stated gl 1 -skein modules with defects | 85                              |
| References                                          |                                 |

## 1. Introduction

At the crossroads of low-dimensional topology and quantum algebra, skein modules have emerged as powerful invariants of 3-manifolds. Since their inception in the 1990s [Prz99, Tur91], they have driven advances in quantum topology and created bridges to areas spanning 3- and 4-dimensional TQFTs [Wal06, GJS23, Jor24], factorization homology [MW12, BZBJ18, Coo23], character varieties [Bul97, PS00], and - crucial to this work - cluster algebras [BW11, Mul16, JLSS21]. Central to this cluster-theoretic picture are 'abelianization' maps that, given an ideal triangulation of a surface, send its skein algebra to a quantum torus, and that, in the 3-manifold setting, produce certain quotients of quantum tori, thereby providing cluster-like coordinates on skein modules that are natural under changes of triangulation. There are two such maps known in the literature: the quantum trace map and the quantum UV-IR map.

Let Σ be a surface equipped with an ideal triangulation τ . The quantum trace map for Σ, introduced by Bonahon and Wong [BW11], is an algebra homomorphism

<!-- formula-not-decoded -->

from the sl 2 -skein algebra SkAlg sl 2 A (Σ) of Σ into a quantum torus called the square-root quantum Teichm¨ uller space SQTS τ (Σ), commonly known as the square-root Chekhov-Fock algebra of (Σ , τ ).

The quantum UV-IR map for Σ, introduced by Neitzke and Yan [NY20] drawing inspirations from [GLM15, Gab17], is an algebra homomorphism

<!-- formula-not-decoded -->

from the gl 2 -skein algebra SkAlg gl 2 q (Σ) of Σ to the gl 1 -skein algebra SkAlg gl 1 q ( ˜ Σ τ ) of the branched double cover ˜ Σ τ of Σ associated to the ideal triangulation τ .

Both the quantum trace map and the quantum UV-IR map behave naturally under changes of triangulation: for different triangulations, the corresponding maps are related via quantum cluster transformations. Moreover, both of these maps admit generalizations to 3-manifolds. The first is the 3d quantum trace map for an ideal triangulation T of a 3-manifold Y ,

<!-- formula-not-decoded -->

which was recently introduced in [PPar, GY24b]. Here, SQGM T ( Y ) denotes the 3d analog of the square-root quantum Teichm¨ uller space, called the square-root quantum gluing module . The second is the 3d quantum UV-IR map

<!-- formula-not-decoded -->

which is described in Section 3.4 of this paper.

While these two maps are both homomorphisms of skein algebras and modules into quantum tori and their quotients, it is worth noting that the construction of the quantum trace map is largely algebraic, whereas the quantum UV-IR map has a more geometric and topological flavor. The quantum trace map is defined explicitly in terms of generators and relations in

91

the skein module, reflecting combinatorial data of the triangulation. In contrast, the quantum UV-IR map relies on a 1-dimensional foliation of the 3-manifold induced by the triangulation, which has its origin in symplectic geometry.

The main purpose of this paper is to bridge the gap between the two maps - the quantum trace map and the quantum UV-IR map - and show that they are compatible in the following sense. Given a 3-manifold Y with an ideal triangulation T and the associated branched double cover ˜ Y T (see Section 3.2), we construct a commutative square

<!-- formula-not-decoded -->

where the four arrows are as follows.

- The bottom horizontal arrow is the 3d quantum trace map [PPar]

<!-- formula-not-decoded -->

for the ideal triangulation T of Y , which we review in Section 2.2, tensored with the identity map on Sk gl 1 -A ( Y ).

- The top horizontal arrow

<!-- formula-not-decoded -->

is the 3d generalization of the quantum UV-IR map of [NY20], which we describe in detail in Section 3.4.

- The left vertical arrow π is the ' gl 2 -sl 2 map'

<!-- formula-not-decoded -->

that factors the gl 2 -skeins into tensor products of sl 2 - and gl 1 -skeins, defined in Section 4.1.

- The right vertical arrow is the 'evaluation map'

<!-- formula-not-decoded -->

which we construct in Sections 4 and 5.

We do this by constructing such commutative squares locally (i.e., for elementary pieces) and then showing that they can be glued consistently.

Theorem A (Compatibility theorem) . The quantum trace map is compatible with the quantum UV-IR map in the sense that there are commutative squares (1) . Moreover, these commutative squares are compatible with Pachner moves.

Along the way, we prove the conjecture of [NY20, Sec. 9.2] on the relation between the 2d quantum trace map and the 2d quantum UV-IR map.

Theorem B (Proof of Neitzke-Yan conjecture; detailed version in Theorem 4.24) . The conjecture of Neitzke-Yan [NY20] on the relation between the 2d quantum trace map and the 2d quantum UV-IR map is true.

There is complementary work in the literature touching on related themes: [KLS23] established a connection between the 2d quantum trace map and the map of Gabella [Gab17], while [KQ22] studied its relation to the non-abelianization map of [GMN13, HN16], which may be viewed as the classical UV-IR map for character varieties. Our Theorem B, however, furnishes a direct and concrete relationship between the 2d quantum trace map and the quantum UV-IR map.

Our compatibility theorem (Theorem A) shows, in particular, that the quantum trace map Tr T , tensored with the identity map on Sk gl 1 -A ( Y ), can be recovered from the quantum UV-IR map F T . Under a mild assumption on Y to ensure that Sk gl 1 -A ( Y ) is torsion-free, we can fully recover the quantum trace map from the quantum UV-IR map:

Theorem C. Suppose Y is a 3 -manifold such that the intersection pairing between H 1 ( Y ; Z ) and H 2 ( Y ; Z ) is zero (e.g., Y can be any knot complement). Then, for any ideal triangulation T of Y , the quantum trace map

<!-- formula-not-decoded -->

can be recovered from the quantum UV-IR map

<!-- formula-not-decoded -->

Explicitly, for any framed, unoriented link L ⊂ Y , if ⃗ L denotes the same link equipped with an arbitrary choice of orientation, then

<!-- formula-not-decoded -->

where

<!-- formula-not-decoded -->

and w ( ⃗ K, ⃗ L ) denote the relative writhe between the two framed oriented links ⃗ K and ⃗ L , i.e., it is the unique integer such that [ ⃗ K ] gl 1 = ( -A ) w ( ⃗ K, ⃗ L ) [ ⃗ L ] gl 1 in Sk gl 1 -A ( Y ) .

We conclude by highlighting some promising avenues for future work. It should be possible to extend our analysis to higher rank. The sl n 2d quantum trace is constructed in [LY23], while the gl 3 version of the 2d quantum UV-IR map is studied in [NY22]. It is natural to expect that these maps both admit 3d generalizations, and that both the 2d and 3d versions for higher rank fit into a similar commutative square to (1).

Even more interesting is to extend our compatibility theorem to closed surfaces by, e.g., comparing the quantum trace map of [DS25] to the quantum UV-IR map associated to the Fenchel-Nielsen spectral networks [HN16].

Organization of the paper. This paper is organized as follows.

In Section 2, we recall the construction of both the 2d and 3d quantum trace maps, following [BW11, Lˆ e18, PPar]. This includes a review of stated sl 2 -skein modules and their splitting homomorphisms. Moreover, we remind the reader of the naturality of the 2d quantum trace map with respect to flips (changes of the ideal triangulation), and prove a corresponding result for the 3d quantum trace map with respect to 2-3 Pachner moves.

Section 3 is dedicated to the quantum UV-IR map. We begin by reviewing the 2d construction, following [NY20], and demonstrating that it is natural with respect to flips. Next, using the theory of 3d spectral networks developed in [FN24], we generalize the map to ideally triangulated 3-manifolds. It is then shown that this 3d quantum UV-IR map is natural with respect to Pachner moves. We conclude the section by introducing a stated version of the quantum UV-IR map (Theorem 3.18), which will allow for computing the quantum UV-IR map locally. This stated version is paramount, as it brings the topological construction of the quantum UV-IR map closer to the algebraic formulation of the quantum trace map.

With both a quantum trace map and a quantum UV-IR map in hand, Section 4 handles the compatibility of the maps for surfaces. We construct the projection map from stated gl 2 -skeins to sl 2 -skeins (Proposition 4.2) and twist the product on the gl 2 -skein modules so that this map is an algebra homomorphism in the surface case and a bimodule homomorphism in the 3-manifold case.

The main idea in proving the compatibility of both the 2d and 3d maps is to first prove that the maps fit into a commutative square locally, and then 'glue' these local squares together. To this end, the evaluation map for triangles is constructed in such a way that the maps are manifestly compatible on a triangle. It is a simple matter of pasting these local squares together to obtain the main theorem of the section, Theorem 4.22, from which we get Theorem B as a corollary. Section 4 concludes with a proof that the compatibility between the 2d quantum UV-IR map and the 2d quantum trace is also natural with respect to changes in the ideal triangulation of the surface.

Section 5 extends the analysis of the previous section to the 3-manifold case. The commutative square for a face suspension is established and shown to respect the gluing relations imposed by the splitting maps. This culminates in Theorem A. We go on to demonstrate that the compatibility of the 3d quantum trace map and 3d quantum UV-IR map is also natural with respect to 2-3 Pachner moves. The remainder of the section shows that the established compatibility between the maps allows for the 3d quantum trace map to be recovered from the UV-IR map (Theorem C).

Section 6 contains a concrete example of the compatibility established in Theorem A for a skein in the figure-8 knot complement Y = S 3 \ 4 1 .

Finally, the definitions and properties of stated gl 2 - and gl 1 -skein modules are detailed in the appendices.

Acknowledgements. We are deeply grateful to Andy Neitzke for uncountably many helpful discussions. We also thank Thang Lˆ e for stimulating conversations. Part of this work was carried out while S.Park was visiting Yale University, and he thanks both the department and especially Andy Neitzke for their hospitality.

S.Park was supported in part by Simons Foundation through Simons Collaboration on Global Categorical Symmetries.

## 2. Quantum trace map

In this section, we review the quantum trace map for surfaces [BW11] and for 3-manifolds [PPar, GY24b].

2.1. 2d quantum trace map. The quantum trace map of Bonahon and Wong [BW11] is an algebra homomorphism from the sl 2 -skein algebra of a surface to its quantum Teichm¨ uller

space. It is a quantization of the classical trace map that expresses the loop coordinates on the sl 2 -character variety in terms of the trace of matrices of Thurston's sheer coordinates on Teichm¨ uller space. The construction of the quantum trace map can be best described in terms of stated skein modules, which we describe next.

2.1.1. Stated sl 2 -skeins. Here we recall the definition of stated sl 2 -skein modules [Lˆ e18, CL22, CL25, PPar] for boundary-marked 3-manifolds, following [PPar, Sec. 3].

Definition 2.1 ([PPar, Def. 3.1]) . Let Y be an oriented 3-manifold with boundary.

A boundary marking for Y is a smoothly embedded oriented graph Γ ⊂ ∂Y where every vertex is either a source or a sink. 1 For example, a disjoint union of oriented intervals on ∂Y is a boundary marking with univalent vertices.

A boundary-marked 3-manifold is a pair ( Y, Γ) consisting of a 3-manifold Y and a boundary marking Γ on ∂Y .

A ribbon tangle in ( Y, Γ) is a ribbon (i.e., framed) 2 tangle in Y whose boundary points lie on the boundary marking Γ away from the vertices V (Γ) and whose framing at the boundary points are parallel (or anti-parallel) to the tangent vector of the boundary marking.

A stated ribbon tangle is a ribbon tangle equipped with a labeling of each boundary point p by a sign µ p ∈ {±} (i.e., a 'state'). See Figure 1 for an example.

Figure 1. An example of a stated ribbon tangle; this one is in a tetrahedron ( ≈ B 3 ) with boundary marking.

<!-- image -->

Definition 2.2. Let ( Y, Γ) be a boundary marked 3-manifold, and let R := Z [ A ± 1 2 , ( -A 2 ) ± 1 2 ] be the base ring. The stated sl 2 -skein module Sk sl 2 A ( Y, Γ) is the R -module generated by the isotopy classes of unoriented, stated ribbon tangles in Y , modulo the following skein

1 In 2d (i.e. for stated skein algebras), the boundary markings will always be disjoint unions of intervals. In 3d, however, the vertices of the boundary markings play a crucial role.

2 We will allow half-twists of the ribbon, so by 'framing', we mean a choice of a section of the RP 1 -bundle over the tangle, given by the unit normal bundle modulo Z / 2.

relations: 3 4 5

<!-- image -->

For an oriented surface Σ with a choice of a set of marked points P ⊂ ∂ Σ on the boundary, the corresponding cylinder (Σ × I, P × I ) is a boundary-marked 3-manifold, where P × I is oriented according to the orientation of the interval I . The associated stated skein module Sk sl 2 A (Σ × I, P × I ) has a natural (unital, associative) algebra structure given by stacking along the I -direction. The stated skein algebra [Lˆ e18, CL22] of (Σ , P ) is defined to be this algebra:

<!-- formula-not-decoded -->

When Σ is a punctured bordered surface (in the sense of [Lˆ e18, CL22]) so that every connected component of ∂ Σ is an interval, we can choose one marked point for each boundary interval, and this gives a canonical choice of boundary marking for Σ × I . In this case, for simplicity of notation, we will often denote the corresponding stated skein algebra simply by SkAlg sl 2 A (Σ).

Example 2.3. An example of a punctured bordered surface is the n -gon D n , which is the 2-disk D 2 with n punctures on the boundary. The associated canonical boundary marking (in the example of n = 6) is illustrated in Figure 2. The associated stated skein algebras SkAlg sl 2 A ( D n ), especially the ones for biangles ( n = 2) and triangles ( n = 3), will play a crucial role in the construction of quantum trace maps.

The stated skein algebras are designed in such a way that there are natural maps associated to the operation of splitting a punctured bordered surface along an ideal arc.

Theorem 2.4 (2d splitting map [Lˆ e18, Thm. 1]) . Let Σ be a punctured bordered surface and let Σ ′ be the surface obtained by splitting Σ along an ideal arc c embedded in Σ . Then, there

3 All the framed links, thought of as ribbons, are drawn flat on the page, except of course on the LHS of the relation (6) which has a positive half-twist.

4 In the relations (4) and (5), the orange line denotes a boundary marking, which should be thought of as sitting at a fixed height in a thickened version of the pictures drawn.

5 Here we use the manifestly symmetric version of the stated skein relations, as in [PPar, Def. 3.3], using the central half-twist relation (6). In [PPar, Rmk. 3.5], it is explained why these relations are equivalent to the usual non-symmetric version of stated skein relations [Lˆ e18].

defined on stated tangles by

Figure 2. D 6 × I with the canonical boundary marking

<!-- image -->

is an algebra homomorphism (in fact an embedding)

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

where L ⃗ ϵ Σ ′ ⊂ Σ ′ × I denotes the stated tangle obtained by splitting L ⊂ Σ × I along c × I and assigning the state ϵ p ∈ {±} to the two newly created boundary points corresponding to the intersection point p ∈ ( c × I ) ∩ L . 6

The 2d splitting map is illustrated in Figure 3.

Figure 3. 2d splitting map. The gray surface is c × I , the surface we are cutting Σ × I along.

<!-- image -->

Definition 2.5. A bad arc is any stated tangle in Σ × I with 1 component that connects two boundary components abutting the same boundary puncture in a trivial way, with boundary states -and + in the counter-clockwise order when viewed from the boundary puncture; see Figure 4.

Figure 4. A bad arc

<!-- image -->

6 To be more precise, in order to get a ribbon tangle after splitting, we need to split after some isotopy of L so that all the intersection points in ( c × I ) ∩ L are of different height and the framing of L at each intersection point is parallel to the I -direction. Part of the claim of this theorem is that the map is independent of the choice of such isotopy.

Definition 2.6 ([BW11], [CL22, Sec. 7]) . The reduced stated skein algebra SkAlg sl 2 A (Σ) is the quotient

<!-- formula-not-decoded -->

where I bad denotes the two-sided ideal of SkAlg sl 2 A (Σ) generated by the bad arcs.

It is easy to see that the image of a bad arc in Σ under the splitting map (Theorem 2.4) is in the two-sided ideal generated by bad arcs in Σ ′ , and it follows that the splitting map descends to a splitting map of reduced stated skein algebras:

<!-- formula-not-decoded -->

This map also turns out to be an embedding. As a special case, consider a punctured bordered surface Σ equipped with an ideal triangulation τ ; let τ ( d ) denote the set of all d -simplices in the triangulation. Then we have:

Corollary 2.7 ([Lˆ e18]) . There is an algebra homomorphism (in fact an embedding)

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

where E ⊂ Σ denotes the union of all the edges of the triangulation, and L ⃗ ϵ △ denotes the part of L in △× I after splitting, with states assigned to newly created boundary points according to ⃗ ϵ .

The reduced stated skein algebras of biangles and triangles, which for simplicity we will call the biangle algebra and the triangle algebra respectively, have particularly simple structures (e.g. they are quantum tori) and will play a crucial role in the construction of both 2d and 3d quantum trace maps.

Theorem 2.8 ([BW11], [CL22, Sec. 7.6-7.7]) . (1) The biangle algebra is

<!-- formula-not-decoded -->

with the isomorphism given by

<!-- formula-not-decoded -->

(2) The triangle algebra is

<!-- formula-not-decoded -->

with the isomorphism given by

<!-- image -->

<!-- image -->

and likewise for β and γ .

2.1.2. 2d quantum trace map. Here we briefly review the construction of the 2d quantum trace map [BW11, Lˆ e18, CL22]; the details can be found in the cited references.

Definition 2.9. Let Γ be a lattice equipped with an antisymmetric bilinear form ⟨ , ⟩ : Γ × Γ → Z . The quantum torus Q Γ associated to Γ is the unital associative R -algebra with basis { x γ } γ ∈ Γ and the product law

<!-- formula-not-decoded -->

Note, if we are given a basis e 1 , · · · , e n of the lattice, the quantum torus is generated by x 1 , · · · , x n (where x i := x e i ) and their inverses, subject to commutation relations

<!-- formula-not-decoded -->

The Weyl-ordered product [ x i 1 · · · x i k ] of x i 1 , · · · , x i k is defined to be

<!-- formula-not-decoded -->

The Weyl-ordered product is designed to be independent of the ordering of the variables.

Definition 2.10. Fix an invertible scalar c T ∈ R × . The extended triangle algebra ˜ T = ˜ T c T is an extension of the triangle algebra T (as in Theorem 2.8) defined as

<!-- formula-not-decoded -->

equipped with an embedding

<!-- formula-not-decoded -->

We view the generators a, b, c as being associated to the three edges of the triangle in counter-clockwise order, as in Figure 5.

Figure 5. Edges a, b, c (and their inverses) generate the extended triangle algebra ˜ T .

<!-- image -->

Suppose Σ is a punctured surface (without boundary) equipped with an ideal triangulation τ . For each edge e ∈ τ (1) of the ideal triangulation τ , consider the following element ˆ x e of the big quantum torus ⊗ △∈ τ (2) ˜ T .

- (1) If there are two ideal triangles △ 1 , △ 2 ∈ τ (2) abutting e , define

<!-- formula-not-decoded -->

where a 1 , a 2 are the edges of △ 1 , △ 2 which are glued to give e , respectively.

- (2) If there is only one ideal triangle △ abutting e , define

<!-- formula-not-decoded -->

where a 1 , a 2 are the two edges of △ which are glued to give e .

By convention, we are suppressing the entries of the tensor product which are 1. The element ˆ x e ∈ ⊗ △∈ τ (2) ˜ T is called the square-root quantized shear parameter associated to the edge e of the ideal triangulation τ of Σ. The idea behind the definition of ˆ x e is that each edge e ∈ τ (1) is obtained by gluing two bare edges (i.e. edges of the ideal triangles before gluing), as in Figure 6.

Figure 6. An edge e of triangulation τ is split into two bare edges a 1 and a 2 .

<!-- image -->

Definition 2.11. The square-root quantum Teichm¨ uller space (a.k.a. square-root ChekhovFock algebra ), denoted SQTS τ (Σ), is the sub-quantum torus of ⊗ △∈ τ (2) ˜ T generated by { ˆ x e } e ∈ τ (1) .

In other words, SQTS τ (Σ) is the quantum torus Q Γ τ for the lattice Γ τ generated by the edges of the triangulation τ , equipped with the antisymmetric bilinear form given by

<!-- formula-not-decoded -->

where a ee ′ denotes the number of angular sectors delimited by e and e ′ in the faces of τ , with e coming first in counter-clockwise order.

Combining the splitting map (Corollary 2.7) and the embedding of triangle algebras into the extended triangle algebras (Definition 2.10), we obtain the following sequence of maps:

<!-- formula-not-decoded -->

It is easy to see that the image of the composition of these maps is contained in SQTS τ (Σ).

Theorem 2.12 ([BW11, Lˆ e18]) . There is an algebra embedding Tr τ : SkAlg sl 2 A (Σ) → SQTS τ (Σ) defined as the composition:

<!-- image -->

.

Remark 2.13. While the 2d quantum trace map Tr τ = Tr τ, c T , as we presented in Theorem 2.12, depends on the choice of an invertible scalar c T ∈ R × , this dependence can be absorbed into rescaling of the generators of SQTS τ (Σ). That is, for any c T , c T ′ ∈ R × , we have a commutative diagram

<!-- image -->

<!-- formula-not-decoded -->

is an automorphism that simply rescales the generators of the quantum torus.

2.1.3. Naturality of the 2d quantum trace maps with respect to flips. Suppose τ and τ ′ are two ideal triangulations of Σ that differ by a flip, i.e. they look identical outside of an ideal quadrilateral in which we change the triangulation as in Figure 7. Then, there is an

Figure 7. A flip on edge x

<!-- image -->

algebra isomorphism between (some appropriate completions of) the square-root quantum Teichm¨ uller spaces given by 7

<!-- formula-not-decoded -->

7 Here, we describe this for c T = 1; in order to get a transition map compatible with the rescaled quantum trace map Tr c T , replace every ˆ x ′ by c T ˆ x ′ .

where

where f and g are some versions of the quantum dilogarithm given by

<!-- formula-not-decoded -->

and ˆ e ↦→ ˆ e for every edge e ∈ τ (1) other than the 5 edges involved in the flip. Note, f and g satisfy

<!-- formula-not-decoded -->

From these identities, it follows that, if we restrict to the even part of the algebra (i.e., the subalgebra generated by all monomials whose total degree of variables associated to each triangle is even), the transition map θ τ → τ ′ is given by (see [Hia10], [BW11, Sec. 7])

<!-- formula-not-decoded -->

and likewise for monomials obtained by a 180 degree rotation (i.e. after simultaneously replacing ˆ y ↔ ˆ v and ˆ z ↔ ˆ w ). In particular, when restricted to the even part, θ τ → τ ′ restricts to an algebra isomorphism between fractional division algebras of the quantum tori.

These algebra isomorphisms satisfy the pentagon relation (see Figure 8), and as a result, if τ and τ ′ are any two ideal triangulations of Σ, then we can choose any sequence of flips from τ to τ ′ and define θ τ → τ ′ : ̂ SQTS τ (Σ) ∼ → ̂ SQTS τ ′ (Σ) to be the composition of the transition maps associated to those flips; by the pentagon relation, the resulting map θ τ → τ ′ is independent of the choice of the sequence of flips from τ to τ ′ . The 2d quantum trace map is compatible

Figure 8. Pentagon relation ensures that different sequences of flips from τ to τ ′ induce the same transition map.

<!-- image -->

with these transition maps; the following diagram commutes: 8

<!-- image -->

- 2.2. 3d quantum trace map. In this subsection, we review the construction of the 3d quantum trace map of [PPar].
- 2.2.1. Bimodule structure and the splitting map. Let ( Y, Γ) be a boundary-marked 3-manifold. A crucial observation [PPar, Sec. 3.1] is that Sk sl 2 A ( Y, Γ) has a natural module structure over SkAlg sl 2 A ( D n ) for each vertex of the boundary marking, where n is the degree of the vertex. This can be seen as follows. For any vertex v of Γ of degree deg v = n , we can take the complement of a small neighborhood of v to get a local picture homeomorphic to a cylinder with n boundary markings; see Figure 9. The action of a stated skein in SkAlg( D n ) is then

Figure 9. SkAlg( D deg v )-module structure at v ∈ V (Γ)

<!-- image -->

8 Here, we are setting c T = 1. For rescaled quantum trace maps, we need to rescale the transition maps accordingly; see the previous footnote.

obtained by stacking it on top of this cylinder; this action is either a left or right action depending on whether the vertex is a sink or a source. For simplicity of notation, let's write

<!-- formula-not-decoded -->

where V (Γ) + and V (Γ) -denote the set of sink and source vertices of Γ, respectively. The following proposition summarizes this module structure:

Proposition 2.14 ([PPar, Prop. 3.13]) . The stated skein module Sk sl 2 A ( Y, Γ) has a natural SkAlg sl 2 A ( V (Γ) + ) -SkAlg sl 2 A ( V (Γ) -) -bimodule structure.

This bimodule structure plays an essential role in understanding the behavior of stated skein modules under cutting a 3-manifold into simpler pieces. In the natural 3d analog of the splitting homomorphism (Theorem 2.4), the codomain must be the relative tensor product with respect to these bimodule structures; see [PPar, Sec. 3.2]. For our purposes, it is enough to consider the special case of the splitting homomorphism corresponding to cutting an ideally triangulated 3-manifold into elementary pieces called face suspensions , which we review below in Theorem 2.18. Readers interested in the general form of 3d splitting homomorphisms can refer to [PPar, Thms. 3.21, 3.24].

As in the 2d case, the 3d quantum traces we are going to define factor through a quotient of the stated skein module obtained by setting all of the bad arcs to 0. Recall from Definition 2.5 that bad arcs are stated tangles of a specific form near the boundary; for each vertex v ∈ V (Γ) of the boundary marking Γ, the bad arcs of SkAlg sl 2 A ( D deg v ) are shown in Figure 10. 9 Analogously to Definition 2.6, we define:

Figure 10. Bad arcs in SkAlg sl 2 A ( D deg v )

<!-- image -->

Definition 2.15 ([PPar, Def. 3.34]) . The reduced stated skein module Sk sl 2 A ( Y, Γ) is the quotient

<!-- formula-not-decoded -->

where I bad , + denotes the right ideal of SkAlg sl 2 A ( V (Γ) + ) generated by the bad arcs near the sinks, and I bad , -denotes the left ideal of SkAlg sl 2 A ( V (Γ) -) generated by the bad arcs near the sources.

Similarly to (7), let's write, for simplicity of notation,

<!-- formula-not-decoded -->

9 The pictures shown are projections of the stated tangles in ( Y, Γ) on the boundary ∂Y in a neighborhood of v , viewed from outside of Y (so, mirror the pictures when viewed from inside of Y ). The dashed orange lines indicate that there may be more edges of Γ incident to v in those regions of ∂Y .

Then, it is clear from the definition that the reduced stated skein module Sk sl 2 A ( Y, Γ) has a SkAlg sl 2 A ( V (Γ) + )-SkAlg sl 2 A ( V (Γ) -)-bimodule structure.

Suppose Y is a cusped 3-manifold (without boundary) equipped with an ideal triangulation T ; let T ( d ) denote the set of all d -simplices in the triangulation. We call the vertices, edges, and faces of the tetrahedra in T (3) before gluing the bare vertices , bare edges , and bare faces , respectively. For each ideal tetrahedron T ∈ T (3) , we fix a base point in the interior and call it the barycenter of T . For each bare vertex v (resp. bare edge e , resp. bare face f ) of T , the vertex cone Cv (resp. edge cone Ce , resp. face cone Cf ) is the join of v (resp. e , resp. f ) with the barycenter of T . Each face f ∈ T (2) splits into two bare faces, and the face suspension Sf is the union of the two face cones. See Figure 11.

Figure 11. A vertex cone Cv , an edge cone Ce , and a face suspension Sf for the face with vertices a, b, c

<!-- image -->

Consider the decomposition Y = ⋃ f ∈T (2) Sf of Y into face suspensions. Note, under this decomposition, each edge cone splits into two bare edge cones. We equip each face suspension Sf with a boundary marking by drawing an edge from each of the 3 side edges to the 2 cone points; see Figure 12. Each edge of the boundary marking corresponds to a bare edge cone (i.e. a face of Sf ). We may label the top and the bottom bare edge cones abutting the edge a by a 1 and a 2 , respectively, and likewise for the edges b and c . This boundary marking has 2

Figure 12. A face suspension with the standard boundary marking

<!-- image -->

sink vertices of degree 3 and 3 source vertices of degree 2, so the reduced stated skein module Sk sl 2 A ( Sf ) has a natural T ⊗ 2 -B ⊗ 3 -bimodule structure. In [PPar, Sec. 4], the structure of such bimodules is completely determined. To state this structure theorem for face suspensions, let x a denote the elementary tangle connecting the bare edge cones a 1 and a 2 with states ++

(and likewise for x b and x c ), and let α i ( i = 1 , 2) denote the elementary tangle connecting the bare edge cones b i and c i with states ++ (and likewise for β i and γ i ).

Proposition 2.16 ([PPar, Cor. 4.9]) . The reduced stated skein module of a face suspension is given by

<!-- formula-not-decoded -->

as a left T ⊗ 2 ⊗ ( B op ) ⊗ 3 = T ⊗ 2 ⊗ B ⊗ 3 -module, where

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

In order to state the splitting map associated to the decomposition of Y into face suspensions, we need to first define the relative tensor product of these bimodules.

Definition 2.17. The relative tensor product ⊗ f ∈T (2) Sk sl 2 A ( Sf ) of the bimodules Sk sl 2 A ( Sf ), f ∈ T (2) , is defined to be the quotient of the ordinary tensor product (as R -modules) ⊗ f ∈T (2) Sk sl 2 A ( Sf ) by the following relations:

- For each vertex cone Cv , we have the following relations among left actions of ⊗ f ∈T (2) Sf abutting Cv T on ⊗ f ∈T (2) Sk sl 2 A ( Sf ):

<!-- image -->

where each sector in the above diagrams represents one of the three face suspensions surrounding Cv (viewed from the vertex v ), and the markings shown are on the bare edge cones abutting Cv .

and

- For each internal edge e ∈ T (1) we have the following relations among right actions of ⊗ f ∈T (2) f abutting e B on ⊗ f ∈T (2) Sk sl 2 A ( Sf ):

<!-- image -->

where each sector in the above diagrams represents one of the face suspensions surrounding e (as many as the number of tetrahedra abutting e ), and the markings shown are on the bare edge cones abutting e .

Theorem 2.18 ([PPar, Cor. 3.38]) . There is a well-defined splitting map

<!-- formula-not-decoded -->

where L ⃗ ϵ f denotes the part of L in Sf after splitting, with boundary states determined by ⃗ ϵ .

2.2.2. 3d quantum trace map. The codomain of the 3d quantum trace map is the 3d analog of the square-root quantum Teichm¨ uller space called the square-root quantum gluing module . Just like the square-root quantum Teichm¨ uller space was built out of extended triangle algebras, the square-root quantum gluing module can be built out of the following basic building blocks:

Definition 2.19. The face suspension module S f of Sf is the quantum torus

<!-- formula-not-decoded -->

generated by the 6 bare edge cones of Sf , viewed as a regular ˜ T ⊗ 2 -˜ T ⊗ 2 -bimodule.

Theorem 2.20. Let c T , c B ∈ R × be invertible scalars such that

<!-- formula-not-decoded -->

Then there is a well-defined T ⊗ 2 -B ⊗ 3 -bimodule homomorphism (in fact, an embedding)

<!-- formula-not-decoded -->

mapping the empty skein [ ∅ ] to 1 , induced by the embeddings of algebras

<!-- formula-not-decoded -->

and

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Remark 2.21. Note, scaling both c T and c B by a common factor just corresponds to rescaling the generators of S f . Up to scaling, there are two possible choices of the scalars c T and c B : 10

<!-- formula-not-decoded -->

In [PPar], we used ( c T , c B ) = ( A -1 , ( -1) -1 2 ) which, up to scaling, corresponds to the second choice.

For each edge cone Ce corresponding to a bare edge e , consider the element ˆ z e = a ⊗ a ′ of the quantum torus ⊗ f ∈T (2) S f , where a and a ′ are the two bare edge cones corresponding to Ce after splitting into face suspensions; see Figure 13. The element ˆ z e ∈ ⊗ f ∈T (2) S f is called the square-root quantized shape parameter associated to the bare edge e of the ideal triangulation T of Y .

Figure 13. An edge cone Ce splits into two bare edge cones a and a ′ . Note, this is a cone over Figure 6.

<!-- image -->

The square-root quantized shape parameters generate a sub-quantum torus of ⊗ f ∈T (2) S f . The square-root quantum gluing module SQGM T ( Y ) that we define below is a two-sided quotient of this sub-quantum torus. 11

10 While these two choices can be related by scaling the generators of one copy of ˜ T by ( -1) 1 2 and the generators of the other copy by ( -1) -1 2 , there is no canonical such choice since the two copies are identical.

11 The definition of SQGM T ( Y ) we use here is slightly different from the one we gave in [PPar, Sec. 5]; the one we use here is the quotient of the one in [PPar] by setting ˆ z e = ˆ z e ′ for each pair of opposite bare edges e, e ′ . The effect of this quotient is minor, as the vertex relations already imply that ˆ z 2 e = ˆ z 2 e ′ . It may seem like there is another natural quotient given by setting ˆ z e = -ˆ z e ′ , but such a quotient turns out to be inconsistent with the 2-3 Pachner move.

Definition 2.22. With the scaling parameters c B and c T , the square-root quantum gluing module SQGM T ( Y ) is the two-sided quotient of the quantum torus

<!-- formula-not-decoded -->

where ˆ z , ˆ z ′ , ˆ z ′′ are (generators associated to) the 3 pairs of opposite edge cones of T , in the anti-clockwise order when viewed from a vertex

<!-- image -->

by the following relations:

- (V) Vertex relations (central): For each tetrahedron,

<!-- formula-not-decoded -->

- (L) Lagrangian relations (as left actions): For each tetrahedron,

<!-- formula-not-decoded -->

- (G) Gluing relations (as right actions): For each edge,

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

and k is the number of edge cones abutting the edge e .

For the rest of the paper, unless otherwise specified, fix ( c T , c B ) = ( q -1 2 , 1).

Theorem 2.23. There is an R -module homomorphism Tr T : Sk sl 2 A ( Y ) → SQGM T ( Y ) defined as the composition

<!-- formula-not-decoded -->

.

- 2.3. Naturality with respect to Pachner moves. We'll show that the 3d quantum trace map is natural with respect to 2-3 Pachner moves. Suppose that T 2 and T 3 are two ideal triangulations of Y related by the 2-3 Pachner move.

Then, we have:

where

Figure 14. 2-3 Pachner move on the triangular bipyramid

<!-- image -->

Figure 15. The 2-3 Pachner move turns 7 face suspensions (6 exterior + 1 interior) into 9 face suspensions (6 exterior + 3 interior). The interior faces (1 on the left and 3 on the right) are highlighted in blue.

<!-- image -->

Proposition 2.24. There is an R -module homomorphism

<!-- formula-not-decoded -->

defined on the quantum torus generators - each of which corresponds to an edge cone - by

Proof. To check the well-definedness of φ 2 → 3 defined above for the generators of the quantum torus, we need to check that the map descends to the two-sided quotient - that is, we need to check that they satisfy the vertex equation (V)

<!-- image -->

<!-- formula-not-decoded -->

and the Lagrangian equation (L)

<!-- formula-not-decoded -->

where ˆ z, ˆ z ′ , ˆ z ′′ are any triple of edge cones sharing a vertex, which are cyclically ordered in an anticlockwise manner when viewed from a vertex, as

<!-- image -->

Note, there are no gluing equations (G) to check, since the bipyramid triangulated into two tetrahedra has no interior edges.

Vertex equation: For either the top or the bottom vertex of the bipyramid, we have, as a central relation,

<!-- image -->

where we have used red to indicate inverses. Note, for the third equality, we have used the fact that the gluing relation associated to the interior edge of the 3-tetrahedra triangulation of the bipyramid is actually central, as the product of the 3 edge cones is central in the image of φ 2 → 3 .

Since we are identifying the square-root quantized shape parameters associated to the opposite edge cones, the proof is identical for the side vertices of the bipyramid.

Lagrangian equation: The proof is analogous to that of [GY24a, Lem. 4.1]. Let's label the edge cones as in the figure below:

<!-- image -->

We have, as a left relation,

<!-- formula-not-decoded -->

Again, we have used the fact that the gluing relation for the interior edge of the 3-tetrahedra triangulation,

<!-- formula-not-decoded -->

which is a priori a relation among right actions, is actually central in the image of φ 2 → 3 . □

Theorem 2.25. The 3d quantum trace map is compatible with the Pachner move in the sense that we have the following commutative diagram:

<!-- image -->

Proof. Since the 3d quantum trace map factors through the splitting map for reduced stated skein modules, it suffices to check the commutativity for the triangular bipyramid BP on which we are performing the 2-3 Pachner move. Topologically, the triangular bipyramid is

a 3-ball, whose boundary is combinatorially foliated (see Figure 16), so we have an explicit generator-and-relation description of Sk sl 2 A (BP). Concretely, we have

Figure 16. Triangular bipyramid BP with the standard boundary marking (all the edges of the boundary marking are oriented toward the center of each face)

<!-- image -->

<!-- formula-not-decoded -->

where we have one factor of T for each face and one factor of B op for each edge of the triangular bipyramid, and Ann([ ∅ ]) is the left ideal generated by relations coming from each face of the boundary tessellation. The triangular bipyramid splits into 7 face suspensions in the case of T 2 and 9 face suspensions in the case of T 3 , so we need to check the commutativity of the diagram

<!-- image -->

where ∼ denotes the relations defining the square-root quantum gluing module (Definition 2.22). Since the maps are bimodule homomorphisms, it suffices to check this for each generator of the triangle and biangle algebras. Even though there are 6 factors of triangle algebras and 9 factors of biangle algebras, by symmetry, there are only 3 different types of generators we need to check:

- (1) triangle algebra of a face,
- (2) biangle algebra of a non-horizontal edge,
- (3) biangle algebra of a horizontal edge.

The commutativity for the triangle algebra generators is obvious, as the 2-3 Pachner move doesn't affect the tangles localized at the center of a face of the bipyramid. The commutativity for the non-horizontal biangle algebra generators is also immediate; in fact, we have defined φ 2 → 3 in such a way that the diagram commutes. Thus, the only non-trivial check is the

commutativity for the horizontal biangle algebra generators (as right relations):

.

<!-- image -->

This diagram commutes because:

<!-- image -->

where, as before, we have used red to indicate inverses.

□

Remark 2.26. When identifying the square-root quantized shape parameters of opposite edge cones, we had to choose between setting them equal or setting them to negatives of each other, since only their squares agreed a priori. It turns out that for the compatibility with

the 2-3 Pachner move, only the first identification works; for the second choice, we get an extra -sign in the computation above.

## 3. Quantum UV-IR map

In this section, we review the quantum UV-IR map [NY20] for surfaces and discuss its generalization to 3-manifolds.

3.1. 2d quantum UV-IR map. Let Σ be a surface equipped with an ideal triangulation τ . Then, there is a branched double cover ˜ Σ = ˜ Σ τ of Σ associated to the ideal triangulation. This branched double cover can be obtained by putting a branch point at the center of each ideal triangle and drawing branch cuts as in Figure 17; the corresponding branched double cover is an ideal hexagon. We will often trivialize the double cover away from the branch cuts so that we can call the two sheets of ˜ Σ by 'sheet 1' and 'sheet 2'.

Figure 17. Branch point (red dot) and branch cuts (orange squiggly lines) for an ideal triangle

<!-- image -->

Given such a branched double cover ˜ Σ → Σ, [NY20] constructed an algebra homomorphism, called the quantum UV-IR map , from the gl 2 -skein algebra of Σ to the gl 1 -skein algebra of the double cover ˜ Σ. Before reviewing this construction, let's briefly recall the definition of gl 2 - and gl 1 -skein modules.

Definition 3.1. Let Y be an oriented 3-manifold. The gl 2 -skein module Sk gl 2 q ( Y ) of Y is defined as

<!-- formula-not-decoded -->

where the gl 2 -skein relations are given by

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

with [2] = q + q -1 , the quantum 2. 12 13

For our gl 1 -skein modules, we need to twist the usual notion of gl 1 -skein modules by introducing 'sign defects'.

Definition 3.2. Let Y be an oriented 3-manifold decorated by a 'sign defect' B ⊂ Y , an embedded 1-manifold. The gl 1 -skein module Sk gl 1 q ( Y ) = Sk gl 1 q ( Y, B ) of Y is defined as

<!-- formula-not-decoded -->

where the gl 1 -skein relations are given by

<!-- image -->

with the red line in the final skein relation representing a part of the sign defect B . 14

Whenever we talk about the gl 1 -skein module of a branched double cover, we will always assume that we are decorating the branched double cover by a sign defect along the branch locus. 15

The 2d quantum UV-IR map [NY20] is an algebra homomorphism

<!-- formula-not-decoded -->

from the gl 2 -skein algebra of Σ to the gl 1 -skein algebra of the branched double cover ˜ Σ (associated to a choice of ideal triangulation τ ) decorated by sign defects at the branch points. The construction is best described in terms of projection to the leaf space of a

12 Here, we used the half-twist relation (14) - instead of the usual full-twist relation - for convenience; this doesn't change the skein module. We do the same for gl 1 skein modules (see (19)).

13 In [NY20, Sec 3.1], the last two skein relations (15)-(16) (which correspond, for instance, to [QW21, Eqn. 2.3]) were missing, but here we add them for completeness, since the quantum UV-IR map indeed respects all these relations.

14 This last skein relation justifies the use of the name 'sign defect', as our skein acquires an extra sign as it passes through the sign defect.

15 The reason we need to twist the gl 1 -skein module of ˜ Y by the sign defect along the branch locus has to do with the fact that the pull-back of a spin structure on Y to ˜ Y \ B doesn't extend over B , i.e. it is the non-bounding spin structure over the S 1 linking the branch locus B ; see [FN24, ELPS25].

1-dimensional foliation of Σ × I called the WKB foliation . 16 Topologically, the WKB foliation is a 1-dimensional foliation that looks like Figure 18 in each ideal triangle. Note that near each edge of the ideal triangle, the leaves are parallel and thus glue well to give a 1-dimensional foliation of Σ in the complement of the branch locus.

Figure 18. 2d WKB foliation and its orientation

<!-- image -->

The WKB foliation of Σ lifts to an oriented 1-dimensional foliation on ˜ Σ: we orient the leaves of the foliation of ˜ Σ so that the orientation is away from (resp. toward) the ideal vertices in sheet 1 (resp. sheet 2). This orientation is also indicated in Figure 18.

While the generic leaves of the WKB foliation of Σ are homeomorphic to R , there are 3 singular leaves in each ideal triangle which are homeomorphic to R ≥ 0 ; these are the three leaves emanating out of the branch point. The union of singular leaves is called the 2d spectral network [GMN13].

The space of leaves in the WKB foliation, or the leaf space for short, of an ideal triangle times I is given by 3 facets glued along a binder, as in Figure 19. The leaf space for Σ × I is obtained by gluing the leaf space for T × I along the edges. Note, we can canonically identify the binders of the leaf space to the branch locus and the facets of the leaf space to the branch cuts.

Figure 19. Leaf space for an ideal triangle times I

<!-- image -->

3.1.1. 2d quantum UV-IR map. Now, we are ready to review the construction of the 2d quantum UV-IR map. Let L be a framed oriented link in Σ × I . By making a small isotopy if necessary, let's assume that L is in general position with respect to the WKB foliation, so that the projection of L on the leaf space gives a nice link diagram. The link diagram of L on the leaf space will have finitely many crossings, half-twists, and intersection points with the binders. In terms of the WKB foliation, crossings correspond to the leaves that intersect with L twice, half-twists correspond to points of L where the tangent vector is parallel to

16 The name comes from the exact WKB analysis of Schr¨ odinger equations on Riemann surfaces, where such a foliation can be obtained from a holomorphic quadratic differential.

that of the foliation, and intersection points with the binders correspond to singular leaves (i.e. the leaves of the WKB foliation that meet the binder) that intersect with L .

The 2d quantum UV-IR map [NY20] is defined as follows:

<!-- formula-not-decoded -->

where ˜ L ranges over all possible lifts ˜ L ⊂ ˜ Σ of L ⊂ Σ that can be constructed out of direct lifts , detours and exchanges , and c ˜ L ∈ Z [ q ± 1 ] is the product of exchange factors , turning factors , and framing factors , each of which we describe below.

The lifts ˜ L are constructed from L by attaching the leaves of the foliation as follows:

- (Direct lifts) Any part of L can be directly lifted to either sheet 1 or sheet 2; we denote the lift to sheet i ∈ { 1 , 2 } by labeling that part of L by i :

<!-- formula-not-decoded -->

- (Detours) If L intersects a singular leaf at some point (i.e. when L crosses a binding of the leaf space), that part of L can be lifted using a detour in the following way:
- (Exchanges) If there is a generic leaf that meets L at two points (i.e. at a crossing of L on the leaf space), that part of L can be lifted using an exchange in the following way:

<!-- image -->

<!-- formula-not-decoded -->

The coefficient c ˜ L is defined as the product of the following factors:

- (Exchange factor) For each exchange, we get a factor of ± ( q -q -1 ), where the sign is given by the sign of the crossing where the exchange was used.
- (Turning factor) We get a factor q t , where t is the turning number of ˜ L projected on the leaf space; for part of ˜ L in sheet i ∈ { 1 , 2 } , we compute the turning number by viewing the leaf space from the side where the i -direction is pointing away from our eyes.
- (Framing factor) We get an overall framing factor given by the product of q 1 2 , one for each positive half-twist, and q -1 2 , one for each negative half-twist.

See [NY20, Sec. 5] for explicit examples.

Theorem 3.3 ([NY20, Sec. 7]) . The 2d quantum UV-IR map F : SkAlg gl 2 q (Σ) → SkAlg gl 1 q ( ˜ Σ) defined as above is a well-defined algebra homomorphism.

Proof. As shown in [NY20, Sec. 7], the quantum UV-IR map is invariant under isotopy and respects the skein relations (12)-(14). Since the skein relations (15)-(16) were missing in their

paper, we add a proof that the quantum UV-IR map indeed respects these two extra skein relations as well.

We draw all the diagrams on a facet of the leaf space - which locally looks like R 2 - with the leaves oriented in such a way that the '1'(resp., '2')-direction is pointing away from (resp. toward) our eyes.

- The relation (15): Defining

<!-- formula-not-decoded -->

the relation (15) is equivalent to the following:

<!-- formula-not-decoded -->

The following small calculation will be useful:

<!-- formula-not-decoded -->

From this, we see that

<!-- formula-not-decoded -->

as desired.

- The relation (16):

<!-- formula-not-decoded -->

.

.

<!-- formula-not-decoded -->

where, in the second and the third line, we have drawn the two sheets separately, and σ ∈ S 4 2 ranges over all possible ways to perturb the relative positions of the boundary points of the tangle in sheet 1 and 2, on all 4 edges of the square; when the tangle is oriented upward, the trivial permutation 1 ∈ S 2 corresponds to 2 on the left of 1, and the non-trivial permutation σ 1 ∈ S 2 corresponds to 1 on the left of 2. The sum of the lengths of the 4 permutations is denoted l ( σ ).

□

3.1.2. Naturality of the 2d quantum UV-IR map with respect to flips. Suppose τ and τ ′ are ideal triangulations of Σ that differ by a flip. Then, there is a transition map

<!-- formula-not-decoded -->

which is an algebra map defined on the algebra generators by

<!-- image -->

where i ∈ { 1 , 2 } ranges over the two sheets. 17

17 Here, we are using the fact that the gl 1 -splitting map is injective, so it suffices to define the transition map for the branched double cover of the ideal quadrilateral as above. The corresponding stated gl 1 -skein algebra is a quantum torus of rank 8; the first 8 corner tangles generate rank 7 part of it, and together with the last generator, they generate the whole quantum torus.

It is straightforward to check that the following diagram commutes:

<!-- image -->

and hence the 2d quantum UV-IR map is natural with respect to changes of triangulation.

Remark 3.4. The coordinate change map ψ τ → τ ′ can also be characterized as a conjugation by the quantum dilogarithm Ψ = 1 ( qx ; q 2 ) ∞ ; see [NY20, Sec. 8.3]. To be more precise, the branched double cover of a quadrilateral triangulated into two ideal triangles is an annulus, whose (non-stated) gl 1 -skein algebra is isomorphic to the polynomial ring R [ x ± 1 ], where x represents the longitude of this annulus. The quantum dilogarithm Ψ = 1 ( qx ; q 2 ) ∞ is an element of a completion of the gl 1 -skein algebra of the annulus, that solves the recurrence relation

<!-- formula-not-decoded -->

where ˆ y is the meridian so that ˆ y ˆ x = q 2 ˆ x ˆ y . While Ψ itself lives in the completion, conjugation by Ψ gives a well-defined map between ordinary (i.e., non-completed) skein algebras and, in fact, gives exactly the map described above: it is easy to see that conjugation by Ψ acts as the identity on the first 8 corner tangles, as those corner tangles commute with Ψ, while the last relation (22) is exactly the 3-term relation (23) satisfied by Ψ.

The pentagon identity for the transition maps ψ follows from that of the quantum dilogarithm; see, e.g., [KLN + 25, Sec. 2.1] for an illustration and its interpretation in terms of the unlinking moves on symmetric quivers.

3.2. 3d spectral networks. The crucial ingredient in the construction of the 2d quantum UV-IR map was the 1-dimensional foliation on ideally triangulated surfaces and the associated leaf space. Such combinatorial topological structures can be generalized to ideally triangulated 3-manifolds, as studied in [FN24], and are called 3d spectral networks . 18

Let Y be a 3-manifold equipped with a triangulation T into ideal tetrahedra. Then, there is a branched double cover ˜ Y = ˜ Y T of Y associated to the ideal triangulation, obtained by putting the branch locus given by the embedded 4-valent graph connecting the barycenter of each ideal tetrahedron with the barycenters of its 4 faces; see Figure 20. Note that on each face, the branching structure is exactly what we had for surfaces (Figure 17).

A new feature in this 3-dimensional setup is that, unlike the surface case we saw earlier, the branched double cover ˜ Y of Y is no longer a manifold; it is a pseudomanifold. Near the barycenters, the branched double cover is a cone over a torus with 4 branch points, as illustrated in Figure 21.

There is a 1-dimensional foliation on Y in the complement of the branch locus, which we will still call the WKB foliation , even though its connection to exact WKB analysis is less

18 To be more precise, the term spectral network usually refers to the stratification structure given by the union of singular leaves of the WKB foliation, but here we are using the term in a vague sense to refer to the whole combinatorial topological structure of the WKB foliations and the associated leaf space.

Figure 20. Branch locus (red) and branch cuts (orange) for an ideal tetrahedron

<!-- image -->

Figure 21. Cone over a torus with 4 branch points

<!-- image -->

clear. On each ideal tetrahedron, the WKB foliation is given by the cone over the WKB foliation of the boundary surface, which is S 2 with 4 punctures, triangulated into 4 ideal triangles; see Figure 22.

Figure 22. 3d WKB foliation

<!-- image -->

The corresponding leaf space looks exactly like the branch cuts shown in Figure 20 for each tetrahedron; it has 6 facets joined along 4 binders, and has a singular point in the middle where the 4 binders meet.

The singular leaves of the 3d WKB foliation are shown on the right side of Figure 23. As in the previous subsection, we always orient the WKB foliation so that the '1' (resp. '2')-direction is always pointing away from (resp. toward) the ideal vertices. This convention will determine our dictionary between the sheet labels (1 and 2) and the states (+ and -) when we extend the quantum UV-IR map to stated skein modules in Section 3.6.

3.3. gl 1 -skein modules with defects. We saw in the previous subsection that the branched double cover associated to an ideal trangulation of a 3-manifold has a cone point at the barycenter of each tetrahedron. The appropriate notion of the gl 1 -skein module for such a pseudomanifold turns out to be the one with an extra 3-term relation for each cone point,

Figure 23. 2d and 3d spectral networks

<!-- image -->

that depends on a choice of a generalized angle structure of the ideal triangulation. We describe this gl 1 -skein module in this subsection.

## 3.3.1. Generalized angle structures.

Definition 3.5. For an ideally triangulated 3-manifold Y , a generalized angle structure is an assignment of a real number 19 to each dihedral angle of the tetrahedra satisfying the following properties:

- (1) The numbers assigned to the opposite dihedral angles of each tetrahedron are the same; see Fig. 24 below.
- (2) The 3 numbers θ , θ ′ , and θ ′′ add up to π , for each tetrahedron.
- (3) For each internal edge of the ideal triangulation, the angles add up to 2 π .

Figure 24. Dihedral angles θ, θ ′ , θ ′′

<!-- image -->

We will denote a generalized angle structure by Θ = { θ i } i ∈ I , where I is some indexing set for the set of dihedral angles of the triangulation.

Remark 3.6. It is known [LT08, Theorem 1] that, if T is an ideal triangulation with t ideal tetrahedra of a compact 3-manifold Y with v boundary components, then T admits a generalized angle structure if and only if each boundary component of Y is either a torus or a Klein bottle; since we are dealing with oriented 3-manifolds, they have to be tori. Moreover, the space of generalized angle structures (when non-empty) is an affine space of dimension t + v .

For our purposes, what's important is that a choice of a generalized angle structure induces a flat connection on the tangent bundle of the leaf space. That is, once equipped with a

19 Can be thought of as formal commutative parameters satisfying certain linear relations.

generalized angle structure, each corner of the leaf space carries a definite angle; see Figure 25. In particular, because

Figure 25. Euclidean structure on each facet of the leaf space

<!-- image -->

<!-- formula-not-decoded -->

for any three facets of the leaf space of a tetrahedron around a fixed vertex, the sum of the three inner angles add up to 2 π . This, along with the condition that the angles around each edge of the triangulation add up to 2 π , means that the turning number is a smooth isotopy invariant of a closed curve drawn on the leaf space. Unlike the 2d case, however, the turning number on the leaf space of a 3d WKB foliation is some Z -linear combination of θ 2 π 's and 1 2 , so in general is not an integer.

- 3.3.2. gl 1 -skein module of branched double covers. Recall that the branched double cover ˜ Y has branch locus and cone points. Accordingly, there are additional skein relations for Sk gl 1 q ( ˜ Y ):

Definition 3.7. Let Y be an ideally triangulated 3-manifold with an associated branched double cover ˜ Y . Suppose that the ideal triangulation of Y is equipped with a generalized angle structure Θ = { θ i } i ∈ I . Let R Θ := Z [ q ± 1 ] ⊗ Z [ { q ± θ i π } i ∈ I ]. Then, the gl 1 -skein module Sk gl 1 q ( ˜ Y , Θ) of ˜ Y is the free R Θ -module spanned by the isotopy classes of framed oriented links in ˜ Y away from the branch locus, modulo (17)-(19), as well as the following extra relation near the cone points:

.

<!-- image -->

Note, the relations (17)-(19) are drawn in ˜ Y , while the last relation, (24), is drawn in the projection to Y . The meaning of these skein relations except for the last one should be clear; (17)-(18) are the usual relations (with a half twist relation (19)), and (20) says that when a strand of a link passes through the branch locus, the skein acquires a minus sign. The last skein relation (24) is a local relation near the singular cone point, viewed from a vertex of the ideal tetrahedron. It can be drawn 3-dimensionally as in Fig. 26. The labels 1 and 2 denote which sheet (in the complement of the branch cuts) that part of the skein belongs to.

Figure 26. The 3-term relation (24) near the cone point

<!-- image -->

Remark 3.8. When the angles are π, 0 , 0, the 3-term relation (24) is nothing but the recurrence relation for the quantum dilogarithm Ψ in Remark 3.4. This fact is used crucially in [ELPS25] to study a non-singular version of the quantum UV-IR map and give a geometric interpretation in terms of holomorphic curve counts.

Remark 3.9. The appearance of non-integer powers of q in (24) may seem strange, but there is an easy way to see why such factors are necessary for the skein module to be non-zero. Firstly, let's take a look at the following simple observation:

Lemma 3.10. Let T be a quantum torus defined as

<!-- formula-not-decoded -->

Let M be a cyclic left T -module defined by

<!-- formula-not-decoded -->

where I is the left ideal generated by the relations

<!-- formula-not-decoded -->

Then, the Weyl ordered product [ xx ′ x ′′ ] := q -1 xx ′ x ′′ = qx ′ xx ′′ acts on the cyclic vector of M by -q .

Proof. Let [ ∅ ] denote the cyclic vector of M . Then,

<!-- formula-not-decoded -->

□

On the other hand, in the gl 1 skein algebra of the torus with 4 branch points, the Weyl ordered product of the three natural cycles of T 2 is -1:

.

<!-- image -->

It follows that, if we call these three natural cycles

<!-- image -->

then the three-term recursion relations associated to the singular point cannot be the ones in Lemma 3.10. That is, there must be some factor of q in some of the relations. We also see immediately that some non-integer power of q is necessary to have the Z / 3 symmetry of the recursion relations.

3.4. 3d quantum UV-IR map. We are ready to state the 3d generalization of the quantum UV-IR map. The construction is essentially the same as the 2d quantum UV-IR map described in Section 3.1.1: For each framed oriented link L ⊂ Y representing an element [ L ] ∈ Sk gl 2 q ( Y ), which we assume to be in general position with respect to the 3d WKB foliation, we take the linear combination ∑ ˜ L c ˜ L [ ˜ L ] ∈ Sk gl 1 q ( ˜ Y ) of all possible lifts ˜ L that can be constructed out of direct lifts, detours and exchanges (using the 3d WKB foliation), where the coefficients c ˜ L are given by the product of exchange factors, turning factors, and framing factors, exactly as in Section 3.1.1. The new phenomenon in 3d is that the turning numbers t in the turning factors are no longer integers and depend on the choice of generalized angle structure. The claim is that the resulting 3d quantum UV-IR map is well-defined:

Theorem 3.11. Let Y be a 3-manifold equipped with an ideal triangulation and a generalized angle structure. Then, there is a well-defined R -module homomorphism

<!-- formula-not-decoded -->

Proof. We need to show that F : L ↦→ ∑ ˜ L c ˜ L [ ˜ L ] is invariant under the isotopy of L and that this map respects the gl 2 -skein relations. Any isotopy of L is a finite composition of elementary isotopies, each corresponding to a violation of a general position requirement of L with respect to the 3d WKB foliation. The elementary isotopies consist of

- (1) 3 types of framed Reidemeister moves,
- (2) 4 types of moves near the bindings,
- (3) and an extra move near the singular point.

The 3 framed Reidemeister moves and the 4 moves near the bindings are already present in the 2d case and were analyzed in detail in [NY20, Sec. 7], where it is shown that F is indeed invariant under those isotopies. The last move, which is new in 3d, is an elementary isotopy of a link diagram on the leaf space when it crosses the singular point (Figure 27). The only

Figure 27. An isotopy of a link diagram on the leaf space crossing the singular point

<!-- image -->

non-trivial case to check is when there are detours, in which case the images of the left-hand

side and the right-hand side of Figure 27 under F are given by

<!-- image -->

and the invariance is ensured exactly by the 3-term relation (24) near the cone point.

Finally, that F respects the gl 2 -skein relations was already shown in [NY20, Sec. 7] (see Theorem 3.3). □

Remark 3.12. The 3d quantum UV-IR map gives an intuitive explanation for why the 2d quantum UV-IR map gets conjugated by the quantum dilogarithm Ψ under a flip: A flip in an ideal triangulation can be thought of as a bordism given by attaching a taut ideal tetrahedron (i.e., the one with angles π, 0 , 0), and for such an ideal tetrahedron, the corresponding gl 1 -skein module of the branched double cover knows the 3-term relation (24) which is equivalent to an insertion of the quantum dilogarithm, after resolving the singular cone point.

Remark 3.13. Theorem 3.11 admits a natural generalization to a map between HOMFLYPT skein modules

<!-- formula-not-decoded -->

which specializes to the above map after setting a = q and z = q -q -1 . In [ELPS25], such a map is interpreted in terms of skein-valued counts of holomorphic curves (in the spirit of [ES25]) and is vastly generalized to branched covers coming from Lagrangians in T ∗ Y .

3.5. Naturality with respect to Pachner moves. Suppose that T 2 and T 3 are two ideal triangulations of Y related by a 2-3 Pachner move, and let ˜ Y 2 and ˜ Y 3 be the corresponding branched double covers. The leaf space of the WKB foliation corresponding to the ideal triangulation T 3 looks identical to that of T 2 outside of the bipyramid where we are performing the 2-3 Pachner move. Inside the bipyramid, the leaf space undergoes the transformation depicted in Figure 28; the two singular points collide and split into three singular points, which bound a newly created triangular facet.

Figure 28. Change of leaf space under the 2-3 Pachner move

<!-- image -->

Let Θ 2 be a generalized angle structure on T 2 , and let Θ 3 be a generalized angle structure on T 3 compatible with Θ 2 ; there is always a 1-dimensional space of such Θ 3 's. Explicitly, when viewed from the top of Figure 28, the angles of the facets of the leaf space are given as in Figure 29.

Figure 29. Compatible generalized angle structures under the 2-3 Pachner move. Here, η, η ′ , η ′′ are the dihedral angles of the bottom tetrahedron which are directly below θ, θ ′ , θ ′′ , respectively, and ζ is a free parameter. The seams of the leaf space are perpendicular to the boundary.

<!-- image -->

In this setup, there is a natural map

<!-- formula-not-decoded -->

between the two gl 1 -skein modules that can be constructed as follows. Let L be any framed oriented link in ˜ Y 2 . We can visualize it by its projection L to Y ; when put in a general position with respect to the leaf space (thought of as an embedded foam F 2 in Y ), the projection L ⊂ Y carries a sheet label i ∈ { 1 , 2 } in each component of L \ F 2 , and the sheet label flips every time L crosses the leaf space F 2 ⊂ Y . Since L ⊂ Y is disjoint from the branch locus, we can perform the 1-parameter deformation F 2 ⇝ F 3 of the leaf space as in Figure 28, while keeping it in general position with respect to L . As a result, we obtain a link L ⊂ Y in general position with respect to the new foam F 3 ⊂ Y , equipped with a sheet label i ∈ { 1 , 2 } for each component of L \ F 3 , which is nothing but a link in ˜ Y 3 . Let's call this resulting link L ′ ⊂ ˜ Y 3 .

## Proposition 3.14. The map

is well-defined.

Proof. We need to check that the map L ↦→ [ L ′ ] is invariant under isotopy of L and respects the gl 1 -skein relations on L . Invariance under isotopy (away from the branch locus) is trivial, as any isotopy of L ⊂ ˜ Y 2 corresponds to an isotopy of L (where we allow crossing changes between strands with different sheet labels), which induces an isotopy of L ′ ⊂ ˜ Y 3 . This map also straightforwardly respects most of the gl 1 -skein relations. The only non-trivial skein relations we need to check are:

- (1) The sign relation (20) on the branch locus connecting the two singular cone points of ˜ Y 2 .
- (2) The 3-term relations (24) for the two singular cone points of ˜ Y 2 .

<!-- formula-not-decoded -->

For the sign relations, let's say L is a meridian of the branch locus connecting the two singular cone points of ˜ Y 2 . We need to show that the image of L evaluates to -1 in Sk gl 1 q ( ˜ Y 3 ). It is enough to observe that the image of L is (modulo the usual gl 1 -skein relations) a union of three meridians around some branch loci in ˜ Y 3 , and thus evaluates to ( -1) 3 = -1; see Figure 30.

Figure 30. Checking the sign relation. Here, each strand is labeled by both sheets, so to get the corresponding links in ˜ Y 2 and ˜ Y 3 , take the preimage under the projection.

<!-- image -->

Now, it suffices to show that ϕ 2 → 3 respects the following relative version of the 3-term relation

<!-- image -->

which is equivalent to the closed version of the 3-term relations (24). Using the angles as in Figure 29, this can be directly verified as follows:

<!-- image -->











<!-- image -->

where, in the third to last equality, we changed the framing of the last tangle by +1, hence absorbing the factor of q . □

Remark 3.15. Proposition 3.14 is the singular analog of the pentagon relation for the quantum dilogarithm. In fact, when the angle structure is taut, we can resolve the singular cone points in a certain way and insert the quantum dilogarithm - which satisfies the same 3-term recurrence relation - at each of those resolved cone points. Then, Proposition 3.14 becomes exactly the well-known pentagon relation for the quantum dilogarithm.

Theorem 3.16. The quantum UV-IR map is natural with respect to the transition maps ϕ 2 → 3 . That is, the following diagram commutes:

<!-- image -->

Proof. The deformation of the leaf space (Figure 28) preserves the Euclidean structures on the facets of the leaf space, so this is immediate from the construction of the quantum UV-IR map. □

3.6. Stated quantum UV-IR map. So far, we have reviewed the construction of the quantum trace map (Section 2) and the quantum UV-IR map (Section 3.1-3.5). While they are both maps that 'abelianize' the skein modules in a sense, their constructions were quite different - the quantum trace map is defined mostly algebraically, while the quantum UV-IR

map is defined topologically, crucially using the 1-dimensional foliation induced by a choice of ideal triangulation.

In this subsection, we bring the quantum UV-IR map closer to the quantum trace map by constructing a stated version of the quantum UV-IR map

<!-- formula-not-decoded -->

Eventually, this will allow us to use the cut-and-glue approach to check the compatibility between the two maps locally.

The gl 2 -skeins and gl 1 -skeins with defects admit natural extensions to stated tangles; the precise definition of the stated gl 2 -skein modules and the stated gl 1 -skein modules with defects that we use are given in Appendix A and B, respectively, where we also show that these stated skein modules behave nicely under splitting of 3-manifolds, analogously to stated sl 2 -modules. Since the construction of the quantum UV-IR map was local, in order to extend it to a map between stated skein modules, we simply need to carefully determine the dictionary between the boundary states of stated tangles and the boundary condition for lifts (i.e., which of the two sheets it needs to be lifted to).

Before stating the rules for determining the boundary condition, let's briefly recall (from Section 3.1-3.2) our conventions on the sheet labels 1 and 2. Given an ideal triangulation and the associated WKB foliation, we orient the foliation so that, in the complement of the branch cut,

- the 'sheet 2'-direction is always pointing toward the ideal vertices, and
- the 'sheet 1'-direction is always pointing away from the ideal vertices.

With this orientation convention in mind, here's the rule determining the boundary condition for the lifts of stated tangles:

Definition 3.17 (Dictionary between boundary states and sheets) . Depending on the boundary state and orientation of the tangle, we impose the following boundary conditions on the lifts of stated tangles:

,

.

<!-- image -->

˜

˜

˜

˜

Here, ˜ e ij ( ij ∈ { 12 , 21 } ) denotes the lift of the boundary marking e to ˜ Y , for which the sheet labels are i and j on the left and the right of ˜ e ij , respectively, if ˜ e ij is pointing upward and viewed from outside of ˜ Y .

Theorem 3.18. With the boundary conditions given as in Definition 3.17, we get a welldefined quantum UV-IR map

<!-- formula-not-decoded -->

between the stated skein modules, which factors through the reduced gl 2 -skein module

<!-- formula-not-decoded -->

Proof. The proof of invariance under isotopy and that it respects the interior (i.e., non-stated) gl 2 -skein relations (12)-(16) is the same as in the proof of Theorem 3.11. Thus, we need to check that the quantum UV-IR map respects the boundary (i.e., stated) gl 2 -skein relations (45)-(48).

- The first boundary skein relation (45):

<!-- formula-not-decoded -->

- The second boundary skein relation (46):

<!-- formula-not-decoded -->

- The third boundary skein relation (47):

<!-- formula-not-decoded -->

<!-- image -->

- The fourth boundary skein relation (48):

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

For other skein relations obtained by simultaneous orientation reversal of the tangles, the proof is essentially identical.

Finally, it is easy to see that the bad arcs map to 0, and hence F descends to a map from the reduced gl 2 -skein module. □

Recall (e.g. from Proposition 2.14) that the stated skein modules carry natural bimodule structures; specifically, Sk gl 2 q ( Y, Γ) is a SkAlg gl 2 q ( V (Γ) + )-SkAlg gl 2 q ( V (Γ) -)-bimodule, and Sk gl 1 q ( ˜ Y , ˜ Γ) is a SkAlg gl 1 q ( V ( ˜ Γ) + )-SkAlg gl 1 q ( V ( ˜ Γ) -)-bimodule. Since the quantum UV-IR map F : Sk gl 2 q ( Y, Γ) → Sk gl 1 q ( ˜ Y , ˜ Γ) is defined locally, it is in fact a bimodule map when Sk gl 1 q ( ˜ Y , ˜ Γ) is regarded as a SkAlg gl 2 q ( V (Γ) + )-SkAlg gl 2 q ( V (Γ) -)-bimodule. This bimodule structure is induced from the algebra homomorphisms

<!-- formula-not-decoded -->

determined by the quantum UV-IR map computed near the vertices of the boundary markings.

Example 3.19. The algebra homomorphism

<!-- formula-not-decoded -->

depends in general on the angles at the vertices. Here, we describe this map explicitly in the case of vertices of degree 2 and 3. Around those vertices, the leaf space is locally modeled by the angled biangular and triangular prism depicted in Figure 31 and 32.

Figure 31. An angled biangular prism ( D 2 × I ) θ (left) and the associated leaf space (right)

<!-- image -->

Figure 32. An angled triangular prism ( D 3 × I ) θ a ,θ b ,θ c (left) and the associated leaf space (right)

<!-- image -->

Direct calculation shows that the quantum UV-IR map on the angled biangular prism ( D 2 × I ) θ is the algebra homomorphism given by 20 21

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Likewise, the quantum UV-IR map on the angled triangular prism ( D 3 × I ) θ a ,θ b ,θ c is the algebra homomorphism given by 22

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

20 Here, we used a notational convention where -- → x µ y ν ( µ, ν ∈ {±} ) denotes the elementary stated gl 2 tangle connecting the boundary markings x and y - with boundary states µ and ν , respectively - in the simplest possible way, oriented from x to y . We use similar notations for elementary gl 1 tangles, with x ij ( ij ∈ { 12 , 21 } ) denoting the lift of the boundary marking x to the double cover, where the sheet labels are i and j on the left and right of x ij , when viewed from outside of the 3-manifold and if x ij is oriented upward; we have already used a similar notational convention in Definition 3.17.

21 These elementary tangles generate the reduced gl 2 -skein algebra of the biangle; see Lemma 5.2.

22 These elementary tangles (and their inverses) generate the reduced gl 2 -skein algebra of the triangle, as shown in Lemma 4.17.

and

<!-- formula-not-decoded -->

Proposition 3.20. Considering Sk gl 1 q ( ˜ Y , ˜ Γ) as a SkAlg gl 2 q ( V (Γ) + ) -SkAlg gl 2 q ( V (Γ) -) -bimodule, with the bimodule structure induced from the algebra homomorphism

<!-- formula-not-decoded -->

the quantum UV-IR map

<!-- formula-not-decoded -->

is a bimodule homomorphism.

Proof. This is immediate from the locality of the quantum UV-IR map.

□

It also follows immediately from the fact that the quantum UV-IR map is constructed locally that it is compatible with the splitting maps on both sides:

Proposition 3.21. The quantum UV-IR map is compatible with the splitting maps on both sides. That is, if Y is obtained by gluing Y i 's with compatible branched covers and generalized angle structures, we have a commuting square

<!-- formula-not-decoded -->

.

Here, ⊗ i Sk gl 1 q ( ˜ Y i ) denotes the relative tensor product of the stated gl 1 -skein modules Sk gl 1 q ( ˜ Y i ), i.e., the quotient of the ordinary tensor product ⊗ i Sk gl 1 q ( ˜ Y i ) by the gluing relations, as well as extra 3-term relations for each cone point of ˜ Y ; see Appendix B.

## 4. Compatibility for surfaces

Now that we have carefully reviewed both the quantum trace map and the quantum UV-IR map (both in 2d and 3d), we are ready to compare them; we will do this for surfaces in this section, and for 3-manifolds in Section 5.

Let Σ be a surface without boundary (but with punctures), and let ˜ Σ = ˜ Σ τ be its branched double cover associated to some ideal triangulation τ of Σ. In this section, we will construct algebra homomorphisms

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

which fit into the following commutative square of algebra maps:

<!-- formula-not-decoded -->

.

We will do so by first constructing a similar commutative square for ideal triangles and then show that they glue consistently. For surfaces with boundary (such as ideal triangles), the top left and right corner of such commutative diagrams will carry a product twisted by a sign that we describe below.

- 4.1. From gl 2 to sl 2 . For each stated oriented tangle ⃗ L in Y , we will define a quantity b ( ⃗ L ) ∈ 1 2 Z / 2 Z that depends only on the boundary behavior of ⃗ L (hence b for boundary). This will play a crucial role in the construction of the left vertical map π in the commutative square. We will also use it to twist the product structures on SkAlg gl 2 q (Σ) and on SkAlg gl 1 q ( ˜ Σ).

Definition 4.1. Let ⃗ L be a stated oriented tangle in a boundary marked 3-manifold ( Y, Γ). Define b ( ⃗ L ) ∈ 1 2 Z / 2 Z to be

<!-- formula-not-decoded -->

where the second sum is over all (unordered) pairs { p 1 , p 2 } of boundary points of ⃗ L on the edge e of the boundary marking Γ, and b ( { p 1 , p 2 } ) ∈ {± 1 2 , 0 } is determined by the following rules:

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

and all other contributions are zero. 23

Now, we are ready to construct the natural ' gl 2 -sl 2 map' from the gl 2 -skeins to the tensor product of sl 2 and gl 1 -skeins.

## Proposition 4.2. The map

<!-- formula-not-decoded -->

is a well-defined R -module homomorphism.

Proof. We need to check that this map respects all the stated gl 2 -skein relations, i.e., the relations (12)-(16) and the boundary relations (45)-(48).

- The first skein relation (12):

<!-- image -->

<!-- formula-not-decoded -->

- The second skein relation (13):

<!-- formula-not-decoded -->

- The third skein relation (14):

<!-- formula-not-decoded -->

· The fourth skein relation (15): Using

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

- The fifth skein relation (16):

<!-- formula-not-decoded -->

- The first boundary skein relation (45):

<!-- image -->

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

- The second boundary skein relation (46):

<!-- formula-not-decoded -->

- The third boundary skein relations (47):

<!-- formula-not-decoded -->

<!-- image -->

- The fourth boundary skein relations (48):

<!-- image -->

<!-- image -->

For the other skein relations obtained by simultaneous orientation reversal of the tangles, the proof is identical. □

4.2. Sign-twisted products. Because of the extra factor b ( ⃗ L ), the gl 2 -sl 2 map

<!-- formula-not-decoded -->

is not an algebra homomorphism. We would like to fix this by modifying the product on SkAlg gl 2 q (Σ) accordingly.

The quantity b ( ⃗ L ) itself unfortunately does not behave nicely with respect to the gl 2 skein relations (i.e., it does not give a grading on the gl 2 skein module). However, the relative version of b that we define below behaves well.

Definition 4.3. Given two stated oriented tangles ⃗ L 1 and ⃗ L 2 in Σ × I , define

<!-- formula-not-decoded -->

where ⃗ L 1 · ⃗ L 2 denotes the stated oriented tangle in Σ × I obtained by stacking ⃗ L 1 above ⃗ L 2 as usual.

It is useful to note that b ( ⃗ L 1 , ⃗ L 2 ) = -b ( ⃗ L 2 , ⃗ L 1 ).

Proposition 4.4. The quantity b ( ⃗ L 1 , ⃗ L 2 ) is invariant under stated gl 2 -skein relations applied to ⃗ L 1 or to ⃗ L 2 . Therefore, for any fixed ⃗ L ,

<!-- formula-not-decoded -->

gives a grading on Sk gl 2 q (Σ × I ) .

Proof. The only thing we need to check is the invariance under the stated skein relations (14) and (45) which creates (or annihilates) a pair of boundary points, which is immediate from our definition of b ( ⃗ L ). □

Definition 4.5. Define the sign-twisted gl 2 -skein algebra SkAlg gl 2 , st q (Σ) to be the usual gl 2 -skein module Sk gl 2 q (Σ × I ), but with the product structure twisted by a sign determined by ( -1) b ( · , · ) . That is,

<!-- formula-not-decoded -->

This gives a well-defined associative product, as b ( · , · ) satisfies the following obvious cocycle condition:

<!-- formula-not-decoded -->

Likewise, define the sign-twisted gl 2 -skein module Sk gl 2 , st q ( Y, Γ) to be the usual gl 2 -skein module Sk gl 2 q ( Y, Γ), but with the sign-twisted bimodule structure, i.e., considered as a ⊗ v ∈ V (Γ) + SkAlg gl 2 , st q ( D deg v )-⊗ w ∈ V (Γ) -SkAlg gl 2 , st q ( D deg w )-bimodule.

Remark 4.6. Since our sign-twisting only changes the product by a sign, bad arcs generate the same ideal as before, and it follows that the sign-twisting descends well to reduced skein modules.

Proposition 4.7. The map

<!-- formula-not-decoded -->

is an algebra homomorphism.

Proof. This is immediate from the definition of the sign-twisted product:

<!-- formula-not-decoded -->

□

From the above, it follows that both Sk gl 2 , st q ( Y ) and Sk sl 2 A ( Y ) ⊗ Sk gl 1 -A ( Y ) can be considered as ⊗ v ∈ V (Γ) + SkAlg gl 2 , st q ( D deg v )-⊗ w ∈ V (Γ) -SkAlg gl 2 , st q ( D deg w )-bimodules, where the bimodule structure on the latter is defined using π , and Γ is the boundary marking for Y . The following proposition is clear:

Proposition 4.8. The map

<!-- formula-not-decoded -->

is a bimodule homomorphism.

We can define the sign-twisted gl 1 -skein algebra of ˜ Σ in exactly the same way, using the dictionary translating between the states and the lift described in Section 3.6. That is, for any tangle ⃗ L in ˜ Y whose boundary points are in general position so that no two boundary points project down to the same point in Y , we define b ( ⃗ L ) ∈ 1 2 Z / 2 Z to be

<!-- formula-not-decoded -->

where the second sum is over all pairs ( p 1 , p 2 ) of boundary points of ⃗ L which lie in different lifts ˜ e 12 and ˜ e 21 of the same edge e of the boundary marking Γ of Y , and b ( p 1 , p 2 ) ∈ {± 1 2 } is determined by the following rules:

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Then, given two oriented tangles ⃗ L 1 and ⃗ L 2 in ˜ Σ × I , define

<!-- formula-not-decoded -->

where ⃗ L 1 · ⃗ L 2 denotes the oriented tangle in ˜ Σ × I obtained by stacking ⃗ L 1 above ⃗ L 2 as usual. Analogous to Proposition 4.4, we have the following:

Proposition 4.9. The quantity b ( ⃗ L 1 , ⃗ L 2 ) is invariant under gl 1 -skein relations or isotopies that exchange the height of some pair of boundary points p 1 ∈ ˜ e 12 and p 2 ∈ ˜ e 21 , applied to ⃗ L 1 or to ⃗ L 2 . Therefore, for any fixed ⃗ L in ˜ Σ × I ,

<!-- formula-not-decoded -->

gives a grading on Sk gl 1 q ( ˜ Σ × I ) .

Definition 4.10. The sign-twisted gl 1 -skein algebra SkAlg gl 1 , st q ( ˜ Σ) of ˜ Σ is defined to be the usual gl 1 -skein module Sk gl 1 q ( ˜ Σ × I ), but with the product structure twisted by a sign determined by ( -1) b ( · , · ) . That is,

<!-- formula-not-decoded -->

Since we are using the same sign-twisting for both gl 2 -skeins and gl 1 -skeins, the following is clear.

Proposition 4.11. The quantum UV-IR map is still a bimodule homomorphism after signtwisting. That is,

<!-- formula-not-decoded -->

is still an algebra homomorphism, and

<!-- formula-not-decoded -->

is a bimodule homomorphism.

The gl 2 -sl 2 map π also behaves nicely with respect to reduced stated skein modules, as well as to 3-manifolds split into face suspensions.

## Proposition 4.12. The map

<!-- image -->

is well-defined.

Proof. We need to show that the composition

<!-- image -->

factors through ⊗ f ∈T (2) Sk gl 2 , st q ( Sf ). This can be seen immediately by comparing the gluing relations in ⊗ f ∈T (2) Sk gl 2 , st q ( Sf ) to those in ⊗ f ∈T (2) Sk sl 2 A ( Sf ) ⊗ ⊗ f ∈T (2) Sk gl 1 -A ( Sf ). □

Under splitting, the factors ( -1) b ( ⃗ L ) come in pairs and cancel themselves out. Hence, the following is immediate:

Proposition 4.13. The gl 2 -sl 2 map is compatible with the splitting maps on both sides. That is, if Y is obtained by gluing Y i 's, we have a commuting square

<!-- formula-not-decoded -->

.

- 4.3. Commutative square for a triangle. Before defining the evaluation map for ˜ Σ, we will do so first for the triangle. Let's first recall some basic facts about skein algebras of the triangle and its branched double cover.
- Lemma 4.14. The gl 1 -skein algebra SkAlg gl 1 -A ( △ ) of the triangle △ - with vertices labeled by α, β, γ in a counterclockwise manner and the opposite edges by a, b, c - is the quantum torus generated by

<!-- image -->

,

and their inverses, with relations

<!-- formula-not-decoded -->

Lemma 4.15. The sign-twisted gl 1 -skein algebra SkAlg gl 1 , st q ( ˜ △ ) of the branched double cover ˜ △ of the triangle is the quantum torus generated by

<!-- image -->

and their inverses, with commutation relations

<!-- formula-not-decoded -->

All other pairs of generators commute, modulo one relation:

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Remark 4.16. The -1 in the last relation comes from the evaluation of a small unknot around the branch point. The sign-twisting does not change the sign of the Weyl-ordered product in this case.

Lemma 4.17. The sign-twisted reduced gl 2 -skein algebra SkAlg gl 2 , st q ( △ ) of the triangle △ is generated by and their inverses, where

<!-- image -->

and likewise for β and γ .

<!-- formula-not-decoded -->

Proof. It is straightforward to verify that

<!-- formula-not-decoded -->

in addition to the relations obtained from those above by cyclic permutations of α, β, and γ . Let ⃗ L be any gl 2 -stated tangle in △× I representing an element [ ⃗ L ] ∈ SkAlg gl 2 , st q ( △ ). Consider its projection to the leaf space of △× I . On each facet of the leaf space, we can 'push' the boundary marking closer to the binder while absorbing the crossings, cups, and caps, using the gl 2 -stated skein relations. An example of this process, carried out on one facet of the leaf space, is shown below:

.

<!-- image -->

Once the boundary marking is pushed sufficiently close to the binder, we obtain an expression of [ ⃗ L ] as a linear combination of link diagrams that - after a minor isotopy - are products of stated tangles at constant heights. Such diagrams can be written as a word in - → α µν , - → β µν , - → γ µν , ← -α µν , ← -β µν , and ← -γ µν , which can be written in terms of the ++ generators using the relations in (28).

□

Lemma 4.18. The quantum UV-IR map on a triangle

<!-- formula-not-decoded -->

is surjective.

Proof. Since it is an algebra homomorphism, it is enough to observe that all the generators of SkAlg gl 1 , st q ( ˜ △ ) are in the image of F △ :

<!-- formula-not-decoded -->

Corollary 4.19. If there exists a linear map

<!-- formula-not-decoded -->

□

making the following square

<!-- formula-not-decoded -->

commutative, then it is unique, and it must be an algebra homomorphism.

Proof. This follows immediately from the surjectivity of F (Lemma 4.18) and the fact that the other 3 arrows in the diagram are algebra homomorphisms. □

Below, we show that the evaluation map ev indeed exists:

Theorem 4.20. There is an algebra homomorphism

<!-- formula-not-decoded -->

defined on the generators by and

Indeed,

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

which makes the square (29) commutative.

Proof. Choosing the preimages of the generators as in Lemma 4.18, we get

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Images of the remaining generators can be computed in the same way. Now, it suffices to check that this map respects the relation (27):

<!-- formula-not-decoded -->

□

- 4.4. Gluing the commutative squares. To finish the construction of the desired commutative square (25), we simply need to 'glue' together the commutative squares for triangles (29) built in Theorem 4.20. That is, given an ideally triangulated surface Σ without boundary, we have the following commutative diagram (commutativity follows from commutativity of the 3 squares):

<!-- image -->

,

where σ sl 2 , σ gl 2 , σ gl 1 are the splitting homomorphisms for sl 2 , gl 2 , and gl 1 -skein algebras, respectively.

Moreover, note that the image of Tr τ ⊗ σ gl 1 (i.e., the bottom arrow) is contained in the subalgebra

<!-- formula-not-decoded -->

where ( ⊗ △∈ τ (2) SkAlg gl 1 -A ( △ ) ) 0 denotes the 0-graded part of ⊗ △∈ τ (2) SkAlg gl 1 -A ( △ ), with respect to the Z τ (1) -grading given by the number of end points on each edge e ∈ τ (1) , counted with sign by orientation. The same is true for the image of ( ⊗ △∈ τ (2) ev △ ) ◦ σ gl 1 .

A useful fact is that:

## Lemma 4.21. The splitting map

<!-- formula-not-decoded -->

is an isomorphism of R -algebras.

Proof. Each copy of SkAlg gl 1 -A ( △ ) has a basis labeled by { [ L ⃗ n △ ] | ⃗ n △ ∈ Z 3 , n △ , 1 + n △ , 2 + n △ , 3 = 0 } , where L ⃗ n △ denotes the distinguished gl 1 -web in △× I with boundary condition ⃗ n △ ; see Figure 33. Likewise, ( ⊗ △∈ τ (2) SkAlg gl 1 -A ( △ ) ) 0 has a basis given by the form ⊗ △∈ τ (2) [ L ⃗ n △ ] with matching boundary conditions along each e ∈ τ (1) (i.e., the sum of the two n 's must vanish on each edge e ). Since the gl 1 -webs with matching boundary conditions can be glued,

Figure 33. Left: the gl 1 -web L n 1 ,n 2 ,n 3 ; Right: an example, L 1 , 2 , -3 , is shown, which should be understood as the Weyl-ordered product of the shown tangles.

<!-- image -->

the splitting map is surjective. Also, such gluing gives the inverse of the splitting map, showing that the splitting map is injective. □

As a result, we can replace the bottom right corner of the commutative square by SQTS τ (Σ) ⊗ SkAlg gl 1 -A (Σ) to obtain:

Theorem 4.22 (Compatibility theorem for surfaces) . The 2d quantum trace map Tr τ is compatible with the 2d quantum UV-IR map F τ in the sense that they fit into the commutative square

<!-- formula-not-decoded -->

where π is the gl 2 -sl 2 map (Proposition 4.7), and ev is the composition ( ⊗ △∈ τ (2) ev △ ) ◦ σ gl 1 .

- 4.5. Proof of Neitzke-Yan conjecture. A version of the above compatibility was conjectured earlier by Neitzke and Yan. To state their conjecture precisely, let Γ be the lattice H 1 ( ˜ Σ; Z ) with the standard intersection pairing Γ × Γ → Z , and consider 3 sublattices 2Γ, Γ odd , Γ even , where

<!-- formula-not-decoded -->

with σ being the Z / 2-deck transformation action on Γ. Each of those sublattices has an induced intersection pairing, so we can consider the corresponding quantum tori Q 2Γ , Q Γ odd , and Q Γ even (as in Definition 2.9) based on those lattices. As explained in [NY20, Sec. 3.5], there is an algebra isomorphism

<!-- formula-not-decoded -->

where n ( ⃗ L ) denotes the number of non-local crossings (i.e., crossings in the projection to Σ which do not come from a crossing on ˜ Σ), and ∗ denotes the twist of the product structure by a sign given by the mod 2 intersection number between the two links in the projection to Σ. 24 By twisting the product structures on SkAlg gl 2 q (Σ) and SkAlg gl 1 -A (Σ) in the same way (i.e., by a sign given by the mod 2 intersection number), the quantum UV-IR map F and the gl 2 -sl 2 map remain algebra homomorphisms.

24 Here, the factor ( -1) n ( ⃗ L ) ensures that the map is invariant under an isotopy of ⃗ L across a branch point, and the twist in the product structure is due to the fact that the monomials in Q 2Γ are A 2 = -q -commuting, instead of q -commuting.

Conjecture 4.23 ([NY20, Sec. 9.2]) . There is a commutative diagram

<!-- formula-not-decoded -->

where

· ρ is an algebra homomorphism given by

<!-- formula-not-decoded -->

· F odd is an algebra homomorphisms given by 25

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

where ⃗ L is L with an arbitrary choice of orientation, w ( ⃗ L ) is the writhe, and ρ odd : Q 2Γ → Q Γ odd is the R -linear map x 2 γ ↦→ x γ -σ ( γ ) , and · F even is an algebra homomorphism given by

<!-- formula-not-decoded -->

with p : ˜ Σ → Σ being the projection.

Morever, F odd coincides with the Bonahon-Wong quantum trace map.

Theorem 4.24. The Neitzke-Yan conjecture 4.23 is true.

Proof. Thanks to Theorem 4.22, it suffices to construct an algebra embedding

<!-- formula-not-decoded -->

such that

<!-- formula-not-decoded -->

where i even = F even , and

<!-- formula-not-decoded -->

For each square-root quantized shear parameter ˆ x e , set

<!-- formula-not-decoded -->

where γ e ∈ Γ odd denotes the 1-cycle in ˜ Σ τ depicted below:

<!-- image -->

From the commutation relations, we see that this extends to an algebra homomorphism i odd : SQTS τ (Σ) → Q Γ odd . It is easy to see that each monomial in SQTS τ (Σ) gets sent to a different lattice point in Γ odd (e.g., by looking at the intersection numbers with ideal arcs in ˜ Σ), so i odd is indeed an embedding.

It remains to show equations (36) and (37). Just like how we constructed the compatibility map, we follow the strategy of cutting and gluing. To that end, we first show the following lemma which extends the isomorphism between gl 1 -skeins and quantum tori (31) to surfaces with boundary:

Lemma 4.25. Let Σ be a punctured, bordered surface with ideal triangulation τ , and let ˜ Σ τ be the corresponding branched double cover. Set Γ := H 1 ( ˜ Σ τ , ∂ ˜ Σ τ ) , equipped with the intersection pairing Γ × Γ → 1 2 Z . Then, there is an algebra isomorphism

<!-- formula-not-decoded -->

where n ( ⃗ L ) , w ( ⃗ L ) ∈ 1 2 Z are the non-local writhe and the writhe of ⃗ L , respectively, w ( p ( ⃗ L )) = n ( ⃗ L ) + w ( ⃗ L ) ∈ 1 2 Z is the writhe of the projection p ( ⃗ L ) of ⃗ L to Σ , and the product structure on SkAlg gl 1 , st q ( ˜ Σ τ ) is - on top of the sign-twisting ( -1) -b ( ⃗ L 1 , ⃗ L 2 ) - further twisted by ( -1) -w ( p ( ⃗ L 1 ) ,p ( ⃗ L 2 )) , where w ( p ( ⃗ L 1 ) , p ( ⃗ L 2 )) := w ( p ( ⃗ L 1 · ⃗ L 2 )) -w ( p ( ⃗ L 1 )) -w ( p ( ⃗ L 2 )) ∈ 1 2 Z . 26

Proof. Firstly, this map is well-defined because it respects the ordinary gl 1 -skein relations thanks to the factor q w ( ⃗ L ) - and the relation for sign defects - thanks to the factor ( -1) n ( ⃗ L ) . This is an algebra map because, for any two tangles ⃗ L 1 and ⃗ L 2 flat on ˜ Σ τ so that w ( ⃗ L 1 ) = w ( ⃗ L 2 ) = 0,

<!-- formula-not-decoded -->

Finally, both the domain and codomain of this map are graded by H 1 ( ˜ Σ τ , ∂ ˜ Σ τ ), with each graded piece being isomorphic to the base ring R . It is easy to see that ι respects this grading and is an isomorphism in each graded piece. Therefore, ι is an algebra isomorphism. □

Now, back to the case where Σ is an ideally triangulated surface without boundary. For each ideal triangle △ , we have:

<!-- image -->

where Γ ˜ △ := H 1 ( ˜ △ , ∂ ˜ △ ) is a lattice of rank 5, and i odd and i even are algebra maps (in fact, isomorphisms) between rank 3 and 2 quantum tori, respectively, defined by for each generator e ∈ { a, b, c } of ˜ T , and

<!-- image -->

and similarly for the remaining generators β, γ of SkAlg gl 1 , ∗ -A ( △ ). 27 A straightforward computation on each generator of SkAlg gl 1 , st , ∗ q ( ˜ △ ) shows that the composition ( i odd ⊗ i even ) ◦ ev is indeed given by ρ ◦ ι where ρ is as in (33) but for Γ = Γ ˜ △ .

Now, what remains is a simple matter of gluing these commutative squares back together to get the desired commutative diagram (32). The splitting of Σ into ideal triangles induces the corresponding splitting map on the quantum tori

<!-- formula-not-decoded -->

and analogous maps for Q Γ odd and Q Γ even , which are isomorphisms onto the degree-0 subalgebra (i.e., the subalgebra generated by elements of the form ⊗ △∈ τ (2) x γ △ such that ∂γ △ | e + ∂γ △ ′ | e = 0 for each edge e , if △ and △ ′ are the two triangles sharing the edge e ). From the construction, it is clear that, after gluing, i odd becomes the map described earlier in (38). That (36) holds follows directly, since we have already checked it locally. That i even = F even is indeed given by the formula (35) is straightforward: for links flat on Σ, this is evident from our local definition of i even , and the prefactor ( -A ) w ( ⃗ L ) is uniquely determined in order for the map to be well-defined. Finally, that (37) holds, or equivalently, that i odd ◦ Tr is indeed given by the formula (34) is an immediate corollary of the commutative diagram: for any flat link L on Σ, we have

<!-- formula-not-decoded -->

The prefactor ( -A ) -w ( ⃗ L ) is uniquely determined for the map to be well-defined. □

27 Note, these generators α, β, γ , which were originally -A -commuting, become A -commuting after the twist in the product.

4.6. Naturality with respect to flips. Here, we show that the commutative squares constructed in Theorem 4.22 are natural with respect to change of triangulation of the surface.

Theorem 4.26. Under a change of triangulation τ → τ ′ , we have the following commutative diagram

<!-- image -->

where θ τ → τ ′ are the transition maps of the square-root quantum Teichm¨ uller space discussed in Section 2.1.3, and ψ τ → τ ′ are the transition maps for the gl 1 -skein algebras of the branched double covers discussed in Section 3.1.2.

Proof. In the triangular-prism-shaped diagram above, it suffices to check that the front right face commutes, as we already know that all the other faces commute. Thanks to the local nature of our construction, it suffices to check this just for the ideal quadrilateral where the flip happens. Since the stated quantum UV-IR map F τ is surjective for the ideal quadrilateral, the commutativity of the front right face follows from commutativity of the other faces. □

Remark 4.27. That the evaluation map ev is natural with respect to coordinate transformation maps ψ τ → τ ′ and θ τ → τ ′ , i.e. that the following diagram commutes

<!-- image -->

,

can also be checked explicitly. Since every arrow is an algebra map, it suffices to check the commutativity for the generators of the top left corner of this diagram, which is a rank 8 quantum torus. 28 There are 8 corner tangles (4 corners × 2 sheets) spanning the rank 7 part

28 Rank 8 because that's the rank of the relative first homology group of the branched double cover of the ideally triangulated quadrilateral, which is an annulus with 4 punctures on each of the two S 1 boundary components.

of the quantum torus. For each of them, we get a simple commutative diagram like

<!-- image -->

,

where we are labeling the edges of the ideal triangulations as in Figure 7. For the remaining 1 generator of the quantum torus, we have

5. Compatibility for 3-manifolds

<!-- image -->

In this section, we extend the analysis of the previous section to 3-manifolds. That is, given a (non-compact) 3-manifold Y without boundary, with an ideal triangulation T , we will construct a commutative square

<!-- formula-not-decoded -->

.

Compared to the surface cases, the main difference is that there are extra gluing relations (in the 'relative tensor products') coming from the splitting maps, and we have to check that commutative squares for little pieces (i.e., face suspensions) glue well (i.e., respects the gluing relations).

- 5.1. Commutative square for a face suspension. In order to define the evaluation map of a face suspension, we first need to establish the structure of various skein algebras of the biangle. The following lemmas are easy to show:

Lemma 5.1. The gl 1 -skein algebra SkAlg gl 1 q ( D 2 ) is isomorphic to the ring of Laurent polynomials in 1 -variable,

<!-- formula-not-decoded -->

<!-- image -->

.

with

Furthermore, the sign-twisted gl 1 -skein algebra of the double cover ˜ D 2 is

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

That is, in this case, the sign-twisted gl 1 -skein algebra is isomorphic to the untwisted one.

Lemma 5.2. The sign-twisted reduced gl 2 -skein algebra SkAlg gl 2 , st q ( D 2 ) of the biangle D 2 is generated by

<!-- formula-not-decoded -->

with and their inverses, where

<!-- formula-not-decoded -->

These generators satisfy and

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Here, we have used a notational convention for stated tangles similar to the one used in Example 3.19. For the rest of the paper, we adopt similar notation for the generators of the various skein algebras we consider.

To describe the structure of gl 1 and gl 2 stated skein modules of face suspensions, first recall the following definition:

Definition 5.3 ([PPar, Def. 3.17]) . A combinatorial foliation of a bordered, punctured surface Σ is a decomposition of Σ into pieces, each of which is topologically an elementary quadrilateral depicted below:

<!-- image -->

That is, each piece is a quadrilateral with a diagonal marking and two vertices removed.

The boundary of a face suspension, as well as its double cover, admit a combinatorial foliation. The corresponding boundary marking determines a tessellation of the boundary into polygonal faces, one for each boundary puncture.

The following theorems follow from an argument almost identical to that in [PPar, Sec. 4.1].

Theorem 5.4. Let B be a 3 -ball whose boundary is combinatorially foliated, and let Γ ⊂ ∂B be the associated boundary marking. Then, the stated gl 2 skein module of B has the following presentation:

<!-- formula-not-decoded -->

where Ann([ ∅ ]) is the left ideal generated by the following relations (as well as simultaneous orientation reversal of all of the tangles) for each face of the tessellation of ∂B : 29

<!-- image -->

A similar statement holds for gl 1 skein modules of 3-balls:

Theorem 5.5. In the setup of Theorem 5.4, the stated gl 1 skein module of B has the following presentation:

<!-- formula-not-decoded -->

where Ann([ ∅ ]) is the left ideal generated by the following relations (as well as simultaneous reversal of all of the tangles) for each face of the tessellation of ∂B :

<!-- image -->

From Theorems 5.4 and 5.5, we immediately obtain the following corollaries on the structure of stated skein modules of face suspensions. For simplicity of notation, in the rest of the paper, we will often suppress tensor product symbols; for example, we express the element - → L ⊗ - → K ∈ SkAlg gl 1 -A ( D 3 ) ⊗ 2 as - → L - → K .

Corollary 5.6. As a SkAlg gl 2 , st q ( D 3 ) ⊗ 2 -SkAlg gl 2 , st q ( D 2 ) ⊗ 3 -bimodule, Sk gl 2 , st q ( Sf ) is a cyclic bimodule generated by the empty skein [ ∅ ] . More explicitly,

<!-- formula-not-decoded -->

29 In both this and the following theorem, the relation is drawn in a hexagon for illustrative purposes, but the face associated to a puncture can be any 2 n -gon.

and Ann([ ∅ ]) is the left ideal generated by relations, one for each face of the boundary tessellation:

<!-- formula-not-decoded -->

Corollary 5.7. As a SkAlg gl 1 , st q ( ˜ D 3 ) ⊗ 2 -SkAlg gl 1 , st q ( ˜ D 2 ) ⊗ 3 -bimodule, SkAlg gl 1 , st q ( ˜ Sf ) is a cyclic bimodule generated by the empty skein [ ∅ ] . More explicitly,

<!-- formula-not-decoded -->

where Ann([ ∅ ]) is the left ideal generated by the relations, one for each face of the boundary tessellation:

<!-- formula-not-decoded -->

Corollary 5.8. As a SkAlg gl 1 -A ( D 3 ) ⊗ 2 -SkAlg gl 1 -A ( D 2 ) ⊗ 3 -bimodule, Sk gl 1 -A ( Sf ) is a cyclic bimodule generated by the empty skein [ ∅ ] . More explicitly,

<!-- formula-not-decoded -->

where Ann([ ∅ ]) is the left ideal generated by the relations, one for each face of the boundary tesselation:

<!-- formula-not-decoded -->

Recall from Section 2.2.2 that the generators of the face suspension module are each naturally associated to an edge cone, and thus to an edge of some tetrahedron in the triangulation of Y . For the face suspension module generator x , set θ x to be the angle associated to this edge.

Furthermore, label edge cones of the double cover of a face suspension as in Figure 34. S (resp. T ) is the top (resp. bottom) tetrahedron. Vertices of the double cover of the face suspension are decorated with sheet labels. While we have used the notations like ˜ x 12 and ˜ x 21 in Section 3.6 to denote the two lifts of the boundary marking x , to further simplify the notation, here we use asterisks (*) to denote the two lifts; we simply write x and x ∗ for ˜ x 12 and ˜ x 21 , respectively.

Lemma 5.9. The quantum UV-IR map on a face suspension,

<!-- formula-not-decoded -->

Figure 34. Labeling the edge cones of a face suspension and its double cover with face suspension module variables and their lifts.

<!-- image -->

is surjective.

Proof. Recall from Proposition 3.20 that F is a bimodule homomorphism mapping the empty skein to the empty skein. Thus, it is enough to check that the associated algebra maps and

<!-- formula-not-decoded -->

are surjective. Recall from Example 3.19 that these maps are given by (the relevant leaf

Figure 35. The leaf space of a face suspension with some angles labeled. θ a S and θ a T are the generalized angles assigned to the edges of the tetrahedra associated to the face suspension module variables a S and a T , respectively.

<!-- image -->

space is shown in Figure 35)

<!-- formula-not-decoded -->

for xy ∈ { a S b S , b S c S , c S a S , b T a T , c T b T , a T c T } , and

<!-- formula-not-decoded -->

for xy ∈ { a S a T , b S b T , c S c T } . The surjectivity then follows immediately from Lemmas 4.15 and 5.1, as those images generate the gl 1 -skein algebras. □

Corollary 5.10. If there exists a linear map

<!-- formula-not-decoded -->

making the following square

<!-- formula-not-decoded -->

commutative, then it is unique, and it must be a bimodule homomorphism.

Proof. This follows immediately from the surjectivity of F Sf (Lemma 5.9) and the fact that the other 3 arrows in the diagram are bimodule homomorphisms. □

Below, we show that the evaluation map ev Sf indeed exists.

Theorem 5.11. There is a bimodule homomorphism

<!-- formula-not-decoded -->

mapping the empty skein [ ∅ ] to 1 ⊗ [ ∅ ] , defined on the algebra generators by

<!-- formula-not-decoded -->

for xy ∈ { a S b S , b S c S , c S a S , b T a T , c T b T , a T c T } , and

<!-- formula-not-decoded -->

for xy ∈ { a S a T , b S b T , c S c T } , which makes the square (43) commutative.

Proof. Choosing the preimages of the generators as in Lemma 5.9, we find, for the generators of SkAlg gl 1 q ( ˜ D 3 ) ⊗ 2 ,

<!-- formula-not-decoded -->

and

<!-- formula-not-decoded -->

Likewise, for the generators of SkAlg gl 1 q ( ˜ D 2 ) ⊗ 3 ,

<!-- formula-not-decoded -->

and

<!-- formula-not-decoded -->

The fact ev respects relation (27) in both copies of SkAlg gl 1 q ( ˜ D 3 ) follows from an almost identical computation to the one performed in Theorem 4.20.

Now, it suffices to check that the relations (41) are preserved by ev, and by symmetry, we just need to check just one of the relations. Observe

<!-- formula-not-decoded -->

and

<!-- formula-not-decoded -->

Using (11) and the relations in (42), we see that these are equal in S f ⊗ Sk gl 1 -A ( Sf ). □

5.2. Gluing the commutative squares. In this subsection, we complete the construction of the commutative square in (39). To do so, consider the following diagram analogous to

<!-- formula-not-decoded -->

<!-- image -->

.

Here, σ sl 2 , σ gl 2 , σ gl 1 are the splitting homomorphisms for sl 2 , gl 2 , and gl 1 -skein modules, respectively; see Theorem 2.18, Corollary A.8, Corollary B.5 and B.10. The tensor products in the front square are the relative tensor products. In the top-right corner of the front square, the relative tensor product ⊗ f ∈T (2) Sk gl 1 , st q ( ˜ Sf ) must remember the extra 3-term relations near the cone points in ˜ Y ; see Corollary B.10 in Appendix B.

We need to first check that all of the maps in the front square of (44) are well-defined; i.e., that the naive tensor product of maps descend to the corresponding quotients. For the top horizontal arrow and the left vertical arrow, this follows from Propositions 3.21 and 4.12, respectively. That the bottom horizontal arrow is well-defined is by design; the relative tensor product ⊗ f ∈T (2) S f is defined in such a way that ⊗ f ∈T (2) Tr Sf is well-defined; see [PPar, Sec. 5.2].

As for the right vertical arrow, ⊗ f ∈T (2) ev Sf , this is addressed in the following proposition.

## Proposition 5.12. The map

<!-- image -->

is well-defined.

Proof. We need to show that the composition

<!-- image -->

First, let's check that the relations for the pre-relative tensor product - (56) and (57) in Definition B.7 - are respected.

- To check the relation (56), suppose a vertex cone is surrounded by the face suspensions Sf 1 , Sf 2 , and Sf 3 . Then, as a left relation,

<!-- formula-not-decoded -->

where ˆ z, ˆ z ′ , and ˆ z ′′ are the square-root quantized shape parameters (as in Section 2.2.2) associated to the three edge cones around the vertex cone.

- To check the relation (57), consider face suspensions surrounding an edge. Then, as a right relation,

<!-- image -->

<!-- formula-not-decoded -->

<!-- image -->

where ˆ x e i is the square-root quantized shape parameter associated to the edge cone e i . For the remaining relations, which can be obtained from the above diagrams through orientation reversal and/or an involution of the double cover, the computations proceed identically to the ones given above.

Lastly, let's check that the relations for the relative tensor product - (58) in Definition B.9 are respected.

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Now that we have checked that all the arrows in the the diagram (44) are well-defined, the fact that it is a commutative diagram is almost immediate; the top (Proposition 3.21) and left (Proposition 4.13) squares commute simply because all of the maps are locally defined, and the front square commutes since it is the relative tensor product of the commutative squares constructed in Theorem 5.11.

From the construction, note that the images of both Tr T ⊗ σ gl 1 and of ⊗ f ∈T (2) ev Sf ◦ σ gl 1 are contained in the submodule

<!-- formula-not-decoded -->

Moreover, by Lemma B.6, the gl 1 -splitting map

<!-- formula-not-decoded -->

is an isomorphism. Therefore, by replacing the bottom right corner of the commutative diagram by SQGM T ( Y ) ⊗ Sk gl 1 -A ( Y ), we obtain the first part of Theorem A.

- 5.3. Naturality with respect to Pachner moves. Here, we show the second part of Theorem A, that the commutative squares constructed above behave naturally with respect to 2-3 Pachner moves:
- Theorem 5.13. Under a 2-3 Pachner move T 2 →T 3 , we have the following commutative diagram

,

<!-- image -->

where φ 2 → 3 and ϕ 2 → 3 are the coordinate change maps given in Propositions 2.24 and 3.14, respectively.

Proof. It suffices to check that the front right face commutes, since we already know that all the other faces commute. Thanks to the local nature of our construction, it is enough to check this for the triangular bipyramid BP where the Pachner move is applied, and since we know that all the other faces commute, it suffices to show that the stated quantum UV-IR map Sk gl 2 , st q (BP) F T 2 → Sk gl 1 , st q ( ˜ BP T 2 ) is surjective for the bipyramid triangulated into 2 ideal tetrahedra.

Indeed, by the same argument as in [PPar, Lem. 4.1], Sk gl 1 , st q ( ˜ BP T 2 ) is generated by the empty skein [ ∅ ] as a SkAlg gl 1 , st q ( ˜ D 3 ) ⊗ 6 -SkAlg gl 1 , st q ( ˜ D 2 ) ⊗ 9 bimodule, and the desired surjectivity of F T 2 follows, essentially by the same computation as in the proof of Lemma 5.9. □

- 5.4. Recovering the quantum trace map from the quantum UV-IR map. In this short subsection, we show Theorem C; that, under a mild assumption on the homology of Y , it is possible to recover the genuine quantum trace map from the quantum UV-IR map.

From Theorem A, using the quantum UV-IR map, we can compute

<!-- formula-not-decoded -->

for any unoriented link L ⊂ Y , where ⃗ L is L endowed with an arbitrary orientation. This is almost as good as having the quantum trace map

<!-- formula-not-decoded -->

itself, since, as long as [ ⃗ L ] ∈ Sk gl 1 -A ( Y ) is not torsion, we can recover Tr T ([ L ]) from Tr T ([ L ]) ⊗ [ ⃗ L ]. This is because, if [ ⃗ L ] ∈ Sk gl 1 -A ( Y ) is not torsion and α ∈ H 1 ( Y ; Z ) is the homology class of ⃗ L , the α -graded part of the gl 1 -skein module is Sk gl 1 -A ( Y ) α = R [ ⃗ L ] ∼ = R , and the map

<!-- formula-not-decoded -->

is an isomorphism.

The structure of gl 1 -skein modules was completely characterized in [Prz98, Thm 2.3] (see also [Prz06, Thm IX.3.7]) where it is shown that, for α ∈ H 1 ( Y ; Z ), the α -graded part Sk gl 1 -A ( Y ) α of Sk gl 1 -A ( Y ) is torsion-free (and isomorphic to the base ring R ) iff the intersection pairing ( α, β ) ∈ Z is 0 for all β ∈ H 2 ( Y ; Z ). In particular, we obtain Theorem C as a corollary.

## 6. Examples

In this section, we demonstrate the compatibility theorem - Theorem A (and also Theorem C) - for a skein in the figure-8 knot complement Y = S 3 \ 4 1 . Consider the triangulation T of Y shown in Figure 36. Let us compute the quantum trace of the knot K b ⊂ Y from [PPar,

Figure 36. A triangulation of the figure-8 knot complement, as well as the tangle ⃗ K b contained inside of it (shown in blue). The gluing of the tetrahedra is controlled by the edge markings. The faces of the tetrahedra are labeled N,S,E, and W , and edges are labeled with their corresponding square-root quantized shape parameters.

<!-- image -->

Sec. 6], using the quantum UV-IR map; an oriented version of this knot, ⃗ K b , is shown in blue in Figure 36.

While the image of ⃗ K b under the quantum UV-IR map can be computed without any reference to splitting, we will do our computation locally, by applying the splitting map, to demonstrate the compatibility in each face suspension and illustrate how our proof works in practice. Our knot ⃗ K b , after splitting into face suspensions, is depicted in Figure 37; only the face suspensions containing part of ⃗ K b are shown. Before iterating through the compatible states, observe that the components of ⃗ K b in the face suspensions above can be written explicitly as elements of SkAlg gl 2 , st q ( D 3 ) ⊗ 2 and SkAlg gl 2 , st q ( D 2 ) ⊗ 3 acting on the empty skein as follows:

<!-- image -->

<!-- formula-not-decoded -->

<!-- image -->

Figure 37. The image of ⃗ K b under the splitting map. The face suspension variables associated to edge cones that will be used in later computations are labeled with the corresponding shape parameter.

<!-- image -->

There are three nonzero compatible states. In the computations that follow, we will denote by x f the generator of S f corresponding to the shape parameter x .

<!-- formula-not-decoded -->

and

<!-- formula-not-decoded -->

so that

<!-- formula-not-decoded -->

yielding the same result as above, as expected.

- ϵ 1 = -and ϵ 2 = +: The computation proceeds almost identically to above; the result is

<!-- formula-not-decoded -->

- ϵ 1 = ϵ 2 = -: First, observe that in Sk gl 1 , st q ( ˜ Sf ),

<!-- image -->

Then,

<!-- formula-not-decoded -->

and

<!-- formula-not-decoded -->

Tensoring these together, we get

<!-- formula-not-decoded -->

where, in the last line, we have used the following relations in Sk gl 1 -A ( Sf ):

<!-- formula-not-decoded -->

On the other hand, easy computations also show that,

<!-- formula-not-decoded -->

and

<!-- formula-not-decoded -->

which after combining yields

<!-- formula-not-decoded -->

as expected.

Lastly, since

<!-- formula-not-decoded -->

under the isomorphism in Lemma B.6, we have

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

confirming Theorem C in this case. 30

## Appendix A. Stated gl 2 -skein modules

We need to first define precisely what we mean by the stated (i.e. relative) version of the gl 2 -skein module.

Definition A.1. Let ( Y, Γ) be a boundary marked 3-manifold, and R := Z [ q ± 1 2 ]. The (stated) gl 2 -skein module of ( Y, Γ), Sk gl 2 q ( Y, Γ) is defined as the R -span of isotopy classes of all framed, oriented, stated tangles in ( Y, Γ), modulo usual relations (12)-(16), as well as the following boundary skein relations:

<!-- formula-not-decoded -->

plus all the skein relations obtained by simultaneous orientation reversal of the tangles in the skein relations above.

Remark A.2. Note, the boundary height exchange relations (i.e., the ones involving matrices) correspond exactly to the second part of Lemma 3.26 in [PPar], upon projection to sl 2 -skeins;

30 This result for Tr( K b ) agrees with that from [PPar, Sec. 6.2.2] after adjusting for the difference in convention for shape parameters and recalling that c B = ( -1) -1 2 in that work.

see Proposition 4.2. These are also the R -matrices for the Jones polynomial: 31

<!-- formula-not-decoded -->

etc., with cups and caps

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

etc. In particular, as we gradually collide a link with a boundary marking, we obtain the usual state sum formula for its Jones polynomial.

For convenience, again write

<!-- formula-not-decoded -->

where V (Γ + ) and V (Γ -) denote the sets of sink and source vertices of Γ, respectively. By an identical argument to the sl 2 case, we have:

Proposition A.3. The stated skein module Sk gl 2 q ( Y, Γ) has a natural SkAlg gl 2 q ( V (Γ) + ) -SkAlg gl 2 q ( V (Γ) -) -bimodule structure.

The stated skein relations above are necessary to obtain a well-defined splitting map for stated gl 2 -skein modules.

31 It is worth noting that these R -matrices are exactly the matrices we get by applying the quantum UV-IR map to a positive and a negative crossing.

Theorem A.4. Let ( Y 1 , Γ 1 ) and ( Y 2 , Γ 2 ) be boundary marked 3 -manifolds. Suppose that Σ 1 ⊂ ∂Y 1 and Σ 2 ⊂ Y 2 , along with their markings, are homeomorphic combinatorial foliated surfaces (see Definition 5.3) of opposite orientations. Write Σ for the common image of Σ 1 and Σ 2 after gluing, and set Y = Y 1 ∪ Σ Y 2 and Γ = (Γ 1 \ int Σ 1 ) ∪ (Γ 2 \ int Γ 2 ) . Let ⃗ L be a stated, oriented tangle representing [ ⃗ L ] ∈ Sk gl 2 ( Y, Γ) , isotoped so that ⃗ L ∩ Σ ⊂ Γ , guaranteeing that ⃗ L 1 := ⃗ L ∩ Y 1 and ⃗ L 2 := ⃗ L ∩ Y 2 are tangles in ( Y 1 , Γ 1 ) and ( Y 2 , Γ 2 ) . Then, there is an R-module homomorphism,

<!-- formula-not-decoded -->

where the sum is over all functions ⃗ ϵ : ⃗ L ∩ Σ →{±} assigning states to endpoints of ⃗ L , ⃗ L ⃗ ϵ 1 and ⃗ L ⃗ ϵ 2 denote the stated tangles obtained from ⃗ L 1 and ⃗ L 2 by assigning states according to ⃗ ϵ, and the relative tensor product Sk gl 2 q ( Y 1 , Γ 1 ) ⊗ Sk gl 2 q ( Y 2 , Γ 2 ) denotes the quotient of the usual tensor product Sk gl 2 q ( Y 1 , Γ 1 ) ⊗ Sk gl 2 q ( Y 2 , Γ 2 ) by the following relations:

- (1) For each internal edge e of of Σ adjacent to a sink, we have the following relations among left actions:

<!-- image -->

along with the relations obtained by simultaneous orientation reversal of the tangles in both figures.

- (2) Likewise, for each internal edge e of Σ adjacent to a source, there are the following relations among right actions:

<!-- image -->

along with the relations obtained by simultaneous orientation reversal of the tangles in both figures.

Proof. As in [PPar, Sec. 3], we need to study how the element

<!-- formula-not-decoded -->

behaves under isotopy of ⃗ L . The proof is completely analogous to that of [PPar, Thm 3.21]. The only difference lies in the invariance under the height exchange isotopy near a component of Γ:

<!-- image -->

Using the new gl 2 -stated skein relations and starting from the image of the RHS, we compute,

<!-- image -->

The computation is similar for other choices of orientations of the tangles. □

Reduced stated gl 2 -skein modules are defined analogously to their sl 2 counterparts. Bad arcs in the gl 2 -skein module are shown below:

.

Definition A.5. The reduced stated skein module Sk gl 2 q ( Y, Γ) is the quotient

<!-- image -->

<!-- formula-not-decoded -->

where I bad , + denotes the right ideal of SkAlg gl 2 q ( V (Γ) + ) generated by the bad arcs near the sinks, and I bad , -denotes the left ideal of SkAlg gl 2 q ( V (Γ) -) generated by the bad arcs near the sources.

As before, set

<!-- formula-not-decoded -->

and note that the SkAlg gl 2 q ( V (Γ) + )-SkAlg gl 2 q ( V (Γ) -)-bimodule structure on Sk gl 2 q ( Y, Γ) clearly descends to a SkAlg gl 2 q ( V (Γ) + )-SkAlg gl 2 q ( V (Γ) -)-bimodule structure on Sk gl 2 q ( Y, Γ).

The following corollary - the 2d splitting map for gl 2 -skeins - follows from Theorem A.4; this is because each ideal arc times an interval is homeomorphic to an elementary quadrilateral and hence admits a natural combinatorial foliation as follows:

<!-- image -->

Since the boundary markings involved in the decomposition of an ideally triangulated surface into the ideal triangles have no vertices, there are no extra relations in the tensor product.

Corollary A.6. Let Σ = ⋃ △∈ τ (2) △ be a decomposition of an ideally triangulated surface Σ into ideal triangles. Then, there is a well-defined splitting map

<!-- formula-not-decoded -->

where E ∈ Σ denotes the union of all of the edges of the triangulation, and ⃗ L ⃗ ϵ △ denotes the part of ⃗ L in △× I after splitting, with states assigned to the newly created boundary points according to ⃗ ϵ.

To obtain the gl 2 splitting map for ideally triangulated 3-manifolds, we first need the following definition:

Definition A.7. The relative tensor product ⊗ f ∈T (2) Sk gl 2 q ( Sf ) of the bimodules Sk gl 2 q ( Sf ), f ∈ T (2) is defined to be the quotient of the ordinary tensor product (as R -modules) ⊗ f ∈T (2) Sk gl 2 q ( Sf ) by the following relations:

- For each vertex cone Cv , we have the following relations among left actions on ⊗ f ∈T (2) Sk gl 2 q ( Sf ):

<!-- image -->

<!-- image -->

where each sector in the above diagrams represents one of the three face suspensions surrounding Cv (viewed from the vertex v ), and the markings shown are on the bare edge cones abutting Cv .

- For each internal edge e ∈ T (1) we have the following relations among right actions of on ⊗ f ∈T (2) Sk gl 2 q ( Sf ):

<!-- image -->

where each sector in the above diagrams represents one of the face suspensions surrounding e (as many as the number of tetrahedra abutting e ), and the markings shown are on the bare edge cones abutting e .

The splitting map of Theorem A.4 then implies the following important corollary:

Corollary A.8. Let Y = ⋃ f ∈T (2) Sf be a decomposition of an ideally triangulated 3 -manifold (without boundary except for cusps at infinity) into face suspensions. Then, there is a well-defined splitting map

<!-- formula-not-decoded -->

where ⃗ L ⃗ ϵ f denotes the part of ⃗ L in Sf after splitting, with boundary states determined by ⃗ ϵ .

Appendix B. Stated gl 1 -skein modules with defects

In this section of the appendix, we define the stated gl 1 -skein module, both with and without sign defects. 32 Furthermore, we construct and prove the well-definedness of the splitting maps for the gl 1 -skein modules above. The key results are Corollaries B.5 and B.10, which allow us to split the gl 1 -skein modules of both an ideally triangulated 3-manifold as well as its double cover.

Definition B.1. Let ( Y, Γ) be a boundary marked 3-manifold, and let R := Z [ q ± 1 ]. The (stated) gl 1 -skein module Sk gl 1 q ( Y, Γ) of Y is defined as the R -span of isotopy classes of all

32 Even though we call it the 'stated' gl 1 -skein module in analogy with the stated sl 2 - or gl 2 -skein modules, there is a unique 'state' that we can assign to the boundary of a gl 1 -skein, so there's no extra data at the boundary points.

framed, oriented tangles in ( Y, Γ) modulo the usual gl 1 -skein relations (17), (18), and (19) in addition to the following:

<!-- formula-not-decoded -->

,

as well as all of the skein relations obtained by simultaneous orientation reversal of the tangles in the aforementioned relations.

Let ˜ Y be the associated double cover with boundary marking ˜ Γ. The stated gl 1 -skein module with defects Sk gl 1 q ( ˜ Y , ˜ Γ) is the R -module spanned by isotopy classes of framed oriented links away from the branch locus modulo the relations above in addition to (20).

These skein modules of course admit natural bimodule structures.

Proposition B.2. The stated gl 1 -skein modules Sk gl 1 q ( Sf, Γ) and Sk gl 1 q ( ˜ Sf, ˜ Γ) ) admit natural SkAlg gl 1 q ( V (Γ) + ) -SkAlg gl 1 q ( V (Γ) -) and SkAlg gl 1 q ( V ( ˜ Γ) + ) -SkAlg gl 1 q ( V ( ˜ Γ) -) -bimodule structures, respectively.

In much the same way as Theorem A.4, we have:

Theorem B.3. Let ( Y, Γ) be either a boundary marked 3 -manifold or its double cover ( ˜ Y , ˜ Γ) . We write Sk gl 1 q ( Y, Γ) for the corresponding skein module in each case. Then, there is an R -module homomorphism,

<!-- formula-not-decoded -->

where the relative tensor product Sk gl 1 q ( Y 1 , Γ 1 ) ⊗ Sk gl 1 q ( Y 2 , Γ 2 ) denotes the quotient of the usual tensor product Sk gl 1 q ( Y 1 , Γ 1 ) ⊗ Sk gl 1 q ( Y 2 , Γ 2 ) by the following relations:

- (1) For each internal edge e of of Σ adjacent to a sink, we have the following relations among left actions:

<!-- image -->

along with the relations obtained by simultaneous orientation reversal of the tangles in both figures.

- (2) Likewise, for each internal edge e of Σ adjacent to a source, there are the following relations among right actions:

<!-- image -->

along with the relations obtained by simultaneous orientation reversal of the tangles in both figures.

For the purpose of comparing the 3d quantum trace map with the 3d quantum UV-IR map, we are most interested in the special case of splitting an ideally triangulated 3-manifold Y into face suspensions and the corresponding splitting for the branched double cover (i.e., splitting ˜ Y into ˜ Sf 's).

Splitting Y into face suspensions. Here, we use the gl 1 -skein modules with parameter -A , since that is the relevant skein module used in our main construction.

Definition B.4. The relative tensor product ⊗ f ∈T (2) Sk gl 1 -A ( Sf ) of the bimodules Sk gl 1 -A ( Sf ) is the quotient of the naive tensor product ⊗ f ∈T (2) Sk gl 1 -A ( Sf ) by the following relations:

- For each vertex cone Cv , we have the following relations among left actions on ⊗ f ∈T (2) Sk gl 1 -A ( Sf ):

<!-- image -->

where each sector in the above diagrams represents one of the three face suspensions surrounding Cv (viewed from the vertex v ), in addition to the relation obtained by simultaneously reversing the orientations of all of the tangles in the figures above.

- For each internal edge e ∈ T (1) we have the following relations among right actions on ⊗ f ∈T (2) Sk gl 1 -A ( Sf ):

,

<!-- image -->

where each sector in the above diagrams represents one of the face suspensions surrounding e (as many as the number of tetrahedra abutting e ), in addition to the relation obtained by simultaneously reversing the orientations of all of the tangles in the above figures.

Corollary B.5. Let Y = ⋃ f ∈T (2) Sf be a decomposition of an ideally triangulated 3-manifold into face suspensions. Then, there is a well-defined splitting map

<!-- formula-not-decoded -->

where ⃗ L f denotes the part of ⃗ L in Sf after splitting.

The relative tensor product ⊗ f ∈T (2) Sk gl 1 -A ( Sf ) is graded by ⊕ Ce Z ; i.e., for each edge cone Ce , it has a Z -grading given by the signed count of the end points of a gl 1 -tangle on that edge cone. The image of the splitting map lies in the 0-graded part, with respect to this grading. We denote by ( ⊗ f ∈T (2) Sk gl 1 -A ( Sf ) ) 0 this 0-graded piece.

Lemma B.6. The splitting map

<!-- formula-not-decoded -->

is an isomorphism of R -modules.

Proof. The proof is similar to that of Lemma 4.21, except that in this case we have some relations to check. Firstly, it is easy to see that this map is surjective; ( ⊗ f ∈T (2) Sk gl 1 -A ( Sf ) ) has a spanning set given by the form {⊗ f ∈T (2) [ L ⃗ n f ] | ⃗ n f ∈ Z 6 , n f, 1 + · · · + n f, 6 = 0 } , where L ⃗ n f ∈ Sk gl 1 -A ( Sf ) denotes the distinguished gl 1 -skein ( gl 1 -web) in Sf with boundary condition ⃗ n f ; see Figure 38. Likewise, ( ⊗ f ∈T (2) Sk gl 1 -A ( Sf ) ) 0 has a spanning set given by the form

Figure 38. The gl 1 -web L ⃗ n in Sf ; each strand labeled by n ∈ Z denotes the n -colored strand (equivalent to n parallel strands), and they are all flat on the leaf space.

<!-- image -->

⊗ f ∈T (2) [ L ⃗ n f ] but with matching boundary conditions along boundary markings that are glued, which is the image of the corresponding glued gl 1 -skein in Sk gl 1 -A ( Y ).

To show the injectivity of the splitting map, we can construct the inverse of the splitting map in the following way. There is a map

<!-- formula-not-decoded -->

which can be defined on the basis elements ⊗ f ∈T (2) [ L ⃗ n f ] (with matching boundary conditions) by just gluing the skeins. Furthermore, it is easy to see that this map respects the left and right relations, and thus descends to the quotient to give a map

<!-- formula-not-decoded -->

By construction, g ◦ σ gl 1 is an identity on Sk gl 1 -A ( Y ). Therefore, the splitting map is an isomorphism.

<!-- formula-not-decoded -->

Splitting ˜ Y into branched double covers of face suspensions. Obtaining a splitting map for Sk gl 1 q ( ˜ Y , Θ) requires a bit more work, due to the presence of the 3-term relations (24) around the cone points.

Definition B.7. The pre-relative tensor product ⊗ ◦ f ∈T (2) Sk gl 1 q ( ˜ Sf ) of the bimodules Sk gl 1 q ( ˜ Sf ) is the quotient of ⊗ f ∈T (2) Sk gl 1 q ( ˜ Sf ) by the following relations (as well as the ones obtained by simultaneous orientation reversal of all the tangles):

- For each vertex cone Cv , we have the following relations among left actions on ⊗ f ∈T (2) Sk gl 1 q ( ˜ Sf ):

<!-- image -->

where each sector in the above diagrams represents the projection of one of the three face suspensions surrounding Cv (viewed from the vertex v ), and i denotes the sheet labels of the tangles.

- For each internal edge e ∈ T (1) we have the following relations among right actions on ⊗ f ∈T (2) Sk gl 1 q ( ˜ Sf ):

<!-- image -->

where each sector in the above diagrams represents the projection one of the face suspensions surrounding e and i labels the sheet of the tangles.

Corollary B.8. Suppose Y is an ideally triangulated 3 -manifold with an associated branched double cover ˜ Y . Let ˜ Y ◦ denote ˜ Y with a small neighborhood of the cone points removed, and let Sk gl 1 q ( ˜ Y ◦ ) denote the gl 1 -skein module of ˜ Y ◦ , i.e., with sign defects along the branch locus but without the 3-term relations at cone points. Then, for the decomposition ˜ Y ◦ = ⋃ f ∈T (2) ˜ Sf of ˜ Y ◦ into branched double covers of face suspensions, there is a well-defined splitting map

<!-- formula-not-decoded -->

For each neighborhood of a cone point that is removed from ˜ Y , ˜ Y ◦ has a boundary component which is a torus T 2 with 4 branch points. It follows that Sk gl 1 q ( ˜ Y ◦ ) is a module over ⊗ T ∈T (3) SkAlg gl 1 q ( T 2 ), where T 2 here means the torus with 4 branch points; see Figure 39 for an illustration of this boundary torus. While the naive tensor product ⊗ f ∈T (2) Sk gl 1 q ( ˜ Sf ) is no

Figure 39. The boundary of a tetrahedron has been unraveled; its double cover is a torus. Vertices A, B, C, and D are lifted to the double cover as A 1 , B 1 , C 1 , and D 1 on sheet 1 and as A 2 , B 2 , C 2 , and D 2 on sheet 2. We also show a cycle, its image under projection to the boundary, and its appearance in the double cover.

<!-- image -->

longer a module over ⊗ T ∈T (3) SkAlg gl 1 q ( T 2 ), the pre-relative tensor product ⊗ ◦ f ∈T (2) Sk gl 1 q ( ˜ Sf ) recovers this module structure. Indeed, while an isotopy of a link diagram on T 2 does not preserve the corresponding element in SkAlg gl 1 q ( ˜ D 3 ) ⊗ 4 upon splitting of T 2 into 4 hexagons, the relations (56) around each vertex cone ensures that we have a well-defined splitting

<!-- formula-not-decoded -->

where the relative tensor product on the right-hand side is over the 4 face suspensions around the same barycenter. It follows that ⊗ ◦ f ∈T (2) Sk gl 1 q ( ˜ Sf ) is a ⊗ T ∈T (3) SkAlg gl 1 q ( T 2 )-module. It is also immediate that the splitting map σ gl 1 : Sk gl 1 q ( ˜ Y ◦ ) → ⊗ ◦ f ∈T (2) Sk gl 1 q ( ˜ Sf ) is a ⊗ T ∈T (3) SkAlg gl 1 q ( T 2 )-module homomorphism.

To this end, in order to get a splitting map out of Sk gl 1 q ( ˜ Y ), we simply need to take a further quotient of the pre-relative tensor product ⊗ ◦ f ∈T (2) Sk gl 1 q ( ˜ Sf ) by imposing the 3-term relations (24) using this ⊗ T ∈T (3) SkAlg gl 1 q ( T 2 )-module structure.

Definition B.9. The relative tensor product ⊗ f ∈T (2) Sk gl 1 q ( ˜ Sf ) of the bimodules Sk gl 1 q ( ˜ Sf ) is the quotient of the pre-relative tensor product ⊗ ◦ f ∈T (2) Sk gl 1 q ( ˜ Sf ), thought of as a left ⊗ T ∈T (3) SkAlg gl 1 q ( T 2 )-module, by the 3-term relations (24) among left actions:

.

<!-- image -->

The following corollary is immediate:

Corollary B.10. Suppose Y is an ideally triangulated 3 -manifold with an associated branched double cover ˜ Y . Further suppose that Y is equipped with a generalized angle structure Θ . Let ˜ Y = ⋃ f ∈T (2) ˜ Sf be the decomposition of ˜ Y into branched double covers of face suspensions. Then, there is a well-defined splitting map

<!-- formula-not-decoded -->

## References

- [Bul97] Doug Bullock. Rings of sl2 (c)-characters and the kauffman bracket skein module. Commentarii Mathematici Helvetici , 72(4):521-542, 1997.
- [BW11] Francis Bonahon and Helen Wong. Quantum traces for representations of surface groups in SL 2 ( C ). Geom. Topol. , 15(3):1569-1615, 2011.

[BZBJ18] David Ben-Zvi, Adrien Brochier, and David Jordan. Integrating quantum groups over surfaces. J. Topol. , 11(4):874-917, 2018.

- [CL22] Francesco Costantino and Thang T. Q. Lˆ e. Stated skein algebras of surfaces. J. Eur. Math. Soc. (JEMS) , 24(12):4063-4142, 2022.
- [CL25] Francesco Costantino and Thang T. Q. Lˆ e. Stated skein modules of 3-manifolds and TQFT. J. Inst. Math. Jussieu , 24(3):663-703, 2025.

[Coo23] Juliet Cooke. Excision of skein categories and factorisation homology. Adv. Math. , 414:Paper No. 108848, 51, 2023.

[DS25] Renaud Detcherry and Ramanujan Santharoubane. An embedding of skein algebras of surfaces into localized quantum tori from Dehn-Thurston coordinates. Geom. Topol. , 29(1):313-348, 2025.

[ELPS25] Tobias Ekholm, Pietro Longhi, Sunghyuk Park, and Vivek Shende. Skein traces from curve counting, 2025. to appear.

- [ES25] Tobias Ekholm and Vivek Shende. Skeins on branes, 2025. https://arxiv.org/abs/1901.08027 .

- [FN24] Daniel S. Freed and Andrew Neitzke. 3d spectral networks and classical Chern-Simons theory. In Surveys in differential geometry 2021. Chern: a great geometer of the 20th century , volume 26 of Surv. Differ. Geom. , pages 51-155. Int. Press, Boston, MA, 2024.
- [Gab17] Maxime Gabella. Quantum Holonomies from Spectral Networks and Framed BPS States. Commun. Math. Phys. , 351(2):563-598, 2017.
- [GJS23] Sam Gunningham, David Jordan, and Pavel Safronov. The finiteness conjecture for skein modules. Invent. Math. , 232(1):301-363, 2023.
- [GLM15] Dmitry Galakhov, Pietro Longhi, and Gregory W. Moore. Spectral networks with spin. Comm. Math. Phys. , 340(1):171-232, 2015.
- [GMN13] Davide Gaiotto, Gregory W. Moore, and Andrew Neitzke. Spectral networks. Ann. Henri Poincar´ e , 14(7):1643-1731, 2013.
- [GY24a] Stavros Garoufalidis and Tao Yu. The 3d-index of the 3d-skein module via the quantum trace map, 2024. https://arxiv.org/abs/2406.04918 .
- [GY24b] Stavros Garoufalidis and Tao Yu. A quantum trace map for 3-manifolds, 2024. https://arxiv. org/abs/2403.12424 .
- [Hia10] Christopher Hiatt. Quantum traces in quantum teichm¨ uller theory. Algebraic and Geometric Topology , 10:1245-1283, 06 2010.
- [HN16] Lotte Hollands and Andrew Neitzke. Spectral networks and Fenchel-Nielsen coordinates. Lett. Math. Phys. , 106(6):811-877, 2016.
- [JLSS21] David Jordan, Ian Le, Gus Schrader, and Alexander Shapiro. Quantum decorated character stacks, 2021.
- [Jor24] David Jordan. Langlands duality for skein modules of 3-manifolds. In String-Math 2022 , volume 107 of Proc. Sympos. Pure Math. , pages 127-149. Amer. Math. Soc., Providence, RI, [2024] © 2024.
- [KLN + 25] Piotr Kucharski, Pietro Longhi, Dmitry Noshchenko, Sunghyuk Park, and Piotr Suglyph[suppress] lkowski. Quivers and BPS states in 3d and 4d, 2025.
- [KLS23] Hyun Kyu Kim, Thang T Q Lˆ e, and Miri Son. SL 2 quantum trace in quantum Teichm¨ uller theory via writhe. Algebraic &amp; Geometric Topology , 23(1):339-418, mar 2023.
- [KQ22] J. Korinman and A. Quesney. The quantum trace as a quantum non-abelianization map. J. Knot Theory Ramifications , 31(6):Paper No. 2250032, 49, 2022.
- [Lˆ e18] Thang T. Q. Lˆ e. Triangular decomposition of skein algebras. Geom. Topol. , 9:591-632, 2018.
- [LT08] Feng Luo and Stephan Tillmann. Angle structures and normal surfaces. Trans. Amer. Math. Soc. , 360(6):2849-2866, 2008.
- [LY23] Thang T. Q. Lˆ e and Tao Yu. Quantum traces for sl n -skein algebras, 2023. https://arxiv.org/ abs/2303.08082 .
- [Mul16] Greg Muller. Skein and cluster algebras of marked surfaces. Quantum Topol. , 7(3):435-503, 2016. [MW12] Scott Morrison and Kevin Walker. Blob homology. Geom. Topol. , 16(3):1481-1607, 2012.
- [NY20] Andrew Neitzke and Fei Yan. q -nonabelianization for line defects. J. High Energy Phys. , (9):153, 65, 2020.
- [NY22] Andrew Neitzke and Fei Yan. The quantum UV-IR map for line defects in gl (3)-type class S theories. J. High Energy Phys. , (9):Paper No. 81, 50, 2022.
- [PPar] Samuel Panitch and Sunghyuk Park. 3d quantum trace map. Algebraic &amp; Geometric Topology , to appear.
- [Prz98] J´ ozef H. Przytycki. A q -analogue of the first homology group of a 3-manifold. In Perspectives on quantization (South Hadley, MA, 1996) , volume 214 of Contemp. Math. , pages 135-144. Amer. Math. Soc., Providence, RI, 1998.
- [Prz99] J´ ozef H. Przytycki. Fundamentals of Kauffman bracket skein modules. Kobe J. Math. , 16(1):45-66, 1999.
- [Prz06] Jozef H. Przytycki. Skein modules, 2006. https://arxiv.org/abs/math/0602264 .
- [PS00] J´ ozef H. Przytycki and Adam S. Sikora. On skein algebras and Sl 2 ( C )-character varieties. Topology , 39(1):115-148, 2000.
- [QW21] Hoel Queffelec and Paul Wedrich. Khovanov homology and categorification of skein modules. Quantum Topol. , 12(1):129-209, 2021.
- [Tur91] Vladimir G. Turaev. Skein quantization of Poisson algebras of loops on surfaces. Annales scientifiques de l' ´ Ecole Normale Sup´ erieure , 24(6):635-704, 1991.

[Wal06] Kevin Walker. TQFTs. https://canyon23.net/math/tc.pdf , 2006.

Department of Mathematics, Yale University, New Haven, CT 06511, USA Email address : sam.panitch@yale.edu

Department of Mathematics, Harvard University, Cambridge, MA 02138, USA and Center of Mathematical Sciences and Applications, Harvard University, Cambridge, MA 02138, USA Email address : sunghyukpark@math.harvard.edu