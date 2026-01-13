## Shape-specific fluctuations of an active colloidal interface

Arvin Gopal Subramaniam, 1, 2, ∗ Tirthankar Banerjee, 3, † and Rajesh Singh 1, 2, ‡

1 Department of Physics, Indian Institute of Technology Madras, Chennai, India

2 Center for Soft and Biological Matter, IIT Madras, Chennai, India 3 Department of Physics and Materials Science, University of Luxembourg, Luxembourg, Luxembourg

## Abstract

Motivated by a recently synthesized class of active interfaces formed by linked self-propelled colloids, we investigate the dynamics and fluctuations of a phoretically (chemically) interacting active interface with roto-translational coupling. We enumerate all steady-state shapes of the interface across parameter space and identify a regime where the interface acquires a finite curvature, leading to a characteristic 'C-shaped' topology, along with persistent self-propulsion. In this phase, the interface height fluctuations obey Family-Vicsek scaling but with novel exponents: a dynamic exponent z h ≈ 0 . 6, a roughness exponent α h ≈ 0 . 9 and a super-ballistic growth exponent β h ≈ 1 . 5. In contrast, the orientational fluctuations of the colloidal monomers exhibit a negative roughness exponent, reflecting a surprising smoothness law , where steady-state fluctuations diminish with increasing system size. Together, these findings reveal a unique non-equilibrium universality class associated with self-propelled interfaces of non-standard shape.

## I. INTRODUCTION

Out-of-equilibrium agents with suitably chosen interactions are known to exhibit emergent collective order, most prominently in the form of global polar order [1, 2]. Such order can arise from alignment rules [2, 3], topological interactions [4], long-ranged attraction or repulsion [5, 6], and/or other generic behavioral couplings [7]. In parallel, a distinct body of work in statistical physics has established a deep understanding of fluctuating interfaces, where the statistical properties of the interface height obey universal scaling laws [8-11]. A cornerstone result in this field is the Family-Vicsek (FV) scaling law, which relates the roughness of the interface to the system size and time through universal exponents [12, 13]. These exponents are known for certain solvable models, such as the Edwards-Wilkinson (EW) and Kardar-Parisi-Zhang (KPZ) equations [8, 9, 14], while numerical and experimental studies have revealed a wealth of deviations, suggesting novel universality classes [15-18]. Bridging these two domains, the study of active interfaces , i.e., interfaces driven out of equilibrium by internal activity, is a subject of recent interest [19-23], raising new questions about how activity reshapes interfacial fluctuations and scaling behavior.

∗ ph22d800@smail.iitm.ac.in

† tirthankar.banerjee@uni.lu

‡ rsingh@physics.iitm.ac.in

In this article, we study a polar colloidal chain as a non-equilibrium interface in 1 + 1 dimensions. This is inspired by recent experiments that have been able to synthesize an autonomous polar chain via chemical self-interactions [24], as opposed to other propulsion mechanisms via external actuation [25-27]. We study the dynamical steady-states of this system and show that within a specific parameter regime the chain ballistically propels with a deterministic 'C-shape' (reported previously in [24, 28]). As opposed to conventional growing (circular) interfaces, the non-equilibrium interface we study here displays the additional phenomena (in addition to being propelled) of curvature acquisition during its dynamics, attained via the inter-monomeric phoretic interactions, thus adding additional time scales to the conventional early time deterministic plus late time diffusive behaviour. The polar nature of the interface (and hence the reason it breaks translational symmetry and propels in a given direction) enables calculations of height fluctuations of the monomers about a mean height [10] defined across the interface.

We report that the interface height fluctuations of this chain obey FV scaling, characterized by a dynamic ( z h ≈ 0 . 6) and a roughness ( α h ≈ 0 . 9) exponent. Correspondingly, the fluctuations preceding the steady state are captured by a super-ballistic growth exponent β h ≈ 1 . 5. The steady-state orientational fluctuations of the colloidal monomers, in contrast, decrease with the chain length, exhibiting a negative roughness (i.e. 'smoothness') exponent ( α θ ≈ -0 . 5). Moreover, we report a distinct set of scaling exponents characterizing the locally flat regime, defined by monomer-level averages restricted to flat portions of the chain and, independently, by taking the infinite-chain limit. Given that the shape of the propelling interface does not fall under conventionally studied circular or flat interfaces [29], these exponents thus constitute a unique non-equilibrium signature of an interface with a stereotypic 'C-shape' topology.

The paper is structured as follows. In Sect. II, we introduce the model and enumerate the relevant length and time scales of our model. In Sec. III , we study the dynamical regimes of the model as a function of selected dimensionless numbers and obtain a phase diagram [Fig. 1]. We report the results for the scalings of height and orientation fluctuations in Sec. IV. We finally discuss the significance of our findings and future directions in Sec. V.

102

10'

Ã

10°

10-

10-6

10-3

10'

FIG. 1. Phase diagram in Λ- ˜ Λ plane. The log curvature has been used to delineate the phases. The phase diagram has been drawn with for a chain with N = 256 number of monomers. Representative images for each phase is shown along with the marker key for a smaller chain for clarity. See text for details.

<!-- image -->

## II. MODEL

We model the i th active droplet as a colloid particle centered at r i = ( x i , y i ). The particle is confined to move in two-dimensions. It self-propels with a speed v s , along the directions e i = (cos θ i , sin θ i ). The direction of the particle, given by the angle θ , changes due to coupling to a phoretic field c ( r , t ). The position and orientation of the i th particle are determined by the following evolution equations:

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

In the above equations, µ is mobility, D t and D r , respectively, are translational and rotational diffusion constants of the particle, while ξ t and ξ r are white noises with zero mean and unit variances. The constant χ r is taken to be positive so that the particles rotate away from each other (chemo-repulsive). The force on the i th particle is given as:

<!-- formula-not-decoded -->

Here, U b ( r i , r j ) = k ( r ij -r 0 ) 2 is a harmonic potential of stiffness k and natural length r 0 which holds the chain together, while r ij = | r i -r j | . The chemical interactions between the monomers of the chain are contained in J i = -∂c/∂ r i , where c is the concentration of the phoretic field. It is worthwhile to note that the positional and orientational dynamics are coupled in our model. This roto-translational coupling leads to rich phenomenology of our model, as we describe below.

C

C-shape

Stiff

Disordered

Frustrated

In the steady-state, the solution of the concentration profile c ( r ) follows from the equation: D c ∇ 2 c ( r , t ) + ∑ N i =1 c 0 δ ( r -r i ) = 0, where D c is the diffusion coefficient of the filled micelles and c 0 is emission constant of the micelles. This gives the expression for the current:

̸

<!-- formula-not-decoded -->

This current is thus interpreted as that which is instantaneously deposited on the centers of the colloids. For the entire paper (unless specified otherwise) we use the following initial conditions:

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

