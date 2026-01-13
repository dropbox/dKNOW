## From Link Homology to Topological Quantum Field Theories

Paul Wedrich

## Abstract

This survey reviews recent advances connecting link homology theories to invariants of smooth 4-manifolds and extended topological quantum field theories. Starting from joint work with Morrison and Walker, I explain how functorial link homologies that satisfy additional invariance conditions become diagram-independent, give rise to braided monoidal 2-categories, extend naturally to links in the 3-sphere, and globalize to skein modules for 4-manifolds. Later developments show that these skein lasagna modules furnish invariants of embedded and immersed surfaces and admit computation via handle decompositions. I then survey structural properties, explicit computations, and applications to exotic phenomena in 4-manifold topology, and place link homology and skein lasagna modules within the framework of extended topological quantum field theories.

## Contents

|   1 | Introduction                               |   1 |
|-----|--------------------------------------------|-----|
|   2 | Skein Theory from Link Homology            |   3 |
|   3 | Properties, Computations, and Applications |  10 |
|   4 | Topological Quantum Field Theory Context   |  13 |

## 1 Introduction

The study of link homology theories has revealed profound connections between low-dimensional topology, representation theory, and higher category theory. Originally pioneered by Khovanov [Kho00] in the form of a categorification of the Jones polynomial, link homology provides powerful knot invariants that detect subtle topological and geometric structures. By a link homology theory I mean a functor

<!-- formula-not-decoded -->

from the category of links in R 3 and link cobordisms in R 3 × I to the bounded homotopy category of chain complexes of graded projective R -modules, where

R denotes a graded commutative ring. The most important examples in what follows are the general linear link homologies , pioneered by Khovanov and Rozansky [KR08], which categorify the Reshetikhin-Turaev invariants for gl N , see § 2.1.

This survey reviews recent advances connecting link homology theories to invariants of smooth 4-manifolds 1 and extended topological quantum field theories (TQFTs). The starting point is joint work with Morrison and Walker [MWW22], which showed that link homology theories satisfying certain functoriality and monoidality requirements extend far beyond invariants of links in R 3 . They give rise to skein-theoretic invariants of oriented smooth 4-manifolds with boundary links, now known as skein lasagna modules 2 . These invariants admit computation along handle decomposition [MN22, MWW23, HRW22], furnish invariants of embedded and immersed surfaces [MWW24], and exhibit striking sensitivity-they distinguish smooth structures on 4-manifolds [RW24] and detect exotic surfaces [Sul25], while vanishing on CP 2 and S 2 × S 2 [SZ24].

The framework of skein theory also clarifies how link homology fits into the broader context of extended topological quantum field theories, as organized by the cobordism hypothesis , the tangle hypothesis , and the periodic table of k -tuply monoidal ( n -k )-categories [BD95, Lur09]. Classical skein modules of 3-manifolds, first studied by Przytycki and Turaev [Prz91, Tur88], form a (3 + ϵ )-dimensional TQFT 3 , which is completely determined by its value on the point 4 , namely the ribbon category encoding the local relations of the underlying link invariants, e.g. the Jones polynomial. Skein lasagna modules likewise form a (4 + ϵ )-dimensional TQFT, determined by a braided monoidal 2-category that encodes the local relations of the underlying link homology theory, e.g. Khovanov homology.

Skein theory based on link homology thus realizes part of the vision of Crane-Frenkel [CF94]: to use categorification , as motivated by Lusztig's canonical bases [Lus90], to construct algebraically computable 4-dimensional TQFTs of sensitivity comparable to Donaldson invariants (and later: Seiberg-Witten invariants). A fascinating early perspective on this proposal and its connection to many contemporary developments, which proved important, such as KapranovVoevodsky's braided monoidal 2-categories [KV94], the movie moves of CarterSaito [CS93], and the Crane-Yetter state-sum invariants for 4-manifolds [CY93], appears in Baez's This Week's Finds [Bae].

Outline This survey, written on the occasion of the 2025 International Congress of Basic Science, aims to provide an accessible overview of the state of the art in topological quantum field theories based on link homology via skein theory. After reviewing the extension from link homology to skein theory (Theorem 2.1), I discuss the general linear link homologies KhR N and the proof of their functoriality in S 3 via the sweep-around move, followed by the operadic setting for the definition

1 All 4-manifolds considered in this text are compact, smooth and oriented and links are framed and oriented, unless stated otherwise.

2 The name comes from [MN22]. Lasagna diagrams appear in [MWW22] as higher-dimensional analog of the spaghetti and meatball pictures for planar algebras attributed to Jones [Wal06].

3 An ( n + ϵ )-dimensional TQFT is an extended TQFT defined on manifolds of dimension between 0 and n , with diffeomorphisms of n -manifolds acting as isomorphisms on the top level.

4 The project of making this statement rigorous is still ongoing, see e.g. [Sch14, BJS21, Coo23].

of skein lasagna modules and the relation to braided monoidal 2-categories. Basic properties, computational techniques, and applications are collected in § 3, with a focus on invariants of embedded and immersed surfaces, handle-attachment formulas, and explicit computations leading to the detection of exotic smooth structures in [RW24]. Finally, § 4 situates skein lasagna modules in the context of extended TQFTs and discusses recent progress toward homotopy-coherent, chain-level versions.

Acknowledgements I am deeply grateful to Kim Morrison and Kevin Walker for our collaboration and many illuminating discussions, and to the organizers of the 2025 ICBS for the opportunity to present this work. 5

Funding I acknowledge support from the Deutsche Forschungsgemeinschaft (DFG, German Research Foundation) under Germany's Excellence Strategy - EXC 2121 'Quantum Universe' - 390833306 and the Collaborative Research Center - SFB 1624 'Higher structures, moduli spaces and integrability' - 506632645.

## 2 Skein Theory from Link Homology

The following theorem summarizes the main constructions of [MWW22] and is an extension of [MWW24, Theorem 2.1], allowing as target category V an arbitrary symmetric monoidal cocomplete 1-category, whose tensor product preserves colimits separately in each variable, and adding conclusion 4. A typical example is V = D( R -gmod), the derived category of chain complexes of graded modules over a (graded) commutative ring R , with link homology factoring through K b ( R -gpmod), the bounded homotopy category of graded projective R -modules.

Theorem 2.1 Suppose we are given a link homology functor for links in R 3 ,

<!-- formula-not-decoded -->

such that H is

