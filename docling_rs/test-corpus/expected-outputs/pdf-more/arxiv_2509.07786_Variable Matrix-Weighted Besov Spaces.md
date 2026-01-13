## Variable Matrix-Weighted Besov Spaces

Dachun Yang * , Wen Yuan and Zongze Zeng

Abstract: In this article, applying matrix A p ( · ) , ∞ weights introduced in our previous work, we introduce the matrix-weighted variable Besov space via the matrix weight W or the reducing operators A of order p ( · ) for W , Then we show that, defined either by the matrix weight W or the reducing operators A of order p ( · ) for W , the matrix-weighted variable Besov spaces (respectively, the matrix-weighted variable Besov sequence spaces) are both equal. Next, we establish the φ -transform theorem for matrix-weighted variable Besov spaces and, using this, find that the definition of matrix-weighted variable Besov spaces is independent of the choice of φ . After that, for the further discussion of variable Besov spaces, we establish the theorem of almost diagonal operators and then, by using this, we establish the molecular characterization. Then, with applying the molecular characterization, we obtain the wavelet and atomic characterizations of matrix-weighted variable Besov spaces. Finally, as an application, we consider some classical operators. By using the wavelet characterization, we establish the trace operator and obtain the theorem of trace operators. Moreover, with applying the molecular characterization, we establish the theorem of Calder´ on-Zygmund operators on matrix-weighted variable Besov spaces.

## Contents

| 1 Introduction                          | 1 Introduction                                  | 1 Introduction                                            |   2 |
|-----------------------------------------|-------------------------------------------------|-----------------------------------------------------------|-----|
| 2 Matrix A p ( · ) , ∞ Weights          | 2 Matrix A p ( · ) , ∞ Weights                  | 2 Matrix A p ( · ) , ∞ Weights                            |   5 |
| 3 Matrix-Weighted Variable Besov Spaces | 3 Matrix-Weighted Variable Besov Spaces         | 3 Matrix-Weighted Variable Besov Spaces                   |   8 |
|                                         | 3.1                                             | Matrix-Weighted Variable Besov Spaces . . . . . . . .     |   9 |
|                                         | 3.2                                             | Matrix-Weighted Variable Besov Sequence Spaces . .        |  20 |
|                                         | 3.3                                             | The φ -Transform . . . . . . . . . . . . . . . . . . . .  |  23 |
| 4                                       | Almost Diagonal Operators                       | Almost Diagonal Operators                                 |  27 |
| 5                                       | Molecules Characterization and Its Applications | Molecules Characterization and Its Applications           |  36 |
|                                         | 5.1                                             | Molecules Characterization . . . . . . . . . . . . . . .  |  36 |
|                                         | 5.2                                             | Wavelet Characterizations and Atomic Decompositions       |  40 |
| 6                                       | Boundedness of Classical Operators              | Boundedness of Classical Operators                        |  43 |
|                                         | 6.1                                             | Trace Operators . . . . . . . . . . . . . . . . . . . . . |  43 |
|                                         | 6.2                                             | Calder´ on-Zygmund Operators . . . . . . . . . . . . .    |  51 |

2020 Mathematics Subject Classification . Primary 46E35; Secondary 47A56, 15A15, 46E40, 42B35.

Key words and phrases . variable Besov space, A p ( · ) , ∞ weight, almost diagonal operators, molecule characterization, wavelet characterization, trace operator, Calder´ on-Zygmund operator.

This project is partially supported by the National Key Research and Development Program of China (Grant No. 2020YFA0712900), the National Natural Science Foundation of China (Grant Nos. 12431006 and 12371093), and the Fundamental Research Funds for the Central Universities (Grant Nos. 2253200028 and 2233300008).

* Corresponding author, E-mail: dcyang@bnu.edu.cn / September 9, 2025 / Final version.

## 1 Introduction

The study of Besov spaces B s p , q was started in 1951, during which Nikol'ski˘ ı [78] introduced the Nikol'ski˘ ı-Besov spaces, nowadays denoted by B s p , ∞ . In the later work, through introducing the third index q , Besov [5, 6] complemented this scale. From then on, the theory of Besov spaces has found wide applications in harmonic analysis and partial di ff erential equation. We refer to [21, 23, 24, 27, 28, 29, 58, 59, 100] for more studies about Besov spaces. Besov spaces with variable smoothness s ( · ) and fixed p = q was first studied by Leopold [68, 69, 70] and Leopold and Schrohe [71] during the study of pseudo-di ff erential operators, which were further generalized to the case that p , q by Besov [7, 8, 9]. Besov spaces with variable integrability p ( · ) and fixed q and s were later introduced by Xu [93, 94] along a di ff erent line of study. Through introducing the concept of variable mixed Lebesgue-sequence l q ( · ) ( L p ( · ) ), Almeida and H¨ ast¨ o [2] first mixed up the variable integrability p ( · ) and q ( · ) with the variable smoothness s ( · ), where they introduced variable Besov spaces B s ( · ) p ( · ) , q ( · ) and established the embedding theorem. Since the concept of variable Besov spaces was introduced, the theory of variable Besov spaces developed quickly. In [50], Drihem obtained the boundedness of Peetre's maximal operators and then, using this, established the atomic decomposition of variable Besov spaces. The interpolation theorem was later established by Almeida and H¨ ast¨ o [3], where, as an application of the interpolation theorem, they proved the trace theorem for variable Besov spaces. As a more general Besov space, through adding the fourth variable exponent τ ( · ), Drihem [51] introduced the variable Besov-type space B s ( · ) ,τ ( · ) p ( · ) , q ( · ) , where they proved the embedding theorem, and moreover, established the atomic decomposition of variable Besov-type spaces in [52]. In the meanwhile, a more general variable Besov-type space B s ( · ) ,ϕ p ( · ) , q ( · ) with a more general fourth exponent, a measurable function ϕ , was independently introduced by Yang et.al. [97]. In this article, they established the φ -transform theorem and the atomic decompositions and, using this, proved the trace theorem of variable Besov-type spaces. We refer to [1, 3, 49, 53, 99] for more studies about variable Besov space.

On the other hand, the variable weights was first introduced by Cruz-Uribe et.al. [37], during the study of the boundedness of the Hardy-Littlewood maximal operator on weighted variable Lebesgue spaces. From then on, the theory of variable weights developed quickly. In [39], CruzUribe showed the weak boundedness of the maximal operators, and then, Cruz-Uribe and Wang [45] established the extrapolation theorem of variable weights. Recently, Cruz-Uribe and Penrod [41] proved the reverse H¨ older inequality on variable Lebesgue spaces. We refer to [44] for more studies about variable weights on weighted Lebesgue spaces. Recently, after these developments in weighted Lebesgue spaces, weighted variable Besov spaces were introduced by Wang and Xu [91], where they proved the embedding theorem and the interpolation theorems. Then Guo et.al. [63] obtained a continuous equivalent expression of weighted variable Besov spaces and Wang et.al. [90] further established the atomic, molecule, and wavelet characterization of weighted variable Besov spaces. We refer to [31] for the recent study about weighted variable Besov spaces associated with operators.

The study of the matrix weight can be tracked back to the work of Wiener and Masani [92] on the prediction theory for multivariate stochastic processes. In 1990s, Nazarov and Treil [75], Treil and Volberg [85], and Volberg [88] generalized the scaler Muckenhoupt Ap weights to the matrix Ap weights acting on vector-valued functions. After the concept of matrix Ap weights was introduced, a lot of attentions have been paid to the theory of matrix Ap weights; see, for instance, [11, 32, 61, 10, 74, 13]. With the development of the theory of matrix weights, matrix Ap weighted Besov spaces B s p , q ( W ) were introduced by Roudenko [86] for p ∈ (1 , ∞ ) and by Frazier and Roudenko [55] for p ∈ (0 , 1]. In these works, they proved the boundedness of almost diagonal operators and, using this result, studied the boundedness of the Calder´ on-Zygmund operators and established the wavelet characterizations of matrix Ap weighted Besov spaces; and moreover, in the recent work, Frazier and Roudenko [56] also obtain these similar results for the matrix Ap

weighted Triebel-Lizorkin spaces. However, they believed that the ranges of the index fot their results are still improvable and left this as an open question. Recently, this open question was solved by Bu et.al. [16, 17, 18] with introducing a new exquisite estimate of matrix weights, that is, the upper and lower Ap dimensions. By using these new estimates, they achieved a more optimal ranges of the index of the almost diagonal operator theorem and further improved many ranges of the index for those results in [86, 55, 56]. After the work in matrix Ap weighted Besov spaces, Bu et.al. studied the Muckenhoupt A ∞ condition in matrix cases, which was first introduced by Volberg [88] with the concept of matrix Ap , ∞ weights as an analogue of the A ∞ condition for matrix weights. The systemic study about the theory of matrix Ap , ∞ weights was started by Bu et.al. in [19], where they introduced an analogue expression of Ap , ∞ weights and further obtained the theory of matrix Ap , ∞ weights in vector-valued Lebesgue spaces; and then they introduced the concept of matrix Ap , ∞ weighted Besov spaces B s p , q ( W ) for p ∈ (0 , ∞ ) and established the characteristic theory of matrix Ap , ∞ weighted Besov spaces B s p , q ( W ) in [20]. We refer to [15] for more studies about the Ap , ∞ weights on function spaces.

With the development of the theories of variable weights and matrix weights, the concept of variable matrix A p ( · ) weights was introduced by Cruz-Uribe and Penrod [40], where they established the identity approximation theorem and studied the theory of variable matrix A p ( · ) weighted Sobolev spaces; and moreover, they proved the reverse H¨ older inequality for matrix A p ( · ) weights on variable Lebesgue spaces in [41]. Nieraeth and Penrod [77] later obtained the boundedness of Christ-Goldberg maximal operators and Calder´ on-Zygmund operators on matrix A p ( · ) weighted variable Lebesgue spaces. Inspired by the definition of matrix A p ( · ) weights and matrix Ap , ∞ weights, we introduced the concept of variable matrix A p ( · ) , ∞ weights in [96]; and then, we obtained the theory of A p ( · ) , ∞ weights in variable Lebesgue spaces and established the upper and lower A p ( · ) , ∞ dimensions for further studies on variable function spaces.

,

In this article, inspired by our previous work in [96], we introduce the matrix A p ( · ) , ∞ weighted variable Besov spaces B s ( · ) p ( · ) , q ( · ) ( W ) and the matrix A p ( · ) , ∞ weighted variable Besov sequence spaces b s ( · ) p ( · ) , q ( · ) ( W ) with p ( · ) , q ( · ) ∈ P 0, s ( · ) ∈ L ∞ , and W ∈ A p ( · ) , ∞ . Since the reducing operators is the 'average' of matrix weights, we can use reducing operators to take place of the weight W during the definition of matrix A p ( · ) , ∞ weighted variable Besov (sequence) spaces. Indeed, we prove that, if p ( · ) , q ( · ) , s ( · ) ∈ LH (see Definition 2.4), then the matrix A p ( · ) , ∞ weighted variable Besov (sequence) space defined with the matrix A p ( · ) , ∞ weight W is equivalent with those defined with the reducing operators A of order p ( · ) for W (see Definitions 3.5 and 3.33). By using this, we establish the φ -transform theorem for B s ( · ) p ( · ) , q ( · ) ( W ); and then, as an application of the φ -transform, we prove that the definition of B s ( · ) p ( · ) , q ( · ) ( W ) is independent of the choice of φ . After that, we first establish the theorem of almost diagonal operators on B s ( · ) p ( · ) , q ( · ) ( W ). Based on this and the previous established φ -transform theorem, we prove the molecular characterization of B s ( · ) p ( · ) , q ( · ) ( W ). Using this, we obtain the wavelet characterizations of B s ( · ) p ( · ) , q ( · ) ( W ) and then, as an application of the wavelet characterization, we establish the atomic decomposition of B s ( · ) p ( · ) , q ( · ) ( W ). Then, with applying the precious obtained wavelet characterization, we introduce the trace and extension operators in B s ( · ) p ( · ) , q ( · ) ( W ) and then, together this with the molecular characterization, obtain the trace and extension theorems. Finally, by using the molecular characterization, we establish the theorem of Calder´ on-Zygmund operators on B s ( · ) p ( · ) q ( · ) ( W ).

We point out that, since the targets we consider are vectors and matrices, where the times principle is di ff erent from the scalar case, many methods used for scalar weighted variable Besov spaces might be failed for the matrix case. To overcome this obstacle, we recall the concept of the reducing operators of variable matrix weights, which is the average of variable matrix weights in the sense of variable Lebesgue norm, and then, by using this, we introduce corresponding averaging weighted variable Besov spaces defined with the reducing operators and later show

that the matrix weighted variable Besov space and the corresponding averaging weighted one are equivalent. Moreover, during the proof of the boundedness of almost diagonal operators, due to the variable exponent, it will involve something closely related to variable exponents, for example, the variable exponent power of constant 2 jp ( · ) for j ∈ Z + , which is di ff erent from the case p ( · ) is a constant exponent. To overcome this obstacle, we fully use the properties of log-H¨ older continuous and obtain the exquisite estimates.

The organization of the reminder of this article is as follows.

In Section 2, we recall some basic concepts and properties of matrix A p ( · ) , ∞ weights obtained in [96], including the definition of matrix A p ( · ) , ∞ weights (see Definition 2.6), the reducing operators for A p ( · ) , ∞ weights (see Definition 2.8), and the upper and lower dimensions of A p ( · ) , ∞ weights (see Theorem 2.12), which are widely used in this article.

In Section 3, before giving the definition of matrix-weighted variable Besov space, we first recall the concept of mixed variable Lebesgue-sequence spaces. Then, in Subsection 3.1, we introduce the concepts of matrix A p ( · ) , ∞ weighted variable Besov spaces B s ( · ) p ( · ) , q ( · ) ( W ) and corresponding averaging weighted variable Besov spaces B s ( · ) p ( · ) , q ( · ) ( A ); and then, we show that these two definitions are equivalent (see Theorem 3.8). Then, in Section 3.2, we introduce the concepts of matrix A p ( · ) , ∞ weighted variable Besov sequence spaces b s ( · ) p ( · ) , q ( · ) ( W ) and corresponding averaging weighted variable Besov sequence spaces b s ( · ) p ( · ) , q ( · ) ( A ). Then we prove b s ( · ) p ( · ) , q ( · ) ( W ) = b s ( · ) p ( · ) , q ( · ) ( A ) (see Theorem 3.34). Finally, in Subsection 3.3, using these previous results that b s ( · ) p ( · ) , q ( · ) ( W ) = b s ( · ) p ( · ) , q ( · ) ( A ), we establish the φ -transform characterization for B s ( · ) p ( · ) , q ( · ) ( W ) (see Theorem 3.35) and, as an application of the φ -transform, we find that the definition of matrix A p ( · ) , ∞ weighted variable Besov spaces is independent of the choice of φ (see Proposition 3.36).

In Section 4, we first recall the concept of the almost diagonal operators; and then, we establish the boundedness of the almost diagonal operators on B s ( · ) p ( · ) , q ( · ) ( W ) under conditions that will reduce to the known best result with constant exponents p ( · ) , q ( · ) , s ( · ).

In Section 5, we apply the theorem of almost diagonal operators to obtain several characterizations of B s ( · ) p ( · ) , q ( · ) ( W ). Precisely, in Subsection 5.1, we establish the molecular characterization on B s ( · ) p ( · ) , q ( · ) ( W ) (see Theorem 5.8) by combining the theorem of φ -transform with the boundedness of almost diagonal operators. Then, in Subsection 5.2, as an application of molecular characterization, we obtain the wavelet characterization of B s ( · ) p ( · ) , q ( · ) ( W ) (see Theorem 5.12) and then, by using this, we establish the atomic decompositions of B s ( · ) p ( · ) , q ( · ) ( W ) (see Theorem 5.16).

,

Finally, in Section 6, we apply the previous obtained results to the boundedness of trace operators and Calder´ on-Zygmund operators on B s ( · ) p ( · ) , q ( · ) ( W ). In Subsection 6.1, under the assumption that all index p ( · ) , q ( · ) , s ( · ) are independent of the n -th variable xn , we introduce the trace and extend operators on B s ( · ) p ( · ) , q ( · ) ( W ) by using the wavelet characterization and then, together this with the obtained molecular characterizations, we establish the trace and extend theorem (see Theorems 6.3 and 6.6). In Subsection 6.2, we further obtain the boundedness of the Calder´ on-Zygmund operators on B s ( · ) p ( · ) q ( · ) ( W ) by using the molecular characterizations.

In the end, we make some conventions on notion. Let Z be the collection of all integers, Z + : = { 0 , 1 , . . . } , and N : = { 1 , 2 , . . . } . For any measurable set E of R n , denote by the symbol M ( E ) the set of all measurable functions on E and, when E = R n , simply write M ( R n ) as M . In addition, we use the symbol L p loc ( R n ) with p ∈ (0 , ∞ ) to denote the set of all locally p -integrable functions on R n . For any x ∈ R n and r ∈ (0 , ∞ ), the open ball B ( x , r ) is defined to be the set { y ∈ R n : | x -y | &lt; r } and let B : = { B ( x , r ) : x ∈ R n and r ∈ (0 , ∞ ) } . A cube Q of R n always has finite edge length and edges of cubes are always assume to be parallel to coordinate axes, bu Q is not necessary to be open or closed. For any cube Q of R n , we always use l ( Q ) to denote the edge length of Q . For any k ∈ Z n and j ∈ Z , let Q ( R n ) : = { Qk , j : = 2 -j ([0 , 1) n + k ) : k ∈ Z n and j ∈ Z } and, for any j ∈ Z , let Q j ( R n ) : = { Qk , j : = 2 -j ([0 , 1) n + k ) : k ∈ Z n } and Q + ( R n ) : = { Qk , j : =

2 -j ([0 , 1) n + k ) : k ∈ Z n and j ∈ Z + } . If E is a measurable set of R n , then we denote by 1 E its characteristic function and, for any bounded measurable set E ⊂ R n with | E | , 0 and for any f ∈ L 1 loc ( R n ), let &gt; E f ( x ) dx : = 1 | E | R E f ( x ) dx . For any p ∈ [1 , ∞ ], let p ′ be the conjugate number of p , that is, 1 p + 1 p ′ = 1. We always use C to denote a positive constant independent of the main parameters involved. The symbol f ≲ g means f ≤ Cg and, if f ≲ g ≲ f , we then write f ∼ g . To simplify the symbol, when there is no confusion about base space, we ignore the symbol R n . In the end, when we prove a theorem (and the like), in its proof we always use the same symbols as those appearing in the statement itself of the theorem (and the like).

## 2 Matrix A p ( · ) , ∞ Weights

In this section, we recall some basic properties of matrix A p ( · ) , ∞ weights obtained in our precious work [96].

We begin with the variable Lebesgue spaces. A measurable function p : R n → (0 , ∞ ] is called an exponent function . Let P be the set of all exponent functions p : R n → [1 , ∞ ] and P 0 be the set of all exponent functions p : R n → (0 , ∞ ] satisfying ess inf x ∈ R n p ( x ) &gt; 0. For any p ( · ) ∈ P 0 and any set E in R n , let

<!-- formula-not-decoded -->

in particular, simply write p + : = p + ( R n ) and p -: = p -( R n ).

Then we recall the definition of variable Lebesgue spaces (see, for instance, [38, Definition 2.16]).

Definition 2.1. The variable Lebesgue space L p ( · ) associated with p ∈ P 0 is defined to be the set of all f ∈ M such that

<!-- formula-not-decoded -->

where ρ L p ( · ) is the variable exponent modular defined by setting

<!-- formula-not-decoded -->

with Ω ∞ : = { x ∈ R n : p ( x ) = ∞} .

Next, we recall some basic concepts of matrices and matrix weights. For any m , n ∈ N , the set of all m × n complex-valued matrices is denoted by Mm , n , and Mm , m is simply denoted by Mm . For any A ∈ Mm , let

<!-- formula-not-decoded -->

Then ( Mm , ∥ · ∥ ) is a Banach space. Moreover, we have the following well-known result (see, for instance, [16, Lemma 2.3]).