where b is the radius of each colloid. Note that r 0 = 2 b . The above model has been studied recently [24, 28]; a detailed phase diagram of the model was obtained in [28] in terms of the dimensionless group of the system. Here, we focus on the fluctuations in the so-called 'C-shape' phase, where the chain spontaneously acquires a stereotypic shape resembling the alphabet C, and propels in a direction normal to its tangent [24, 28]. We note that the 'C-shape' topology we refer to is specific to systems in which roto-translational coupling in the dynamics induces the steady-state shape. The system we study being 'dry' with such a coupling, is distinct from other similar topologies seen in (for instance) the physics of sedimenting filaments [30]. In the limit of N →∞ it corresponds to an interface which is moving. A schematic of this is shown in Fig. 2. We focus on the fluctuations of that moving interface in this paper.

All numerical results in this paper are generated with an explicit Euler-Maruyama integrator with time step dt = 0 . 01 and simulation time T = 7 . 2 × 10 5 . For all simulations here (unless stated otherwise) b = 1, c 0 = 1, D c = 1, ˆ µ = µk b = 91 . 6 s -1 , and v s = 1. Wherever realization average is performed, we use 50 realizations, with the exception of 10 used in Appendix 7.

## III. DYNAMICAL REGIMES

We first define some typical time scales, ratios of which determine the relevant dimensionless numbers needed to understand the phase diagram (Fig. 1):

<!-- formula-not-decoded -->

Here, τ is a spontaneous propulsion time scale, which is the time it takes for an isolated particle to move a distance equaling its radius in absence of any reorientation, τ f sets the average time during which the orientation of the colloid changes in response to the chemical field. τ t and τ f are the time scales set by the translation noise D t and the rotational noise D r , respectively. Competition between these time scales gives rise to different dynamical and scaling regimes of the propelling interface.

We define two dimensionless activity parameters:

<!-- formula-not-decoded -->

The dimensionless number Λ quantifies the competition between deterministic rotations (due to phoretic interactions) and deterministic propulsion. The dimensionless number ˜ Λ quantifies deterministic and random motion in the positional sector. A variant on the above model, section II, was introduced in [24] and studied further in [28]. In both these papers, we had considered the role of trail created by the particles. Here, we first assume instantaneous chemical interactions, and discuss the effect of trail-mediated history later (in Appendix 7). Additionally, we note that the previous works ignored the role of translational noise D t . In [28], it was found that a competition between τ r and the chemical diffusivity of trails leads to the formation of a stable C-shape chain.

We now present the dynamics regimes of the model described above in terms of the two dimensionless numbers. In Fig. 1, we present a phase diagram in the plane of Λ and ˜ Λ. We find that the C-shape is sustained for a selected range of dimensionless numbers Λ and ˜ Λ; the details of this phase have already been discussed in [28]. For regions ˜ Λ &lt; 10 0 , we find a disordered phase with effectively zero positional order in the chain. Using the exact simulation values, we may define a length scale D t v s which would correspond to D t v s &gt; b , thus we conclude that long-wavelength fluctuations (that is, greater than the monomer size) are not

supported by our colloidal chain. Focusing in the regime ˜ Λ &gt; 10 1 and Λ &lt; 10 -3 corresponds to the case where deterministic rotations are instantaneous across the system ( τ f &lt;&lt; τ ), such that the chemical interactions are instantaneously correlated. Again, by constructing a typical length scale one finds in this regime that the deterministic interactions &gt; ∼ 10 3 b , thus correlations much larger than the system size. The dynamics of the chain displays 'frustrated' behavior, not attaining any non-trivial spatial structure as there is not enough time to respond to instantaneous chemical gradients. Such a phase was not reported in [28] due to the existence of chemical trails that break the forward-backward symmetry; this is also discussed in Appendix 7. The limit of Λ &gt; 10 3 , ˜ Λ &gt; 10 0 corresponds to the situation when there are insufficient rotations and the chain remains rigid within the simulation time scale [28]. This instead corresponds to the region of extremely short-wavelength fluctuations and small deterministic correlation lengths. We thus conclude that the C-shape thus corresponds to regimes where the correlation lengths χ r b 3 are of the order of the chain length, which is in addition stable to short-wavelength fluctuations (of typical distance less that one monomer radius). It is this dynamical steady-state (DSS) phase that we study in the rest of this paper.

We also note that, within the C-shape phase, its existence as a DSS depends on the choice of initial conditions of Eq. (4). Deviating from these does not render a universality of the DSS, unless one includes the existence of chemical trails in the model, as discussed in [24, 28]. This scenario is further discussed in Section 7. If one deviates from the purely chemo-repulsive scenario considered here, the presence of trails are also necessary for other non-trivial DSS, such as the swimming/undulating state. However, various structural SS (e.g. designer crystallites that break handedness symmetry) is possible via the instantaneous chemical deposition model considered here [28].

## IV. INTERFACE FLUCTUATIONS

For the C-shape chain propelling in the positive x-direction, one can define a height function h ( y ) along the chain. An example of its temporal evolution is shown in Fig. 2. To differentiate the height of this shape from other typically measured shapes (circular and flat), let us call this the 'C-interface' (CI). One can thus choose a particular length segment

t = 0

T&lt;t&lt;TS

FIG. 2. Schematic of dynamical evolution of colloidal chain. From left to right: evolution of the chain at different times t = 0, until the final t = T . At times above τ and τ f the chain acquires the steady-state C-shape. Note that the chain propels in the positive x direction, and that the time points are not linearly scaled. The inset on each show the monomer orientations for a particular segment at each time. N ′ is labelled in red (middle panel), along with ∆ h (rightmost panel) and θ (in inset).

<!-- image -->

along the chain and compute the height fluctuations, and measure its root-mean-squared

<!-- formula-not-decoded -->

for the CI. Here, ⟨⟩ is taken over both the segment and realizations. We can do the same for the angular fluctuations, and define

<!-- formula-not-decoded -->

The quantities of interest are both the dynamical ( W h ( t ) and W θ ) and steady-state ( W ss h ( t ) and W ss θ ( t )) properties of these fluctuations. These are:

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Here, ( β h , β θ ) and ( α h , α θ ) are sets of growth and roughness exponents, respectively. As we show below, the positional fluctuations can be expressed via the FV scaling law [12]

<!-- formula-not-decoded -->

where the scaling function f h is given as:

<!-- formula-not-decoded -->

For the orientational sector, we find that, though there is a growth followed by a plateau, there is a persistent smoothening phenomena, where there is no system-size dependent saturation time scale, but instead a system-size dependent scaling of the fluctuations at all times , which is instead negative (hence smoothness). We discuss this in detail below. In this case, the scaling reads:

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

with the important difference between the two being the N dependence of the characteristic saturation time of fluctuations t ∗ , as explained below. Given (15), W SS θ can be equivalently be taken to be the time average over the entire simulation; we denote this as ⟨ W θ ⟩ t wherever used.

We note that the results in this Section are based on the assumption of instantaneous chemical depositions and the initial conditions of Eq. (4); we display in Appendix 7 that similar growth exponents are obtained when chemical trails are present; where the C-shape is instead universal (independent of initial conditions) [28].