- a. invariant under the trace of 2 π rotation of R 3 .
- b. monoidal 6 under disjoint union: H( L 1 ) ⊗ H( L 2 ) → H( L 1 ⊔ L 2 ) ,
- c. invariant under the sweep-around move, [MWW22, (1.1)]:

<!-- image -->

Then H extends naturally to:

1. A link homology for links in S 3 [MWW22, Definition 4.8], with values in V .

5 Portions of this article have been edited for clarity using generative AI.

6 Lax monoidality is sufficient. This structure makes H compatible with the E 3 -monoidal structures on Links ( R 3 ) and V , which are symmetric as we are working with plain 1-categories.

(2.1)

2. An algebra for the lasagna operad [MWW22, § 5.1], with values in V .
3. A (4 + ϵ ) -dimensional TQFT, whose top-dimensional layer is given by skein modules associated to pairs ( W,L ) of 4 -manifolds W with links L ⊂ ∂W taking values in V [MWW22, § 5.2].
4. A locally V -enriched braided monoidal 2-category with duals for objects, adjoints for 1-morphisms, and the analog of a ribbon structure [MWW22, § 6].

The constructions summarized in Theorem 2.1, which I survey in the following subsections, were already implicitly present in unpublished work of Morrison and Walker around 2007 [Wal06, Wal07], and in particular influenced the formulation of blob homology [MW11, MW12]. The theorem becomes meaningful once a link homology theory H satisfying the listed properties is provided.

## 2.1 General linear link homology

A main contribution of [MWW22] is to show that the gl N link homology theories satisfy the hypotheses of Theorem 2.1. These link homology theories are categorifications of the Reshetikhin-Turaev link invariants associated with the complex Lie algebra gl N . They were pioneered by Khovanov and Rozansky [KR08] and have been rediscovered and reconstructed using a variety of mathematical techniques, see e.g, [Str23] for a recent survey. Especially useful for our purposes is the combinatorial formulation using foams [MSV09, LQR15, QR16, RW16, RW20], which is functorial for links in R 3 by [ETW18] following [Bla10] for the case N = 2:

<!-- formula-not-decoded -->

The definition of the functor CKhR N proceeds diagrammatically and assigns algebraic data at three levels:

- Link diagrams: to each generic planar projection of a link, i.e. to each link diagram , CKhR N assigns a chain complex.
- Movies of link diagrams: each link cobordism, visualized as a movie of link diagrams, is decomposed into elementary movies (Reidemeister moves and Morse moves), and CKhR N assigns a chain map to each movie.
- Movie moves: the assignments must respect isotopies of cobordism relative to the boundary; specifically, for movies related by the so-called Carter-Saito movie moves [CS93], the associated chain maps must be homotopic.

Taking the homology of chain complexes produced by (2.2) yields a link homology theory KhR N valued in the category of bigraded abelian groups

<!-- formula-not-decoded -->

where one grading is by internal quantum degree , and the other is the homological grading of the chain complex. A framed link cobordism Σ from L 0 to L 1 induces a well-defined homomorphism

<!-- formula-not-decoded -->

that is homogeneous of quantum degree (1 -N ) χ (Σ) and homological degree zero.

Moreover, the requirements a. and b. of Theorem 2.1 are straightforward to verify for CKhR N and KhR N by means of the diagrammatic construction.

Example 2.2 The gl 2 homology agrees with Khovanov homology [Kho00] up to changes in normalization and, possibly, passing to the mirror link.

Example 2.3 The gl 1 homology of any framed link L is free of rank 1 , supported in quantum degree -f and homological degree f , where f denotes the self-linking of L [MWW24, § 3.2].

## 2.2 Link homology in the 3-sphere

The 3-sphere S 3 may be regarded as a one-point compactification of R 3 . A link in S 3 generically avoids the point at infinity and can thus be assumed to live in R 3 . Link cobordisms in S 3 × [0 , 1] can similarly be modeled entirely within R 3 × [0 , 1], away from infinity. The only subtlety arises when considering isotopies of cobordisms that pass through the point at infinity, which give rise to the sweeparound move (2.1).

Theorem 2.4 Let N ∈ Z ≥ 1 , then the gl N link homology functor (2.2) assigns the identity morphism to every instance of the sweep-around move (2.1) and hence satisfies the hypotheses of Theorem 2.1.

The difficulty of proving the sweep-around move, the main reason for the delay between [Wal07] and [MWW22], is that it is not a single move but an infinite family of moves, indexed by choices of tangles T in (2.1), which are non-local, at least from the perspective of link diagrams, far away from the point at infinity. The key idea for our proof in [MWW22] is a categorification of the Kauffman trick from [Kau87, Lemma 2.4], which exploits the dependence between the skein relation and Reidemeister moves of type 2 and 3. In our case, this enables a systematic comparison of chain maps associated to Reidemeister moves of type 3, which happen when the closure strand in the sweep-around move passes either in front or past the back of a diagram of the tangle T . This technique of proof applies to variations of gl N homology [MWW24, RW24, Sul25] and has been adapted to related settings [LS22, CY25].

Once the sweep-around move is established, the extension of the link homology functor to links in S 3 , as asserted in Theorem 2.1.1, proceeds in two main steps. Here I discuss them only at the level of links and refer to [MWW22, § 4.1-2] for details on the behavior under link cobordisms.

- Removal of the parametrization of R 3 . The groupoid of parametrizations of R 3 forms a torsor over the group of orientation-preserving diffeomorphisms Diff + ( R 3 ), which is path-connected with fundamental group π 1 (Diff + ( R 3 )) ∼ = Z / 2 Z , generated by a 2 π rotation of R 3 . By functoriality of the link homology in R 3 , any smooth path of parametrizations induces a link isotopy, and hence an isomorphism on link homology. Assumption a. in Theorem 2.1 guarantees that these isomorphisms depend only on the endpoints

- of the path, not on the particular choice of isotopy. The resulting invariant of a link in the unparametrized ambient R 3 is therefore the transitive system of all such homologies equipped with the canonical isomorphisms; equivalently, it can be described as the colimit over the groupoid of parametrizations.
- Extension to links in unparametrized 3-spheres. To define the invariant for a link L in S 3 , observe that any choice of base point p ∈ S 3 \ L presents L as a link in the 3-ball S 3 \ { p } . Moving p along a path in the link complement induces a canonical isomorphism between the corresponding link homologies. Since the fundamental group of the complement is generated by meridians of L , it suffices to check monodromy around these loops, which is precisely captured by the sweep-around move . Assumption c. in Theorem 2.1 ensures that this monodromy acts trivially. The invariant of L in S 3 is defined as the transitive system of all such link homologies equipped with the canonical isomorphisms, equivalently described as the colimit over the fundamental groupoid of the complement S 3 \ L .