Lemma 2.2. Let A , B ∈ Mm be two nonnegative definite matrices. Then ∥ AB ∥ = ∥ BA ∥ .

Now, we recall the concept of matrix weights (see, for instance, [16, Definition 2.7]).

Definition 2.3. A matrix-valued function W : R n → Mm is called a matrix weight if W satisfies that

- (i) for almost every x ∈ R n , W ( x ) is nonnegative definite,
- (ii) for almost every x ∈ R n , W ( x ) is invertible,
- (iii) the entries of W are all locally integrable.

We now recall the concept of the log-H¨ older continuous condition of variable exponents (see, for instance, [38, Definition 2.2]).

Definition 2.4. Ameasurable real-valued function r on R n is said to be locally log-H¨ older continuous , denoted by r ( · ) ∈ LH 0, if there exists a positive constant C 0 such that, for any x , y ∈ R n with | x -y | &lt; 1 2 ,

<!-- formula-not-decoded -->

A measurable real-valued function r on R n is log-H¨ older continuous at infinity , denoted by r ( · ) ∈ LH ∞ , if there exist positive constants r ∞ and C ∞ such that, for any x ∈ R n ,

<!-- formula-not-decoded -->

Furthermore, a measurable real-valued function r on R n is said to be globally log-H¨ older continuous , denoted by r ( · ) ∈ LH , if r ( · ) is both locally log-H¨ older continuous and log-H¨ older continuous at infinity.

Remark 2.5. (i) If r ( · ) ∈ LH , then (2.2) can be replaced by the following condition:

<!-- formula-not-decoded -->

- (ii) From [38, Proposition 2.3], we infer that, if r ( · ) ∈ LH , then 1 r ( · ) ∈ LH .

Then we recall the definition of matrix A p ( · ) , ∞ weights introduced in our recent work [96, Definition 1.1(ii)].

Definition 2.6. Let p ( · ) ∈ P 0. A matrix weight W on R n is called a matrix A p ( · ) , ∞ weight if

<!-- formula-not-decoded -->

where the supremum is taken over all cubes Q in R n .

- Remark 2.7. (i) If p ( · ) ≡ p is a constant exponent, then, for any W ∈ A p , ∞ , the p -th power of W is a matrix Ap , ∞ weight (see, for example, [88, 20] for the definition of Ap , ∞ weights).
- (ii) From [96, Theorem 3.1], it follows that, for any scalar-valued weight w , if p ( · ) ∈ P 0 with p ( · ) ∈ LH , then w ∈ A p ( · ) , ∞ if and only if w p ( · ) ∈ A ∞ .

Next, we recall the concept of the reducing operators for matrix A p ( · ) , ∞ weights (see [40, p. 1142] for reducing operators for matrix A p ( · ) weights and [96, Definition 3.8] for reducing operators for matrix A p ( · ) , ∞ weights).

Definition 2.8. Let p ( · ) ∈ P 0 and W be a matrix weight and let Q be any cube in R n . The matrix AQ ∈ Mm is called a reducing operator of order p ( · ) for W if AQ is positive definite and self-adjoint such that, for any ⃗ z ∈ C m ,

<!-- formula-not-decoded -->

where the positive equivalence constants depend only on m and p ( · ).

The following lemma guarantees the existence of reducing operators of order p ( · ) for matrix weights, which is exactly [96, Proposition 3.9].

Lemma 2.9. Let p ( · ) ∈ P 0 . Then, for any matrix weight W and cube Q in R n , the reducing operator AQ of order p ( · ) for W exists.

The following extends (2.5) from any vector ⃗ z to any matrix M ∈ Mm , which is precisely [96, Lemma 3.10].

Lemma 2.10. Let p ( · ) ∈ P 0 and W be a matrix weight and let Q be any cube in R n . If AQ is a reducing operator of order p ( · ) for W, then, for any matrix M ∈ Mm,

<!-- formula-not-decoded -->

where the positive equivalence constants depend only on m and p ( · ) .

Finally, we recall the concept of A p ( · ) , ∞ weight dimensions introduced in [96, Definition 3.21].

Definition 2.11. Let p ( · ) ∈ P 0 and d ∈ R . A matrix weight W is said to have A p ( · ) , ∞ -lower dimension d , denoted by W ∈ D lower p ( · ) , ∞ , d , if there exists a positive constant C such that, for any λ ∈ [1 , ∞ ) and any cube Q in R n ,

<!-- formula-not-decoded -->

A matrix weight W is said to have A p ( · ) , ∞ -upper dimension d , denoted by W ∈ D upper p ( · ) , ∞ , d , if there exists a positive constant C such that, for any λ ∈ [1 , ∞ ) and any cube Q in R n ,

<!-- formula-not-decoded -->

We have the following basic properties, which is exactly [96, Proposition 3.22].

Proposition 2.12. Let p ( · ) ∈ P 0 with p ( · ) ∈ LH. Then the following statements hold:

- (i) For any d ∈ ( -∞ , 0) , D lower p ( · ) , ∞ , d = ∅ and D upper p ( · ) , ∞ , d = ∅ .
- (ii) For any W ∈ A p ( · ) , ∞ , there exists d 1 ∈ [0 , n p -) such that W ∈ D lower p ( · ) , ∞ , d 1 .
- (iii) For any W ∈ A p ( · ) , ∞ , there exists d 2 ∈ [0 , ∞ ) such that W ∈ D upper p ( · ) , ∞ , d 2 .

Let p ( · ) ∈ P 0 with p ( · ) ∈ LH . Then, for any matrix weight W ∈ A p ( · ) , ∞ , let

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

and

Let and

Remark 2.13. If p ( · ) ≡ p is a constant exponent, then Proposition 2.12(ii) shows that, for any W ∈ A p , ∞ , d lower p ( · ) , ∞ ( W ) ∈ [0 , n p ). From Remark 2.7(i), it follows that W ∈ A p , ∞ if and only if e W : = W p ∈ Ap , ∞ Hence, by this and Proposition 2.12(ii), we find that, if e W ∈ Ap , ∞ and d ∈ ( -∞ , ∞ ) satisfying

<!-- formula-not-decoded -->

then d ∈ (0 , n ), which is precisely [16, Proposition 6.3(ii)].

The following result is an application of the upper and lower A p ( · ) , ∞ -dimensions, which is exactly [96, Lemma 3.27] (see, for instance, [19, Proposition 6.5] for the similar result for matrix Ap , ∞ weights).