## A. Specific shape about which to measure fluctuations

We emphasize here that the fluctuations reported in this work correspond to those exclusively about the C-shape. In the 'stiff' phase Λ → ∞ these would trivially reproduce

Here, we have defined:

an early-time growth for W h , similar to that for a driven EW model, as shown explicitly in Appendix 1. Fluctuations could, in principle, be computed about various other DSS as well. To emphasize the uniqueness of this shape let us list some generic properties of a DSS:

1. Continuously varying orientations along the chain
2. Finite curvature along chain (or the averaged local curvature)
3. Ballistically propelling in one direction
4. Positional and orientational symmetry along the body(y) axis

As explained in Appendix 4, there are various other DSS that meet some of the above requirements but not all. The aforementioned 'stiff' DSS satisfies only (i), (iii) and (iv). Another example of this is that of a C with a 'pinch' defect, where defects in the orientation along the chain renders a finite number of segments of the chain to exhibit a C shape (Fig. 7(D) in Appendix 4). Here τ f ∼ τ t ∼ τ , thus sharp reorientations occur alongside a positional relaxation, allowing for occasional orientational defects, all of this whilst global curvature sets in. Indeed, the C-shape is unique in that it is the only DSS that meets requirements (i)-(iv), with its propulsion direction to the body axis. It is this DSS about which the fluctuations are computed.

To compute the quantities W h ( t ) and W θ ( t ) [Eq. (8) and (9)] respectively, we discuss two possibilities. As displayed in Fig. 2, there are two separate segments about which fluctuations are measurable. Let us label the length of this segment to be N ′ . The first obvious segment corresponds to N ′ = N , corresponding to the average over the entire CI. The second corresponds to that of a locally flat region; here N ′ &lt;&lt; N , which is taken to be sufficiently far from the edges of the chain. Though established results exist for fluctuations about curved segments of an interface [9], these typically correspond to that of a (subset of a) circular interface, thus different from that of our CI (see Appendix 3). For the latter, it is to be noted that fluctuations of growing (circular) interfaces are typically measured in this regime [31, 32] ; the scaling properties in this region are almost always equal to that of globally measured quantities - for instance in [16] for a flat surface and [31] for a curved surface, and indeed we extend this equivalence to the CI.

A

Wh/Nan

10-1.

10-3.

10-5.

N= 128

N= 192

N= 256

N =312

~ 0.25

0.5

10-1

4x 101

3× 101

2× 101

<!-- image -->

10°

101

(t/T)/N2h

FIG. 3. Scaling of height fluctuations. A W h for different chain lengths N , with the three exponents β h indicated via dashed black line. The same scaling is plotted in the inset , with t ∗ h ( N ) labelled in dashed vertical line. B Roughness scaling for W SS h ( N ). Here, τ f = 0 . 2, Λ = 0 . 2, ˜ Λ = 0 . 01.

## B. Height fluctuations

The results for the interface height fluctuations W h are shown in Fig. 3(A). We see that there are four separate dynamical regimes, with three growth exponents which are defined as.

<!-- formula-not-decoded -->

In all the three cases above t &lt; t ∗ h . Around t ∼ t ∗ h , the fluctuation W h saturates to its steady-sate value W ss h due to the finite system size. We now discuss these four regimes in detail.

(i) Early-time regime I . This corresponds to times t &lt;&lt; τ (first few time steps of simulation). Here the growth of the interface height is dominated purely by noise, the mean-squared displacement of the height scales as t , and thus β (0) h ∼ 0 . 5.

(ii) Early-time regime II . Now the colloids start to feel interactions from their respective neighbors which leads to slowing down of the pure diffusive growth. Deterministic chemical self-interactions, though, are yet to set in. Here, β (1) h ≈ 0 . 25. These first two regimes correspond to those observed in the EW model [33]. For t &lt; τ f , it is straightforward

Вы ~ 1.55

10-1

W(N)

ah=0.9

to show that our chain approximates an EW interface, as shown in Appendix 1. These early time scalings correspond to (as expected) a purely diffusive regime, equivalent to random deposition of particles on a substrate accompanied by surface diffusion. An alternate way to characterize the early time growth is via the probability distribution of W 2 h ; this (and a comparison with the corresponding EW distribution) is presented in Appendix 1.

(iii) Super-ballistic regime: In this regime t &gt; τ f with t &lt; t ∗ h . During this time, the self-phoretic interactions cause the chain to morph into a C-shape topology. As a result, W h swells up to its steady-state value, with a super-ballistic exponent (quoted below). Let us study the behaviour of these fluctuations by defining a correlation length l c . Via dimensional analysis, l c ∼ b ( t τ ) 1 z h . In this region l c typically corresponds to a finite fraction along the interface. For instance, at t = 10 τ time steps into the simulation (at the onset of swelling, see inset of Fig. 3(A)), l c ≈ 53 b . These correlations further build-up until the next regime is reached.

(iv) Steady-state regime. This regime corresponds to t &gt;&gt; τ f and t &gt;&gt; t ∗ h . We identify t ∗ h = N z h τ , a system-size dependent characteristic timescale over which the fluctuations reach a steady-state (see inset of Fig. 3)(A). For example for N = 256 (Fig.3(A) inset , green), t ∗ h ≈ 27 . 6 τ ∼ 10 2 τ f . Here, the fluctuations cease to grow and W h reaches a plateau. The aforementioned correlations now scale as ( t τ ) 1 z h ∼ N (spanning the entire chain length).

The growth and roughness exponents, respectively characterizing the height fluctuations in regimes (iii) and (iv) are:

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

From (18) and Fig. 3, we find that regimes (iii) and (iv) exhibit a clear FV scaling with a roughness exponent α h and a dynamic exponent z h = α h β h ≈ 0 . 6. To our knowledge this set of exponents does not correspond to any previously reported universality class. We thus interpret this as a novel 'CI roughness' scaling, distinct to those found for either circular or flat interfaces [15, 31]. The reason for the super-ballistic exponent arises from the fact that as the C-shape formation takes place, displacements of each monomer from the mean

is enhanced in either direction (along the x-axis) along the chain. These squared deviations from the mean height are in addition symmetric along the y-axis. l c is clearly enhanced due to this symmetry. It is to be noted that in a conventional ballistic scaling one simply has uni-directional deviations along the propulsion axis; in our system this is applicable to the mean h ( y ); clearly l c in this case is smaller than for a symmetric C-shape. We suggest that these combined effects give rise to the super-ballistic growth exponent β h . The presence of a super-ballistic exponent can be rationalized via a mean-field analysis of the EOM 2. This is presented in Appendix 2. It is to be noted that varying τ r changes the time at which super-ballistic growth sets in (exponent and roughness is unchanged).

The roughness exponent α h , on the other hand, indicates the relative growth of height fluctuations when l c spans the system size. Here, the system is is at ∼ 10 2 τ f and the Cshape is fully formed. The value that we report is thus quite appreciably larger than those reported in EW [33], KPZ [34] and most numerical deposition models such as for instance the solid-on-solid growth model [16].