## 2.3 Lasagna algebra

We are now ready to consider the relevant types of skeins. From now on we let W be a 4-manifold and L ⊂ ∂W a link, unless stated otherwise. We also fix N ∈ Z ≥ 1 and work with the link homology theory KhR N from § 2.1.

## Definition 2.5 One defines:

1. A lasagna skein F = (Σ , { ( B i , L i ) } ) of W with boundary L consists of:
- A finite collection of disjointly embedded 4 -balls B i ↪ → int W ; and
- A framed oriented surface Σ properly embedded in W \∪ i int B i , meeting ∂W in L and each ∂B i in a link L i .
2. A lasagna filling of W with boundary L is a lasagna skein as above with:
- For each i , a homogeneous label v i ∈ KhR N ( ∂B i , L i ) .
3. A lasagna diagram is a lasagna skein for W = B 4 .
4. The lasagna operad is a colored operad with
- set of colors given by the framed oriented links in S 3 ;
- set of operations given by lasagna diagrams as above, with the L i serving as inputs and L as output; and
- composition given by gluing a lasagna diagram to an input sphere of another lasagna diagram.

For a comparison with concepts from skein theory for 3-manifolds, see Table 1.

Theorem 2.6 ([MWW22, Theorem 5.2]) The link homology functor KhR N from (2.3) extends to an algebra for the lasagna operad.

This means that KhR N assigns a bigraded abelian group KhR N ( L ) to each link L ⊂ S 3 and further, a homogeneous morphism

<!-- formula-not-decoded -->

Figure 1: A lasagna filling of a generic 4-manifold W and a lasagna diagram.

<!-- image -->

to every lasagna diagram, such that gluing lasagna diagrams is compatible with composing morphisms. The idea of the proof uses that the relevant maps are already provided by the functoriality statement of Theorem 2.1.1 in the case of at most one input link. When considering more than one input link, the input spheres first have to be tubed together along an embedded graph, yielding a single input sphere with a split disjoint union of input links. One then uses the (lax) monoidality to define the associated map. Independence on the choice of tubing graph is a consequence of the sweep-around move.

Example 2.7 Each lasagna filling F of B 4 with boundary L and surface Σ yields an element KhR N ( F ) ∈ KhR N ( L ) by evaluating KhR N (Σ) on the tensor product of input labels v i of F .

## 2.4 Skein modules for 4-manifolds

An algebra for the lasagna operad provides both the labeling data for skein modules of 4-manifolds as well as the skein relations. As before we let W be a 4-manifold, L ⊂ ∂W a link, and N ∈ Z ≥ 1 . I will describe three equivalent definitions of the skein modules associated to the link homology KhR N .

Definition 2.8 The skein lasagna module

<!-- formula-not-decoded -->

is obtained as quotient of the bigraded abelian group freely generated by all lasagna fillings of W with boundary L , by the subgroup that enforce the transitive and linear closure of the following relations:

1. Linearity in the labels v i ∈ KhR N ( L i ) .
2. Equivalence under replacement of an input ball B i with a lasagna filling F

of a 4 -ball such that v i = KhR N ( F ) , followed by isotopy rel ∂W :

<!-- image -->

For the following alternative description, let C ( W,L ) denote the set of lasagna skeins of W with boundary L . For each such skein S ∈ C ( W,L ), we let input( S ) denote the finite set of input links L i in S .

Definition 2.9 The skein lasagna module is the quotient

<!-- formula-not-decoded -->

by the subgroup generated by the relators

<!-- formula-not-decoded -->

Here S, S ′ are lasagna skeins such that S ′ is obtained from S by attaching a lasagna diagram, i.e. a lasagna skein of B 4 , with underlying surface Σ , followed by an isotopy. The set J indexes input links of Σ , there is a unique output link of Σ , which also serves as input link for S and K indexes the remaining input links of S , which then also appear in S ′ .

The following reformulation appears in [RW24]. To formulate it, one considers C ( W,L ) as a category with morphisms S ′ → S generated by attachments of lasagna diagrams to input spheres of S , yielding S ′ up to a specified isotopy rel boundary, and with composition given by iterated attachments and composed isotopies. As a consequence of Theorem 2.6, one obtains a functor:

<!-- formula-not-decoded -->

Definition 2.10 The skein lasagna module is the colimit

<!-- formula-not-decoded -->

Note that this description directly generalizes to link homology theories with values in other symmetric monoidal cocomplete target categories V , provided the conditions of Theorem 2.1 are satisfied.

Remark 2.11 The setting of Theorem 2.1 is not the only one in which an extension from a categorical link invariant to a skein-theoretic 4-manifold invariant can be envisioned. For example, a construction based on link Floer homology appears in [Che22]. It uses a modified notion of skeins that accommodates links with multiple basepoints and link cobordisms with embedded arcs that connect basepoints.

## 2.5 Braided monoidal 2-categories

Given a link homology theory H satisfying the hypotheses of Theorem 2.1, the construction summarized there produces, in addition to a functorial link homology in S 3 , a lasagna algebra, and skein lasagna modules, a locally V -enriched braided monoidal 2-category C H . This construction was first provided in [MWW22, § 6] for the prototypical case of KhR N , carefully accounting for (semi-)strictness; here I describe the generalization only informally.

Objects of C H correspond to finite sequences of framed points in a 2-disk D 2 ⊂ R 2 , 1-morphisms are tangles in D 2 × [0 , 1] with horizontal composition implemented by stacking. The 2-morphisms between two tangles S and T with the same boundary are computed as the link homology H ( T ∪ ∂S = ∂T S ) ∈ V of the link obtained by gluing T to the mirror-reverse of S . The horizontal and vertical composition of 2-morphisms is implemented using the functoriality of H under link cobordisms, leading to V -enriched hom categories. The braided monoidal structure on this 2-category is inherited from the naturality of the construction under embedding little 2-disks in larger 2-disks. The requisite braiding data [KV94, BN96] on the level of 1-morphisms and 2-morphisms can be described explicitly in terms of certain shuffle braids and tangle cobordisms respectively. By construction, all objects in C H admit duals and all 1-morphisms admit adjoints [MWW22, § 6.4] and a categorification of the ribbon equation holds [MWW22, § 6.5].