Lemma 2.14. Let p ( · ) ∈ P 0 with p ( · ) ∈ LH and let W ∈ A p ( · ) , ∞ , d 1 ∈ [ [ d lower p ( · ) , ∞ ( W ) , n p -) , d 2 ∈ [ [ d upper p ( · ) , ∞ ( W ) , ∞ ) , and { AQ } be a family of reducing operators of order p ( · ) for W. Then there exists a positive constant C such that, for any cubes Q and R in R n ,

<!-- formula-not-decoded -->

where xQ and xR are any points in Q and R, respectively, and ∆ : = d 1 + d 2 .

Remark 2.15. Let A : = { AQ } Q ∈Q + be a sequence of positive definite and self-adjoint matrices. Then A is said to be strong ( d 1 , d 2) -doubling if there exists a positive constant C such that (2.6) holds for any Q , R ∈ Q + .

## 3 Matrix-Weighted Variable Besov Spaces

In this section, we introduce the matrix-weighted variable Besov spaces and the related sequence spaces, including:

- the (pointwise) matrix-weighted spaces B s ( · ) p ( · ) , q ( · ) ( W ) and the related sequence spaces b s ( · ) p ( · ) , q ( · ) ( W ), where W : R n → Mm is a matrix weight,
- the averaging matrix-weighted spaces B s ( · ) p ( · ) , q ( · ) ( A ) and the related averaging sequence spaces b s ( · ) p ( · ) , q ( · ) ( A ), where A : = { AQ } Q ∈Q + is the reducing operators of order p ( · ) for W .

We prove thw equivalence betwebn B s ( · ) p ( · ) , q ( · ) ( W ) and B s ( · ) p ( · ) , q ( · ) ( A ) in Subsection 3.1 and the equivalence between b s ( · ) p ( · ) , q ( · ) ( W ) and b s ( · ) p ( · ) , q ( · ) ( A ) in Subsection 3.2. Finally, in Subsection 3.3, we establish the φ -transform theorem for matrix-weighted variable Besov spaces and find that the definition of weighted-matrix variable Besov spaces is independent of the choice of { φ j } j ∈ Z + .

Now, we begin with the following spaces introduced by Almeida and H¨ ast¨ o in [2].

Definition 3.1. Let p ( · ) , q ( · ) ∈ P 0. The variable mixed Lebesgue-sequence space l q ( · ) ( L p ( · ) ) is defined to be the set of all measurable function sequences { f v } v ∈ N ⊂ M such that

<!-- formula-not-decoded -->

where the modular ρ l q ( · ) ( L p ( · ) ) is defined as

<!-- formula-not-decoded -->

Remark 3.2. (i) From Definitions 3.1 and 2.1, we infer that, if q + &lt; ∞ , then

<!-- formula-not-decoded -->

- (ii) By [2, Proposition 3.3], we know that, if p ( · ) , q ( · ) are both constant exponents, then the norm ∥ · ∥ l q ( L p ) defined by Definition 3.1 is exactly the mixed Lebesgue-sequence norm.
- (iii) From [2, Proposition 3.5], it follows that ρ l q ( · ) ( L p ( · ) ) in Definition 3.1 is a semimodular and, if p + , q + &lt; ∞ , then ρ l q ( · ) ( L p ( · ) ) is continuous (see [2, Definition 2.1] or [47, Definition 2.1.1] for more details).
- (iv) Let p ( · ) , q ( · ) ∈ P 0 with p + , q + &lt; ∞ and let r ∈ (0 , ∞ ). Then, by the definition of ∥·∥ l q ( · ) ( L p ( · ) ) , it is easy to find that, for any sequence of measurable functions { f j } j ∈ Z + ,

<!-- formula-not-decoded -->

Finally, we recall the concept of admissible pairs (see, for instance, [2, Definition 5.1]).

Definition 3.3. A pair of measurable functions ( φ, Φ ) is said to be admissible if φ, Φ ∈ S satisfy

<!-- formula-not-decoded -->

and

<!-- formula-not-decoded -->

where c is a positive constant independent of ξ ∈ R n . Let φ 0 : = Φ and φ j : = 2 jn φ (2 j · ) for any j ∈ N .

## 3.1 Matrix-Weighted Variable Besov Spaces

In this subsection, we first introduce the (pointwise) matrix-weighted variable Besov space (see [20, Definition 3.22] for the definition of matrix Ap , ∞ weighted Besov spaces).

Definition 3.4. Let p ( · ) , q ( · ) ∈ P 0, s ( · ) ∈ L ∞ , and { φ j } j ∈ Z + be as in Definition 3.3 and let W ∈ A p ( · ) , ∞ . The (pointwise) matrix-weighted variable Besov space B s ( · ) p ( · ) , q ( · ) ( W , φ ) is defined to be the set of all ⃗ f ∈ ( S ′ ) m such that

<!-- formula-not-decoded -->

Next we introduce the averaging matrix-weighted variable Besov space (see [20, Definition 3.11] for the definition of averaging matrix Ap , ∞ weighted Besov spaces).

Definition 3.5. Let p ( · ) , q ( · ) ∈ P 0, s ( · ) ∈ L ∞ , { φ j } j ∈ Z + be as in Definition 3.3 and let W ∈ A p ( · ) , ∞ and A : = { AQ } Q ∈Q + be reducing operators of order p ( · ) for W . The averaging matrix-weighted variable Besov space B s ( · ) p ( · ) , q ( · ) ( A , φ ) is defined to be the set of all ⃗ f ∈ ( S ′ ) m such that

<!-- formula-not-decoded -->

where, for any j ∈ Z + ,

<!-- formula-not-decoded -->

To show the equivalence of B s ( · ) p ( · ) , q ( · ) ( W , φ ) and B s ( · ) p ( · ) , q ( · ) ( A , φ ), we recall the concept of variable Besov sequence space (see [50, Definition 3]).

Definition 3.6. Let p ( · ) , q ( · ) ∈ P 0, and s ( · ) ∈ L ∞ . The variable Besov sequence space b s ( · ) p ( · ) , q ( · ) is defined to be the set of all sequences t : = { tQ } Q ∈Q + ⊂ C such that

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Remark 3.7. If p ( · ) , q ( · ), and s ( · ) are all constant exponents, then, from Remark 3.2(ii), we infer that b s ( · ) p ( · ) , q ( · ) defined by Definition 3.6 reduces to the Besov sequence space.

For any reducing operators A : = { AQ } Q ∈Q + of order p ( · ) for W , any φ ∈ S , and any ⃗ f ∈ ( S ′ ) m , we define

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

The following equivalence is the main result of this subsection (see [20, Theorem 3.24] for the similar result about matrix-weighted Besov spaces).

Theorem 3.8. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH, s ( · ) ∈ LH, and { φ j } j ∈ Z + be the same as in Definition 3.3 and let W ∈ A p ( · ) , ∞ and A : = { AQ } Q ∈Q be reducing operators of order p ( · ) for W. Then ⃗ f ∈ B s ( · ) p ( · ) , q ( · ) ( W , φ ) if and only if ⃗ f ∈ B s ( · ) p ( · ) , q ( · ) ( A , φ ) . Moreover, for any ⃗ f ∈ ( S ′ ) m ,

<!-- formula-not-decoded -->

where the positive equivalence constants are independent of ⃗ f .

For the sake of convenience, we break the proof of Theorem 3.8 into the following two parts: proofs of the first equivalence (see Lemmas 3.13 and 3.24) and the second equivalence of Theorem 3.8 (see Lemma 3.9). Here, we first show the latter equivalence of Theorem 3.8, which is exactly the following result.

Lemma 3.9. Let p ( · ) , q ( · ) , s ( · ) , { φ j } j ∈ Z + , W, and A be the same as in Theorem 3.8. Then, for any ⃗ f ∈ ( S ′ ) m , ⃗ f ∈ B s ( · ) p ( · ) , q ( · ) ( A , φ ) if and only if sup A ,φ ( ⃗ f ) ∈ b s ( · ) p ( · ) , q ( · ) and, moreover,

<!-- formula-not-decoded -->

where the positive equivalence constants are independent of ⃗ f .

To prove Lemma 3.9, we need some basic tools. The following lemma can be found in the proof of [56, Theorem 2.4] (see also [56, Lemma 3.15]).

where, for any j ∈ Z + , where, for any Q ∈ Q + ,

Lemma 3.10. Let γ ∈ S with b γ ( ξ ) = 1 for any ξ ∈ R n with | ξ | ≤ 2 and supp b γ ⊂ { ξ ∈ R n : | ξ | ≤ π } . Then, for any j ∈ Z + and any f ∈ S ′ with supp b f ⊂ { ξ ∈ R n : | ξ | ≤ 2 j + 1 } , one has f ∈ C ∞ and, for any x , y ∈ R n , pointwise.

For any x ∈ R n , let η j , m ( x ) : = 2 jn (1 + 2 j | x | ) m , with j ∈ N and m ∈ (0 , ∞ ). The following lemma is exactly [67, Lemma 19].

Lemma 3.11. Let s ( · ) ∈ LH, x ∈ R n , and j , m ∈ N . If R ∈ ( Clog , ∞ ) , where Clog is the same as in (2.4), then there exists a positive constant C, independent of x and j, such that, for any f ∈ L 1 loc ,

<!-- formula-not-decoded -->

Recall that in the variable exponent setting, the Fe ff erman-Stein vector-valued inequality for the Hardy-Littlewood maximal operator may fails, and then the following vector-valued inequality (see [2, Lemma 4.7]) involving η -functions serves as a substitute. We refer to [2, 48] for more details about Hardy-Littlewood maximal operators and η -functions on variable Lebesgue-sequence spaces.

Lemma 3.12. Let p ( · ) , q ( · ) ∈ P with p ( · ) , q ( · ) ∈ LH. For any m ∈ ( n , ∞ ) , there exists a positive constant C such that, for any sequence of measurable functions { f v } v ∈ N ,

<!-- formula-not-decoded -->

where C is independent of { f v } v ∈ Z + .

Now, we give the proof of Lemma 3.9.

Proof of Lemma 3.9. From (3.5), it follows that, for any j ∈ Z + , any cube Q ∈ Q j , and any x ∈ Q ,

<!-- formula-not-decoded -->

which, combined with the definition of Aj and the disjointness of Q + , further implies that, for any j ∈ Z + and any x ∈ R n ,

<!-- formula-not-decoded -->

Hence, by this and the definition of ∥ · ∥ B s ( · ) p ( · ) , q ( · ) ( W ,φ ) , we conclude that

<!-- formula-not-decoded -->

This shows that the left-hand side of (3.6) is less than the right one.

Next, we prove the converse inequality. Since supp b φ j ⊂ { ξ ∈ R n : | ξ | ≤ 2 j + 1 } for any j ∈ Z + , we infer that, for any ⃗ f ∈ ( S ′ ) m , supp [ φ j ∗ ⃗ f ⊂ { ξ ∈ R n : | ξ | ≤ 2 j + 1 } . Using this and Lemma 3.10, we find that, for any j ∈ Z + and any x , y ∈ R n ,

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

where γ ∈ S is the same as in Lemma 3.10. Fix constants r ∈ (0 , min { p -, q -, 1 } ) and M ∈ ( n r + C log ( s ) +∆ , ∞ ), where C log ( s ) is the same as in (2.4) and ∆ the same as in Lemma 2.14. From (3.7) and the fact γ ∈ S , we deduce that, for any j ∈ Z + and any Q ∈ Q j and for any x ∈ Q , any y ∈ R n , and any x ′ ∈ Q ,

<!-- formula-not-decoded -->

which, together with (3.5), further implies that, for any x ′ ∈ Q and y ∈ R n ,

<!-- formula-not-decoded -->

Since (3.8) holds for any y ∈ R n , by integrating over all y in the cube (0 , 2 -j ] n , we infer that

<!-- formula-not-decoded -->

Using this, Lemma 2.14, Tonelli's theorem, and the disjointness of cubes in Q j , we obtain, for any j ∈ Z + , Q ∈ Q j , and any x ∈ R n ,

<!-- formula-not-decoded -->

where e M : = M -∆ .

For any j ∈ Z + , let gj : = P Q ∈Q j sup A ,φ, Q ( ⃗ f ) e 1 Q and hj : = | Aj ( φ j ∗ ⃗ f ) | . By this and (3.9), we find that, for any j ∈ Z + and any x ∈ R n ,

<!-- formula-not-decoded -->

Using this and Lemma 3.11 with choosing R ′ ∈ ( rC log ( s ) , ∞ ) such that e Mr -R ′ &gt; n , we obtain, for any j ∈ Z + and any x ∈ R n ,

<!-- formula-not-decoded -->

From this, Remark 3.2(iv), Lemma 3.12, and the fact p ( · ) r , q ( · ) r ∈ LH , we infer that

<!-- formula-not-decoded -->

This finishes the proof of Lemma 3.9.

<!-- formula-not-decoded -->

Next, we show the first equivalence of Theorem 3.8. Here, we first prove the inequality that ∥ ⃗ f ∥ B s ( · ) p ( · ) , q ( · ) ( W ,φ ) ≲ ∥ sup A ,φ ( ⃗ f ) ∥ b s ( · ) p ( · ) , q ( · ) .

Lemma 3.13. Let p ( · ) , q ( · ) , s ( · ) , { φ j } j ∈ Z + , W, and A be the same as in Theorem 3.8. Then, for any ⃗ f ∈ ( S ′ ) m ,

<!-- formula-not-decoded -->

where the implicit positive constant is independent of ⃗ f .

Before giving the proof of Lemma 3.13, we recall some basic tools. The following one is exactly [38, Theorem 2.34].

Lemma 3.14. Let p ( · ) ∈ P . Then, for any f ∈ M , f ∈ L p ( · ) if and only if

<!-- formula-not-decoded -->

and, moreover, ∥ f ∥ L p ( · ) ∼ ∥ f ∥ ′ L p ( · ) , where the positive equivalence constants depend only on p ( · ) .

The following lemma is exactly [40, Lemma 2.4]. In what follows, for any p ( · ) ∈ P , we use p ′ ( · ) to denote its conjecture, that is, p ′ ( · ) satisfies 1 p ( x ) + 1 p ′ ( x ) = 1 for almost every x ∈ R n .

Lemma 3.15. Let p ( · ) ∈ P with p ( · ) ∈ LH. Then there exists a positive constant C, depending only on n and p ( · ) , such that, for any f ∈ L p ( · ) and g ∈ L p ′ ( · ) ,

<!-- formula-not-decoded -->

The following lemma shows the relationship between the modular and the norm of variable Lebesgue spaces, which is a special case of [47, Lemma 2.1.14] with the modular ρ : = ρ L p ( · ) .

Lemma 3.16. Let p ( · ) ∈ P 0 with p + &lt; ∞ , then for any f ∈ M , ∥ f ∥ L p ( · ) ≤ 1 if and only if ρ L p ( · ) ( f ) ≤ 1 and, moreover, ∥ f ∥ L p ( · ) = 1 if and only if ρ L p ( · ) ( f ) = 1 .

The following lemma is a result of the convexification for L p ( · ) and it has already been used in [2]. We omit the details here.

Lemma 3.17. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH. Then, for any f ∈ M , ∥| f | q ( · ) ∥ L p ( · ) q ( · ) ≤ 1 if and only if ∥ f ∥ L p ( · ) ≤ 1 .

If q ( · ) is a constant, then we have a stronger result about the convexification for variable Lebesgue spaces, which is the following lemma (see, for instance, [38, Proposition 2.18] and [47, Lemma 3.2.6]).

Lemma 3.18. Let p ( · ) ∈ P 0 with p + &lt; ∞ . Then, for any r ∈ (0 , ∞ ) and any f ∈ M , ∥ f ∥ L rp ( · ) = ∥| f | r ∥ 1 r L p ( · ) .

We also need the following H¨ older's inequality in variable Lebesgue spaces, which is exactly [38, Theorem 2.26].

Lemma 3.19. Let p ( · ) ∈ P . If f ∈ L p ( · ) and g ∈ L p ′ ( · ) , then f g ∈ L 1 and there exists a positive constant C, depending only on p ( · ) , such that

<!-- formula-not-decoded -->

The following lemma is exactly [38, Proposition 2.21].

<!-- formula-not-decoded -->

The following lemma shows the relation between the norm ∥ · ∥ l q ( · ) ( L p ( · ) ) and the modular ρ l q ( · ) ( L p ( · ) ) , which is a direct application of [47, Lemma 2.1.14] with the fact that ρ l q ( · ) ( L p ( · ) ) is a semimodular. We omit the details here.

Lemma 3.21. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH. Then, for any sequence of measurable functions { f j } j ∈ Z + , the norm ∥{ f j } j ∈ Z + ∥ l q ( · ) ( L p ( · ) ) ≤ 1 if and only if ρ l q ( · ) ( L p ( · ) ) ( { f j } j ∈ Z + ) ≤ 1 and, moreover, ∥{ f j } j ∈ Z + ∥ l q ( · ) ( L p ( · ) ) = 1 if and only if ρ l q ( · ) ( L p ( · ) ) ( { f j } j ∈ Z + ) = 1 .

The following result can be obtained directly by Definition 3.1; we omit the details here.

Lemma 3.22. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH. For any sequence of measurable functions { f j } j ∈ Z + , if there exists a positive constant C such that ρ l q ( · ) ( L p ( · ) ) ( { f j } j ∈ Z + ) ≤ C, then

<!-- formula-not-decoded -->

The following result can be found in the proof of [50, Theorem 1].

Lemma 3.23. Let p ( · ) ∈ LH. Then, for any j ∈ Z + and for any cube Q ∈ Q j and x , y ∈ Q,

<!-- formula-not-decoded -->

where the positive equivalence constants depend only on p ( · ) and n. Moreover, for any j ∈ Z + and any δ ∈ [1 + 2 -j , 1 + 2 -j + 1 ] and for any cube Q ∈ Q j and x , y ∈ Q,

<!-- formula-not-decoded -->

where the positive equivalence constants depend only on p ( · ) and n.

Now, we give the proof of Lemma 3.13.

,

Proof of Lemma 3.13. We first consider the case ∥ sup A ,φ ( ⃗ f ) ∥ b s ( · ) p ( · ) , q ( · ) = 0. In this case, by the fact that ∥ · ∥ b s ( · ) p ( · ) , q ( · ) is a quasi-norm, we obtain sup A ,φ ( ⃗ f ) = 0 and hence ⃗ f = 0, which further implies that ∥ ⃗ f ∥ B s ( · ) p ( · ) q ( · ) ( W ,φ ) = 0. Thus, (3.10) holds under this condition.

Next, we assume ∥ sup A ,φ ( ⃗ f ) ∥ b s ( · ) p ( · ) , q ( · ) , 0. From the fact that ∥·∥ b s ( · ) p ( · ) , q ( · ) and ∥·∥ B s ( · ) p ( · ) , q ( · ) ( W ,φ ) are both quasi-norms, it follows that we only need to show that there exists a positive constant C such that

<!-- formula-not-decoded -->

for any measurable function ⃗ f satisfying

<!-- formula-not-decoded -->

For any j ∈ Z + , let t j : = P Q ∈Q j sup A ,φ, Q ( ⃗ f ) e 1 Q . We claim that, to prove (3.13), it is su ffi cient to show that there exists a positive constant C such that, for any j ∈ Z + ,

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

and δ j ∈ [2 -j , 1 + 2 -j ]. Indeed, if (3.15) holds, then, for any j ∈ Z + ,

<!-- formula-not-decoded -->

Applying this with Lemma 3.17, we find that

<!-- formula-not-decoded -->

which further implies that where, for any j ∈ Z + ,

<!-- formula-not-decoded -->

Using this, Remark 3.2(i), (3.16), Lemma 3.21, and the assumption ∥ sup A ,φ ( ⃗ f ) ∥ b s ( · ) p ( · ) , q ( · ) = 1, we conclude that

<!-- formula-not-decoded -->

From this and Lemma 3.22, it follows that ∥{ C -1 2 js ( · ) | W ( · )( φ j ∗ ⃗ f )( · ) |} j ∈ Z + ∥ l q ( · ) ( L p ( · ) ) ≲ 1, which further implies that ∥{ 2 js ( · ) | W ( · )( φ j ∗ ⃗ f )( · ) |} j ∈ Z + ∥ l q ( · ) ( L p ( · ) ) ≲ 1 . This finishes the proof of this claim.

Now, we turn to prove (3.15). Let r : = min { 1 , p -} . Then, by Lemmas 3.18 and 3.14 and by the disjointness of Q j , we find that, for any j ∈ Z + ,

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Now, let xQ be the center of Q . Then, from Lemma 3.23, it follows that, for any j ∈ Z + , Q ∈ Q j and any x ∈ Q ,

<!-- formula-not-decoded -->

By this, (3.17), (3.5), and Lemma 3.19, we obtain

<!-- formula-not-decoded -->

From Lemmas 3.18 and 2.10, we deduce that, for any cube Q in R n ,

<!-- formula-not-decoded -->

Using this, (3.18), (3.19), Lemmas 3.15, 3.18, and the disjointness of cubes of Q j , we conclude that

<!-- formula-not-decoded -->

From (3.16), it follows naturally that ∥ δ -1 j 2 js ( · ) q ( · ) t q ( · ) j ∥ L p ( · ) q ( · ) ≤ 1 , which, combined with Lemma 3.17, further implies that ∥ δ -1 q ( · ) j 2 js ( · ) t j ∥ L p ( · ) ≤ 1 . From this and (3.21), we conclude that

<!-- formula-not-decoded -->

This finishes the proof of (3.15) and hence Lemma 3.13.

<!-- formula-not-decoded -->

Finally, we show the last part of the proof of Lemma 3.13.

Lemma 3.24. Let p ( · ) , q ( · ) , s ( · ) , { φ j } j ∈ Z + , W, and A be the same as in Theorem 3.8. Then, for any ⃗ f ∈ ( S ′ ) m ,

<!-- formula-not-decoded -->

where the implicit positive constant is independent of ⃗ f .

Before giving the proof of Lemma 3.24, we recall some necessary tools. For any N ∈ Z + and ⃗ f ∈ ( S ′ ) m , let

<!-- formula-not-decoded -->

where, for any Q ∈ Q + ,

<!-- formula-not-decoded -->

For any sequence t : = { tQ } Q ∈Q + ⊂ C , r ∈ (0 , ∞ ], and λ ∈ (0 , ∞ ), let t ∗ r ,λ : = { ( t ∗ r ,λ ) Q } Q ∈Q + , where, for any Q ∈ Q + ,

<!-- formula-not-decoded -->

The following lemma is exactly [51, Lemma 3.13].

Lemma 3.25. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH and let s ( · ) ∈ LH, r ∈ (0 , p -) , e R : = r min { 2 clog ( q ) + clog ( s ) , 2( 1 q --1 q + ) + s + -s -} , and λ &gt; n + e R. Then, for any t : = { tQ } Q ∈Q + , ∥ t ∗ r ,λ ∥ b s ( · ) p ( · ) , q ( · ) ∼ ∥ t ∥ b s ( · ) p ( · ) , q ( · ) , where the positive equivalence constants are independent of t.

The following lemma is exactly [20, Lemma 3.15] via replacing the definition of bQ , N from using the reducing operators A for Ap , ∞ to using the reducing operators A for A p , ∞ . But noticing that the proof of [20, Lemma 3.15] just needs the strong doubling property of A , which is also guaranteed by Lemma 2.14 for the reducing operators A for A p ( · ) , ∞ . Hence, we obtain the following result; we omit the details here.

Lemma 3.26. Let j ∈ Z + , ⃗ f ∈ ( S ′ ) m satisfying supp b ⃗ f ⊂ n ξ ∈ R n : | ξ | ≤ 2 j + 1 o , A : = { AQ } Q ∈Q + be strongly doubling of order ( d 1 , d 2) for some d 1 , d 2 ∈ [0 , ∞ ) , and N ∈ Z + su ffi ciently large. For any Q ∈ Q + , let aQ : = | Q | 1 2 supp y ∈ Q | AQ ⃗ f ( y ) | and

<!-- formula-not-decoded -->

Let a : = { aQ } Q ∈Q + , b : = { bQ , N } Q ∈Q + , r ∈ (0 , ∞ ) , and λ ∈ ( n , ∞ ) . Then, for any Q ∈ Q j , ( a ∗ r ,λ ) Q ∼ ( b ∗ r ,λ ) Q, where the positive equivalence constants are independent of ⃗ f , j, and Q.

The following lemma is exactly [41, Lemma 2.8].

Lemma 3.27. Let p ( · ) ∈ P 0 with p ( · ) ∈ LH. Then there exists a positive constant C, depending only on p ( · ) and n, such that, for any cube Q in R n and any x , y ∈ Q, | Q | -| p ( x ) -p ( y ) | ≤ C .

We also need the following [42, Proposition 3.8]. In what follows, for any p ( · ) ∈ P 0 and any measurable set E in R n with | E | ∈ (0 , ∞ ), let pE ∈ (0 , ∞ ) satisfy 1 pE = &gt; E 1 p ( x ) dx .

Lemma 3.28. Let p ∈ P 0 with p ( · ) ∈ LH. Then, for any measurable set E in R n with | E | ∈ (0 , ∞ ) ,

<!-- formula-not-decoded -->

where the positive equivalence constants depend only on p ( · ) and n.

The following lemma shows an estimate about ∥ 1 EQ ∥ L p ( · ) .

Lemma 3.29. Let p ( · ) ∈ P 0 with p ( · ) ∈ LH and let δ ∈ (0 , 1) . Then, for any cube Q in R n with | Q | ∈ (0 , 1] and any measurable set EQ ⊂ Q with | EQ | ∈ (0 , δ | Q | ] , there exists a positive constant C, independent of Q and EQ, such that

<!-- formula-not-decoded -->

Proof. By Lemma 3.28, we find that, for any EQ ⊂ Q with | EQ | ≥ δ | Q | ,

<!-- formula-not-decoded -->

From this and the assumption | Q | ≤ 1 and from Lemma 3.27, the fact 1 p ( · ) ∈ LH , and Jensen's inequality, we deduce that

<!-- formula-not-decoded -->

Thus, using this, Lemma 3.28, and (3.25), we conclude that

<!-- formula-not-decoded -->

which completes the proof of Lemma 3.29.

□

Lemma 3.30. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH and let s ( · ) ∈ LH and δ ∈ (0 , 1) . If { EQ } Q ∈Q + is a sequence of measurable sets in R n satisfying EQ ⊂ Q and | EQ | ≥ δ | Q | for any Q ∈ Q + , then, for any sequence t : = { tQ } Q ∈Q + ⊂ C ,

<!-- formula-not-decoded -->

where the positive equivalence constants are independent of t.

Proof. It follows immediately from the assumption that EQ ⊂ Q for any Q ∈ Q + that

<!-- formula-not-decoded -->

Now, we prove the converse inequality of (3.26). Similar to the claim (3.15), to show (3.26), it is su ffi cient to prove that, for any t : = { tQ } Q ∈Q + ⊂ C , if t satisfies

<!-- formula-not-decoded -->

then, for any j ∈ Z + ,

<!-- formula-not-decoded -->

where the implicit positive constant is independent of j and t and

<!-- formula-not-decoded -->

Let r : = min { 1 , p -} . From Lemma 3.14 and the disjointness of Q j , we infer that, for any j ∈ Z + ,

<!-- formula-not-decoded -->

Using this, (3.18), Lemmas 3.19, and 3.29 with the fact that | Q | ≤ 1 for any Q ∈ Q j and the assumption | EQ | ≥ δ | Q | and using Lemma 3.15, we conclude that

<!-- formula-not-decoded -->

which, combined with Lemma 3.17 and 3.18, further implies that

<!-- formula-not-decoded -->

This finishes the proof of Lemma 3.30.

□

The following lemma is exactly [96, Lemma 3.25] (see [16, Corollary 3.9] for the related result for Ap , ∞ -matrix weights).

Lemma 3.31. Let p ( · ) ∈ P 0 with p ( · ) ∈ LH and let W ∈ A p ( · ) , ∞ . Then there exists a positive constant C such that, for any cube Q of R n and any M ∈ (0 , ∞ ) ,

<!-- formula-not-decoded -->

Now, we give the proof of Lemma 3.24.

Proof of Lemma 3.24. From Lemma 3.26 and the fact that supp [ φ j ∗ ⃗ f ⊂ { ξ ∈ R n : | ξ | ≤ 2 j + 1 } for any j ∈ Z + , we deduce that, for any r ∈ (0 , ∞ ), λ ∈ ( n , ∞ ), j ∈ Z + , and Q ∈ Q j , ( a ∗ r ,λ ) Q ∼ ( b ∗ r ,λ ) Q , where a : = { aQ } Q ∈Q + and b : = { bQ , N } Q ∈Q + are the same as in Lemma 3.26. By this and Lemma 3.25, we conclude that

<!-- formula-not-decoded -->

Notice that, by (3.22) and (3.24), we find that, for any e Q ∈ Q jQ + N with e Q ⊂ Q and for any y ∈ e Q ,

<!-- formula-not-decoded -->

Let EQ : = { y ∈ e Q : ∥ A e Q W -1 ( y ) ∥ &lt; ( C [ W ] A p ( · ) , ∞ ) 2 } , where C is the same as in Lemma 3.31. Then it follows from (3.29) and the assumption EQ ⊂ Q that, for any Q ∈ Q + ,

<!-- formula-not-decoded -->

Observe that, by the definition of EQ and Lemma 3.31, we obtain | EQ | = | e Q | - | e Q \ EQ | ≥ 1 2 | e Q | = 2 -Nn -1 | Q | . Using this, (3.28), and Lemma 3.30 and using (3.29) and (3.30), we conclude that

<!-- formula-not-decoded -->

which completes the proof of Lemma 3.24. □

Finally, we give the proof of Theorem 3.8.

Proof of Thorem 3.8. By Lemmas 3.24 and 3.13, we obtain ∥ ⃗ f ∥ B s ( · ) p ( · ) , q ( · ) ( A ,φ ) ≲ ∥ ⃗ f ∥ B s ( · ) p ( · ) , q ( · ) ( A ,φ ) ≲ ∥ sup A ,φ ( ⃗ f ) ∥ b s ( · ) p ( · ) , q ( · ) , which, together with Lemma 3.9, gives the equivalence of all above norms and hence Theorem 3.8. □

## 3.2 Matrix-Weighted Variable Besov Sequence Spaces

In this subsection, we introduce two matrix-weighted variable Besov sequences spaces, b s ( · ) p ( · ) , q ( · ) ( W ) and b s ( · ) p ( · ) , q ( · ) ( A ), and give their equivalence. We begin with the following sequence spaces.

Definition 3.32. Let p ( · ) , q ( · ) ∈ P 0, and s ( · ) ∈ L ∞ and let W be a matrix weight. The (pointwise) matrix-weighted variable Besov sequence space b s ( · ) p ( · ) , q ( · ) ( W ) is defined to be the set of all sequences ⃗ t : = { ⃗ tQ } Q ∈Q + ⊂ C m such that

<!-- formula-not-decoded -->

where, for any j ∈ Z + , ⃗ t j : = P Q ∈Q j ⃗ tQ e 1 Q .

Next, we introduce the concept of the averaging matrix-weighted variable Besov sequence spaces.

Definition 3.33. Let p ( · ) , q ( · ) ∈ P 0, and s ( · ) ∈ L ∞ and let W be a matrix weight and A : = { AQ } Q ∈Q + reducing operators of order p ( · ) for W . The averaging matrix-weighted variable Besov sequence space b s ( · ) p ( · ) , q ( · ) ( A ) is defined to be the set of all sequences ⃗ t : = { ⃗ tQ } Q ∈Q + ⊂ C m such that

<!-- formula-not-decoded -->

where, for any j ∈ Z + , Aj : = P Q ∈Q j AQ 1 Q .

Similarly to the equivalence between the pointwise matrix-weighted Besov space and averaging matrix-weighted one, the above two types of matrix-weighted variable Besov sequence spaces are also equivalent, which is exactly the following result.

Theorem 3.34. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH, and s ( · ) ∈ LH and let W ∈ A p ( · ) , ∞ and A : = { AQ } Q ∈Q + be reducing operators of order p ( · ) for W. Then, for any sequence ⃗ t : = { ⃗ tQ } Q ∈Q + ⊂ C m ,

<!-- formula-not-decoded -->

where the positive equivalence constants are independent of ⃗ t.

Proof. We first prove

<!-- formula-not-decoded -->

Similar to the claim (3.15), to show (3.32), it is su ffi cient to prove that, for any ⃗ t : = { tQ } Q ∈Q + ⊂ C satisfies P j ∈ Z + ∥ 2 js ( · ) q ( · ) | Aj ⃗ t j | q ( · ) ∥ L p ( · ) q ( · ) = 1 and for any j ∈ Z + ,

<!-- formula-not-decoded -->

where the implicit positive constant is independent of ⃗ t and j and

<!-- formula-not-decoded -->

Letting r : = min { 1 , p -} , by (3.18), Lemmas 3.18, 3.14, and the disjointness of Q j , we find that

<!-- formula-not-decoded -->

From this, (3.20), Lemmas 3.19, and 3.15, we infer that

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Since δ j : = ∥ 2 js ( · ) q ( · ) | Aj ⃗ t j | q ( · ) ∥ L p ( · ) q ( · ) + 2 -j , it follows that ∥ δ -1 j 2 js ( · ) q ( · ) | Aj ⃗ t j | q ( · ) ∥ L p ( · ) q ( · ) ≤ 1 , which, combined with Lemma 3.17, further implies that ∥ δ -1 q ( · ) j 2 js ( · ) | Aj ⃗ t j |∥ L p ( · ) ≤ 1 . By this and (3.33), we conclude that ∥ δ -1 q ( · ) j 2 js ( · ) | W ( · ) ⃗ t j |∥ L p ( · ) ≲ 1 and hence the proof of (3.32).

Next, we prove the converse inequality of (3.31), that is,

<!-- formula-not-decoded -->

Similar to the claim (3.15), to prove (3.34), it is su ffi cient to show that, for any ⃗ t : = { tQ } Q ∈Q + ⊂ C satisfying P ∞ j ∈ Z + ∥ 2 js ( · ) q ( · ) | W ( · ) ⃗ t j | q ( · ) ∥ L p ( · ) q ( · ) = 1 and for any j ∈ Z + ,

<!-- formula-not-decoded -->

where the implicit positive constant is independent of ⃗ t and j and δ j : = ∥ 2 js ( · ) q ( · ) | W ( · ) ⃗ t j | q ( · ) ∥ L p ( · ) q ( · ) + 2 -j .

Let r : = min { 1 , p -} . Then, using Lemmas 3.18, 3.14, and Lemma 3.19 and using the disjointness of Q j , we obtain

<!-- formula-not-decoded -->

Combining this with Lemmas 2.10, 3.15, and 3.18, we conclude that

<!-- formula-not-decoded -->

Notice that, by the definition of δ j , we obtain ∥ δ -1 j 2 js ( · ) q ( · ) | W ( · ) ⃗ t j | q ( · ) ∥ L p ( · ) q ( · ) ≤ 1 , which, together with Lemma 3.17, further implies that ∥ δ -1 q ( · ) j 2 js ( · ) | W ( · ) ⃗ t j |∥ L p ( · ) ≤ 1 . Using this and (3.35), we obtain ∥ δ -1 q ( · ) j 2 js ( · ) | Aj ⃗ t j |∥ L p ( · ) ≲ 1 . This finishes the proof of (3.34) and hence the proof of Theorem 3.34.

□

## 3.3 The φ -Transform

In this subsection, we establish the φ -transform characterization of matrix-weighted variable Besov spaces. We first recall some basic notions and properties. Let { φ j } j ∈ Z + be as in Definition 3.3. Then there exists { ψ } j ∈ Z + , satisfying the same conditions as { φ j } j ∈ Z + as in Definition 3.3, such that, for any ξ ∈ R n ,

<!-- formula-not-decoded -->

The φ -transform S φ is defined to be the map taking each ⃗ f ∈ ( S ′ ) m to the sequence S φ ⃗ f : = { ( S φ ⃗ f ) Q } Q ∈Q + , where ( S φ ⃗ f ) Q : = ⟨ ⃗ f , φ Q ⟩ for any Q ∈ Q + . The inverse φ -transform T ψ is defined to be the map taking a sequence ⃗ t : = { ⃗ tQ } Q ∈Q + ⊂ C m to T ψ ⃗ t : = P Q ∈Q + ⃗ tQ ψ Q in ( S ′ ) m .

The following theorem is the main result of this subsection. In what follows, for any x ∈ R n , let e φ ( x ) : = φ ( -x ).

Theorem 3.35. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH, and s ( · ) ∈ LH and let { φ j } j ∈ Z + be as in Definition 3.3 and { ψ j } j ∈ Z + be a sequence of functions satisfy (3.36). Then the operators S φ : B s ( · ) p ( · ) , q ( · ) ( W , e φ ) → b s ( · ) p ( · ) , q ( · ) ( W ) and T ψ : b s ( · ) p ( · ) , q ( · ) ( W ) → B s ( · ) p ( · ) , q ( · ) ( W , φ ) are bounded. Furthermore, T ψ ◦ S φ is the identity on B s ( · ) p ( · ) , q ( · ) ( W , e φ ) .

Before giving the proof of Theorem 3.35, we first point out that Theorem 3.35 implies that B s ( · ) p ( · ) , q ( · ) ( W , φ ) is independent of the choice of ( Φ,φ ).

Proposition 3.36. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH, and s ( · ) ∈ LH and let { φ j } j ∈ Z + be as in Definition 3.3 and W ∈ A p ( · ) , ∞ . Then B s ( · ) p ( · ) , q ( · ) ( W , φ ) is independent of the choice of φ .

Proof. Let { φ (1) j } j ∈ Z + and { φ (2) j } j ∈ Z + be as in Definition 3.3 and let { ψ (2) j } j ∈ Z + be as in (3.36) such that (3.36) holds for { φ (2) j } j ∈ Z + and { ψ (2) j } j ∈ Z + . Then, using Theorem 3.35, we conclude that, for any ⃗ f ∈ B s ( · ) p ( · ) , q ( · ) ( W , φ (2) ),

<!-- formula-not-decoded -->

Bysymmetry, we also obtain the reverse inequality. This finishes the proof of Proposition 3.36. □

Now, to prove Theorem 3.35, we first recall some basic lemmas. This following lemma is exactly [54, (12.4)].

Lemma 3.37. Let { φ j } j ∈ Z + be as in Definition 3.3 and let { ψ j } j ∈ Z + be as in (3.36). Then, for any f ∈ S ′ ,

<!-- formula-not-decoded -->

where the equivalence is in the sense of S ′ .

The following lemma is exactly [98, Lemma 2.4].

Lemma 3.38. Let M ∈ Z + and ψ, φ ∈ S with ψ satisfying R R n x γ ψ ( x ) dx = 0 for all multi-indices γ ∈ Z n + satisfying | γ | ≤ M. Then, for any j ∈ Z + and any x ∈ R n ,

<!-- formula-not-decoded -->

where the implicit positive constant depends only on n and M.

The following lemma is precisely [95, Lemma 2.2].

Lemma 3.39. Let M ∈ Z + and ψ, φ ∈ S∞ . Then, for any j , i ∈ Z + and any x ∈ R n ,

<!-- formula-not-decoded -->

where the implicit positive constant depends only on n and M.

The following lemma guarantees the convergence of the T ψ ⃗ t for any ⃗ t ∈ b s ( · ) p ( · ) , q ( · ) ( W ).

Lemma 3.40. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH, and s ( · ) ∈ LH and let { ψ j } j ∈ Z + be as in Definition 3.3 and W ∈ A p ( · ) , ∞ . Then, for any ⃗ t : = { ⃗ tQ } Q ∈Q + ∈ b s ( · ) p ( · ) , q ( · ) ( W ) , P Q ∈Q + ⃗ tQ ψ Q converges in ( S ′ ) m . Moreover, if M ∈ Z + also satisfies

<!-- formula-not-decoded -->

where ∆ is the same as in Lemma 2.14, then, for any ⃗ t ∈ b s ( · ) p ( · ) , q ( · ) ( W ) and ϕ ∈ S ,

<!-- formula-not-decoded -->

where the implicit positive constant is independent of ⃗ t.

Proof. Let { AQ } Q ∈Q + be reducing operators of order p ( · ) for W . From Lemma 3.28 and the fact 1 pQ ≤ 1 p -, we obtain, for any j ∈ Z + and Q ∈ Q j ,

<!-- formula-not-decoded -->

where t j is the same as in (3.3) and Aj the same as in (3.2). By [2, Example 3.4], we find that, for any function sequence { f j } j ∈ Z + with f j : = 0 for any j ≥ 1, ∥{ f j }∥ l q ( · ) ( L p ( · ) ) = ∥ f 0 ∥ L p ( · ). Using this, (3.38), and Theorem 3.34, we conclude that, for any j ∈ Z + and Q ∈ Q j ,

<!-- formula-not-decoded -->

which further implies that, for any ϕ ∈ S ,

<!-- formula-not-decoded -->

By Lemma 2.14 and the fact that l ( Q ) ≤ 1 for any Q ∈ Q + , we have

<!-- formula-not-decoded -->

where d 2 ∈ [ [ d upper p ( · ) , ∞ ( W ) , ∞ ) is a fixed parameter. Let M ∈ N satisfy M &gt; max { d 2 + n p --s -, ∆ } . Then, if j ≥ 1, by Lemma 3.38 and the fact ψ j ∈ S∞ , we obtain, for any ϕ ∈ S and Q ∈ Q j ,

<!-- formula-not-decoded -->

From this, (3.39), and (3.40), we deduce that that

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Now, if j = 0, then, using the definition of ∥ · ∥S M + 1 and the fact Ψ ∈ S , we find that

<!-- formula-not-decoded -->

which, combined with (3.39) and (3.40), further implies that

<!-- formula-not-decoded -->

From this and (3.42), we infer that (3.39) converges absolutely. Thus, P Q ∈Q + ⃗ tQ ψ Q converges in S ′ , which completes the proof of Lemma 3.40. □

The following lemma is exactly [16, Lemma 2.31].

Lemma 3.41. For any cubes Q , R ⊂ R n , any x , x ′ ∈ Q, and any y , y ′ ∈ R,

<!-- formula-not-decoded -->

where the positive equivalence constants depend only on n.

The following lemma gives a su ffi cient condition ensuring that ∥ · ∥ l q ( · ) ( L p ( · ) ) is a norm, which is exactly [2, Theorems 3.6 and 3.8].

Lemma 3.42. Let p ( · ) , q ( · ) ∈ P 0 . Then ∥ · ∥ l q ( · ) ( L p ( · ) ) is a quasi-norm. Moreover, if p ( · ) , q ( · ) ∈ P satisfy either 1 p ( · ) + 1 q ( · ) ≤ 1 pointwise or q is a constant, then ∥ · ∥ l q ( · ) ( L p ( · ) ) is a norm.

Finally, we give the proof of Theorem 3.35.

Proof of Theorem 3.35. We first show the boundedness of S φ . For any ⃗ f ∈ B s ( · ) p ( · ) , q ( · ) ( W , e φ ), letting sup A , e φ ( ⃗ f ) be as in (3.4), then, by the definition of S φ , we obtain, for any Q ∈ Q + ,

<!-- formula-not-decoded -->

which, together with Theorems 3.34 and 3.8, further implies that

<!-- formula-not-decoded -->

This finishes the proof of the boundedness of S φ .

Next, we show the boundedness of T ψ . By Lemma 3.40, we find that T ψ is well defined for any ⃗ t ∈ b s ( · ) p ( · ) , q ( · ) and hence, for any j ∈ Z + , Q ∈ Q j , and any x ∈ Q ,

<!-- formula-not-decoded -->

Notice that, for any { ψ i } i ∈ Z + and { φ j } j ∈ Z + as in Definition 3.3 and any i , j ∈ Z + , if | i -j | &gt; 1, then ψ i ∗ φ j = 0 . Using this and (3.43), we conclude that, for any j ∈ Z + , Q ∈ Q j , and any x ∈ Q ,

<!-- formula-not-decoded -->

By Lemma 2.14, we find that, for any j , i ∈ Z + with | i -j | ≤ 1 and for any Q ∈ Q j and R ∈ Q i ,

<!-- formula-not-decoded -->

where d 1 , d 2 , ∆ are the same as in Lemma 2.14. Let M satisfy (3.37). Then, from Lemmas 3.38 and 3.39 (or, when both j , i = 0, from the fact that, for any M &gt; 0 and any x ∈ R n , φ 0 ∗ ψ 0( x ) ≲ C (1 + | x | ) -( n + M ) ), it follows that, for any j , i ∈ Z + with | i -j | ≤ 1 and for any R ∈ Q i and x ∈ R n ,

<!-- formula-not-decoded -->

Let u : = { uQ } Q ∈Q + , where uQ : = | AQ ⃗ tQ | for any Q ∈ Q + . Then, by (3.44), (3.45), and (3.46), we conclude that, for any j ∈ Z + , Q ∈ Q j , and any x ∈ Q ,

<!-- formula-not-decoded -->

where, for any i ∈ Z + ,

<!-- formula-not-decoded -->

Notice that, by the definition of dyadic cubes, for any x ∈ R n and j ∈ Z + , there exist a unique cube Q ∈ Q j such that x ∈ Q . Combining this, (3.48), and Lemma 3.41, we obtain

<!-- formula-not-decoded -->

where ( u ∗ 1 , n + M -∆ ) Q is the same as in (3.23). Applying this with (3.47), we conclude that, for any j ∈ Z + ,

<!-- formula-not-decoded -->

In what follows, for simplicity of presentation, we ( u ∗ 1 , n + M -∆ ) -1 : = 0. By this, Lemmas 3.8, 3.42, and s ( · ) ∈ LH and by Lemma 3.25, we find that

<!-- formula-not-decoded -->

This finishes the proof of the boundedness of T ψ .

Finally, if { φ j } j ∈ Z + and { ψ j } j ∈ Z + satisfy (3.36), then it follows immediately from Lemma 3.37 than T ψ ◦ S φ is the identity on B s ( · ) p ( · ) , q ( · ) ( W , e φ ), which completes the proof of Theorem 3.35. □

## 4 Almost Diagonal Operators

In this section, we focus on the boundedness of the almost diagonal operators, which is a very useful tool for establishing the characterizations of Besov spaces and the boundedness of operators (see, for instance, [54, 56, 17]). We now recall the basic concept of infinity matrices. Let B : = { bQ , R } Q , R ∈Q + ⊂ C . For any sequence ⃗ t : = { ⃗ tR } R ∈Q + ⊂ C m , we define B ⃗ t : = { ( B ⃗ t ) Q } Q ∈Q + by setting, for any Q ∈ Q + , ( B ⃗ t ) Q : = P R ∈Q + bQ , R ⃗ tR if the above summation is absolutely convergent. Then, we recall the concept of almost diagonal operators, which was first introduced by Frazier and Jawerth in [54]).

Definition 4.1. Let D , E , F ∈ R . We define the special infinite matrix B DEF : = { b DEF Q , R } Q , R ∈Q + ⊂ C by setting, for any Q , R ∈ Q + ,

<!-- formula-not-decoded -->

An infinite matrix B : = { bQ , R } Q , R ∈Q + ⊂ C is said to be ( D , E , F ) -almost diagonal if there exists a positive constant C such that, for any Q , R ∈ Q + , | bQ , R | ≤ Cb DEF Q , R .

- Remark 4.2. (i) If E + F &gt; 0, which is always the only case interested to us, then the second factor on the right-hand side of (4.1) is equivalent to

<!-- formula-not-decoded -->

- (ii) It is obvious that the special infinite matrix B DEF is ( D , E , F )-almost diagonal.

The following is the boundedness of the almost diagonal operators on matrix-weighted variable Besov space, which is the main result of this section. We refer to [17] for the known best result about almost diagonal operators on matrix Ap weighted Besov spaces and to [20] for the result on matrix Ap , ∞ weighted Besov spaces.

Theorem 4.3. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH and let s ( · ) ∈ LH and W ∈ A p ( · ) . If B is ( D , E , F ) -almost diagonal, then B is bounded on b s ( · ) p ( · ) , q ( · ) ( W ) whenever

<!-- formula-not-decoded -->

where

<!-- formula-not-decoded -->

C ( s , q ) : = C log ( s ) + C log ( q -1 ) with C log ( s ) and C log ( q -1 ) being the same as in (2.4).

Remark 4.4. When we reduce to the scalar-valued case with W : = 1, the result of Theorem 4.3 is better than [64, Theorem 2]. What is more, when p ( · ) , q ( · ) , s ( · ) are constant exponents, it is obvious that the constant C ( s , q ) = 0 and s + = s -= s and hence the result of Theorem 4.3 coincides with the result of [20, Theorem 4.6] with τ : = 0.

Before giving the proof of Theorem 4.3, we first give some basic tool. The following result is the estimate about 2 js ( · ) .

Lemma 4.5. Let s ( · ) ∈ C log . Then, for any j , l ∈ Z + and x , y ∈ R n with | x -y | ≤ 2 l -j ,

<!-- formula-not-decoded -->

where the implicit positive constant is independent of j and l, and, moreover, for any δ ∈ [2 -j , 2 -j + 1] ,

<!-- formula-not-decoded -->

where C log ( s ) is the same as in (2.4) and the implicit positive constant is independent of j and l.

Proof. First, we give the proof of (4.4). Indeed, by (2.4), we find immediately that, for any x , y ∈ R n with | x -y | ≤ 2 l -j ,

<!-- formula-not-decoded -->

Now, we first consider the case j ≤ l . In this case, by (4.6), we obtain immediately

<!-- formula-not-decoded -->

Next, we consider the case j &gt; l . In this case, since j &gt; l , we deduce that 2 l -j ≤ 1. From this, (4.6), and Lemma 3.23, it follows immediately that

<!-- formula-not-decoded -->

This finishes the proof of (4.4).

Next, we give the proof of (4.5). If j ≤ l , then, combining (4.6) with the assumption δ ∈ [2 -j , 2 -j + 1], we conclude that, if s ( x ) -s ( y ) ≥ 0, then

<!-- formula-not-decoded -->

conversely, if s ( x ) -s ( y ) &lt; 0, then δ s ( y ) -s ( x ) ≤ max { 1 , 2 s ( y ) -s ( x ) } ≤ max { 1 , 2 C log ( s ) } ≤ 2 C log ( s ) . Thus, from these, we deduce that δ s ( y ) = δ s ( y ) -s ( x ) δ s ( x ) ≲ 2 lC log ( s ) δ s ( x ) .

Then we consider the case j &gt; l . By Lemma 3.23 and the fact j &gt; l ≥ 0, we find that

<!-- formula-not-decoded -->

Since (4.6) and (2.4), we deduce that

<!-- formula-not-decoded -->

Now, if s ( x ) -s ( y ) ≥ 0, then, using (4.8) and (4.7) and using the assumption δ ∈ [2 -j , 2 -j + 1], we find that

<!-- formula-not-decoded -->

which further implies that δ s ( y ) ≲ 2 lC log ( s ) δ s ( x ) .

Conversely, if s ( x ) -s ( y ) &lt; 0, then, by (4.8) and (4.7) and by the fact δ ≤ 2 j , we conclude that

<!-- formula-not-decoded -->

which completes the proof of (4.5) and hence Lemma 4.5.

Now, we prove Theorem 4.3.

Proof of Theorem 4.3. Indeed, for any j ∈ Z + , Q ∈ Q j , and x ∈ Q , if ( B ⃗ t ) Q converges absolutely, then we have W ( x )( B ⃗ t ) Q = ( B [ W ( x ) ⃗ t ]) Q . From this, Definition 4.1, and Lemma 3.41, we deduce that, for any j ∈ Z + , Q ∈ Q j , and any x ∈ Q ,

<!-- formula-not-decoded -->

Using this and the definition of ( B ( W ( x ) ⃗ t )) j and using the disjointness of Q j , we conclude that, for any j ∈ Z + and any x ∈ R n ,

<!-- formula-not-decoded -->

□

Notice that, for any i , j ∈ Z + and any x ∈ R n ,

<!-- formula-not-decoded -->

Let r : = min { 1 , p -} . Then, from the fact 2 l -i ∧ j ≥ ≥ 2 -i , we deduce that, for any Q ∈ Q i with Q ∩ B ( x , 2 l -i ∧ j ) , ∅ , Q ⊂ B ( x , cn 2 l -i ∧ j ) with cn : = 1 + √ n . Using this, the fact that t i ( y ) is a constant for any y ∈ Q with Q ⊂ Q i , and the disjointness of Q i yields

<!-- formula-not-decoded -->

which, together with (4.10) and the fact i -i ∧ j = ( i -j ) ( + ) , further implies that

<!-- formula-not-decoded -->

where the last inequality comes from reindexing the summation in l . Thus, combining this, Lemma 4.5, (4.9), and the facts i + j 2 -i ∧ i = 1 2 | i -j | and j -i ∧ j = ( j -i ) ( + ) , we conclude that

<!-- formula-not-decoded -->

where R ′ ∈ ( C log ( s ) , ∞ ) is a fixed constant.

From Lemma 3.42, Remark 3.2(iv), and the fact p ( · ) , q ( · ) ∈ P , we deduce that there exists a positive constant a ∈ (0 , 1] such that a p ( x ) + a q ( x ) ≤ 1 and hence ∥ · ∥ l q ( · ) a ( L p ( · ) a ) is a norm. Then, by (4.11) with letting k : = i -j and rearranging the order of the summation, we find that, for any x ∈ R n ,

<!-- formula-not-decoded -->

Hence, using this and the precious discussion that ∥ · ∥ l q ( · ) a ( L p ( · ) a ) is a norm, we conclude that

<!-- formula-not-decoded -->

From this with rearranging the order of the summation, it follows that

<!-- formula-not-decoded -->

where and

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

.

To give the estimate of I1 and I2, we claim that, for any l ∈ Z + and k ∈ Z , there exists a positive constant C , independent of l and k , such that

<!-- formula-not-decoded -->

where d 2 ∈ [ [ d upper p ( · ) ∞ ( W ) , ∞ ) is a fixed constant.

, Before giving the proof of this claim, we now assume that the claim (4.13) holds. Then, by (4.13), we obtain immediately, for any D ∈ ( R ′ + n r + C log ( 1 q ) + d 2 , ∞ ), E ∈ ( n 2 + s + ), and F ∈ ( n r -n 2 -s -+ d 2 , ∞ ),

<!-- formula-not-decoded -->

and

<!-- formula-not-decoded -->

which, combined with (4.12), further implies the boundedness of the almost diagonal operators.

Thus, to prove Theorem 4.3, it is su

k

≤ -

1. Since equal with

k

≤ -

1, it follows that

k

ffi

+

cient to show the claim (4.13). We first consider the case

j

≤

j

and hence, under this condition, the claim (4.13) is

<!-- formula-not-decoded -->

To prove this inequality, similarly to the claim (3.15), we only need to show that, for any ⃗ t satisfies

<!-- formula-not-decoded -->

there exists a positive constant C such that, for any j ∈ Z + ,

<!-- formula-not-decoded -->

where δ j : = ∥ 2 jq ( · ) s ( · ) | W ( · ) ⃗ t j | q ( · ) ∥ L p ( · ) q ( · ) + 2 -j , which, together with Lemma 3.17, is equivalent with

<!-- formula-not-decoded -->

From Lemmas 3.18, 3.14, and 4.5 and from the disjointness of Q j -l , we infer that

<!-- formula-not-decoded -->

Indeed, by the geometric observation, for any x ∈ R n and any Q ∈ Q j -l with x ∈ Q , B ( x , 2 l -j ) ⊂ 3 Q . Using this, (4.16), Lemmas 3.19, and 2.10, we find that

<!-- formula-not-decoded -->

Notice that, by the definition of the dyadic cubes, for any cube Q ∈ Q j -l , 3 Q can be overlapped by a sequence of cubes of Q j . From this, (4.17), and Corollary 2.14, we deduce that

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

By this and by Lemmas 3.19, 3.15, and 3.18, we conclude that

<!-- formula-not-decoded -->

which, combined with the definition of δ j and Lemma 3.17, further implies that

<!-- formula-not-decoded -->

This finishes the proof of (4.14) and hence the proof of (4.13) under the case k ≤ -1. Next, we consider the case k ≥ 0. Here, in this condition, (4.13) is equal with

<!-- formula-not-decoded -->

Noticing that, for any k ∈ Z + , we have

<!-- formula-not-decoded -->

Notice that, during the estimation under the condition k ≤ -1, the factor 2 js ( y ) does not influence the constants. Thus, through repeating the precious proof when k ≤ -1 from (4.14) to (4.17), we obtain, for any j ∈ Z + ,

<!-- formula-not-decoded -->

where ⃗ t ∈ b s ( · ) p ( · ) , q ( · ) ( W ) satisfies ∥ ⃗ t ∥ b s ( · ) p ( · ) , q ( · ) ( W ) = 1 and δ j : = ∥ 2 j + kq ( · ) s ( · ) | W ( · ) ⃗ t j + k | q ( · ) ∥ L p ( · ) q ( · ) + 2 -j . Then, using this and repeating the rest discussions under the condition k ≤ -1 with just selecting R from Q j -1 replaced by Q j + k -l , we conclude that

<!-- formula-not-decoded -->

Applying this and (4.19) yields

<!-- formula-not-decoded -->

, which completes the proof of (4.13) and hence Theorem 4.3.

□

- Remark 4.6. (i) Inspired by the proof of Theorem 4.3, as a slight strong result of Theorem 4.3, with just replaced | B ( W ( · ) ⃗ t ) | ( x ) by e 1 Q P R ∈Q + | W ( x ) bQ , R ⃗ tR | , we have the following result that, for any ⃗ t ∈ b s ( · ) p ( · ) , q ( · ) ( W ),

<!-- formula-not-decoded -->

which further implies that, for any Q ∈ Q + and almost every x ∈ Q , P R ∈Q + | W ( x ) bQ , R ⃗ tR | is finite. From this, it follows immediately that

<!-- formula-not-decoded -->

Thus, we find that P R ∈Q + | bQ , R ⃗ tR | convergences absolutely and hence, for any ⃗ t ∈ b s ( · ) p ( · ) , q ( · ) ( W ) and any bounded almost diagonal operator B , B ⃗ t is well defined.

- (ii) Let B (1) : = { b (1) Q , R } and B (2) : = { b (2) Q , R } be b s ( · ) p ( · ) , q ( · ) ( W )-almost diagonal operators. Then, by the boundedness of the almost diagonal operators, it is easy to find that the operator B : = B (1) ◦ B (2) is b s ( · ) p ( · ) , q ( · ) ( W )-almost diagonal. Moreover, if assume that B : = { bQ , R } Q , R ∈Q + , then bQ , R = P P ∈Q + b (1) Q , P b (2) P , R . Indeed, from Remark 4.6(i), it follows that, for any ⃗ t ∈ b s ( · ) p ( · ) , q ( · ) ( W ), B (1) ⃗ t and B (2) ⃗ t are well defined. Hence, for any Q ∈ Q + ,

<!-- formula-not-decoded -->

which further implies that bQ , R = P P ∈Q + b (1) Q , P b (2) P , R .

## 5 Molecules Characterization and Its Applications

In this section, we focus on the molecules characterization of the matrix-weighted variable Besov space. In Subsection 5.1, we establish the molecule characterization of B s ( · ) p ( · ) , q ( · ) ( W ) and then, in Subsection 5.2, by using the obtained molecule characterization, we show the wavelets characterization and atom decomposition of B s ( · ) p ( · ) , q ( · ) ( W ).

## 5.1 Molecules Characterization

In this subsection, we establish the molecule characterization of the matrix-weighted variable Besov space. First, We recall some basic notions. For any r ∈ R , let

<!-- formula-not-decoded -->

and r ∗ : = r - ⌊ r ⌋ and r ∗∗ : = r - ⌊ ⌊ r ⌋ ⌋ .

Next, we recall the concept of molecules.

Definition 5.1. Let K , M ∈ [0 , ∞ ) and L , N ∈ R . For any K ∈ [0 , ∞ ) and Q ∈ Q + with l ( Q ) ≤ 1 and for any x ∈ R n , let

<!-- formula-not-decoded -->

A function mQ ∈ M is called a (smooth) ( K , L , M , N ) -molecule on a cube Q if, for any x , y ∈ R n and any multi-index γ ∈ Z n + in the specified ranges below, it satisfies

- (i) GLYPH&lt;12&gt; GLYPH&lt;12&gt; GLYPH&lt;12&gt; mQ ( x ) GLYPH&lt;12&gt; GLYPH&lt;12&gt; GLYPH&lt;12&gt; ≤ ( uK ) Q ( x ) ,
- (ii) R R n x γ mQ ( x ) dx = 0 if | γ | ≤ L and l ( Q ) &lt; 1 ,
- (iii) GLYPH&lt;12&gt; GLYPH&lt;12&gt; GLYPH&lt;12&gt; ∂ γ mQ ( x ) GLYPH&lt;12&gt; GLYPH&lt;12&gt; GLYPH&lt;12&gt; ≤ [ l ( Q )] -| γ | ( uM ) Q ( x ) if | γ | &lt; N ,
- (iv) GLYPH&lt;12&gt; GLYPH&lt;12&gt; GLYPH&lt;12&gt; ∂ γ mQ ( x ) -∂ γ mQ ( y ) GLYPH&lt;12&gt; GLYPH&lt;12&gt; GLYPH&lt;12&gt; ≤ [ l ( Q )] -| γ | h | x -y | l ( Q ) i N ∗∗ sup | z |≤| x -y | ( uM ) Q ( x + z ) if | γ | = ⌊ ⌊ N ⌋ ⌋ .

The following is the relationship between molecules and almost diagonal operators.

Theorem 5.2. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH, s ( · ) ∈ LH, and W ∈ A p ( · ) , ∞ . Let { mQ } Q ∈Q + be a family of ( Km , Lm , Mm , Nm ) -molecules and let { bQ } Q ∈Q + be another family of ( Kb , Lb , Mb , Nb ) -molecules. Then the infinite matrix {⟨ mQ , bQ ⟩} Q ∈Q + is almost diagonal and bounded on b s ( · ) p ( · ) , q ( · ) ( W ) if

<!-- formula-not-decoded -->

and

<!-- formula-not-decoded -->

where J ( W ) is the same as in (4.3) and C ( s , q ) as in (4.2).

Remark 5.3. If p ( · ) , q ( · ) , s ( · ) are constant exponents, then Theorem 5.2 coincides with [20, Theorem 5.3]. Moreover, when W reduces to the scalar-valued case, Theorem 5.2 is stronger than [90, Theorem 4.2].

Before showing this theorem, we give some basic properties of molecules. The following lemma is exactly [20, Lemma 5.2].

Lemma 5.4. Let mQ be a ( Km , Lm , Mm , Nm ) -molecule on cube Q and let bp be a ( Kb , Lb , Mb , Nb ) -molecule on cube P, where Km , Mm , Kb , Mb ∈ ( n , ∞ ) and Lm , Nm , Lb , Nb are real numbers. Then, for any α ∈ (0 , ∞ ) , there exists a positive constant C such that

<!-- formula-not-decoded -->

where b MGH Q , P is the same as in (4.1) with M : = Km ∧ Mm ∧ Kb ∧ Mb ∈ ( n , ∞ ) , and

<!-- formula-not-decoded -->

Now, we give the proof of Theorem 5.2

Proof of Theorem 5.2. It follows from Lemma 5.4 and Theorem 4.3 that, to show the boundedness of {⟨ mQ , bP ⟩} Q , P ∈Q + , it is su ffi cient to keep

<!-- formula-not-decoded -->

where M , G , and H are the same as in Lemma 5.4.

By Lemma 5.4, we obtain M = Km ∧ Mm ∧ Kb ∧ Mb and hence, combined this with (5.4), to keep (5.4) holding, we need Km , Mm , Kb , Mb &gt; J ( W ) + C ( s , q ) . Moreover, from Lemma 5.4 and (5.4), we deduce that we need Nb ∧ ⌈ ⌈ Lm ⌉ ⌉ ∧ ( Km -n -α ) &gt; s + , which further implies that Nb &gt; s + , ⌈ ⌈ Lm ⌉ ⌉ &gt; s + , and Km -n -α &gt; s + . By the alternative of α , we conclude that Km &gt; n + s + . Next, we give the estimate of Lm . Indeed, from facts that ⌈ ⌈ y ⌉ ⌉ = ⌊ y ⌋ + 1 for any y ∈ R and ⌈ ⌈ x ⌉ ⌉ ≥ ⌈ ⌈ y ⌉ ⌉ for any x , y ∈ R with x &gt; y , it follows that ⌊ Lm ⌋ + 1 = ⌈ ⌈ Lm ⌉ ⌉ ≥ ⌈ ⌈ s + ⌉ ⌉ = ⌊ s + ⌋ + 1 and hence Lm ≥ ⌊ Lm ⌋ ≥ ⌊ s + ⌋ . Noticing that the molecule condition of Lm only relied on its integer part, we may as well take Lm ≥ s + without changing the condition if Lm ≥ ⌊ s + ⌋ . Thus, summarizing all the above discussions, we conclude that Nb &gt; s + , Lm ≥ s + , and Km &gt; n + s + .

Finally, similarly to the discussion about the case G &gt; n 2 + s + with replaced G by H and Nb , Lm , Km by Nm , Lb , Kb , we obtain immediately Nm &gt; J ( W ) -n -s -, Lb ≥ J ( W ) -n -s -, and Km &gt; J ( W ) -s -. This finishes the proof of Theorem 5.2. □

Next, by using Theorem 5.2, we introduce the concepts of synthesis molecule and analysis molecules of B s ( · ) p ( · ) , q ( · ) ( W ) (see [18] for those molecules of matrix Ap weighted Besov spaces and [20] for molecules of matrix Ap , ∞ weighted Besov spaces).

Definition 5.5. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH and let s ( · ) ∈ LH and W ∈ A p ( · ) , ∞ . A ( K , L , M , N )-molecule mQ is called an B s ( · ) p ( · ) , q ( · ) ( W ) -analysis molecule on Q if K , L , M , N satisfy (5.1). Moreover, a ( K , L , M , N )-molecule mQ is called an B s ( · ) p ( · ) , q ( · ) ( W ) -synthesis molecule on Q if K , L , M , N satisfy (5.2).

From Theorems 5.2 and 4.3, we have the following results.

Lemma 5.6. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH, s ( · ) ∈ LH, and W ∈ A p ( · ) , ∞ and let { φ j } j ∈ Z + be as in Definition 3.3 and { ψ j } j ∈ Z + satisfy (3.36) with { φ j } j ∈ Z + . Suppose that { m ( i ) Q } Q ∈Q + with i ∈ { 1 , 2 } are families of B s ( · ) p ( · ) , q ( · ) ( W ) -analysis molecules and { b ( i ) Q } Q ∈Q + with i ∈ { 1 , 2 } are families of B s ( · ) p ( · ) , q ( · ) ( W ) -synthesis molecules. Then

- (i) for any i ∈ { 1 , 2 } , the infinity matrices

<!-- formula-not-decoded -->

are b s ( · ) p ( · ) , q ( · ) ( W ) -almost diagonal.

- (ii) if ⃗ t : = { ⃗ tQ } Q ∈Q + ∈ b s ( · ) p ( · ) , q ( · ) ( W ) , then ⃗ sP : = P Q , R ∈Q + ⟨ m (1) P , b (1) Q ⟩⟨ m (2) Q , b (2) R ⟩ ⃗ tR converges unconditionally for any P ∈ Q + and ⃗ s : = { ⃗ sP } P ∈Q + satisfying ∥ ⃗ s ∥ b s ( · ) p ( · ) , q ( · ) ( W ) ≲ ∥ ⃗ t ∥ b s ( · ) p ( · ) , q ( · ) ( W ) , where the implicit positive constant is independent of ⃗ t, { m ( i ) Q } Q ∈Q + , and { b ( i ) Q } Q ∈Q + .

Proof. Notice that, for any pairs of { φ R } R ∈Q + and { ψ R } R ∈Q + satisfies (3.36), { φ R } R ∈Q + (respectively, { ψ R } R ∈Q + ) is a family of B s ( · ) p ( · ) , q ( · ) ( W )-synthesis molecules (respectively, a family of B s ( · ) p ( · ) , q ( · ) ( W )-analysis molecules) (with harmless constant multiples). Combining this with Theorem 5.2, we conclude that matrices {⟨ m ( i ) P , b ( i ) Q ⟩} P , Q ∈Q + , {⟨ m ( i ) P , ψ Q ⟩} P , Q ∈Q + , and {⟨ φ P , b ( i ) Q ⟩} P , Q ∈Q + with i ∈ { 1 , 2 } are bounded almost diagonal operators, which completes the proof of (i).

Next, we give the proof of (ii). By Theorem 5.2, we find that {⟨ m ( i ) P , b ( i ) Q ⟩} P , Q ∈Q + with i ∈ { 1 , 2 } are b s ( · ) p ( · ) , q ( · ) ( W )-almost diagonal. Using this and Remark 4.6(ii), we conclude that B : = { bP , R } P , R ∈Q + with

<!-- formula-not-decoded -->

is a b s ( · ) p ( · ) , q ( · ) ( W )-almost diagonal operator. Hence, from this and the assumption ⃗ t ∈ b s ( · ) p ( · ) , q ( · ) ( W ) and from Remark 4.6(i), we infer that, for any P ∈ Q + ,

<!-- formula-not-decoded -->

This finishes the proof of (ii) and hence Lemma 5.6.

□

Next, we recall the concept of ⟨ ⃗ f , mQ ⟩ . By the definition of B s ( · ) p ( · ) , q ( · ) ( W ), it is obvious that B s ( · ) p ( · ) , q ( · ) ( W ) is a subset of ( S ′ ) m . However, since the analysis molecule mQ might not be in S , it follows that the notion ⟨ ⃗ f , mQ ⟩ may be meaningless. The following lemma gives the definition of ⟨ ⃗ f , mQ ⟩ and guarantees that this notion is well-defined. Its proof is similar to that of [18, Lemma 3.16] with [18, Corollary 3.15] replaced by Lemma 5.6; we omit the details here.

Lemma 5.7. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH and let s ( · ) ∈ LH and W ∈ A p ( · ) , ∞ . If ⃗ f ∈ B s ( · ) p ( · ) , q ( · ) ( W ) and mQ is a B s ( · ) p ( · ) , q ( · ) ( W ) -analysis molecule on cube Q, then, for any pair of { φ R } R ∈Q + and { ψ R } R ∈Q + as in (3.36), the pairing

<!-- formula-not-decoded -->

is well-defined; moreover, the series above converges absolutely and its value is independent of the choice of { φ R } R ∈Q + and { ψ R } R ∈Q + .

The following result is the molecules characterization of the matrix-weighted variable Besov spaces (see [90, Theorem 4.7] for the molecular characterizations of the scalar weighted variable Besov spaces).

Theorem 5.8. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH and let s ( · ) ∈ LH and W ∈ A p ( · ) , ∞ .

- (i) If { mQ } Q ∈Q + is a family of B s ( · ) p ( · ) , q ( · ) ( W ) -analysis molecules, then, for any ⃗ f ∈ B s ( · ) p ( · ) , q ( · ) ( W ) ,

<!-- formula-not-decoded -->

where the implicit positive constant C is independent of ⃗ f .

- (ii) If { bQ } Q ∈Q + is a family of B s ( · ) p ( · ) , q ( · ) ( W ) -synthesis molecules, then, for any ⃗ t ∈ b s ( · ) p ( · ) , q ( · ) ( W ) ,

<!-- formula-not-decoded -->

where the implicit positive constant is independent of ⃗ t.

Remark 5.9. When p , q , s all are constant exponent, the definition of analysis molecules and synthesis molecule reduces to the [18, Definition 3.10] and Theorem 5.8 goes back to [18, Theorem 3.17] with the case when τ = 0.

Now, we give the proof of Theorem 5.8.

Proof of Theorem 5.8. By (5.5), we obtain, for any cube Q ∈ Q + ,

<!-- formula-not-decoded -->

Lett bR , Q : = ⟨ ψ R , mQ ⟩ and B : = {⟨ ψ R , mQ ⟩} Q , R ∈Q + . Then, from Lemma 5.6(i), it follows that B is a bounded almost diagonal operator. Using this, (5.6), and Theorem 3.35, we conclude that

<!-- formula-not-decoded -->

This finishes the proof of Theorem 5.8(i).

Now, we prove (ii). By Lemma 5.6(i) with respectively m (1) P , b (1) Q and m (2) Q replaced by ϕ ∈ S , φ Q , and ψ Q , we obtain

<!-- formula-not-decoded -->

converges absolutely and hence ⃗ f is well defined. Let ϕ : = φ P , bP , R : = ⟨ ψ Q , φ P ⟩⟨ bR , φ Q ⟩ , and B : = { bP , R } P , R ∈Q + . By Lemma 5.6(i) and Remark 4.6(ii), we obtain B is b s ( · ) p ( · ) , q ( · ) ( W )-almost diagonal, and

<!-- formula-not-decoded -->

Thus, from this, Theorems 3.35, and 4.3, we deduce that

<!-- formula-not-decoded -->

which completes the proof of Theorem 5.8(ii).

## 5.2 Wavelet Characterizations and Atomic Decompositions

We now begin with the concept of the Daubechies wavelet (see, for example, [46]).

Definition 5.10. Let N ∈ N and Λ : = { 0 , 1 } n \ { 0 } . Then { θ ( 0 ) , θ ( λ ) : λ ∈ Λ } are called Daubechies wavelet of class C N if θ ( 0 ) ∈ C N and each θ ( λ ) ∈ C N are real-valued with bounded support and

<!-- formula-not-decoded -->

is an orthonormal basis of L 2 .

The following wavelet basis were constructed by Daubechies (see, for instance, [46] and [73, Chapter 3.9]).

Lemma 5.11. Let Λ : = { 0 , 1 } n \ { 0 } . For any N ∈ N , there exist functions { θ ( 0 ) , θ ( λ ) : λ ∈ Λ } ⊂ C N satisfy the following conditions:

- (i) there exists a positive constant γ ∈ (1 , ∞ ) such that θ ( 0 ) , θ ( λ ) with λ ∈ Λ support on γ Q ( 0 , 1) ;
- (ii) for any α ∈ Z n + with | α | ≤ N and λ ∈ Λ , R R n x α θ ( λ ) ( x ) dx = 0 ;
- (iii) The systems of { θ ( 0 ) , θ ( λ ) : λ ∈ Λ } , that is, { θ ( 0 ) P : P ∈ Q 0 } ∪ { θ ( λ ) Q : Q ∈ Q + and λ ∈ Λ } is an orthonormal basis of L 2 .

The following theorem is he Daubechies wavelets characterizations of the matrix-weighted variable Besov spaces (see [90, Theorem 5.12] for the wavelets characterizations of the scalar weighted variable Besov spaces).

Theorem 5.12. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH, s ( · ) ∈ LH, and W ∈ A p ( · ) , ∞ and let { θ ( 0 ) , θ ( λ ) : λ ∈ Λ } be a class of C N Daubechies wavelets and { φ j } j ∈ Z + and { ψ j } j ∈ Z + the same as in (3.36). Then, for any ⃗ f ∈ B s ( · ) p ( · ) , q ( · ) ( W ) ,

<!-- formula-not-decoded -->

in ( S ′ ) m , where ⟨ ⃗ f , θ ( 0 ) P ⟩ and ⟨ ⃗ f , θ ( λ ) Q ⟩ are the same as in (5.5), and

<!-- formula-not-decoded -->

where the positive equivalent constants are independent of ⃗ f .

Remark 5.13. When p ( · ) , q ( · ) , s ( · ) all are constant exponents, Theorem 5.12 reduces to [18, Theorem 4.10] with the case τ = 0. This result about wavelet characterization is new even when w is a scalar variable weight.

The following is the relationship between molecules and wavelets.

Lemma 5.14. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH, s ( · ) ∈ LH, and W ∈ A p ( · ) , ∞ and let N ∈ N and { θ ( 0 ) , θ ( λ ) : λ ∈ Λ } be a class of C N Daubechies wavelets. If

<!-- formula-not-decoded -->

then { θ ( 0 ) P : P ∈ Q 0 }∪{ θ ( λ ) Q : Q ∈ Q + and λ ∈ Λ } is both a family of B s ( · ) p ( · ) , q ( · ) ( W ) -analysis molecules and a family of B s ( · ) p ( · ) , q ( · ) ( W ) -synthesis molecules with multiplying harmless constants.

Proof. By Definition 5.1, we only need to show { θ ( 0 ) P : P ∈ Q 0 } ∪ { θ ( λ ) Q : Q ∈ Q + and λ ∈ Λ } is a family of ( K , L , M , N )-molecules (with multiplying harmless constants) satisfying (5.2) and (5.1). Since θ ( 0 ) and θ ( λ ) with λ ∈ Λ have bounded support, it follows that { θ ( 0 ) P : P ∈ Q 0 } ∪ { θ ( λ ) Q : Q ∈ Q + and λ ∈ Λ } satisfies (i) and (iii) of Definition 5.1 immediately. Then, by Lemma 5.11(ii), we find that, for any L ∈ Z + with L &lt; N , θ ( λ ) satisfies 5.1(ii) for any λ ∈ Λ . Moreover, using Lemma 5.11, we obtain θ ( 0 ) P , θ ( λ ) Q ∈ C N and hence, for any N ∈ R with N &lt; N , { θ ( 0 ) P : P ∈ Q 0 } ∪ { θ ( λ ) Q : Q ∈ Q + and λ ∈ Λ } satisfies 5.1(iv). Thus, for any L , N with max { L , N } &lt; N , { θ ( 0 ) P : P ∈ Q 0 } ∪ { θ ( λ ) Q : Q ∈ Q + and λ ∈ Λ } is a family of ( K , L , M , N )-molecules with multiplying harmless constant. Now, combining this with both (5.1) and (5.2), we conclude that, for any N ∈ Z + with N &gt; max { s + , J ( W ) -n -s -} , { θ ( 0 ) P : P ∈ Q 0 } ∪ { θ ( λ ) Q : Q ∈ Q + and λ ∈ Λ } is both a family of B s ( · ) p ( · ) , q ( · ) ( W )-analysis molecules and a family of B s ( · ) p ( · ) , q ( · ) ( W )-synthesis molecules. This finishes the proof of Lemma 5.14. □

Now, we give the proof of Theorem 5.12.

Proof of Theorem 5.12. By (3.36), to show (5.7) converges in ( S ′ ) m , it is su ffi cient to prove that, for any ϕ ∈ S ,

<!-- formula-not-decoded -->

Let ⃗ tR : = ⟨ ⃗ f , φ R ⟩ for any R ∈ Q + and ⃗ t : = { ⃗ tR } R ∈Q + . Then, using Theorem 3.35 and the assumption ⃗ f ∈ B s ( · ) p ( · ) , q ( · ) ( W ), we find that ⃗ t ∈ b s ( · ) p ( · ) , q ( · ) ( W ). Since ϕ ∈ S , it follows that ϕ is both analysis and synthesis molecule with multiplying harmless constant. Thus, by this, Lemma 5.6, and Remark 4.6(ii), we conclude that S converges absolutely. Applying this with Lemma 5.11(iii), we find that

<!-- formula-not-decoded -->

which proves that (5.7) holds in the sense of ( S ′ ) m .

Next, it follows from Lemma 5.14 that { θ ( 0 ) P : P ∈ Q 0 } ∪ { θ ( λ ) Q : Q ∈ Q + and λ ∈ Λ } is a family of both analysis and synthesis molecules with multiplying harmless constants. Hence, using this and using Theorem 5.8, we conclude that, for any λ ∈ Λ ,

<!-- formula-not-decoded -->

which further implies that GLYPH&lt;13&gt; GLYPH&lt;13&gt; GLYPH&lt;13&gt; GLYPH&lt;13&gt; ⃗ f GLYPH&lt;13&gt; GLYPH&lt;13&gt; GLYPH&lt;13&gt; GLYPH&lt;13&gt; B s ( · ) p ( · ) , q ( · ) ( W ) w ≲ GLYPH&lt;13&gt; GLYPH&lt;13&gt; GLYPH&lt;13&gt; GLYPH&lt;13&gt; ⃗ f GLYPH&lt;13&gt; GLYPH&lt;13&gt; GLYPH&lt;13&gt; GLYPH&lt;13&gt; B s ( · ) p ( · ) , q ( · ) ( W ) .

Now, let ⃗ f (0) : = P P ∈Q 0 ⟨ ⃗ f , θ ( 0 ) P ⟩ θ ( 0 ) P and, for any λ ∈ Λ , ⃗ f ( λ ) : = P Q ∈Q + ⟨ ⃗ f , θ ( λ ) Q ⟩ θ ( λ ) Q . Then, by (5.7), we obtain ⃗ f = ⃗ f (0) + P λ ∈ Λ ⃗ f ( λ ) , which, together with Theorem 5.8(ii), further implies that

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

and, for any λ ∈ Λ ,

From these, we infer that

<!-- formula-not-decoded -->

which completes the proof of Theorem 5.12.

□

Now, using the wavelets characterizations, we establish the atomic decompositions of matrixweighted variable Besov spaces. We first recall the concept of ( r , L , N )-atoms.

Definition 5.15. Let r , L , N ∈ (0 , ∞ ). Afunction aQ is called an ( r , L , N ) -atom on a cube Q , if, for any γ ∈ Z n + and any x ∈ R n ,

- (i) supp aQ ⊂ rQ ,
- (ii) R R n x γ aQ ( x ) dx = 0 if l ( Q ) &lt; 1 and | γ | ≤ L ,
- (iii) | D γ aQ ( x ) | ≤ | Q | -1 2 - | γ | n if | γ | ≤ N .

The following theorem is the atomic decompositions of matrix-weighted variable Besov spaces (see [90, Corollary 4.8] for the atomic decompositions of scalar weighted variable Besov spaces).

Theorem 5.16. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH, s ( · ) ∈ LH, and W ∈ A p ( · ) , ∞ and let L , N ∈ R satisfy L &gt; J ( W ) -n -s -and N &gt; s + . Then, there exists r ∈ (0 , ∞ ) , depending only on L and N, such that the following statements hold:

- (i) For any ⃗ f ∈ B s ( · ) p ( · ) , q ( · ) ( W ) , there exist sequence ⃗ t : = { ⃗ tR } R ∈Q + ∈ b s ( · ) p ( · ) , q ( · ) ( W ) and ( r , L , N ) -atoms { aQ } Q ∈Q + , each on the cube indicated by its subscript, such that ⃗ f = P Q ∈Q + ⃗ tQaQ in ( S ′ ) m and, moreover, ∥ ⃗ t ∥ b s ( · ) p ( · ) , q ( · ) ( W ) ≲ ∥ ⃗ f ∥ B s ( · ) p ( · ) , q ( · ) ( W ) , where the implicit positive constant is independent of ⃗ f .
- (ii) If { aQ } Q ∈Q + is a family of ( r , L , N ) -atoms, then, for any ⃗ t : = { ⃗ tQ } Q ∈Q + ∈ b s ( · ) p ( · ) , q ( · ) ( W ) , ⃗ f : = P Q ∈Q + ⃗ tQaQ converges in ( S ′ ) m and, moreover, ∥ ⃗ f ∥ B s ( · ) p ( · ) , q ( · ) ( W ) ≲ ∥ ⃗ t ∥ b s ( · ) p ( · ) , q ( · ) ( W ) , where the implicit positive constant is independent of ⃗ t and { aQ } Q ∈Q + ,

Remark 5.17. When p ( · ) , q ( · ) , s ( · ) all are constant exponents, Theorem 5.16 comes back to [18, Theorem 4.13]. When comes back to the scalar-valued case, Theorem 5.16 is equal with [50, Theorem 3], (see also, for instance, [97]).

Now, we give the proof of Theorem 5.16.

Proof of Theorem 5.16. Notice that an ( r , L , N )-atom must be a ( K , L , M , N )-molecule for any K and M . Thus, by this, (5.2), and assumptions L &gt; J ( W ) -n -s -and N &gt; s + , we obtain { aQ } Q ∈Q + is a family of synthesis molecules, which combined with Theorem 5.8(ii), further implies that Theorem 5.16(ii) holds.

Next, we give the proof of Theorem 5.16(i). Let N ∈ Z + with N &gt; max { L , N } . Then, by Theorem 5.12, there exists a class of C N Daubechies wavelets { θ ( 0 ) P : P ∈ Q 0 } ∪ { θ ( λ ) Q : Q ∈ Q + and λ ∈ Λ } such that

<!-- formula-not-decoded -->

in ( S ′ ) m and

<!-- formula-not-decoded -->

From these, it follows that, to prove (i) of Theorem 5.16, it is su ffi cient to rearrange a new suitable order of { θ ( 0 ) P : P ∈ Q 0 } ∪ { θ ( λ ) Q : Q ∈ Q + and λ ∈ Λ } such that, for any Q ∈ Q + and λ ∈ { 0 , 1 } n , there exists a unique aP with P ∈ Q + satisfying aP = θ ( λ ) Q .

Now, if Q ∈ Q 0, then let aQ : = c 1 θ ( 0 ) Q and ⃗ tQ : = c -1 1 ⟨ ⃗ f , θ ( 0 ) Q ⟩ , where c 1 is a harmless constant such that θ ( 0 ) Q is a ( r , L , N )-atom on Q . For any Q ∈ cq + , let Qi , i ∈ { 0 , 1 , . . . , 2 n } be an enumeration of the dyadic child-cubes of Q . Then, there exist constants c 2 and r 2 such that c 2 θ ( λ ) Q is a ( r 2 , L , N )-atom on Qi for any λ ∈ Λ . Rearranging θ ( λ ) with λ ∈ Λ by θ ( i ) with i ∈ { 1 , . . . , 2 n -1 } , then let

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

By this, we obtain immediately ⃗ f = P Q ∈Q + ⃗ tQaQ . Moreover, since the set of ⃗ t : = { ⃗ tQ } Q ∈Q + is the same as

<!-- formula-not-decoded -->

with shifted by one level at most, it follows from the definition of norms that the shift changes the norm at most by a positive constant C , which is independent of ⃗ t . This finishes the proof of Theorem 5.16. □

## 6 Boundedness of Classical Operators

In this section, we focus on the boundedness of some classical operators on matrix-weighted variable Besov spaces. In Subsection 6.1, we show the boundedness of the trace operators and then, in Subsection 6.2, we show the boundedness of the Calder´ on-Zygmund operators.

## 6.1 Trace Operators

In this subsection, we establish trace operators theorem of matrix-weighted Besov spaces. Since the trace operators maps the factor from R n to R n -1 , to avoid the confusion, it follows that we keep the notions R n and R n -1 in this subsection and, moreover, we assume that all variable exponents p ( · ) , q ( · ), and s ( · ) are independent of the n -th parameter xn .

We first recall some basic notions. For any x ∈ R n , let x : = ( x ′ , xn ), where x ′ ∈ R n -1 and xn ∈ R . We also denote λ ∈ { 0 , 1 } n by λ = ( λ ′ , λ n ) with λ ′ ∈ { 0 , 1 } n -1 and λ n ∈ { 0 , 1 } . Let 0 n be the origin of R n . To recall the concept of trace operators, we first recall some properties of Daubechies wavelets (see, for instance, [46]).

Lemma 6.1. For any N ∈ N , there exist two real-valued C N ( R ) functions φ and ψ with bounded support such that, for any n ∈ N ,

<!-- formula-not-decoded -->

and

form an orthonormal basis of L 2 ( R n ) , where, for any λ : = ( λ 1 , . . . , λ n ) ∈ { 0 , 1 } n and any x : = ( x 1 , . . . , xn ) ∈ R n , θ ( λ ) ( x ) : = Q n i = 1 ϕ ( λ i ) ( xi ) with ϕ (0) : = φ and ϕ (1) : = ψ .

Remark 6.2. In Lemma 6.1, from [18, Remark 5.2], there exists k 0 ∈ Z such that φ ( -k 0) , 0.

For any I ∈ Q + ( R n -1 ) and k ∈ Z , let

<!-- formula-not-decoded -->

By the construction of Q ( I , k ), it is easy to find that, for any cube Q ∈ Q + ( R n ), there exist a unique I ∈ Q + ( R n -1 ) and a unique k ∈ Z such that Q = Q ( I , k ) and we denote I by I ( Q ). Let W ∈ A p ( · ) , ∞ ( R n ), V ∈ A p ( · ) , ∞ ( R n -1 ), and N large enough such that (5.8) holds for both B s ( · ) p ( · ) , q ( · ) ( W )

and B s ( · ) -1 p ( · ) p ( · ) , q ( · ) ( V ). Thus, by Theorem 5.12 and Lemma 6.1, we find that there exists a family of functions { θ ( λ ) } λ ∈{ 0 , 1 } n ⊂ C N ( R n ) (respectively, { θ ( λ ′ ) } λ ∈{ 0 , 1 } n -1 ⊂ C N ( R n -1 )), being the Daubechies wavelet of B s ( · ) p ( · ) q ( · ) ( W ) (respectively, B s ( · ) -1 p ( · ) p ( · ) q ( · ) ( V )).

, , We now introduce the trace operators via the Daubechies wavelet. For any λ : = ( λ ′ , λ n ) ∈ Λ n and any cube Q : = Q ( I , k ) ∈ Q + ( R n ) with I ∈ Q + ( R n -1 ) and k ∈ Z and for any x ′ ∈ R n -1 , let

<!-- formula-not-decoded -->

From Theorem 5.12, it follows that, for any ⃗ f ∈ B s ( · ) p ( · ) , q ( · ) ( W ),

<!-- formula-not-decoded -->

in [ S ′ ( R n )] m . Hence, for any ⃗ f ∈ B s ( · ) p ( · ) , q ( · ) ( W ), we define

<!-- formula-not-decoded -->

Next, we introduce the extension operators. For any functions g on R n -1 and h on R and for any x : = ( x ′ , xn ) ∈ R n , let g ⊗ h ( x ) : = g ( x ′ ) h ( xn ). Then, for any λ ′ ∈ Λ n -1, I ∈ Q + ( R n -1 ), and any x : = ( x ′ , xn ) ∈ R n , let

<!-- formula-not-decoded -->

where φ and k 0 are the same as in Lemma 6.1 and Remark 6.2. For λ = 0 , we have the analogous definitions. Now, similarly to the case of trace operator, by Lemma 5.12, we find that, for any ⃗ f ∈ B s ( · ) -1 p ( · ) p ( · ) , q ( · ) ( V , R n -1 ),

<!-- formula-not-decoded -->

in [ S ′ ( R n -1 )] m and hence we define the extension operator for any ⃗ f ∈ B s ( · ) -1 p ( · ) p ( · ) , q ( · ) ( V , R n -1 ) as follows

<!-- formula-not-decoded -->

The following theorem is the trace theorem (see [97, Theorem 6.1] for the trace theorem on scalar variable Besov-type spaces).

Theorem 6.3. Let p ( · ) , q ( · ) ∈ P 0( R n ) with p ( · ) , q ( · ) ∈ LH ( R n ) and s ( · ) ∈ LH ( R n ) and let W ∈ A p ( · ) , ∞ ( R n ) and V ∈ A p ( · ) , ∞ ( R n -1 ) with ( s -1 p ) -&gt; d upper p ( · ) , ∞ ( V ) + ( n -1)( 1 p --1) ( + ) . Then the trace operator

<!-- formula-not-decoded -->

defined as in (6.2) is a bounded linear operator if and only if, for any I ∈ Q + ( R n -1 ) and ⃗ z ∈ C m ,

<!-- formula-not-decoded -->

where the implicit positive constant is independent of I and ⃗ z.

Remark 6.4. When p ( · ) , q ( · ) , s ( · ) are all constant exponents, Theorem 6.3 reduces to [20, Theorem 6.3] with τ = 0. Moreover, even comes back to the scalar-valued case, Theorem 6.3 is new and it coincides with [97, Theorem 6.1] with τ = 0.

The following result shows the relationship between reducing operators of V and W . We omit the details here.

Lemma 6.5. Let p ( · ) , q ( · ) ∈ P 0( R n ) with p ( · ) , q ( · ) ∈ LH ( R n ) and s ( · ) ∈ LH ( R n ) and let W ∈ A p ( · ) , ∞ ( R n ) and V ∈ A p ( · ) , ∞ ( R n -1 ) . If (6.5) holds, then, for any I ∈ Q + ( R n -1 ) , k ∈ Z , and ⃗ z ∈ C m , | AI , V ⃗ z | ≲ (1 + | k | ) ∆ W | AQ ( I , k ) , W ⃗ z | , where ∆ W is the same as in Lemma 2.14 and the implicit positive constant is independent of I, k, and ⃗ z.

Now, we give the proof of Theorem 6.3.

Proof of Theorem 6.3. First, We prove the necessity. Suppose that the trace operator Tr is bounded. Then, for any fixed cube I 0 ∈ Q + ( R n -1 ) and any ⃗ z ∈ C m , let ⃗ t : = { ⃗ t I } I ∈Q + ( R n -1 ) , where, for any cube I ∈ Q + ( R n -1 ),

<!-- formula-not-decoded -->

and xI 0 is the center of I 0. Now, denoting ⃗ g : = ⃗ t I 0 θ ( λ ′ ) I 0 for some λ ′ ∈ Λ n -1, then, from Theorems 5.12 and 3.34 and from [2, Example 3.4], Lemma 3.23, and the assumption s ( · ) , 1 p ( · ) ∈ LH ( R n ), we infer that

<!-- formula-not-decoded -->

Now, assume ⃗ y : = { ⃗ uQ } Q ∈Q + ( R n ) with

<!-- formula-not-decoded -->

and, for any x : = ( x ′ , xn ) ∈ R n ,

<!-- formula-not-decoded -->

Then, from this and (6.2), it follows that, for any x ′ ∈ R n -1 ,

<!-- formula-not-decoded -->

Notice that, by Lemma 6.1, θ ( λ ′ ) ⊗ φ is a Daubechies wavelet of B s ( · ) p ( · ) , q ( · ) ( W , R n ). From Lemmas 3.23, 3.28, and the assumption that s ( · ) is independent of the n -th parameter, we deduce that, for any x ∈ Q ( I 0 , k 0), 2 s ( x ) ∼ 2 s ( xI 0 ) and ∥ 1 Q ( I 0 , k 0) ∥ L p ( · ) ∼ [ l ( I 0)] n p ( x I 0 ) . By this, Theorems 5.12 and 3.34, and [2, Example 3.4], we obtain

<!-- formula-not-decoded -->

Combining this with (6.6), (6.7), and the assumption that Tr is bounded, we conclude that

<!-- formula-not-decoded -->

From this and Lemma 6.5, we infer that

<!-- formula-not-decoded -->

which, combined with Definition 2.8, further implies (6.5). This finishes the proof of the necessity. Next, we show the su ffi ciency. We first prove that the trace operator Tr defined as in (6.2) is well defined. For any Λ ∈ { 0 , 1 } n , let ⃗ u ( λ ) : = { ⃗ u ( λ ) Q } Q ∈Q + ( R n ), with ⃗ u ( λ ) Q : = ⟨ ⃗ f , θ ( λ ) Q ⟩ for any Q ∈ Q + ( R n ); and let ⃗ t ( λ ) : = { ⃗ t ( λ ) Q } Q ∈Q + ( R n ) and ⃗ t ( λ ) Q : = [ l ( Q )] -1 2 ⃗ u ( λ ) Q for any Q ∈ Q + ( R n ) with the analogous definition when λ = 0 n . By the fact that θ ( λ ) has bounded support, there exists N ∈ N such that, for any λ ∈ { 0 , 1 } n , supp θ ( λ ) ⊂ B ( 0 n , N ). Then, using this, for any I ∈ Q + ( R n -1 ) and k ∈ Z with | k | &gt; N , we obtain, for any λ ∈ { 0 , 1 } n and x ′ ∈ R n -1 ,

<!-- formula-not-decoded -->

which further implies that θ ( λ ) Q ( I , k ) = 0 for any k ∈ Z with | k | &gt; N . From this, (6.2), and (6.1), it follows that

<!-- formula-not-decoded -->

where, for any λ ∈ Λ ( R n ),

<!-- formula-not-decoded -->

Notice that, by Theorem 5.14 and the assumption that N satisfies (5.8) for B s ( · ) -1 p ( · ) p ( · ) , q ( · ) ( V , R n -1 ), { θ ( 0 n -1) I : I ∈ Q 0( R n -1 ) } and { θ ( λ ′ ) I : I ∈ Q + ( R n -1 ) } both are families of synthesis molecules. Thus, together this with Theorem 5.8, to show Tr f convergences in [ S ′ ( R n -1 )] m , we only need to show, for any k ∈ {-N , . . . , N } and λ ∈ { 0 , 1 } n , ⃗ t ( λ ) k : = { ⃗ t ( λ ) Q ( I , k ) } I ∈Q + ( R n -1 ) ∈ b s ( · ) -1 p ( · ) p ( · ) , q ( · ) ( V , R n -1 ), where t ( 0 n ) Q ( I , k ) : = 0 for any I &lt; Q 0( R n -1 ); or more precisely,

<!-- formula-not-decoded -->

Since Theorem 5.12 and the assumption { θ ( 0 n ) Q : Q ∈ Q 0( R n ) } ∪ { θ ( λ ) Q : Q ∈ Q + ( R n ) } is a family of wavelets of B s ( · ) p ( · ) , q ( · ) ( W , R n ), it follows that

<!-- formula-not-decoded -->

and hence, together this with (6.9), we find that, to prove (6.9), we only need to show, for any λ ∈ { 0 , 1 } n and k ∈ {-N , . . . , N } ,

<!-- formula-not-decoded -->

Now, fix λ ∈ { 0 , 1 } n and k ∈ {-N , . . . , N } . Similarly to the claim (3.15), to prove (6.10), we only need to show that, for any j ∈ Z + ,

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

where and

<!-- formula-not-decoded -->

Notice that, by Lemma 3.16, (6.11) is equivalent with the modular ρ L p ( · ) ( R n -1 ) of the left-hand side of (6.11) less than a constant, that is,

<!-- formula-not-decoded -->

Using Lemma 6.5 and the fact that (1 + | k | ) ∆ W ≤ (1 + | N | ) ∆ W for any | k | ≤ N , we find that, for any I ∈ Q + ( R n -1 ), | AI , V ⃗ t ( λ ) Q ( I , k ) | ≲ | AQ ( I , k ) , W ⃗ t ( λ ) Q ( I , k ) | . Hence, by this, the disjointness of Q j ( R n -1 ), the assumption that p ( · ) , q ( · ) , s ( · ) are independent of the n -th parameter, and the fact that ⃗ u ( λ ) Q = l ( Q ) 1 2 ⃗ t ( λ ) Q for any Q ∈ Q j ( R n ), we have

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Noticing that, by the definition of δ j and Lemma 3.17, we have

<!-- formula-not-decoded -->

which, combined with Lemma 3.17, further implies that

<!-- formula-not-decoded -->

This, together with (6.13) and Lemma 3.16, further implies that (6.12) holds and hence (6.10) holds. Thus, we have Tr f convergences in [ S ′ ( R n -1 )] m and hence Tr is well defined. Moreover, using Lemma 3.42, (6.8), Theorem 5.8, and (6.9), we conclude that

<!-- formula-not-decoded -->

which further implies that Tr is continuous. This finishes the proof of Theorem 6.3.

□

Next, we establish the extension theorem for matrix-weighted variable Besov spaces.

Theorem 6.6. Let p ( · ) , q ( · ) ∈ P 0( R n ) with p ( · ) , q ( · ) ∈ LH ( R n ) , s ( · ) ∈ LH ( R n ) , W ∈ A p ( · ) , ∞ ( R n ) , and V ∈ A p ( · ) , ∞ ( R n -1 ) . If there exists a positive constant C such that, for any I ∈ Q + ( R n -1 ) and ⃗ z ∈ C m ,

<!-- formula-not-decoded -->

Then the extension operator Ext can be extended to a bounded linear operator

<!-- formula-not-decoded -->

Moreover, if s ( · ) satisfies ( s -1 q ) -&gt; d upper p ( · ) , ∞ ( V ) + ( n -1)( 1 p --1) ( + ) and (6.5) holds, then Tr ◦ Ext is the identity on B s ( · ) -1 p ( · ) p ( · ) , q ( · ) ( V , R n -1 ) .

Remark 6.7. We note that Theorem 6.6 coincides with [20, Theorem 6.5] when p ( · ) , q ( · ) , s ( · ) are constant exponents.

Before giving the proof of Theorem 6.6, we give one basic tool, which is the converse estimate of Lemma 6.5.

Lemma 6.8. Let p ( · ) , q ( · ) ∈ P 0( R n ) with p ( · ) , q ( · ) ∈ LH ( R n ) , s ( · ) ∈ LH ( R n ) . Let W ∈ A p ( · ) , ∞ ( R n ) and V ∈ A p ( · ) , ∞ ( R n -1 ) . If (6.14) holds, then there exists a positive constant C such that, for any I ∈ Q + ( R n -1 ) , k ∈ Z , and ⃗ z ∈ C m ,

<!-- formula-not-decoded -->

Proof. If (6.14) holds, then, by (2.5) and Lemma 2.14, we obtain, for any I ∈ Q + ( R n -1 ),

<!-- formula-not-decoded -->

This finishes the proof of Lemma 6.8.

Now, we give the proof of Theorem 6.6.

Proof of Theorem 6.6. We first show Ext ⃗ f is well defined and Ext is a bounded linear operator. For any λ ′ ∈ { 0 , 1 } n -1 , let ⃗ t ( λ ′ ) : = { ⃗ t ( λ ′ ) Q } Q ∈Q + ( R n ), where, for any Q ∈ Q + ( R n ), let ⃗ u ( λ ′ ) I : = ⟨ ⃗ f , θ ( λ ′ ) I ⟩ , with ⃗ u ( 0 n -1) I : = 0 if I &lt; Q 0, and

<!-- formula-not-decoded -->

with k 0 the same as in Remark 6.2. Thus, by this and (6.3), we obtain, for any λ ′ ∈ { 0 , 1 } n -1 , any I ∈ Q + ( R n -1 ), and x ∈ R n ,

<!-- formula-not-decoded -->

where k 0 is the same as in Remark 6.2. Hence, using this and (6.4), we find that

<!-- formula-not-decoded -->

Since Theorem 5.12 and the fact that { [ θ λ ′ ⊗ φ ] Q ( I , k 0) } I ∈Q + ( R n -1 ) is a subset of wavelets { θ ( 0 ) Q : Q ∈ Q 0 } ∪ { θ ( λ ) Q : Q ∈ Q + , λ ∈ Λ n } , it follows that, to show Ext ⃗ f convergences in [ S ′ ( R n -1 )] m , we only need to show that, for any λ ′ ∈ { 0 , 1 } n -1 ,

<!-- formula-not-decoded -->

Similarly to the claim (3.15), to show (6.16), we only need to prove that, for any j ∈ Z + ,

<!-- formula-not-decoded -->

□

where and satisfies

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Combining this with Lemma 3.16, further implies that (6.17) is equivalent with the modular ρ L p ( · ) of the left-hand side on (6.17) is less than a constant, that is,

<!-- formula-not-decoded -->

By the definition of ρ L p ( · ) q ( · ) ( R n ) , the disjointness of Q j ( R n -1 ), and the assumption that p ( · ) , q ( · ) , s ( · ) are independent of the n -th index, we find that

<!-- formula-not-decoded -->

Using the definition of δ j , we find that

<!-- formula-not-decoded -->

which, combined with Lemmas 3.17 and 3.16, further implies that

<!-- formula-not-decoded -->

Thus, from this and (6.19), we infer that (6.16) holds and hence Ext ⃗ f converges. Moreover, by (6.16) and Theorem 5.12, we find that

<!-- formula-not-decoded -->

which further implies that Ext is bounded.

Finally, by the definition of Tr and Ext , we conclude that, for any ⃗ f ∈ B s ( · ) -1 p ( · ) p ( · ) , q ( · ) ( V , R n -1 ),

<!-- formula-not-decoded -->

in [ S ′ ( R n -1 )] m . This finishes the proof of Theorem 6.6.

□

Remark 6.9. When p , q , s are all constant exponents, Theorems 6.3 and 6.6 comes back to [18, Theorems 5.6 and 5.10] with τ = 0. For the unweighted variable Besov space, Theorems 6.3 and 6.6 are equal with [79, Theorem 1] and these results are new even when W is a scalar variable weight.

## 6.2 Calder´ on-Zygmund Operators

In this subsection, we establish the boundedness of Calder´ on-Zygmund operators on B s ( · ) p ( · ) , q ( · ) ( W ) under some essential assumptions (see, for instance, [87, 18]).

Now, we begin to discuss about Calder´ on-Zygmund operators. The following notions are standard. Let D : = C ∞ c equipped with the classical topology and D ′ be the space of all continuous linear functionals on D , equipped with the weak-∗ topology. We note that, if the Calder´ onZygmund operator T ∈ L ( S , S ′ ), then, by the well-known Schwartz kernel theorem, we obtain there exists K ∈ S ′ ( R n × R n ) such that, for any φ, ϕ ∈ S ,

<!-- formula-not-decoded -->

where K is called the Schwartz kernel of T .

The following definition is about some essential assumptions of K .

Definition 6.10. Let T ∈ L ( S , S ′ ) and K ∈ S ′ ( R n × R n ) the Schwartz kernel of T .

- (i) The Calder´ on-Zygmund operator T is said to satisfy the weak boundedness property , denoted by T ∈ WBP, if, for any bounded subset B of D , there exists a positive constant C , depending on B , such that, for any φ, η ∈ B , h ∈ R n , and r ∈ (0 , ∞ ),

<!-- formula-not-decoded -->

- (ii) For any l ∈ (0 , ∞ ), we say T has a Calder´ on-Zygmund kernel of order l , denoted by T ∈ CZO( l ), if the restriction of K on the set { ( x , y ) ∈ R n × R n : x , y } is a continuous function with continuous partial derivatives in the x variable up to order ⌊ ⌊ l ⌋ ⌋ satisfying that there exists a positive constant C such that, for any γ ∈ Z n + with | γ | ≤ ⌊ ⌊ l ⌋ ⌋ dan for any x , y ∈ R n with x , y , | ∂ γ x K ( x , y ) | ≤ C | x -y | -n -| γ | and, for any γ ∈ Z n + with | γ | = ⌊ ⌊ l ⌋ ⌋ and for any x , y , h ∈ R n with | h | &lt; 1 2 | x -y | ,

<!-- formula-not-decoded -->

For any l ∈ ( -∞ , 0], we interpret T ∈ CZO( l ) as a void condition.

Remark 6.11. By the definition of CZO( l ), it is obvious that, for any l 1 , l 2 ∈ R with l 1 &lt; l 2, CZO( l 1) ⊂ CZO( l 2).

To discuss the following important cancellation conditions, we need to define the action of Calder´ on-Zygmund operators on polynomials, which does not lie on S . To extend the definition of Calder´ on-Zygmund operators, we recall the following result, which is a special case of [87, Lemma 2.2.12].

Lemma 6.12. Let l ∈ (0 , ∞ ) and T ∈ CZO( l ) , and let { ϕ i } i ∈ N ⊂ D be a sequence of function such that sup j ∈ N ∥ ϕ ∥ L ∞ &lt; ∞ and, for any compact set K of R n , there exists a jK ∈ N such that, for any j ≥ j k and any x ∈ K, ϕ j ( x ) = 1 . Then the limit

<!-- formula-not-decoded -->

exists for any polynomials f ( y ) = y γ with | γ | ≤ ⌊ ⌊ l ⌋ ⌋ and any g ∈ D⌊⌊ l ⌋ ⌋ , where

<!-- formula-not-decoded -->

and (6.20) is independent of the choice of { ϕ j } j ∈ N .

Now, we give the following definition.

Definition 6.13. Let l ∈ (0 , ∞ ). For any T ∈ CZO( l ) and f ( y ) = y γ with y ∈ R n and | γ | ≤ ⌊ ⌊ l ⌋ ⌋ , we define T ( y γ ) = T f : D⌊⌊ l ⌋ ⌋ → C given by (6.20).

Definition 6.14. Let E , F ∈ R , T ∈ L ( S , S ′ ), and K ∈ S ′ ( R n × R n ) be its Schwartz kernel. We say that T ∈ CZK 0 ( E ; F ) if the restriction of K to { ( x , y ) ∈ R n × R n : x , y } is a continuous function such that, for any α ∈ Z n + with | α | ≤ ⌊ ⌊ E ⌋ ⌋ , ∂ α x K exists as a continuous function and there exits a positive constant C such that, for any x , y ∈ R n with x , y | ∂ α x K ( x , y ) | ≤ C | x -y | -n -| α | , and, for any α ∈ Z n + with | α | = ⌊ ⌊ E ⌋ ⌋ and x , y , u ∈ R n with | u | &lt; 1 2 | x -y | ,

<!-- formula-not-decoded -->

and, for any α, β ∈ Z n + with | α | ≤ ⌊ ⌊ E ⌋ ⌋ and | β | = ⌊ ⌊ F -| α |⌋ ⌋ and for any x , y , v ∈ R n with | v | &lt; 1 2 | x -y | ,

<!-- formula-not-decoded -->

We say that T ∈ CZK 1 ( E ; F ) if T ∈ CZK 0 ( E ; F ) and, in addition, for any α, β ∈ Z n + with | α | = ⌊ ⌊ E ⌋ ⌋ and | β | = ⌊ ⌊ F -E ⌋ ⌋ and for any x , y , u , v ∈ R n with | u | + | v | &lt; 1 2 | x -y | ,

<!-- formula-not-decoded -->

Wewrite just CZK( E ; F ) if the parameter values are such that (6.21) is void and hence CZK 0 ( E ; F ) and CZK 1 ( E ; F ) coincide.

Indeed, it is obvious that (6.21) is void unless F &gt; E &gt; 0.

Definition 6.15. Let σ ∈ { 0 , 1 } and E , F , G , H ∈ R . We say T ∈ lnCZO σ ( E , F , G , H ) if T ∈ L ( S , S ′ ) and its Schwartz kernel K ∈ S ′ ( R n × R n ) satisfy

- (i) T ∈ WBP;

- (ii) K ∈ CZK σ ( E ; F );
- (iii) T ( y γ ) = 0 for any γ ∈ Z n + with | γ | ≤ G ;
- (iv) T ∗ ( x θ ) = 0 for any θ ∈ Z n + with | θ | ≤ H ;
- (v) there exists a positive constant C such that, for any α ∈ Z n + with | α | ≤ ⌊ ⌊ E ⌋ ⌋ + 1 and for any x , y ∈ R n with | x -y | &gt; 1, | ∂ α x K ( x , y ) | ≤ C | x -y | -( n + F ) .

Remark 6.16. In Definition 6.15, if we remove the condition (v) of lnCZO σ ( E , F , G , H ), then lnCZO σ ( E , F , G , H ) reduce to CZO σ ( E , F , G , H ), which was defined in [18, Definition 6.17].

Now, we recall the definition of smooth atoms.

Definition 6.17. Let L , N ∈ R , A function aQ is called an ( L , M ) -atom on a cube Q if

- (i) supp aQ ⊂ 3 Q ;
- (ii) R R n x γ aQ ( x ) dx = 0 for any γ ∈ Z n + with | γ | ≤ L ;
- (iii) | D γ aQ ( x ) | ≤ | Q | -1 2 - | γ | n for any x ∈ R n and γ ∈ Z n + and | γ | ≤ N .

Noticing that the atoms defined in Definition 6.17 is the same as in [20, Definition 6,14] and the molecule defined in Definition 5.1 is tha same as in [20, Definition 5.1], we can apply the result about the Calder´ on-Zygmund operator mapping atoms into molecules. The following lemmas are just [18, Proposition 6.19] and [20, Proposition 6.24].

Lemma 6.18. Let σ ∈ { 0 , 1 } , E , F , G , H ∈ R , K , L , M , N ∈ R , and Q ∈ Q + . Suppose that T ∈ CZO σ ( E , F , G , H ) . Then T maps su ffi ciently regular atoms on Q to ( K , L , M , N ) -molecules on Q proved that

<!-- formula-not-decoded -->

Lemma 6.19. Let σ ∈ { 0 , 1 } , E , F , G , H ∈ R , K , M , N ∈ R , and Q ∈ Q 0 . Suppose that

<!-- formula-not-decoded -->

Then T maps su ffi ciently regular non-cancellative atoms on Q to ( K , -1 , M , N ) -molecules on Q proved that

<!-- formula-not-decoded -->

Combining Lemmas 6.18 and 6.19 with (5.2), we obtain the following result immediately, which is the main theorem of this section; we omit details here.

Theorem 6.20. Let p ( · ) , q ( · ) ∈ P 0 with p ( · ) , q ( · ) ∈ LH, s ( · ) ∈ LH. Let W ∈ A p ( · ) and A : = { AQ } Q ∈Q + be a sequence of reducing operators of order p ( · ) for W. Let T ∈ lnCZO σ ( E , F , G , H ) , where σ ∈ { 0 , 1 } and E , F , G , H ∈ R satisfy

<!-- formula-not-decoded -->

where C ( s , q ) is the same as in (4.2).

Remark 6.21. When p , q , s are all constant exponents, Theorem 6.20 comes back to [18, Theorem 6.18] with τ = 0. Moreover, Theorem 6.20 is new even for the unweighted variable Besov space.

## References

- [1] A. Almeida, L. Diening and P. H¨ ast¨ o, Homogeneous variable exponent Besov and TriebelLizorkin spaces, Math. Nachr. 291 (2018), 1177-1190.
- [2] A. Almeida and P. H¨ ast¨ o, Besov spaces with variable smoothness and integrability, J. Funct. Anal. 258 (2010), 1628-1655.
- [3] A. Almeida and P. H¨ ast¨ o, Interpolation in variable exponent spaces, Rev. Mat. Complut. 27 (2014), 657-676.
- [4] S. N. Bernˇ ste˘ ın, On properties of homogeneous functional classes, Dokl. Acad. Nauk SSSR (N. S.) 57 (1947), 111-114.
- [5] O. V. Besov, On some families of functional spaces. Imbedding and extension theorems, Dokl. Acad. Nauk SSSR 126 (1959), 1163-1165.
- [6] O. V. Besov, Investigation of a class of function spaces in connection with imbedding and extension theorems, Trudy Mat. Inst. Steklov. 60 (1961), 42-81.
- [7] O. V. Besov, On spaces of functions of variable smoothness defined by pseudodi ff erential operators, Tr. Mat. Inst. Steklova 227 (1999), Issled. po Teor. Di ff er. Funkts. Mnogikh Perem. i ee Prilozh. 18, 56-74 (in Russian); translation in Proc. Steklov Inst. Math. 1999, no. 4(227), 50-69.
- [8] O.V. Besov, Equivalent normings of spaces of functions of variable smoothness, Tr. Mat. Inst. Steklova: Funkt. Prostran. Priblizh. Di ff er. Uravn. 243 (2003) 87-95 (in Russian); translation in Proc. Steklov Inst. Math. 4(243) (2003) 80-88.
- [9] O.V. Besov, Interpolation, embedding, and extension of spaces of functions of variable smooth-ness, Tr. Mat. Inst. Steklova: Issled. Teor. Funkts. Di ff er. Uravn. 248 (2005) 52-63, (in Russian); translation in Proc. Steklov Inst. Math. 1(248) (2005) 47-58.
- [10] K. Bickel, S. Petermichl, and B. D. Wick, Bounds for the Hilbert transform with matrix A2 weights, J. Funct. Anal. 270 (2016), 1719-1743.
- [11] M. Bownik, Inverse volume inequalities for matrix weights, Indiana Univ. Math. J. 50(2001), 383-410.
- [12] M. Bownik, Anisotropic Triebel-Lizorkin spaces with doubling measures, J. Geom. Anal. 17 (2007), 387-424.
- [13] M. Bownik and D. Cruz-Uribe, Extrapolation and factorization of matrix weights (2022), arXiv:2210.09443.
- [14] M. Bownik and K.-P. Ho, Atomic and molecular decompositions of anisotropic TriebelLizorkin spaces, Trans. Amer. Math. Soc. 358 (2006), 1469-1510.
- [15] F. Bu, Y. Chen, D. Yang and W. Yuan, Maximal function and atomic characterizations of matrix-weighted Hardy spaces with their applications to boundedness of Calder ´ OnZygmund operators, arXiv:2501.18800.
- [16] F. Bu, T. Hyt¨ onen, D. Yang, and W. Yuan, Matrix-weighted Besov-type and TriebelLizorkin-type spaces I: Ap -dimensions of matrix weights and ψ -transform characterizations, Math. Ann. 391 (2025), 6105-6185.
- [17] F. Bu, T. Hyt¨ onen, D. Yang, and W. Yuan, Matrix-weighted Besov-type and TriebelLizorkin-type spaces II: Sharp boundedness of almost diagonal operators, J. Lond. Math. Soc. (2) 111 (2025), Paper No. e70094.
- [18] F. Bu, T. Hyt¨ onen, D. Yang and W. Yuan, Matrix-weighted Besov-type and TriebelLizorkin-type spaces III: characterizations of molecules and wavelets, trace theorems, and boundedness of pseudo-di ff erential operators and Calder´ on-Zygmund operators, Math. Z. 308 (2024), Paper No. 32, 67 pp.
- [19] F. Bu, T. Hyt¨ onen, D. Yang and W. Yuan, New characterizations and properties of matrix A ∞ weights, arXiv:2311.05974.
- [20] F. Bu, T. Hyt¨ onen, D. Yang and W. Yuan, Besov-Triebel-Lizorkin-type spaces with matrix A ∞ weights, Sci. China Math. (2025), https: // doi.org / 10.1007 / s11425-024-2385-x.

- [21] H.-Q. Bui, T. A. Bui and X. T. Duong, Weighted Besov and Triebel-Lizorkin spaces associated with operators and applications, Forum Math. Sigma 8 (2020), Paper No. e11, 95 pp.
- [22] H.-Q. Bui, X. T. Duong and L. Yan, Calder´ on reproducing formulas and new Besov spaces associated with operators, Adv. Math. 229 (2012), 2449-2502.
- [23] T. A. Bui, Besov and Triebel-Lizorkin spaces for Schr¨ odinger operators with inverse-square potentials and applications, J. Di ff erential Equations 269 (2020), 641-688.
- [24] T. A. Bui, Hermite pseudo-multipliers on new Besov and Triebel-Lizorkin spaces, J. Approx. Theory 252 (2020), 105348, 16 pp.
- [25] T. A. Bui, T. Q. Bui and X. T. Duong, Decay estimates on Besov and Triebel-Lizorkin spaces of the Stokes flows and the incompressible Navier-Stokes flows in half-spaces, J. Di ff erential Equations 340 (2022), 83-110.
- [26] T. A. Bui and X. T. Duong, Besov and Triebel-Lizorkin spaces associated to Hermite operators, J. Fourier Anal. Appl. 21 (2015), 405-448.
- [27] T. A. Bui and X. T. Duong, Laguerre operator and its associated weighted Besov and Triebel-Lizorkin spaces, Trans. Amer. Math. Soc. 369 (2017), 2109-2150.
- [28] T. A. Bui and X. T. Duong, Spectral multipliers of self-adjoint operators on Besov and Triebel-Lizorkin spaces associated to operators, Int. Math. Res. Not. IMRN 2021, 1818118224.
- [29] T. A. Bui and X. T. Duong, Higher-order Riesz transforms of Hermite operators on new Besov and Triebel-Lizorkin spaces, Constr. Approx. 53 (2021), 85-120.
- [30] C. Carath´ eodory, ¨ Uber den Variabilit¨ atsbereich der Koe ffi zienten von Potenzreihen, die gegebene Werte nicht annehmen, Math. Ann. 64 (1907), 95-115.
- [31] R. Chichoune, Z. Mokhtari and K. Saibi, Khedoudj, Weighted variable Besov space associated with operators, Rend. Circ. Mat. Palermo (2) 74 (2025), Paper No. 26, 26 pp.
- [32] M. Christ and M. Goldberg, Vector A 2 weights and a Hardy-Littlewood maximal function, Trans. Amer. Math. Soc. 353 (2001), 1995-2002.
- [33] G. Cleanthous, A. G. Georgiadis and M. Nielsen, Discrete decomposition of homogeneous mixed-norm Besov spaces, in: Functional Analysis, Harmonic Analysis, and Image Processing: A Collection of Papers in Honor of Bj¨ orn Jawerth, pp. 167-184, Contemp. Math. 693, Amer. Math. Soc., Providence, RI, 2017.
- [34] G. Cleanthous, A. G. Georgiadis and M. Nielsen, Fourier multipliers on decomposition spaces of modulation and Triebel-Lizorkin type, Mediterr. J. Math. 15 (2018), Paper No. 122, 14 pp.
- [35] G. Cleanthous, A. G. Georgiadis and M. Nielsen, Molecular decomposition and Fourier multipliers for holomorphic Besov and Triebel-Lizorkin spaces, Monatsh. Math. 188 (2019), 467-493.
- [36] D. Cruz-Uribe and J. Cummings, Weighted norm inequalities for the maximal operator on L p ( · ) over spaces of homogeneous type, Ann. Fenn. Math. 47 (2022), 457-488.
- [37] D. Cruz-Uribe, L. Diening and P. H¨ ast¨ o, The maximal operator on weighted variable Lebesgue spaces, Fract. Calc. Appl. Anal. 14 (2011), 361-374.
- [38] D. Cruz-Uribe and A. Fiorenza, Variable Lebesgue Space. Foundations and Harmonic Analysis, Appl. Number. Harmon. Anal., Birkh¨ auser / Springer, Heidelberg, 2013.
- [39] D. Cruz-Uribe, A. Fiorenza and C. J. Neugebauer, Weighted norm inequalities for the maximal operator on variable Lebesgue spaces, J. Math. Anal. Appl. 394 (2012), no. 2, 744-760.
- [40] D. Cruz-Uribe and M. Penrod, Convolution operators in matrix weighted, variable Lebesgue spaces, Anal. Appl. (Singap.) 22 (2024), 1133-1157.
- [41] D. Cruz-Uribe and M. Penrod, The reverse H¨ older inequality for A p ( · ) weights with applications to matrix weights, arXiv: 2411.12849

- [42] D. Cruz-Uribe and T. Roberts, Necessary conditions for the boundedness of fractional operators on variable Lebesgue spaces, arXiv2408.12745.
- [43] D. Cruz-Uribe and F. S ¸irin, O ff -diagonal matrix extrapolation for Muckenhoupt bases, arXiv: 2504.12407.
- [44] D. Cruz-Uribe and B. Sweeting, weighted weak-type inequalities for maximal operators and singular integrals, arXiv: 2311.00828v1.
- [45] D. Cruz-Uribe and L. D. Wang, Extrapolation and weighted norm inequalities in the variable Lebesgue spaces, Trans. Amer. Math. Soc. 369 (2017), no. 2, 1205-1235.
- [46] I. Daubechies, Orthonormal bases of compactly supported wavelets, Comm. Pure Appl. Math. 41 (1988), 909-996.
- [47] L. Diening, P. Harjulehto, P. H¨ ast¨ o and M. R˚ uˇ ziˇ cka, Lebesgue and Sobolev Spaces with Variable Exponents, Lecture Notes in Mathematics, 2017. Springer, Heidelberg, 2011.
- [48] L. Diening, P. H¨ ast¨ o and S. Roudenko, Function spaces of variable smoothness and integrability, J. Funct. Anal. 256 (2009), 1731-1768.
- [49] B. Dong and J. Xu, Local characterizations of Besov and Triebel-Lizorkin spaces with variable exponent, J. Funct. Spaces 2014, Art. ID 417341, 8 pp.
- [50] D. Drihem, Atomic decomposition of Besov spaces with variable smoothness and integrability, J. Math. Anal. Appl. 389 (2012), 15-31.
- [51] D. Drihem, Some properties of variable Besov-type spaces, Funct. Approx. Comment. Math. 52 (2015), 193-221.
- [52] D. Drihem, Some characterizations of variable Besov-type spaces, Ann. Funct. Anal. 6 (2015), 255-288.
- [53] D. Drihem and W. Hebbache, Boundedness of non regular pseudodi ff erential operators on variable Besov spaces, J. Pseudo-Di ff er. Oper. Appl. 8 (2017), 167-189.
- [54] M. Frazier and B, Jawerth, A discrete transform and decompositions of distribution spaces, J. Funct. Anal. 93 (1990), 34-170.
- [55] M. Frazier and S. Roudenko, Matrix-weighted Besov spaces and conditions of Ap type for 0 &lt; p ≤ 1, Indiana Univ. Math. J. 53 (2004), 1225-1254.
- [56] M. Frazier and S. Roudenko, Littlewood-Paley theory for matrix-weighted function spaces, Math. Ann. 380 (2021), 487-537.
- [57] A. G. Georgiadis, J. Johnsen and M. Nielsen, Wavelet transforms for homogeneous mixednorm Triebel-Lizorkin spaces, Monatsh. Math. 183 (2017), 587-624.
- [58] A. G. Georgiadis, G. Kerkyacharian, G. Kyriazis and P. Petrushev, Homogeneous Besov and Triebel-Lizorkin spaces associated to non-negative self-adjoint operators, J. Math. Anal. Appl. 449 (2017), 1382-1412.
- [59] A. G. Georgiadis, G. Kerkyacharian, G. Kyriazis and P. Petrushev, Atomic and molecular decomposition of homogeneous spaces of distributions associated to non-negative selfadjoint operators, J. Fourier Anal. Appl. 25 (2019), 3259-3309.
- [60] A. G. Georgiadis and M. Nielsen, Pseudo di ff erential operators on mixed-norm Besov and Triebel-Lizorkin spaces, Math. Nachr. 289 (2016), 2019-2036.
- [61] M. Goldberg, Matrix Ap weights via maximal functions, Pacific J. Math. 211 (2003), 201220.
- [62] L. Grafakos, Classical Fourier Analysis, third edition, Grad. Texts in Math. 249, Springer, New York, 2014.
- [63] P. Guo, S. Wang and J. Xu, Continuous characterizations of weighted Besov spaces of variable smoothness and integrability, Filomat 37 (2023), 9913-9930.
- [64] Y. He, Q. Sun and C. Zhuo, Pointwise characterizations of variable Besov and TriebelLizorkin spaces via Hajłasz gradients, Fract. Calc. Appl. Anal. 27 (2024), 944-969.
- [65] T. Hyt¨ onen and C. P´ erez, Sharp weighted bounds involving A ∞ , Anal. PDE 6 (2013), 777818.

- [66] S. V. Hruˇ sˇ cev, A description of weights satisfying the A ∞ condition of Muckenhoupt, Proc. Amer. Math. Soc. 90 (1984), 253-257.
- [67] H. Kempka and J. Vyb´ ıral, Spaces of variable smoothness and integrability: characterizations by local means and ball means of di ff erences, J. Fourier Anal. Appl. 18 (2012), 852-891.
- [68] H.G. Leopold, On Besov spaces of variable order of di ff erentiation, Arch. Math. 53(2) (1989) 178-187.
- [69] H.G. Leopold, Interpolation of Besov spaces of variable order of di ff erentiation, Arch. Math. (Basel) 53(2) (1989) 178-187.
- [70] H.G. Leopold, On function spaces of variable order of di ff erentiation, Forum Math. 3(3) (1991) 1-21.
- [71] H.G. Leopold and E. Schrohe, Trace theorems for Sobolev spaces of variable order of differentiation, Math. Nachr. 179 (1996) 223-245.
- [72] O. Kov´ aˇ cik and J. R´ akosn´ ık, On spaces L p ( x ) and W 1 , p ( x ) , Czechoslovak Math. J. 41 (116) (1991) 592-618.
- [73] Y. Meyer, Wavelets and Operators, Di ff erent Perspectives on Wavelets, 35-58, Proc. Sympos. Appl. Math., 47, Amer. Math. Soc., Providence, RI, 1993.
- [74] F. Nazarov, S. Petermichl, S. Treil and A. Volberg, Convex body domination and weighted estimates with matrix weights, Adv. Math. 318 (2017), 279-306.
- [75] F. Nazarov and S. R. Treil, The hunt for a Bellman function: applications to estimates for singular integral operators and to other classical problems of harmonic analysis, (Russian), translated from Algebra i Analiz 8 (1996), 32-162, St. Petersburg Math. J. 8 (1997), 721824.
- [76] Z. Nieraeth, A lattice approach to matrix weights, arXiv:2408.14666.
- [77] Z. Nieraeth and M. Penrod, Matrix-weighted bounds in variable Lebesgue spaces, arXiv: 2503.14398.
- [78] S. M. Nikol'ski˘ ı, Inequalities for entire analytic functions of finite order and their application to the theory of di ff erentiable functions of several variables, Trudy Mat. Inst. Steklov 38 (1951), 244-278.
- [79] T. Noi, Trace and extension operators for Besov spaces and Triebel-Lizorkin spaces with variable exponents, Rev. Mat. Complut. 29 (2016), 341-404.
- [80] W. Orlicz, ¨ Uber konjugierte Exponentenfolgen, Studia Math. 3 (1931) 200-212.
- [81] J. Peetre, Remarques sur les espaces de Besov, Le cas 0 &lt; p &lt; 1, (French) C. R. Acad. Sci. Paris S´ er. A-B 277 (1973), 947-949.
- [82] J. Peetre, On spaces of Triebel-Lizorkin type, Ark. Mat. 13 (1975), 123-130.
- [83] J. Peetre, New Thoughts on Besov Spaces, Duke University Mathematics Series, No. 1, Duke University, Mathematics Department, Durham, N.C., 1976.
- [84] J.-O. Str¨ omberg and A. Torchinsky, Weighted Hardy spaces, Lecture Notes in Mathematics, 1381. Springer-Verlag, Berlin, 1989. vi + 193 pp.
- [85] S. Treil and A. Volberg, Wavelets and the angle between past and future, J. Funct. Anal. 143 (1997), 269-308.
- [86] S. Roudenko, Matrix-weighted Besov spaces, Trans. Amer. Math. Soc. 355 (2003), 273314.
- [87] R. H. Torres, Boundedness Results for Operators with Singular Kernels on Distribution Spaces, Mem. Amer. Math. Soc. 90 (1991)
- [88] A. Volberg, Matrix Ap weights via S -functions, J. Amer. Math. Soc. 10 (1997), 445-466.
- [89] F. Wang, Y. Han, Z. He and D. Yang, Besov and Triebel-Lizorkin spaces on spaces of homogeneous type with applications to boundedness of Calder´ on-Zygmund operators, Dissertationes Math. 565 (2021), 1-113.

- [90] S. Wang, P. Guo and J. Xu, Characterizations of weighted Besov spaces with variable exponents, Acta Math. Sin. (Engl. Ser.) 40 (2024), 2855-2878.
- [91] S. Wang and J. Xu, Weighted Besov spaces with variable exponents, J. Math. Anal. Appl. 505 (2022), Paper No. 125478, 27 pp.
- [92] N.Wiener and P.Masani, The prediction theory of multivariate stochastic processes. II. The linear predictor, Acta Math. 99 (1958), 93-137.
- [93] J.S. Xu, Variable Besov and Triebel-Lizorkin spaces, Ann. Acad. Sci. Fenn., Math. 33(2) (2008) 511-522.
- [94] J.S. Xu, The relation between variable Bessel potential spaces and Triebel-Lizorkin spaces, Integral Transforms Spec. Funct. 19(8) (2008) 599-605.
- [95] D. Yang and W. Yuan, A new class of function spaces connecting Triebel-Lizorkin spaces and Q spaces, J. Funct. Anal. 255 (2008), 2760-2809.
- [96] D. Yang, W. Yuan and Z. Zeng, A ∞ on variable Lebesgue spaces, To be appeared.
- [97] D. Yang, C. Zhuo and W. Yuan, Besov-type spaces with variable smoothness and integrability, J. Funct. Anal. 269 (2015), 1840-1898.
- [98] W. Yuan, W. Sickel and D. Yang, Morrey and Campanato Meet Besov, Lizorkin and Triebel, Lecture Notes in Mathematics, 2005. Springer-Verlag, Berlin, 2010.
- [99] Z. Zeghad and D. Drihem, Variable Besov-type spaces, Acta Math. Sin. (Engl. Ser.) 39 (2023), 553-583.
- [100] C. Zhuo and D. Yang, Variable Besov spaces associated with heat kernels, Constr. Approx. 52 (2020), 479-523.
- [101] A. Zygmund, Smooth functions, Duke Math. J. 12 (1945), 4-76.

Dachun Yang (Corresponding author), Wen Yuan and Zongze Zeng

Laboratory of Mathematics and Complex Systems (Ministry of Education of China), School of Mathematical Sciences, Beijing Normal University, Beijing 100875, The People's Republic of China

```
E-mails : dcyang@bnu.edu.cn (D. Yang) wenyuan@bnu.edu.cn (W. Yuan) zzzeng@mail.bnu.edu.cn (Z. Zeng)
```