## C. Orientational fluctuations

The orientational fluctuations are displayed in Fig. 4(A) (and inset). There are two regimes displayed: (i) Ballistic regime. This corresponds to deterministic orientational changes as the C-shape forms. (ii) Steady-state regime. W θ is distinguished from W h is that there is no N dependent t ∗ during which the fluctuations relax. A rough estimate of this from Fig. 4(A) would be t ∗ θ ∼ 50 τ f , thus of the same order as t ∗ h ( N ). Further, l c spans the system size at all times as displayed in Fig. 4(A) inset .

The ballistic regime arises due to the purely deterministic contribution to the orientational changes that sets in during t ∼ 5 τ f ≈ τ . The emergence of the ballistic regime can be intuitively appreciated by observing the orientation profile θ i in Fig 4(C) for N = 312. We see that at all times θ i maintains strong correlations across the chain, with θ i = -θ N -i (anti-symmetry; here i is the monomer index). Such an anti-symmetric profile of the orientation field along the y-axis, along with the dynamics thus enforces l c to span the entire chain length. Further, as the steady-state is approached, a large number of orientations

A

We/Nao

10°

10-1.

10°

C

2

0

-2

0

N=128

N=192

N=256

N=312

10'

Bo ~ 1

10°-

10-2

102

t/T

t*

10'

1/t 103

103

t/+= 10°

t/t = 102

t/+=103

100

200

Monomer index, i

B 9× 10-1

&lt;6x10-1

Esx10-1

FIG. 4. Scaling of the orientation fluctuations. A W θ for different chain lengths N , with the dynamic β θ labelled. The same scaling is plotted in the inset , with t ∗ labelled in dashed vertical line. B (Anti-) Roughness scaling for W SS θ ( N ). C Orientation profile of the C-shape, taken at different time points. D Dynamical evolution of θ i for selected monomers i . Here, τ f = 0 . 2, Λ = 0 . 2, ˜ Λ = 0 . 01, and N = 311 (for panels C and D).

<!-- image -->

remain close to their initial value (4(d)). For instance, if one takes a threshold θ ∗ = 0 . 2 (corresponding to | max( θ ) -θ ∗ max( θ ) | ≈ 0 . 92 deviation from the edge monomer angles), we find that a fraction of P ( θ i &lt; θ ∗ ) ≈ . 89 of monomers on the chain remain roughly equal to their initial value, with the remaining ≈ 11% drastically varying at the edges. This notion of orientational 'stiffness' is also discussed in [24]. We note that here τ r →∞ ; the results for finite τ r are presented in Appendix 6; there is in addition an early time diffusive regime, though that does not affect the results presented here [35].

We further report that W SS θ scales negatively with the chain length. We may thus call this a smoothness exponent. This form of orientational stabilization with the system size, as opposed to conventional 'roughness' seen in other growing interfaces (e.g. α EW ≈ 0 . 5 and α KPZ ≈ 1 / 3). The source of emergence for such a smoothening exponent can be appreciating the increased orientational rigidity along the chain, which increases with chain length [24]. The angular distribution along the CI becomes increasingly uniform, thereby making the

00 = - 0.5

"e..

A

10-2

10-4

N =20

N'

=25

N

=30

N

= 35

N'

=40

N'

=45

N

=50

N' =55

-B/ ~ 0.25

10-5

10-3

(t/T)/(N) 2n

FIG. 5. Scaling of height fluctuations in locally flat region. A W h for different locally flat lengths N ′ , with the dynamic β ′ h ≈ 0 . 25 indicated via dashed black line. B Roughness scaling for W SS h ( N ′ ). Here, N = 312 and τ f = 0 . 2, Λ = 0 . 2, ˜ Λ = 0 . 01.

<!-- image -->

chain stiffer and more polarized along the x-axis. The RMS deviations from the the mean orientation thus decreases with system size (a larger fraction of the chain is polarized along the x-axis). This is in contrast to the positional steady-state fluctuations, where the mean h ( y ) varies with chain length, and fluctuations about this exhibits roughness. Note that this is although the chain-averaged curvature vanishes in the thermodynamic limit 3.

## D. Locally flat regimes

In the study of fluctuations of circular interfaces, the fluctuations are typically measured about a locally flat region along the growing front [9, 31, 32]; and the roughness scalings thus correspond to those of a locally flat region. We repeat the same procedure here, and vary N ′ under the constraint of N ′ &lt;&lt; N . In this section, N ′ N &lt; 0 . 18 such that the interface is effectively flat. The results are presented in Fig. 5. Probing the locally flat regime, we find that the height fluctuations in the early-time II and steady-state regimes, respectively, can be collapsed onto a FV-type scaling with growth exponent β ′ h ≈ 0 . 25, and roughness exponent α ′ h ≈ 1. This would imply a dynamic exponent z ′ h = 4. However, it is to be noted that this implies essentially neglecting the super-ballistic regime, where the data does not collapse. A dynamic exponent z = 4 is reminiscent of driven interfaces with conservation laws [17, 36, 37], although the individual growth and roughness exponents differ. An appropriate continuum description of our system remains an open problem. The W θ scaling in this regime is presented in Appendix 5.

5=4× 10°

, 3 × 10°

TABLE I. Summary of exponents for W h . Note that β (1) h and β (2) h refer to two sequential regimes for each before reaching saturation.

| Quantity    | β (1) h    | β (2) h    | α h        | z (1) h =   | α h β (1) h   | z (2) h =   | α h β (2) h   |
|-------------|------------|------------|------------|-------------|---------------|-------------|---------------|
| W h ( N )   | 0 . 25(01) | 1 . 53(02) | 0 . 90(03) | 3 . 61(25)  | 3 . 61(25)    | 0 . 58(03)  | 0 . 58(03)    |
| W h ( N ′ ) | 0 . 25(01) | NA         | 1 . 01(01) | 4 . 04(20)  | 4 . 04(20)    | NA          | NA            |

TABLE II. Summary of exponents for W θ .

| Quantity   | β θ        | α θ          |
|------------|------------|--------------|
| W θ ( N )  | 1 . 00(00) | - 0 . 47(03) |

## V. DISCUSSION AND SUMMARY

Roto-translational coupling has been well-studied for their various implications on dynamical and collective behaviour in active matter physics. The effect of this coupling on scaling laws have been established for the mean-squared displacement - i.e enhanced or anomalous diffusion - in various models (see for instance [38-40]). Here, we instead present the effect of roto-translational induced global topological change, and the resultant novel scaling laws for interface fluctuations. Remarkably, we find that the height fluctuations show an FV data collapse, with a new set of dynamic and roughness exponents; with the former rationalized via a simple mean-field analysis of roto-translational coupling. The orientational fluctuations, in addition, display novel scaling behaviour, notably smoothening with system size, which we attribute to the enhanced orientational rigidity of longer chains. The exponents obtained in this paper for height fluctuations are summarized in Table I, while the exponents for orientation fluctuations are summarized in Table II.