Conceptually, this braided monoidal 2-category plays a role for the link homology theory H and its skein lasagna modules for smooth 4-manifolds analogous to that played by ribbon categories, e.g. of quantum group representations, in the decategorified setting of Reshetikhin-Turaev invariants [RT90] and associated skein modules of 3-manifolds, see Table 1. We will return to its role in determining extended TQFTs in § 4.

Table 1: Comparison of features between classical skein modules for 3-manifolds based on ribbon categories and skein lasagna modules for surfaces in 4-manifolds based on a link homology theory.

| feature                 | skein modules      | skein lasagna modules   |
|-------------------------|--------------------|-------------------------|
| link invariant          | Reshetikhin-Turaev | link homology theory H  |
| categorical data        | ribbon category    | ribbon 2-category C H   |
| ambient manifold        | oriented 3d        | oriented smooth 4d      |
| type of skeins          | ribbon graphs      | lasagna fillings        |
| local labelling at      | coupons            | input balls             |
| with boundary condition | points             | links                   |
| labelling by            | morphisms          | link homology classes   |

## 3 Properties, Computations, and Applications

This section discusses structural properties, computational techniques, and selected applications of gl N skein lasagna modules.

## 3.1 Basic Properties

This subsection collects the foundational properties of gl N skein lasagna modules that arise straightforwardly from their construction. These include functoriality, monoidality under disjoint union, and the behavior under standard gluing operations.

- Recovery of link homology: For the 4-ball B 4 and a link L ⊂ ∂B 4 ∼ = S 3 , we have a canonical isomorphism

<!-- formula-not-decoded -->

induced by decorating the radial skein. This further illustrates why the extension of KhR N to links in S 3 is essential for the skein module construction.

- Gradings: The skein module is Z × Z -graded by quantum and homological degree and decomposes further according to classes in relative second homology:

<!-- formula-not-decoded -->

Here H L 2 ( W ; Z ) := ∂ -1 ([ L ]) ⊂ H 2 ( W,L ; Z ) is the preimage of the fundamental class of L under the connecting map ∂ of the long exact sequence for relative homology; it is a torsor over H 2 ( W ; Z ).

- Gluing and functoriality under inclusions: When a 4-manifold W = W 1 ∪ Y W 2 is obtained by gluing 4-manifolds W 1 , W 2 along a common part Y of their boundaries, this induces a map on skein modules

<!-- formula-not-decoded -->

Here the boundary links are presented as unions of tangles L 1 , L 2 , L Y , L Y and L Y gets glued to L Y . As a consequence, skein modules are functorial under embeddings of 4-manifolds that are compatible with the boundary links. A detailed discussion appears in [MWW22, § 2.2]. More generally, (3.1) can be upgraded to a presentation of S N 0 ( W ; L 1 ∪ L 2 ) as relative tensor product of modules S N 0 ( W 1 ; L 1 ∪ -) and S N 0 ( W 2 ; - ∪ L 2 ) over a suitably defined linear skein category associated to ( Y ; ∂L 1 ) [Wal06, 4.4.2].

- Monoidality under disjoint unions and (boundary) connected sum: The skein module is (laxly) monoidal under disjoint union and over a field ❦ , this is a strong monoidal equivalence: S N 0 ( W 1 ; L 1 ; ) ⊗S N 0 ( W 2 ; L 2 ; ) ∼ = - → S N 0 ( W 1 ⊔ W 2 ; L 1 ⊔ L 2 ; ) ,

❦ ❦ ❦ The skein module also behaves monoidally under both connected sum and boundary connected sum [MN22, § 7].

## 3.2 Handle Attachments

Apowerful strategy for computing skein lasagna modules, introduced by Manolescu and Neithalath [MN22] and fully developed in [MWW23], is to proceed inductively along a handle decomposition of the 4-manifold W . Given a link L ⊂ ∂W and a handle decomposition of W ordered by index, the skein module of ( W ; L ) is computed in reverse , by successively removing handles and analyzing the their effect on the skein module-possibly altering the boundary link in the process. The computation terminates in a disjoint union of 4-balls, where the skein module reduces to a link homology calculation.

I now briefly summarize the effect of removing handles of each index on the gl N skein lasagna module, following the reverse computation strategy. For simplicity we work over a field.

- Four-handles: Attaching a 4-handle induces an isomorphism on skein modules, so 4-handles can also be freely removed, c.f. [MN22, Proposition 2.1].
- Three-handles: Attaching a 3-handle induces a surjection on skein modules, also proven in [MN22, Proposition 2.1]. The kernel of this surjection is described in [MWW23, § 3.2] as image of the difference of cobordism maps associated with the two attaching hemispheres of the 3-handle.
- Two-handles: The Manolescu-Neithalath 2-handle formula shows that the effect of attaching a 2-handle can simulated by inserting parallel cables of the attaching knot, performing a symmetrization procedure, and assembling the results into a filtered colimit as the number of strands tends to infinity. This approach was developed in [MN22, MWW23] and further perspectives in terms of Kirby-colored link homology are discussed in [HRW22, vM25].
- One-handles: Every 1-handle corresponds to a boundary connected sumpossibly a self-sum-along disjoint 3-balls in the boundary, and typically interacts with the boundary link. Algebraically, the effect on skein modules is described by computing co-invariants for a skein category associated to B 3 , which acts on both components of the attaching region of the 1handle [MWW23, § 4].
- Zero-handles: After all higher-index handles have been removed, the manifold becomes a disjoint union of 4-balls. The skein module is then computed as a tensor product of link homologies over the remaining boundary links.

In both the 1-handle and 2-handle cases, the reduction to the skein module of a manifold with the handle detached comes at the cost of considering infinite families boundary links. For 2-handles, these are parallel cables of the attaching knot, forming a natural and often partially computable family. In contrast, the 1-handle formula is significantly more difficult to control: the set of resulting boundary links is much less structured, and the co-invariant constructions involve actions by categories that are not yet well understood. As shown in [MWW23, Theorem 1.5], this can lead to skein modules that are not locally finite-dimensional.

It is an open question, for which 4-manifolds W and links L the skein module S N 0 ( W ; L ) is of finite-rank in each Z × Z × H L 2 ( W ; Z )-degree. Work in preparation by Qi-Robert-Sussan-Wagner addresses a refined question of finite generation by considering additional symmetries on equivariant skein lasagna modules.

## 3.3 Invariants of embedded surfaces

Skein lasagna modules provide a natural home for invariants of smoothly embeddedand even immersed-surfaces in 4-manifolds, just as skein modules based on ribbon categories serve as targets for invariants of framed links in 3-manifolds.