In equilibrium (passive) systems, the typical time scales that govern the height fluctuations are predominantly drift and diffusion (in addition to any deterministic interactions and hydrodynamics). In certain models one might in addition have an additional (Arrhenius) time scale that sets a time scale of local equilibration in an energy landscape

[16, 19, 41]. The self-propelled, roto-translational colloidal chain instead possesses an additional timescale within which deterministic spatio-temporal (i.e topological) changes in the interface take place, leading to super-ballistic growth and smoothening phenomena.

We note that existing models of active interfaces have also reported additional time scales that arise due to non-equilibrium effects; for example see [19, 42, 43] that discuss proteins hopping on a membrane. Indeed, these works also report FV scaling laws in their systems, but this is (naively) to be expected as the continuum evolution of the height fields resembles that of a modified KPZ-like model. The FV scaling laws that we report, however, arise from a shape change of the interface, with the C-shape topology being distinct from either a circular or flat interface.

Potential open questions thus naturally arise as to what appropriate (tractable) coarsegrained (continuum) models could describe the active interface presented here. It remains to be seen whether or not both the C-shape and locally flat topologies could be explained by the same continuum model. In addition, instead of periodic boundary conditions typically used to model growing interfaces, 'clamped' boundary conditions are required for the curvature to set in. These ideas provide challenges for further work.

To test the predictions here, we note that the experimental paradigm requires long-ranged internal repulsions within the monomers in the chain, that give it its rigid interface structure [24]. To observe significant fluctuations in these systems, if one equates damping forces to thermal forces, ηbv s ∼ k b T/b ( η the viscosity of the medium and v s the self-propulsion ≈ 10 µms -1 ), we obtain b ≈ 10 -7 m (at least 10 times smaller than typical chemically interacting colloidal systems). Naively, these would correspond to active polymer regimes of biological interest [44, 45]. However, recent findings [46] have reported how phase change in 8CB liquid crystal emulsions can be induced via external temperature changes. In the nematic/isotropic phases, the individual colloids no longer deterministically self-propel but in addition have a fluctuating propulsion direction. This would correspond to finite τ r in our model. In addition, these results could also be relevant to active interfacial systems such as colonies of migrating bacteria [47], where chemical interactions are typically long-ranged. Another open problem, would be the extension of this model to a ring topology, where the

possibility of attaining spin waves is imminent [21].

## APPENDIX/SI

## 1. Early time approximation and probability distribution

We write down here the approximate early time dynamics for our model, showing that it represents a driven one dimensional interface and how the diffusive and sub-diffusive scalings for W 2 h ( t ) can be accounted for. Let us explore the regime where orientation dynamics is yet to affect the translational dynamics. Let us re-write the positional dynamics of (2) for the case of the flat interface, ignoring orientation dynamics. In this case, we have

<!-- formula-not-decoded -->

Taking the continuum limit of the above, we have

<!-- formula-not-decoded -->

which is thus a standard diffusive interface driven by a propulsion velocity v s and a Gaussian white noise ξ . At very early times, interface fluctuations are dominated by ξ , before the diffusive term comes into play.

These height fluctuations are alternatively characterized by the distribution of fluctuations, P ( W 2 ) [32, 48]. For the typical EW interface (non-propelling), known results exist and are re-written here. We can directly quote the result from [48] for arbitrary initial height distribution

<!-- formula-not-decoded -->

where W 2 SS = ND t 12 ν h , a n = 6 ( πn ) 2 (1 -e -τ p n 2 ), and the scaling variables are given by

<!-- formula-not-decoded -->

For a flat initial height profile, one has s n 0 = 0, which gives [48]

̸

<!-- formula-not-decoded -->

A

P(Wh,carly /(W2)sS)

3

1

0

0.0

3

The terms c n (0) are Fourier transforms of the initial height profile. In our case, we start with h ( x, t = 0) = 0, which fixes c n (0) = 0.

0.5

Wh, early/(W2)Ss

FIG. 6. A : Distribution of height fluctuations at early times ( t ≪ τ f ). The distribution of W 2 h of the non-interacting chain at early times (blue, scatter) can be fitted to the known exact solution for the height distribution of the EW interface; here the fitted τ p ≈ 0 . 17. B : The chemically interacting chain (orange scatter) shows a deviation from this, with the best fit of τ p ≈ 0 . 21 again in dashed. The average of least squared errors for A and B are 0 . 0508 and 0 . 0728 respectively, with the latter indicating an enhanced deviation from the EW distribution. Parameters chosen are as follow: for A , Λ →∞ , whilst for B , Λ = 0 . 2. In both cases ˜ Λ = 100.

<!-- image -->

The early time approx of (20) corresponds to setting Λ →∞ . Note that one can either τ &lt;&lt; τ f probe or χ r = 0 globally; we pick the latter. The analytical distribution of (23) can then be compared to this limit; we find that the distribution is reproduced - black line and blue scatter of Fig. 6(A). We can thus suitably compare this with the case of Λ finite, restricting to τ &lt;&lt; τ f . Here, we find that our propelled interface displays deviations from this passive distribution; this is displayed in Fig. 6(B) - yellow scatter. For both cases, we plot alongside the best fit analytical distribution in dashed black, and we further find that the averaged least squared error of the fit is substantially larger in the case of Λ finite; we thus conclude that the early time non-equilibrium distribution deviates from the passive EW case. This marginal deviation at early times arises again via the aforementioned roto-translational coupling; this thus again constitute a non-equilibrium signature that appears via deterministic (but small) orientational interactions. This deviation in the height

distribution is analogous to that seen in so-called 'anomalous diffusion' [49, 50], where microscopic colloidal particles in specific systems display a mean-squared displacement exponent of 1, but the probability distribution of displacements has been found to follow the Laplace (instead of Gaussian) distribution [50, 51]. The exact analytical distribution of the chemically interacting chain is so-far intractable, and thus remains an open future problem.

## 2. Mean-field approximation of super-ballistic growth exponent

In the main text we have reported the growth exponent of the CI to be β h ≈ 1 . 5 (super-ballistic). Although rationalizing this for a curved surface is beyond exact analytical calculation (in this work); we may study the flat interface (thermodynamic limit), via a mean-field analysis.

Let us study the simplest case of roto-translational coupling, in particular where (small) angular dynamics set in. Consider the i th positional dynamics from (2). For small θ i , we can write this as

<!-- formula-not-decoded -->

Note that the x i are assumed to be stationary in this regime (flat interface). We can use this for the angular dynamics to write

<!-- formula-not-decoded -->

where S i = 1 4 b 2 ∑ j 1 ( i -j ) 2 [ 2Θ( i -j ) -1 ] , with Θ( x ) the step function. The deterministic solution to this is

<!-- formula-not-decoded -->

We next use the steady-state approximation t τ f &lt;&lt; 1 b 2 S i . Thus, in the growth regime of t ∼ τ f , this requires b 2 S i &gt;&gt; 1, which is satisfied in the thermodynamic limit.

We then plug (26) into (24). We then use the small argument expansion for cosh( z ) ≈

1 + z 2 2 , with z = χ r S i t 2 . This gives us

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

where ⟨⟩ is performed over both the monomers and realizations. Here, X is a siteindependent average of the chain position. We obtain the scaling for (∆ h ) 2 as follows

<!-- formula-not-decoded -->

̸

where we have used that ⟨ F b ( t ′ ) ⟩ = 0 and ⟨ F b ( t ′ ) F b ( t ′ ) ⟩ = ⟨ F b ( t ) ⟩⟨ F b ( t ′ ) ⟩ . Strictly speaking, the latter will cease to be true when curvature sets in (spring forces then become correlated), but it will suffice for the flat approximation used here. We thus obtain the result that the dynamics is super-ballistic with exponent ∼ 3, when orientational contributions are minimally accounted for such that Var( S ) = 0. We thus conclude that minimal rototranslational coupling on a flat interface can predict super-ballistic growth exponents but clearly overestimates that of the surface with finite curvature.

## 3. Curvature of chain

To calculate the phase diagram in Fig. 1, we use the Monge representation to calculate the curvature in the steady-state [52], following [28]. We compute the curvature as:

<!-- formula-not-decoded -->

where h i is the height function of the chain, evaluated at each monomer location, measured from the vertical line connecting the edge monomers. The average is performed across the monomers in the chain, giving a scalar value. In our case, the inner bracket is simply reduced to ∂ y [ ∂ y x i ( y ) / √ 1 + | ∂ y x i | 2 ] . For Fig. 1, we take the averaged | κ | in the steady state. Further, we note that κ → 0 as N → ∞ [28]. For a circular interface, κ would be scale as 1 /R ( R the radius of the circle), whereas for a flat interface κ would be constant. Thus, this variation κ ( N ) profile can be used as proxy definition for the C-shape.

thus

(i).

N = 64

(ii) N = 128

(iii)

N = 64

## 4. Other DSS for large N

It is to be noted also that the phase diagram in Fig. 1 being N specific and only comparing Λ, ˜ Λ does exclude other DSS that arise from simulation. Examples of these are shown in Fig. 7. Four examples are shown, we label them as (i) 'Alt-C', (ii) 'Random front', (iii) 'Stiff-C', and (iv) 'Pinched front' respectively. We note that DSS(ii) corresponds to the 'Disordered' phase in Fig. 1. For each of these DSS, the chain propels in a deterministic direction along the x-axis; thus (8) can be computed in principle. However, none of these DSS satisfy all the criteria enumerated in Sec. IVA. For DSS(i)-(iv), criteria (i) 2, 3, and 4, (ii) 1 and 3, (iii) 1,3 and 4; and (iv) 2 and 3 are satisfied. Thus, the CI is the unique topology that satisfies all four criteria. The hypothetical fluctuations for W θ for these can also in principle be computed, but we do not pursue it here. In general, whenever there is an orientational symmetry along the body axis (e.g. for DSS(i)), we expect a smoothness exponent.

FIG. 7. Examples of DSS. From left to right: (i) Alt-C, (ii) Random front, (iii) Stiff-C, and (iv) Pinched front respectively. { Λ , ˜ Λ } values are (i) { 10 -5 , 2 } , (ii) { 0 . 05 , 0 . 2 } , (iii) { 10 -6 , 200 } , (iv) { 10 1 , 200 } . Note that (iv) in addition incorporated trails via (31); here t 0 = 1 and τ r = 5 in addition.

<!-- image -->

(iv)

N = 128

10-2

10-3