A key subtlety is that skein lasagna modules are spanned by lasagna fillings, which consist of oriented, framed surfaces. Since not every embedded surface admits a framing, one instead works with punctured surfaces: finitely many 4-balls are excised from the ambient manifold, puncturing the surface so that what remains becomes framable. The new boundary components can then be canonically decorated with specific link homology classes corresponding to nonzero-framed unknots, arising from cobordism maps associated to the Reidemeister move of type 1. This construction extends further to singular surfaces. Isolated singularities-such as transverse double points-can be modeled by removing neighborhoods and decorating their links (e.g., Hopf links) with canonical homology classes. In this way, skein lasagna modules yield invariants of immersed as well as embedded surfaces.

These skein elements can be viewed as generalizations of the relative KhovanovJacobsson classes associated to surfaces in the 4-ball [Jac04, SS22], which are known to distinguish certain exotic pairs of surfaces-embedded surfaces that are topologically but not smoothly isotopic [HS24].

Such skein classes also form the basis for extracting topological information about smooth surfaces in 4-manifolds, for instance lower bounds on the minimal genus in a given relative second homology class. In [MWW24], GL( N )-equivariant gl N link homology is used to establish such bounds by analyzing the grading support of skein modules modulo torsion, over the base ring H ∗ (BGL( N )). A related approach by Ren and Willis [RW24] uses a Lee-type deformation of gl 2 link homology to construct a filtered skein module, with the quantum filtration yielding lower bounds on the genus function. In the case of the 4-ball, this recovers Rasmussen's s-invariant [Ras10, BW08]. A different approach to extending the sinvariants to surfaces in certain other 4-manifold appears in [MMSW23].

Analyzing the torsion in Bar-Natan type deformations of gl 2 skein modules enables the detection of exotic pairs of knotted surfaces that remain exotic after one internal stabilization [Sul25].

## 3.4 Computations and Sensitivity towards Exotica

A range of explicit computations of skein lasagna modules have been carried out for small 4-manifolds and certain classes of links in the boundary, with a view towards testing the sensitivity of the invariant.

The sensitivity with respect to orientation was demonstrated in [MN22] through partial computations for CP 2 with both orientations. For links L in S 2 × S 1 , the invariant depends on the bulk 4-manifold: B 3 × S 1 often leads to invariants that are non locally finite-rank [MWW23], while S 2 × D 2 produces locally finite rank tensor multiples of the Rozansky-Willis invariant RW( L ) [Roz10, Wil21]. The latter description, together with the vanishing for S 2 × S 2 , was established by Sullivan-Zhang [SZ24].

Table 2: Sample computations of skein lasagna modules for various 4-manifolds.

| W 4       | L         | S 2 0 ( W 4 ; L )                                                          | Reference          |
|-----------|-----------|----------------------------------------------------------------------------|--------------------|
| S 4       | ∅         |                                                                            | [MWW22]            |
| B 3 × S 1 | ⊔ 2 m S 1 | loc. finite rank exactly for m ≤ 1                                         | [MWW23]            |
| S 2 × D 2 | ∅ L       | ❦ ❦ [ x,x - 1 ] /x ❦ [ x ] per H 2 class S 2 0 ( S 2 × D 2 ; ∅ ) ⊗ RW( L ) | [MN22] [SZ24]      |
| S 2 × S 2 | ∅         | 0                                                                          | [SZ24]             |
| CP 2      | ∅         | 0                                                                          | [MN22, RW24]       |
| CP 2      | ∅         | nonzero, description conjectural                                           | [MN22, RW24, vM25] |

The remarkable recent preprint of Ren and Willis [RW24] contains many additional computations beyond Table 2, including a comparison of the skein modules of the exotic pair of knot traces X -1 ( -5 2 ) and X -1 ( P (3 , -3 , 8)). In generating second homology classes and homological degree zero, these skein modules differ in quantum degree -1 over Q . Thus skein lasagna modules can detect exotic smooth structure by purely algebro-combinatorial means. Also notable are vanishing results for skein lasagna modules, e.g. for 4-manifolds that contain a positive self-intersection embedded S 2 [RW24, Theorem 1.4], which use the fact that having vanishing skein module is a property that is inherited under embeddings.

## 4 Topological Quantum Field Theory Context

The skein lasagna modules described in this survey arise as the 4-dimensional layer of an extended local topological quantum field theory (TQFT), whose associated manifold invariants can be described in skein-theoretic terms for oriented manifolds of dimensions up to 4. This theory is determined by the braided monoidal 2category C H extracted in § 2.5 from the underlying link homology H.

Braided monoidal 2-categories appear in the periodic table of n -categories [BD95] as categorification of braided monoidal categories 7 . Link homology and functoriality under cobordism maps are natural consequences, in analogy to how Reshetkhin-Turaev tangle invariants are captured by ribbon categories.

| E k \ n - k   | 0             | 1              | 2                | · · ·   |
|---------------|---------------|----------------|------------------|---------|
| -             | sets          | categories     | 2-categories     | · · ·   |
| E 1           | monoids       | monoidal cats  | monoidal 2-cats  | · · ·   |
| E 2           | comm. monoids | braided cats   | braided 2-cats   | · · ·   |
| E 3           | -             | sym. mon. cats | sylleptic 2-cats | · · ·   |
| E 4           | -             | -              | sym. mon. 2-cats | · · ·   |
| . . .         | -             | -              | -                | . . .   |

7 The colors in the table indicate equal total categorical dimension n = ( n -k ) + k ; categorification usually means passing from a cell to the adjacent cell with higher column index.

This perspective situates skein lasagna modules within a broader hierarchy of skein-theoretic topological quantum field theories. Classical instances arise from the 2-dimensional graphical calculus of monoidal categories or the 3-dimensional graphical calculus of braided monoidal categories. These are often referred to as the Turaev-Viro and Crane-Yetter families of TQFTs, though strictly speaking those names are more closely tied to the corresponding state-sum models, which extend such theories up by one dimension, provided additional strong finiteness conditions are satisfied [TV92, CY93].

In my Frontiers of Science Award Lecture at the International Congress of Basic Sciences 2025 [Wed25], I outlined the current progress toward categorified analogues of these theories based on monoidal 2-categories and braided monoidal 2categories, emphasizing in particular the role of the chosen target higher category.