Wo/(N') ao

10-4.

N=5

N=7

N=9

N=11

N=13

Bo ~ 1

10-

2× 10-1.

10-1

## 5. Scaling of W θ in the locally flat regime

10-3

10'

The scalings of W θ in for N ′ &lt;&lt; N is presented in Fig. 8. We find that the scaling law of Eq. (14) is satisfied; the ballistic β ′ θ = 1 is retained whilst the roughness α ′ θ ≈ 1 . 22(03) is obtained. Note that this is opposed to the smoothness exponent obtained on the full CI. We also note that the specific choice of N ′ ∈ [5 , 12] used here, thus at most ≈ 4% of the chain length, is substantially smaller than in Fig. 5. For any appreciably larger N ′ the scaling of (14) is not observed. These results thus further highlight the fact that the smoothness exponent found in Fig. 4 is a sole consequence of the CI; with distinct topology specific results vis-a-viz a flat surface. 101

1°7/1

FIG. 8. Scaling of orientational fluctuations in locally flat region. A W θ for different locally flat lengths N ′ , with the dynamic β ′ θ labelled. The same scaling is plotted in the inset . B Roughness scaling for ⟨ W SS θ ( N ′ ) ⟩ t . Here, N = 312 and τ f = 0 . 2, Λ = 0 . 2, ˜ Λ = 0 . 01.

<!-- image -->

## 6. Effect of τ r

The effect of τ r is not studied any further in the main text. For τ r &gt; ∼ 20 an identical FV scaling for W h is obtained, whereas W θ only approximates the scaling law (15) - See Fig. 9. For W h one sees a crossover at τ r for the angular fluctuations between a diffusive β θ ≈ 0 . 5 and a deterministic β θ ≈ 1. The effect of randomness prohibits a systems-spanning l c . Nevertheless, a smoothness exponent with an identical scaling can be obtained.

For τ r &lt; ∼ 10 there is no longer a propelling C-shape, and indeed other DSS are observed (see Fig. 7). The reason for this is that the C-shape DSS is sensitive to long-wavelength ao =1.22

Wh/Nah

A

10-2

10-4.

10-6-

WS (N)

ah = 0.85

N

10-1

B

10°

C

10-2.

~ 1

fluctuations when D r &gt; ∼ 1, such that any fluctuations on the length scale b 2 D r v s is greater than typical monomer lengths. A physically compelling reason on the other hand is that for equilibrated systems τ r = 8 πηb 3 k b T , with τ t = 6 πηb 2 k b T ( η the viscosity of the medium), thus we will normally have τ r &lt;&lt; τ t , and hence D r can be safely ignored under this consideration. In addition the model in (2) ignores hydrodynamic advection. (t/T)/N*h N= 192 N= 192 10-4.

FIG. 9. Height and orientational fluctuations for finite τ r . A W h for different chain lengths N , with the dynamic β h labelled. Inset : Scaling for W SS h ( N ). B W θ for different chain lengths N , with the dynamic β θ labelled. Inset : Scaling for W SS θ ( N ). C Crossover for W θ in the presence of finite τ r (vertical dashed black line). Here, τ f = 0 . 2, Λ = 0 . 2, ˜ Λ = 0 . 01 and τ r = 70.

<!-- image -->

## 7. History-dependence

The various results quoted above have been attained via the initial conditions specified in (4) (a straight chain with parallel orientation). In this section, we further show that the height fluctuations are universal if one adds history-dependence (chemical trails) into the dynamics; indeed these exist in experimentally realizable chemically interacting colloidal systems [28, 53]. Thus, instead of Eq.(3), for this section, we use the following form of J :

<!-- formula-not-decoded -->

where t 0 sets the upper bound on the memory kernel. This corresponds to monomers sensing chemical trails of their neighbors. We display the results in Fig. 10, for N = 250 (note in this section N ′ = N ). Taking into account the timescale τ c = b 2 D c for diffusion of filled

00 = -0.5

N

A

10°-

110-2.

10-4-

10°

Вы ~ 0.25

10'

i=9

i=20

<!-- image -->

t/T

FIG. 10. Fluctuations of model with chemical trails. Example of A W h and W θ evolution, with the dynamic β h labelled. B Orientation profile of the C-shape, taken at different time points. C Dynamical evolution of θ i for selected monomers i .. Here, D c = 1, t 0 = 1, with N = 64 and T = 10 5 .

micelles across the system, we note that it is necessary to have τ c &lt;&lt; τ f with τ c ∼ τ to obtain the C-shape configuration [24, 28].

We present the results for W h and W θ in Fig. 10 (A). We find that the scalings of W h are reproduced. With the micellar trails, when one starts such a system with arbitrary initial conditions (e.g. random), the trails deposited via (31) give rise to a forward-backward symmetry breaking in the orientational response to the chemicals. Thus, the system breaks the initial symmetry imposed on it and further universally picks up the C-shape. In our system, we choose sufficiently random initial conditions such that this effect is seen; we then compute ∆ h once the chain has a well-defined interface. For a system with N = 64, averaged over 10 realizations, we also find a crossover from the growth exponent β (1) h = 0 . 25 to β (2) h ≈ 1 . 53, similar to the model without chemical trails. We note that the steady-state value is not fully reached, as it is prohibitively long to simulate this region. For the same reason, we could not perform a rigorous systems-size scaling analysis; nevertheless, we conjecture that α h would remain unchanged.

For the orientation fluctuations, though the qualitative form is reproduced, we observe that the exact values of β θ are different from that found in Section IV C. In particular, we see from Fig.10 (A) (yellow line) that β θ is also super-ballistic, its growth comparable to β h . This enhanced growth in the angular sector in this case can be appreciated via both the steady-state profile (Fig.10(B)) and the dynamical evolution (Fig.10(C)) of the orientations

Вы ~ 1.53

B

1

÷0-

— t/r= 10º

t/t = 102

t/t= 103

C

0.05-

- these are to be compared with Fig.4 (C) and (D) respectively. In particular, we note that the time evolution contains higher order temporal derivatives, especially for monomers near the edges (e.g. i = 9 , 50 , 55 in Fig.10(C) versus i = 311 of Fig.4(D)), thus any fluctuations about the mean will deviate from a simple ballistic scaling. In addition, the steady-state angular profile is no longer monotonically varying across the chain (compare Fig.10(B) versus Fig.4(C)). Nevertheless the symmetry of θ i across the chain leads us to conjecture that there will also be a smoothness exponent in this case.

Our conclusions of this section are, thus, that the addition of trail-mediated interactions only appreciably affects the growth of W θ , with the growth and roughness of W h , along with smoothness of W θ unchanged.

## ACKNOWLEDGMENTS

We thank Professors ME Cates and M Muthukumar for useful discussions. AGS acknowledges funding from the DIA Fellowship from the Government of India. TB is supported through the Luxembourg National Research Fund (FNR), grant reference C22/MS/17186249.

- [1] J. Toner, The Physics of Flocking: Birth, Death, and Flight in Active Matter (Cambridge University Press, 2024).
- [2] T. Vicsek, A. Czir´ ok, E. Ben-Jacob, I. Cohen, and O. Shochet, Novel type of phase transition in a system of self-driven particles, Physical review letters 75 , 1226 (1995).
- [3] H. Chat´ e, F. Ginelli, G. Gr´ egoire, and F. Raynaud, Collective motion of self-propelled particles interacting without cohesion, Physical Review E-Statistical, Nonlinear, and Soft Matter Physics 77 , 046113 (2008).
- [4] M. Ballerini, N. Cabibbo, R. Candelier, A. Cavagna, E. Cisbani, I. Giardina, V. Lecomte, A. Orlandi, G. Parisi, A. Procaccini, et al. , Interaction ruling animal collective behavior depends on topological rather than metric distance: Evidence from a field study, Proceedings

of the national academy of sciences 105 , 1232 (2008).

- [5] L. Caprini and H. L¨ owen, Flocking without alignment interactions in attractive active brownian particles, Physical Review Letters 130 , 148202 (2023).
- [6] A. G. Subramaniam, S. Adhikary, and R. Singh, Minimal mechanism for flocking in phoretically interacting active colloids, arXiv preprint arXiv:2504.07050 (2025).
- [7] R. Grossmann, L. Schimansky-Geier, and P. Romanczuk, Self-propelled particles with selective attraction-repulsion interaction: from microscopic dynamics to coarse-grained theories, New Journal of Physics 15 , 085014 (2013).
- [8] M. Kardar, Nonequilibrium dynamics of interfaces and lines, Physics reports 301 , 85 (1998).
- [9] A.-L. Barab´ asi and H. E. Stanley, Fractal concepts in surface growth (Cambridge university press, 1995).
- [10] P. L. Krapivsky, S. Redner, and E. Ben-Naim, A kinetic view of statistical physics (Cambridge University Press, 2010).
- [11] D. S. Dean, S. N. Majumdar, and S. Sabhapandit, Exact height distribution in one-dimensional Edwards-Wilkinson interface with diffusing diffusivity, Journal of Physics A: Mathematical and Theoretical 58 , 235002 (2025).
- [12] T. Vicsek and F. Family, Dynamic scaling for aggregation of clusters, Physical Review Letters 52 , 1669 (1984).
- [13] K. Fujimoto, R. Hamazaki, and Y. Kawaguchi, Family-vicsek scaling of roughness growth in a strongly interacting bose gas, Physical Review Letters 124 , 210604 (2020).
- [14] I. Corwin, The kardar-parisi-zhang equation and universality class, Random matrices: Theory and applications 1 , 1130001 (2012).
- [15] F. Family and T. Vicsek, Scaling of the active zone in the eden process on percolation networks and the ballistic deposition model, Journal of Physics A: Mathematical and General 18 , L75 (1985).
- [16] J. Kim, J. Kosterlitz, and T. Ala-Nissila, Surface growth and crossover behaviour in a restricted solid-on-solid model, Journal of Physics A: Mathematical and General 24 , 5569 (1991).
- [17] S. D. Sarma and P. Tamborenea, A new universality class for kinetic growth: One-dimensional molecular-beam epitaxy, Physical review letters 66 , 325 (1991).
- [18] M. Degawa, T. Stasevich, W. Cullen, A. Pimpinelli, T. L. Einstein, and E. D. Williams, Distinctive fluctuations in a confined geometry, Physical review letters 97 , 080601 (2006).

- [19] F. Cagnetta, M. Evans, and D. Marenduzzo, Active growth and pattern formation in membrane-protein systems, Physical Review Letters 120 , 258001 (2018).
- [20] Q. Goutaland, F. van Wijland, J.-B. Fournier, and H. Noguchi, Binding of thermalized and active membrane curvature-inducing proteins, Soft Matter 17 , 5560 (2021).
- [21] R. Adkins, I. Kolvin, Z. You, S. Witthaus, M. C. Marchetti, and Z. Dogic, Dynamics of active liquid interfaces, Science 377 , 768 (2022).
- [22] F. Caballero, A. Maitra, and C. Nardini, Interface dynamics of wet active systems, Phys. Rev. Lett. 134 , 087105 (2025).
- [23] M. E. Cates and C. Nardini, Active phase separation: new phenomenology from nonequilibrium physics, Reports on Progress in Physics 88 , 056601 (2025).
- [24] M. Kumar, A. Murali, A. G. Subramaniam, R. Singh, and S. Thutupalli, Emergent dynamics due to chemo-hydrodynamic self-interactions in active polymers, Nature Commun. 15 , 4903 (2024).
- [25] D. Nishiguchi, J. Iwasawa, H.-R. Jiang, and M. Sano, Flagellar dynamics of chains of active janus particles fueled by an AC electric field, New J. Phys. 20 , 015002 (2018).
- [26] A. Snezhko and I. S. Aranson, Magnetic manipulation of self-assembled colloidal asters, Nat. Mater. 10 , 698 (2011).
- [27] B. Biswas, R. K. Manna, A. Laskar, P. S. Kumar, R. Adhikari, and G. Kumaraswamy, Linking catalyst-coated isotropic colloids into 'active' flexible chains enhances their diffusivity, ACS Nano 11 , 10025 (2017).
- [28] A. G. Subramaniam, M. Kumar, S. Thutupalli, and R. Singh, Rigid flocks, undulatory gaits, and chiral foldamers in a chemically active polymer, New Journal of Physics 26 , 083009 (2024).
- [29] K. A. Takeuchi, An appetizer to modern developments on the kardar-parisi-zhang universality class, Physica A: Statistical Mechanics and its Applications 504 , 77 (2018).
- [30] L. Li, H. Manikantan, D. Saintillan, and S. E. Spagnolie, The sedimentation of flexible filaments, Journal of Fluid Mechanics 735 , 705 (2013).
- [31] K. A. Takeuchi and M. Sano, Universal fluctuations of growing interfaces: evidence in turbulent liquid crystals, Physical review letters 104 , 230601 (2010).
- [32] K. A. Takeuchi and M. Sano, Evidence for geometry-dependent universal fluctuations of the kardar-parisi-zhang interfaces in liquid-crystal turbulence, Journal of Statistical Physics 147 , 853 (2012).

- [33] S. F. Edwards and D. Wilkinson, The surface statistics of a granular aggregate, Proceedings of the Royal Society of London. A. Mathematical and Physical Sciences 381 , 17 (1982).
- [34] M. Kardar, G. Parisi, and Y.-C. Zhang, Dynamic scaling of growing interfaces, Physical Review Letters 56 , 889 (1986).
- [35] As explained in [28], sufficiently large rotational noise prohibits the C-shape formation. Thus, 'positive D r ' wherever mentioned here implicitly assumes τ f τ r = b 3 D r χ r &lt;&lt; 1. Here, τ f τ r = 0 . 01.
- [36] T. Sun, H. Guo, and M. Grant, Dynamics of driven interfaces with a conservation law, Phys. Rev. A 40 , 6763 (1989).
- [37] T. Banerjee and A. Basu, Symmetries and scaling in generalised coupled conserved kardar-parisi-zhang equations, Journal of Statistical Mechanics: Theory and Experiment 2018 , 013202 (2018).
- [38] R. Golestanian, Anomalous diffusion of symmetric and asymmetric active colloids, Physical review letters 102 , 188305 (2009).
- [39] C. Bechinger, R. Di Leonardo, H. L¨ owen, C. Reichhardt, G. Volpe, and G. Volpe, Active particles in complex and crowded environments, Rev. Mod. Phys. 88 , 045006 (2016).
- [40] C. Kurzthaler and T. Franosch, Intermediate scattering function of an anisotropic brownian circle swimmer, Soft Matter 13 , 6396 (2017).
- [41] Y. Kim and S. Yoon, Scaling properties of self-expanding surfaces, Physical Review E 69 , 027101 (2004).
- [42] F. Cagnetta, M. R. Evans, and D. Marenduzzo, Statistical mechanics of a single active slider on a fluctuating interface, Physical Review E 99 , 042124 (2019).
- [43] F. Cagnetta, M. R. Evans, and D. Marenduzzo, Kinetic roughening in active interfaces, in EPJ Web of Conferences , Vol. 230 (EPJ Web of Conferences, 2020) p. 00001.
- [44] N. Jain and S. Thakur, Structure and dynamics of chemically active ring polymers: swelling to collapse, Soft Matter 19 , 7358 (2023).
- [45] S. Kumar, R. Padinhateeri, and S. Thakur, Shear flow as a tool to distinguish microscopic activities of molecular machines in a chromatin loop, Soft Matter 20 , 6500 (2024).
- [46] M. Kumar, S. Sane, A. Murali, and S. Thutupalli, Temperature switchable self-propulsion activity of liquid crystalline microdroplets, Soft Matter 21 , 3782 (2025).
- [47] M. Hayakawa, T. Hiraiwa, Y. Wada, H. Kuwayama, and T. Shibata, Polar pattern formation induced by contact following locomotion in a multicellular system, Elife 9 , e53609 (2020).

- [48] T. Antal and Z. R´ acz, Dynamic scaling of the width distribution in Edwards-Wilkinson type models of interface dynamics, Phys. Rev. E 54 , 2256 (1996).
- [49] B. Wang, S. M. Anthony, S. C. Bae, and S. Granick, Anomalous yet brownian, Proceedings of the National Academy of Sciences 106 , 15160 (2009).
- [50] B. Wang, J. Kuo, S. C. Bae, and S. Granick, When brownian diffusion is not gaussian, Nature materials 11 , 481 (2012).
- [51] A. V. Chechkin, F. Seno, R. Metzler, and I. M. Sokolov, Brownian yet non-gaussian diffusion: from superstatistics to subordination of diffusing diffusivities, Physical Review X 7 , 021002 (2017).
- [52] W. Helfrich, Elastic properties of lipid bilayers: theory and possible experiments, Z. Naturforsch 28 , 693 (1973).
- [53] C. Jin, C. Kr¨ uger, and C. C. Maass, Chemotaxis and autochemotaxis of self-propelling droplet swimmers, Proceedings of the National Academy of Sciences 114 , 5089 (2017).