| E k \ n - k   | linear categories   | loc. linear 2-categories   | loc. stable ( ∞ , 2)-categories   |
|---------------|---------------------|----------------------------|-----------------------------------|
| monoidal      | Turaev-Viro         | Asaeda-Frohman-Kaiser      | [HRW24] 4.2                       |
| braided       | Crane-Yetter        | [MWW22]                    | [LMGRSW24] 4.1                    |

The main difference is whether one considers local enrichment in a symmetric monoidal 1-category, such as (graded) abelian groups or vector spaces, or in fact in a symmetric monoidal stable ∞ -category, such as chain complexes. In the former case, monoidal 2-categories lead to skein theories studied by Asaeda-Frohman and Kaiser [AF07, Kai25] and many others, or to the invariants of DouglasReutter [DR18], while braided monoidal 2-categories lead to lasagna skein modules as in [MWW22].

The next two sections outline the state of chain level versions, which arise when the base of enrichment is upgraded to a symmetric monoidal stable ∞ -category, such as chain complexes, i.e. the last column of the above table.

## 4.1 Towards a chain level version

Link homology theories are typically constructed from chain complexes associated to link diagrams. This naturally raises the question whether one can promote the functoriality of link homology to the chain level, in such a way that it becomes homotopy-coherent.

Conjecture 4.1 The chain level gl N link invariant from (2.2) arises as truncation of an E 3 -monoidal functor of ( ∞ , 1) -categories

<!-- formula-not-decoded -->

from the ( ∞ , 1) -category Links ∞ ( R 3 ) of links in R 3 , link cobordisms in R 3 × I , isotopies, and higher isotopies, to the ( ∞ , 1) -category of bounded chain complexes, chain maps, homotopies, and higher homotopies.

The traditional approach to functoriality via movie moves breaks down at the chain level, since one encounters an infinite hierarchy of higher relations between

movies. Instead, a more conceptual framework is required. The guiding idea is to capture the necessary homotopy-coherence through local data: namely, an E 2 -monoidal ( ∞ , 2)-category, generated by 2-dualizable objects and equipped with an SO(4)-homotopy-fixed-point structure. This would serve as the chain-level, categorified analogue of the ribbon categories underlying the Reshetikhin-Turaev tangle invariants, with the functor (4.1) recoverable by restriction to tangles without endpoints.

While a complete chain-level theory is still under development, several precursor results point in this direction. In joint work with Stroppel (2021), we established the homotopy-coherent naturality of the Rouquier braiding [Rou17] in a concrete dg model for chain complexes of Soergel bimodules. This result, now documented in [SW24], led to Conjecture 3.8 in Stroppel's ICM article [Str23].

In collaboration with Liu, Mazel-Gee, Reutter, and Stroppel [LMGRSW24], we resolved this conjecture by constructing an E 2 -monoidal ( ∞ , 2)-category of chain complexes of Soergel bimodules. This structure underlies braid invariants feeding into gl N link homologies and provides a categorical foundation for triply graded homology theories [Kho07]. Further joint work with Dyckerhoff [DW25] relates the braiding on complexes of Soergel bimodules to the concept of perverse schobers, as proposed by Kapranov-Schechtman [KS16, KS15, KS21]. A key ingredient in this connection are singular Soergel bimodules [Wil08], which can be obtained from Soergel bimodules via higher-categorical idempotent completion [Rec25].

A current limitation of the E 2 -monoidal ( ∞ , 2)-category of Soergel complexes is that its generating object is not dualizable. As a consequence, the associated invariant extends to braids but not to tangles or general tangle cobordisms, leaving Conjecture 4.1 unresolved. We plan to remedy this issue by proceeding to gl N quotients in future work.

Homotopy-coherent, chain-level link invariants would have important advantages beyond their intrinsic structural appeal. They have the potential to capture finer topological information and improve the computability of associated invariants. This is already visible for Rouquier complexes of braids: when considering braid closures in the annulus via categorical traces [GHW21], higher homotopical data is essential for cabling operations [GW23, § 6.4]. More generally, chain-level tangle invariants provide the natural framework for categorifying skein algebras, avoiding the semisimplification procedures otherwise required [QW21].

## 4.2 Towards a chain level version in 3d

The guiding idea for a chain-level version of skein theory for surfaces in 3-manifolds is that it should yield categorified, partially defined analogs of TQFTs in the Turaev-Viro family. Conceptually, such a theory ought to be based on an E 1 -monoidal locally stable ( ∞ , 2)-category generated by 2-dualizable objects, equipped with an SO(3)-homotopy-fixed-point structure. Assuming link homology has been modeled locally and homotopy coherently as outlined in § 4.1, such a structure can be obtained by forgetting from E 2 -monoidality to E 1 , i.e. by discarding the braiding. Since the braiding is the most intricate part of the higher-categorical data,

E 1 -examples are comparatively more accessible directly-for instance by viewing the locally linear monoidal 2-categories underlying the Asaeda-Frohman-Kaiser TQFT as enriched in chain complexes.

This approach was initiated in [HRW24], which begins with the Bar-Natan monoidal 2-category, the categorification of the Temperley-Lieb monoidal category underlying combinatorial constructions of Khovanov homology [BN05]. Mirroring Roberts's skein-theoretic description of the Turaev-Viro theory [Rob95], we obtain a categorified analog of the Turaev-Viro TQFT in low dimensions. The main result of [HRW24] is the explicit construction and characterization of the invariant of 2dimensional 1-handlebodies, which takes the form of certain dg categories in the simplest cases. These categories are generated by objects parametrized by spin networks adapted to a triangulation of the surface. Their hom pairings yield power series in the variable q , whose graded Euler characteristics recover the Turaev-Viro hermitian pairing when q is specialized to a complex root of unity. Ongoing work with Hogancamp and Rose extends this construction to the 3-dimensional level and closed surfaces.

## References

| [AF07]   | Marta Asaeda and Charles Frohman. A note on the Bar-Natan skein module. Internat. J. Math. , 18(10):1225-1243, 2007.                                             |
|----------|------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| [Bae]    | John C. Baez. This Week's Finds. available at https://math.ucr.edu/home/baez/twf.html, accessed 25.08.2025.                                                      |
| [BD95]   | John C. Baez and James Dolan. Higher-dimensional algebra and topological quantum field theory. J. Math. Phys. , 36(11):6073- 6105, 1995.                         |
| [BJS21]  | Adrien Brochier, David Jordan, and Noah Snyder. On dualizability of braided tensor categories. Compos. Math. , 157(3):435-483, 2021.                             |
| [Bla10]  | Christian Blanchet. An oriented model for Khovanov homology. J. Knot Theory Ramifications , 19(2):291-312, 2010.                                                 |
| [BN96]   | John C. Baez and Martin Neuchl. Higher-dimensional algebra. I. Braided monoidal 2-categories. Adv. Math. , 121(2):196-244, 1996.                                 |
| [BN05]   | Dror Bar-Natan. Khovanov's homology for tangles and cobordisms. Geom. Topol. , 9:1443-1499, 2005.                                                                |
| [BW08]   | Anna Beliakova and Stephan Wehrli. Categorification of the col- ored Jones polynomial and Rasmussen invariant of links. Canad. J. Math. , 60(6):1240-1266, 2008. |
| [CF94]   | Louis Crane and Igor B. Frenkel. Four-dimensional topological quantum field theory, Hopf categories, and the canonical bases. volume 35, pages 5136-5154. 1994.  |
| [Che22]  | Daren Chen. Floer lasagna modules from link Floer homology, 2022. arXiv:2203.07650 .                                                                             |
| [Coo23]  | Juliet Cooke. Excision of skein categories and factorisation homol- ogy. Adv. Math. , 414:Paper No. 108848, 51, 2023.                                            |

- [CS93] J. Scott Carter and Masahico Saito. Reidemeister moves for surface isotopies and their interpretation as moves to movies. J. Knot Theory Ramifications , 2(3):251-284, 1993.
- [CY93] Louis Crane and David Yetter. A categorical construction of 4d topological quantum field theories. In Quantum topology , volume 3 of Ser. Knots Everything , pages 120-130. World Sci. Publ., River Edge, NJ, 1993.
- [CY25] Daren Chen and Hongjian Yang. The flip map and involutions on khovanov homology, 2025. arXiv:2506.00824 .
- [DR18] Christopher L. Douglas and David J. Reutter. Fusion 2-categories and a state-sum invariant for 4-manifolds, 2018. arXiv:1812.11933 .
- [DW25] Tobias Dyckerhoff and Paul Wedrich. Perverse schobers of coxeter type A , 2025. arXiv:2504.08496 .
- [ETW18] Michael Ehrig, Daniel Tubbenhauer, and Paul Wedrich. Functoriality of colored link homologies. Proc. Lond. Math. Soc. (3) , 117(5):996-1040, 2018.
- [GHW21] Eugene Gorsky, Matthew Hogancamp, and Paul Wedrich. Derived traces of Soergel categories. Int. Math. Res. Not. IMRN , 2021.
- [GW23] Eugene Gorsky and Paul Wedrich. Evaluations of annular Khovanov-Rozansky homology. Math. Z. , 303(1):Paper No. 25, 57, 2023.
- [HRW22] Matthew Hogancamp, D. E. V. Rose, and Paul Wedrich. A Kirby color for Khovanov homology, 2022. arXiv:2210.05640 , to appear in J. Eur. Math. Soc.
- [HRW24] Matthew Hogancamp, D. E. V. Rose, and Paul Wedrich. Bordered invariants from Khovanov homology, 2024. arXiv:2404.06301 .
- [HS24] Kyle Hayden and Isaac Sundberg. Khovanov homology and exotic surfaces in the 4-ball. J. Reine Angew. Math. , 809:217-246, 2024.
- [Jac04] M. Jacobsson. An invariant of link cobordisms from Khovanov homology. Algebr. Geom. Topol. , 4:1211-1251 (electronic), 2004.
- [Kai25] Uwe Kaiser. Bar-Natan theory and tunneling between incompressible surfaces in 3-manifolds. Topology Appl. , 369:Paper No. 109390, 54, 2025.
- [Kau87] Louis H. Kauffman. State models and the Jones polynomial. Topology , 26(3):395-407, 1987.
- [Kho00] M. Khovanov. A categorification of the Jones polynomial. Duke Math. J. , 101(3):359-426, 2000.
- [Kho07] Mikhail Khovanov. Triply-graded link homology and Hochschild homology of Soergel bimodules. Internat. J. Math. , 18(8):869-885, 2007.
- [KR08] Mikhail Khovanov and Lev Rozansky. Matrix factorizations and link homology. Fund. Math. , 199(1):1-91, 2008.
- [KS15] Mikhail Kapranov and Vadim Schechtman. Perverse schobers, 2015. arXiv:1411.2772 .
- [KS16] Mikhail Kapranov and Vadim Schechtman. Perverse sheaves over real hyperplane arrangements. Ann. of Math. (2) , 183(2):619-679, 2016.

| [KS21]     | Mikhail Kapranov and Vadim Schechtman. PROBs and perverse sheaves I. Symmetric products, 2021. arXiv:2102.13321 .                                                                                                                                                                                                 |
|------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| [KV94]     | M. M. Kapranov and V. A. Voevodsky. 2-categories and Zamolod- chikov tetrahedra equations. In Algebraic groups and their gener- alizations: quantum and infinite-dimensional methods (University Park, PA, 1991) , volume 56 of Proc. Sympos. Pure Math. , pages 177-259. Amer. Math. Soc., Providence, RI, 1994. |
| [LMGRSW24] | Yu Leon Liu, Aaron Mazel-Gee, David Reutter, Catharina Strop- pel, and Paul Wedrich. A braided ( ∞ , 2)-category of Soergel bi- modules, 2024. arXiv:2401.02956 .                                                                                                                                                 |
| [LQR15]    | A.D. Lauda, H. Queffelec, and D.E.V. Rose. Khovanov homology is a skew Howe 2-representation of categorified quantum sl ( m ). Algebr. Geom. Topol. , 15(5):2517-2608, 2015.                                                                                                                                      |
| [LS22]     | Robert Lipshitz and Sucharit Sarkar. A mixed invariant of nonori- entable surfaces in equivariant Khovanov homology. Trans. Amer. Math. Soc. , 375(12):8807-8849, 2022.                                                                                                                                           |
| [Lur09]    | Jacob Lurie. On the classification of topological field theories. In Current developments in mathematics, 2008 , pages 129-280. Int. Press, Somerville, MA, 2009.                                                                                                                                                 |
| [Lus90]    | G. Lusztig. Canonical bases arising from quantized enveloping al- gebras. J. Amer. Math. Soc. , 3(2):447-498, 1990.                                                                                                                                                                                               |
| [MMSW23]   | Ciprian Manolescu, Marco Marengon, Sucharit Sarkar, and Michael Willis. A generalization of Rasmussen's invariant, with applications to surfaces in some four-manifolds. Duke Math. J. , 172(2):231-311, 2023.                                                                                                    |
| [MN22]     | Ciprian Manolescu and Ikshu Neithalath. Skein lasagna modules for 2-handlebodies. J. Reine Angew. Math. , 788:37-76, 2022.                                                                                                                                                                                        |
| [MSV09]    | M. Mackaay, M. Stoˇ si´ c, and P. Vaz. sl N -link homology ( N ≥ 4) us- ing foams and the Kapustin-Li formula. Geom. Topol. , 13(2):1075- 1128, 2009.                                                                                                                                                             |
| [MW11]     | Scott Morrison and Kevin Walker. Higher categories, colimits, and the blob complex. Proc. Natl. Acad. Sci. USA , 108(20):8139-8145, 2011.                                                                                                                                                                         |
| [MW12]     | Scott Morrison and Kevin Walker. Blob homology. Geom. Topol. , 16(3):1481-1607, 2012.                                                                                                                                                                                                                             |
| [MWW22]    | Scott Morrison, Kevin Walker, and Paul Wedrich. Invariants of 4- manifolds from Khovanov-Rozansky link homology. Geom. Topol. , 26(8):3367-3420, 2022.                                                                                                                                                            |
| [MWW23]    | Ciprian Manolescu, Kevin Walker, and Paul Wedrich. Skein lasagna modules and handle decompositions. Adv. Math. , 425:Pa- per No. 109071, 40, 2023.                                                                                                                                                                |
| [MWW24]    | Scott Morrison, Kevin Walker, and Paul Wedrich. Invariants of surfaces in smooth 4-manifolds from link homology, 2024. arXiv:2401.06600 .                                                                                                                                                                         |
| [Prz91]    | J´ ozef H. Przytycki. Skein modules of 3-manifolds. Bull. Polish Acad. Sci. Math. , 39(1-2):91-100, 1991.                                                                                                                                                                                                         |

- [QR16] H. Queffelec and D.E.V. Rose. The sl n foam 2-category: a combinatorial formulation of Khovanov-Rozansky homology via categorical skew Howe duality. Adv. Math. , 302:1251-1339, 2016.
- [QW21] Hoel Queffelec and Paul Wedrich. Khovanov homology and categorification of skein modules. Quantum Topol. , 12(1):129-209, 2021.
- [Ras10] J. Rasmussen. Khovanov homology and the slice genus. Invent. Math. , 182(2):419-447, 2010.
- [Rec25] Isabela Recio. Higher idempotent completion for soergel bimodules, 2025. arXiv:2508.00767 .
- [Rob95] Justin Roberts. Skein theory and Turaev-Viro invariants. Topology , 34(4):771-787, 1995.
- [Rou17] R. Rouquier. Khovanov-Rozansky homology and 2-braid groups. In Categorification in geometry, topology, and physics , 2017.
- [Roz10] Lev Rozansky. A categorification of the stable SU(2) WittenReshetikhin-Turaev invariant of links in S2 x S1, 2010. arXiv:1011.1958 .
- [RT90] N. Yu. Reshetikhin and V. G. Turaev. Ribbon graphs and their invariants derived from quantum groups. Comm. Math. Phys. , 127(1):1-26, 1990.
- [RW16] David E.V. Rose and Paul Wedrich. Deformations of colored sl ( n ) link homologies via foams. Geom. Topol. , 20(6):3431-3517, 2016.
- [RW20] Louis-Hadrien Robert and Emmanuel Wagner. A closed formula for the evaluation of foams. Quantum Topol. , 11(3):411-487, 2020.
- [RW24] Qiuyu Ren and Michael Willis. Khovanov homology and exotic 4-manifolds, 2024. arXiv:2402.10452 .
- [Sch14] Claudia Scheimbauer. Factorization homology as a fully extended topological field theory, 2014. Dissertation, ETH Zurich, http: //scheimbauer.at/ScheimbauerThesis.pdf .
- [SS22] Isaac Sundberg and Jonah Swann. Relative Khovanov-Jacobsson classes. Algebr. Geom. Topol. , 22(8):3983-4008, 2022.
- [Str23] Catharina Stroppel. Categorification: tangle invariants and TQFTs. In ICM-International Congress of Mathematicians. Vol. II. Plenary lectures , pages 1312-1353. EMS Press, Berlin, [2023] © 2023.
- [Sul25] Ian A. Sullivan. Bar-Natan skein lasagna modules and exotic surfaces in 4-manifolds, 2025. arXiv:2504.03968 .
- [SW24] Catharina Stroppel and Paul Wedrich. Braiding on type A Soergel bimodules: semistrictness and naturality, 2024. arXiv:2412.20587 .
- [SZ24] Ian A. Sullivan and Melissa Zhang. Kirby belts, categorified projectors, and the skein lasagna module of S 2 × S 2 , 2024. arXiv:2402.01081 .
- [Tur88] V. G. Turaev. The Conway and Kauffman modules of a solid torus. Zap. Nauchn. Sem. Leningrad. Otdel. Mat. Inst. Steklov. (LOMI) , 167(Issled. Topol. 6):79-89, 190, 1988.
- [TV92] V. G. Turaev and O. Ya. Viro. State sum invariants of 3-manifolds and quantum 6 j -symbols. Topology , 31(4):865-902, 1992.

- [vM25] Karim Ritter von Merkl. Computing colored Khovanov homology, 2025. arXiv:2505.03916 .
- [Wal06] Kevin Walker. Tqfts, May 2006. Notes available at https://canyon23.net/math/, accessed 25.08.2025.
- [Wal07] Kevin Walker. Khovanov homology as a TQFT, April 2007. Talk notes available at https://canyon23.net/math/talks/, accessed 14.08.2025.
- [Wed25] Paul Wedrich. From link homology to topological quantum field theories, July 2025. 2025 Frontiers of Science Award lecture, recording available at https://www.youtube.com/watch?v= G4ZHSR1S\_oY , accessed 19.08.2025.
- [Wil08] Geordie Williamson. Singular Soergel bimodules, 2008. Dissertation, Freiburg, http://people.mpim-bonn.mpg.de/geordie/ GW-thesis.pdf .
- [Wil21] Michael Willis. Khovanov homology for links in # r ( S 2 × S 1 ). Michigan Math. J. , 70(4):675-748, 2021.

Fachbereich Mathematik, Universit¨ at Hamburg, Bundesstraße 55, 20146 Hamburg, Germany paul.wedrich.at E-mail address : paul.wedrich@uni-hamburg.de