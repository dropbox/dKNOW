## An Entropy-Based Model for Hierarchical Learning

Amir R. Asadi ∗

Statistical Laboratory, University of Cambridge

January 24, 2023

## Abstract

Machine learning is the dominant approach to artificial intelligence, through which computers learn from data and experience. In the framework of supervised learning, a necessity for a computer to learn from data accurately and efficiently is to be provided with auxiliary information about the data distribution and target function through the learning model. This notion of auxiliary information relates to the concept of regularization in statistical learning theory. A common feature among real-world datasets is that data domains are multiscale and target functions are well-behaved and smooth. This paper proposes an entropy-based learning model that exploits this data structure and discusses its statistical and computational benefits. The hierarchical learning model is inspired by human beings' logical and progressive easy-to-hard learning mechanism and has interpretable levels. The model apportions computational resources according to the complexity of data instances and target functions. This property can have multiple benefits, including higher inference speed and computational savings in training a model for many users or when training is interrupted. We provide a statistical analysis of the learning mechanism using multiscale entropies and show that it can yield significantly stronger guarantees than uniform convergence bounds.

Keywordsmachine learning, hierarchical model, multiscale data, smooth function, multiscale entropy.

## 1 Introduction

## 1.1 Background

Nowadays, machine learning is the dominant approach to artificial intelligence, through which computers learn from data and experience. In the framework of supervised learning, data examples are assumed to emerge randomly as pairs ( X,Y ), where X and Y are called the data instance and the data label, respectively. There exists a target function Y = f ( X ) which maps any data instance to its corresponding label. A computer is given a sequence of data instances with their corresponding labels, independently drawn from the underlying data probability distribution. Then, it is expected to learn the target function f relating

∗ aa2345@cam.ac.uk

data instances with their corresponding labels and to predict the label of new randomly drawn data instances. An important observation is that the sequence of training examples usually does not contain all the missing information about the target function. Separate from the training data, auxiliary information should also be given through the learning model so that the computer can learn the target function accurately and efficiently. To shed more light on this discussion, consider the extreme case when no such auxiliary information is given to a computer. In other words, the computer has no information about the target function or the data domain besides the training sequence. What is the optimal task that it can do in this case? The best job is to memorize the training examples, which most likely results in overfitting and poor performance on new data instances. Similarly, much of every human being's knowledge about any task is learned based on training data plus prior knowledge and intuition. This notion of auxiliary information relates with the concept of 'regularization' studied in statistical learning theory, see the paper [1]. This concept has different forms, including restricting the hypothesis class and explicitly and implicitly regularizing the training mechanism. It is also connected to the no-free-lunch theorem (see, for example, [2, Theorem 5.1]), which implies that every learning algorithm should have some prior knowledge about the underlying data probability distribution and the target function to succeed.

A common feature among real-world datasets is that data domains are multiscale. That is, data emerge in different scales of magnitude and have a variety of sizes and complexities. This fact has been used in different topics, for example, wavelet theory, Fourier analysis, and signal processing (see, for example, the book [3]). Many examples of empirical data distributions in physics, biology, and social sciences are from multiscale distributions see, for example, the paper [4]. Moreover, target functions in the real world are usually well-behaved and can be written as compositions of simple functions.

How can one exploit such information about real-world data and target functions in a machine learning model to gain both statistically and computationally? To that aim, in this paper, we propose a model for learning from multiscale data and smooth target functions using a compositional learning architecture and a hierarchical training mechanism. Our learning model is inspired by the logical learning mechanisms of human beings, which learn different tasks progressively from easy to difficult examples. Multiscale data domains and smooth target functions appear ubiquitously in the real world, examples of which are medical datasets, financial datasets, biological datasets, natural language processing, etc. Thus, our learning model may have many applications.

## 1.2 Overview of This Paper

In our proposed learning model, we exploit the multiscale structure of data domains and the smoothness of their target functions. Throughout the paper, for the sake of simplicity, we assume that data instances and data labels are one-dimensional real numbers. We elaborate more with the following two assumptions:

- (a) Data instances X ∈ R emerge in different scales of magnitude. For example, this can be modeled by assuming that X has a power-law distribution µ , see Section 5.2. We assume that the domain consists of d separate scales: For a sequence of scale parameters γ 0 &lt; γ 1 &lt; · · · &lt; γ d , let the domain set X = X 1 ∪ · · · ∪ X d be partitioned into sets

X k := { γ k -1 ≤ | X | &lt; γ k } , where X k denote the domain of data at scale i based on the norm of X .

- (b) Target functions Y = f ( X ) ∈ R are well-behaved and smooth. In this paper, we model this by assuming that the target function f is invertible, and f and its inverse f -1 are twice-differentiable functions. Functions with these properties are known in the literature as diffeomorphism functions (see, for example, [5]).

The proposed hierarchical model learns by starting from the easy smaller-scaled examples and progressing towards the more difficult larger-scaled ones. To elaborate more, we consider the following definition: The dilation of the function f at scale γ is defined as

<!-- formula-not-decoded -->

which can be interpreted as a 'zoomed' version of the original function, where the amount of zooming in is determined by γ . The following important observation appears in the proof of [6, Theorem 2] and is based on the smoothness property of f : The functions f [ γ k ] , 1 ≤ k ≤ d , interpolate between f = f [ γ d ] and f [ γ 0 ] , and f [ γ 0 ] is very close to a linear function (the derivative of f at the origin) if γ 0 is small. We define the following multiscale decomposition of f :

<!-- formula-not-decoded -->

where ∆ k ( x ) := f [ γ k ] ◦ f -1 [ γ k -1 ] for all 1 ≤ k ≤ d , and call it the ladder decomposition of f . Here we used the notation for function composition: ( g 1 ◦ g 2 )( x ) = g 1 ( g 2 ( x )). Due to the smoothness property of f , if subsequent scale parameters γ k and γ k -1 are close to each other, then we expect that ∆ k , which is a transformation between the dilation of f at scale γ k -1 with the finer dilation f [ γ k ] , to be close to the identity function. Thus, we expect function ∆ k to be simple : its difference with the identity function does not vary too much and is easy to predict and learn. Consider a d -level hierarchical learning model defined as follows:

<!-- formula-not-decoded -->

We train this model so that each h k approximates the dilation f [ γ k ] , with the following hierarchical procedure: First, observe data examples at the smallest scale X 1 , corresponding to the easiest examples, and learn ∆ 1 by sampling w 1 from a Gibbs measure (a maximum entropy distribution). Then, given the learned ∆ 1 , observe data belonging to the next scale of magnitude X 2 , learn ∆ 2 similarly by sampling w 2 from a Gibbs measure, and repeat this process. Therefore, learning the target function for data at smaller scales can be described as 'easier' and is a stepping stone for learning more difficult cases at higher scales. This will then be a mathematical model for easy-to-hard learning where scale plays the role of time, similar to how a human learns a topic (or a course) from working on the easiest examples and progressing to the hardest examples. Hierarchically learning the ladder decomposition is analogous to step-by-step climbing a ladder. We elaborate more precisely on the model in the next sections.

Our proposed learning model has the following merits:

1. Hierarchical Learning Model with Interpretable Levels: In the proposed learning model, training is performed to make each level h k approximate the dilation of f

at scale γ k , that is, f [ γ k ] . In other words, the mapping between the input and any level of the learning model is made close to a scaled-down version of the target function. Thus, the role of each level of our compositional learning model has an interpretation, in contrast to black-box hierarchical models. Moreover, in our learning model, the complexity of a particular data instance is modeled with its norm . To extend this model to other applications, the type of norm may be chosen appropriately for the given problem: for example, in finance, the higher the amount of revenue resources of a company (larger norm), the more difficult it is to predict fraud. In medical imaging and image processing, the higher the amount of sparsity (lower glyph[lscript] 0 -norm), the easier it is to predict its target. In natural language processing, the norm can correspond to the frequency level. Similarly, the complexity of a target function can be interpreted with its Lipschitz norm: the higher the variability of any function, the more difficult it is to predict its value.

2. Computational Savings in Inference: An important computational benefit of the learned model is the following: To compute the output of the model given input data instance x ∈ X k and to predict its label (if x is a new data instance), it suffices to process this input only for k levels and compute h k ( x ). Namely, one does not need to pass the instance through all of the d levels of the learning model; only the first k levels are enough. This is true because the model is trained such that h k ( x ) approximates the dilation f [ γ k ] ( x ) with which one can compute the value of f ( x ) with appropriate rescaling, since | x | &lt; γ k . In other words, when using the trained model for predicting the target of new data, the amount of computation on computing the output of the learned model given that particular data instance as input is proportionate to the complexity or difficulty of that instance. Since, by assumption, data instances are distributed heterogeneously at different scales and difficulties, this fact can result in significant computational savings and higher inference speed.
3. Computational Savings in Training for Heterogeneous Users: The proposed learning model can provide computational savings when there are d different users, each requiring accurate prediction of the labels of data instances at scale k , that is, X k . Instead of training separate models for each of the d users, we streamline the process by using the trained model for user k -1 to train the model for user k , for all 1 ≤ k ≤ d .
4. Interruption During Training: Training of current machine learning models on massive datasets may take a very long time. Our training mechanism consists of d stages. Even if this mechanism terminates after stage k , for any reason, we can still guarantee a useful model with which one can accurately predict the label of data instances belonging to X 1 ∪ · · · ∪ X k .
5. Multiscale Statistical Analysis Stronger Than Uniform Convergence: The statistical analysis of the risk of the trained compositional model is tailored to the hierarchical training mechanism and takes its multiscale structure into account when deriving the bound on its statistical risk. Unsurprisingly, the bound can be much sharper than a uniform convergence bound for the empirical-risk-minimizing hypothesis.

This work has been inspired mainly by combining ideas from [6] and from [7]. The paper [6] shows that any smooth bi-Lipschitz function f can be represented as a composition of d

functions f d ◦ · · · ◦ f 1 where each function f k , 1 ≤ k ≤ d , is close to the identity function I in the sense that the Lipschitz constant of f k -I decreases inversely with d . Notably, the proof of [6, Theorem 2] is based on using the notion of dilations of smooth functions. The paper [7] obtains the solution to the multiscale entropy regularization problem which can be interpreted as a multiscale extension of Gibbs measures. However, it is not possible to efficiently sample from the probability distribution. By using its proof technique and by taking a reverse approach, in this paper, we derive the multiscale-entropic regularized loss function that a self-similar and computable distribution minimizes. Moreover, in contrast to the work of [7], the output of the hierarchical learning model is read from different levels depending on the scale of the input data instance. In other words, we match the multiscale architecture of the learning model with the multiscale data domain.

## 1.3 Further Related Work

Information-theoretic approaches to learning and analysis of the generalization error of learning algorithms have been devised in the context of PAC-Bayesian bounds [8], and later in a related form using mutual information in, for example, the work of [9, 10, 11]. These information-theoretic methods have been extended to multiscale techniques [12, 13, 7, 14]. The paper [10] further analyzes the statistical risk of the Gibbs measure, also called maximumentropy training, and a multiscale extension of this result has been derived [7]. In the work [15], some other information-theoretic measures of the stability of learning algorithms and bounds on their generalization ability have been derived.

Multiscale entropies, a linear combination of entropies of a system at different scales, implicitly appear in the classical chaining technique of probability theory. For example, one can rewrite Dudley inequality ([16]) variationally and transform the bound into a linear mixture of metric entropies at multiple scales. These multiscale measures have been further studied [17].

In the paper [18], it is shown that any diffeomorphism defined on the sphere can be written as the composition of bi-Lipschitz functions with small distortion.

Hierarchical learning models composed of near-identity layers have been used in the context of neural network learning as residual networks [19] and via a dynamical systems approach [20].

## 1.4 Organization of This Paper

The rest of the paper is organized as follows: We first provide the preliminaries and notation in Section 2. Then, in Section 3, we present the definition of ladder decompositions for diffeomorphisms and study the Lipschitz continuity and smoothness of each rung of this decomposition. In Section 4, our proposed learning model is described. Section 5 consists of two parts: In Subsection 5.1, we show that the multiscale maximum-entropy type of training achieves low chained risk . Then, in Subsection 5.2, we show if the data distribution µ is a power-law distribution, then the chained risk can bound the statistical risk from above, hence overall yielding an upper bound on the statistical risk of the learned model. Section 6 exemplifies that a set of Lipschitz functions - functions with bounded Lipschitz norm can be represented with a parameterized model with bounded-norm parameters. Finally, in Section 7 we discuss the conclusions of our work.

## 2 Preliminaries and Notation

Throughout the paper, | · | indicates Euclidean distance, I denotes the identity function, and µ ⊗ n denotes the n times tensor product of measure µ with itself. The set of real numbers and integers are denoted with R and Z , respectively.

Random variables and random vectors are represented with capital letters, while small letters are used for their realizations. Throughout the paper, U denotes the equiprobable (uniform) probability distribution where its support set is indicated with a subscript. For a random variable X and probability measure P , the notation X ∼ P means that X is distributed according to P . We denote the Dirac probability measure on w as δ w .

We first state some information-theoretic definitions and tools which we use in our analysis in Section 5. For two distributions P and Q , P glyph[lessmuch] Q means that P is absolutely continuous with respect to Q .

Definition 1 (Entropy) . The Shannon entropy of a discrete random variable X taking values on A is defined as

<!-- formula-not-decoded -->

The relative entropy between two distributions P X and Q X , if P X glyph[lessmuch] Q X is defined as

<!-- formula-not-decoded -->

otherwise, we define D ( P X ‖ Q X ) := ∞ . The conditional relative entropy is defined as

<!-- formula-not-decoded -->

The following extremely useful property of entropy is called the 'chain rule'. For proof, see, for example, [21, Theorem 2.5.3]:

Lemma 1 (Entropy Chain Rule) . Let P XY and Q XY be two distributions. We have

<!-- formula-not-decoded -->

The next definition relates to 'geometric' transformations of probability measures:

Definition 2 (Scaled and Tilted Distributions) . Given a discrete probability measure P defined on a set A , and any λ ∈ [0 , 1] , we define the scaled distribution ( P ) λ for all a ∈ A as

<!-- formula-not-decoded -->

Given two discrete probability measures P and Q defined on a set A , and any λ ∈ [0 , 1] , we define the tilted distribution ( P, Q ) λ as the following geometric mixture:

<!-- formula-not-decoded -->

Clearly, if U is the equiprobable distribution on A , then

<!-- formula-not-decoded -->

We require the definition of R´ enyi divergence to properly state the next lemma.

Definition 3 (R´ enyi Divergence) . For discrete distributions P and Q defined on a set A and for any λ ∈ (0 , 1) ∪ (1 , ∞ ) , the R´ enyi divergence is defined as

<!-- formula-not-decoded -->

For λ = 1 , we define D λ ( P ‖ Q ) := D ( P ‖ Q ) .

In our analysis in Section 5, similar to the paper of [7], we encounter linear combinations of relative entropies. The next lemma shows the role of tilted distributions in such linear combinations. For a proof, see [22, Theorem 30]:

Lemma 2 (Entropy Combination) . Let λ ∈ [0 , 1] . For any distributions P, Q and R such that P glyph[lessmuch] Q and P glyph[lessmuch] R , we have

<!-- formula-not-decoded -->

We provide the following definition to later simplify the notation in the proof of Theorem 2:

Definition 4 (Congruent Functionals) . We call two functionals L 1 ( P ) and L 2 ( P ) of a distribution P congruent and write L 1 ∼ = L 2 if L 1 -L 2 does not depend on P .

For example, Lemma 2 implies that if Q and R are fixed distributions, then as functionals of P , the following congruency holds:

<!-- formula-not-decoded -->

Specifically, if U is the equiprobable distribution, then

<!-- formula-not-decoded -->

The following well-known result, sometimes referred to as the Gibbs variational principle, implies that the distribution that minimizes the sum of average energy (loss) and entropy (regularization) is a Gibbs measure:

Lemma 3 (Gibbs Measure) . Let W be an arbitrary finite set. Given a function f : W → R and λ &gt; 0 , we define the following Gibbs probability measure for all w ∈ W :

<!-- formula-not-decoded -->

Then, for any probability measure P W defined on W , we have

<!-- formula-not-decoded -->

where W ∼ P W .

Particularly, Lemma 3 yields the following congruency identity as functionals of P W :

<!-- formula-not-decoded -->

We later make use of the congruency relations (1) and (2) iteratively in the proof of Theorem 2.

Next, we present some definitions of regularity of functions and the related notation.

Definition 5 (Lipschitz and Smooth Function) . Let V be a compact subset of R . A differentiable function f : V → R is M 1 -Lipschitz if for all x, y ∈ V ,

<!-- formula-not-decoded -->

We say that the function f is M 2 -smooth if its derivative f ′ is M 2 -Lipschitz.

Clearly, if f is a M 1 -Lipschitz function, then we have | f ′ ( x ) | ≤ M 1 for all x ∈ R . In this paper, we require both f and its inverse f -1 to be well-behaved functions, as described in the following definition:

Definition 6 (Diffeomorphism) . Let V be a compact subset of R . A function f : V → R is an ( M 1 , M 2 ) -diffeomorphism if it is invertible and both f and its inverse f -1 are twice differentiable, M 1 -Lipschitz and M 2 -smooth.

If f is a ( M 1 , M 2 )-diffeomorphism, we then have M 1 ≥ 1, since

<!-- formula-not-decoded -->

Next, we define the notion of dilation of a function, which is equivalent to a rescaled version of it:

Definition 7 (Dilation) . For any 0 &lt; γ ≤ 1 , the dilation of function f at scale γ is defined as

<!-- formula-not-decoded -->

It is easy to prove that if f is invertible, then so is its dilation f [ γ ] , where

<!-- formula-not-decoded -->

In other words, the inverse of the dilation is identical to the dilation of the inverse function. Now, we prove the following proposition:

Proposition 1. Let f : ( -R,R ) → R be a M 2 -smooth function. Then, f [ γ ] ( x ) -f ′ (0) x is γM 2 R -Lipschitz.

Proof. The proof is based on a simple extension of the proof of [6, Theorem 2]. Let x, y ∈ ( -R,R ). Based on the mean value theorem, there exists z in between x and y such that

f ( γx ) -f ( γy ) = γf ′ ( γz )( x -y ) . We can write

<!-- formula-not-decoded -->

In particular, if f (0) = 0, then we have

<!-- formula-not-decoded -->

In Section 5, we require the following well-known result on one specific type of Kolmogorov (quasi-arithmetic) mean:

Lemma 4 (Kolmogorov Mean) . Let z = ( z 1 , z 2 , . . . , z N ) ∈ R N . For λ ≥ 0 , the following weighted average

<!-- formula-not-decoded -->

which is a type of Kolmogorov mean, satisfies

<!-- formula-not-decoded -->

The proof of Theorem 3 requires the following tools of the topic of concentration of measures:

Definition 8 (Subgaussian) . A random variable X is called σ -subgaussian if for all λ ∈ R , its cumulant generating function satisfies

<!-- formula-not-decoded -->

The following result is based on [10, Lemma 1], which itself can be derived from the transportation lemma of [23, Lemma 4.18]:

Lemma 5. If g ( ¯ A, ¯ B ) is σ -subgaussian where ( ¯ A, ¯ B ) ∼ P A P B , then for all λ &gt; 0 ,

<!-- formula-not-decoded -->

The Azuma-Hoeffding inequality shows the subgaussianity of the sum of independent and bounded random variables:

Lemma 6 (Azuma-Hoeffding) . Let X 1 , . . . , X n be independent random variables such that a ≤ X i ≤ b for all i . Then,

<!-- formula-not-decoded -->

In other words, ∑ n i =1 X i /n is ( b -a ) / √ n -subgaussian.

The following well-known lemma, which we use in Section 6, bounds the approximation error of the Reimann sum (see, for example, the book [24]):

Lemma 7. The approximation error of the Reimann sum is bounded as follows:

<!-- formula-not-decoded -->

where ˆ M is the maximum absolute value of the derivative of f on [ a, b ] .

In the framework of supervised batch learning, X represents the instances domain, Y denotes the labels domain, and Z = X ×Y is the examples domain. H = { h w : w ∈ W} is the hypothesis set, where the hypotheses are indexed by an index set W . Let glyph[lscript] : W× Z → R + be a loss function. A learning algorithm receives a random training sequence S = ( Z 1 , Z 2 , ..., Z n ) of n examples with i.i.d. random elements drawn from Z with an unknown distribution µ . Namely, S ∼ µ ⊗ n . In the training procedure, it chooses h W ∈ H according to a random transformation P W | S . For any w ∈ W , let L µ ( w ) := E [ glyph[lscript] ( w,Z )] denote the statistical (or population) risk of hypothesis h w , where Z ∼ µ . The aim of statistical learning is to choose a learning algorithm for which the expected statistical risk E [ L µ ( W )] is small.

## 3 Ladder Decompositions of Smooth Functions

In this section, we show that any diffeomorphism defined on a bounded interval ( -R,R ) can be decomposed at multiple scales into what we name ladder decomposition , such that different layers (rungs) of this decomposition are smooth and Lipschitz with small Lipschitz norm.

Definition 9 (Ladder Decomposition) . Let d ≥ 1 be an arbitrary integer. Consider a sequence of scale parameters { γ k } d k =0 such that 0 &lt; γ 0 &lt; γ 1 &lt; · · · &lt; γ d = 1 . For any function f : R → R and for all 1 ≤ k ≤ d , let

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

and

Clearly, for all 1 ≤ k ≤ d , we have

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

We call (3) the ladder decomposition of function f at scale parameters { γ k } d k =0 .

In particular,

Based on Proposition 1, the smaller γ k is, the closer function f [ γ k ] ( x ) is to the linear function f ′ (0) x . We intuitively expect that if two subsequent scale parameters γ k and γ k -1 are close, then ∆ k is a function close to the identity function. The next theorem precisely formulates this intuition and is a key result for the rest of the paper. Assume that C 1 := 3 M 1 M 2 and C 2 := M 2 ( M 2 1 + M 1 ).

Theorem 1. Let f : ( -R,R ) → R be a ( M 1 , M 2 ) -diffeomorphism. For all 1 ≤ k ≤ d , the function ψ k ( x ) is C 1 R ( γ k -γ k -1 ) -Lipschitz and C 2 -smooth.

Proof. For any 1 ≤ k ≤ d , let x, y be arbitrary elements of the domain of ψ k . We can write

<!-- formula-not-decoded -->

where v := f -1 [ γ k -1 ] ( x ) and u := f -1 [ γ k -1 ] ( y ). Define r ( z ) := f ( γ k z ) -f ( γ k -1 z ). We have r ′ ( z ) = γ k f ′ ( γ k z ) -γ k -1 f ′ ( γ k -1 z ). Based on the mean value theorem, there exists z 1 , z 2 between u and v such that r ( u ) -r ( v ) = r ′ ( z 1 )( u -v ) and f ( γ k -1 u ) -f ( γ k -1 v ) = f ′ ( γ k -1 z 2 ) γ k -1 ( u -v ). Note that | z 1 | , | z 2 | ≤ max {| u | , | v |} ≤ R . Hence,

<!-- formula-not-decoded -->

Therefore,

<!-- formula-not-decoded -->

Since f -1 is M 1 -Lipschitz, we have

<!-- formula-not-decoded -->

Combining (4) and (5), we get

<!-- formula-not-decoded -->

Hence, ψ ( x ) is C 1 R ( γ k -γ k -1 )-Lipschitz.

We now prove the smoothness property. Let g ( x ) := f -1 ( x ). Based on the chain rule of derivatives, we can write

<!-- formula-not-decoded -->

Therefore,

<!-- formula-not-decoded -->

Based on the assumption that f and g are both M 1 -Lipschitz and M 2 -smooth and γ k -1 , γ k ≤ 1, we deduce

<!-- formula-not-decoded -->

Therefore, ψ k ( x ) is C 2 -smooth.

Remark 1. The proof of [6, Theorem 2] only implies that, for all 1 ≤ k ≤ d , function ψ k is C ( γ k -γ k -1 ) /γ k -Lipschitz for some constant C , which is weaker than our result. For example, when scale parameters are chosen as γ k = k/d , then our result yields that ψ k is O (1 /d )-Lipschitz, whereas [6, Theorem 2] only concludes that ψ k is O ((log d ) /d )-Lipschitz. However, our result is currently restricted to functions f with domain and range in R .

We now present an example in which the functions ψ k ( x ), 1 ≤ k ≤ d , have a closed-form expression:

Example 1. Let f = tanh( x ). We have f -1 ( x ) = 1 2 ln ( 1+ x 1 -x ) . Assume that γ k = 2 k -d for all 0 ≤ k ≤ d . We can write

<!-- formula-not-decoded -->

Figure 1 depicts the plot of ψ k ( x ) for all 1 ≤ k ≤ d , where d = 5.

Figure 1: A plot of ψ k ( x ) for different values k : k = 1 the solid line, k = 2 the dotted line, k = 3, the dashed line, k = 4, the dotted-dashed line and k = 5 the large dashed line.

<!-- image -->

## 4 The Proposed Learning Model

In this section, we precisely formulate the learning model. Let d ≥ 1 be an integer and ε &gt; 0 and β &gt; 1 be real numbers. Assume that the data instance domain is defined as

<!-- formula-not-decoded -->

where R := εβ d . Let the scale parameters { γ k } d k =0 form a geometric sequence such that for all 0 ≤ k ≤ d ,

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Clearly, X = ∪ d k =1 X k . We call each set X k the domain of instances at scale k . It is obvious that ε and R are the smallest and largest magnitude of data instances, respectively.

We let the label set to be Y = R and assume that the target function f is a ( M 1 , M 2 )-diffeomorphism. Suppose that from previous knowledge or intuition, we know f [ γ 0 ] , that is, we know the behavior of function f at extremely small scales 0 ≤ | x | &lt; ε . For example, it may be assumed that f [ γ 0 ] is equal to the derivative of f at the origin, namely, the linear function f ′ (0) x . For simplicity, we further assume that f (0) = 0. Based on Theorem 1, For all 1 ≤ k ≤ d , the function ψ k ( x ) is C 1 ε ( β -1) β k -1 -Lipschitz and C 2 -smooth.

To learn the target function f on X progressively and stage by stage, we define the following d -level hiearchical learning model: Assume that h 0 ( x ) := f [ γ 0 ] ( x ) and for all 1 ≤ k ≤ d ,

<!-- formula-not-decoded -->

We call w k the parameters of the k th level of the learning model and allow it to take a value from a set W k during training. Let w = ( w 1 , . . . , w d ) be the sequence of parameters of this model. The learning model aims to make the mapping between the input and each layer h k approximate the dilated version of the target function f at scale γ k , that is, f [ γ k ] . Thus, for a successfully trained model, F ( h k -1 ( x ) , w k ) should well approximate ψ k ( h k -1 ( x )). Given that ψ k ( · ) is C 1 R ( γ k -γ k -1 )-Lipschitz and by assumption ψ k (0) = 0, it is enough to assume that for all w k ∈ W k ,

<!-- formula-not-decoded -->

where ρ k ≈ C 1 R ( γ k -γ k -1 ). The function F should be chosen per enough representation power of the model; we give an example in Section 6.

Given a successfully trained model, the number of steps that an input instance x ∈ X needs to be processed is proportionate to the scale of the instance magnitude | x | . This can be interpreted as a measure of the difficulty or complexity of that particular instance. More precisely, we define the output of the model h ( x ) as follows:

<!-- formula-not-decoded -->

For all 1 ≤ k ≤ d , we define

Let the n -tuple of training instances be denoted with s = ( x 1 , . . . , x n ). We assume that we are given the instance-label pairs ( x i , f ( x i )) for all 1 ≤ i ≤ n . We now mathematically model the training mechanism. This mechanism starts from the simplest training examples whose instances are at the smallest scale X 1 , and progressively trains the layers of the model by using the larger-scaled (more complex) examples. At each level, corresponding to each scale of the data, training is modeled as sampling w k from a Gibbs measure with loss (energy) as the empirical risk evaluated for that specific scale of the training data. It is well-known that such Gibbs measures are maximum-entropy distributions, see the paper of [25]. Precisely, for all 1 ≤ k ≤ d , given trained values for w k -1 1 , we sample the vector value for w k from the following probability distribution:

<!-- formula-not-decoded -->

where h ′ k denote the levels of a learning model with parameters ( w ′ 1 , . . . , w ′ d ). For this reason, this training mechanism is hierarchical, stochastic, and self-similar (at each scale we sample from a Gibbs measure with a similar loss function). We call this training mechanism multiscale entropic training . This mechanism has the following benefit: if we stop training after sampling the first k levels, then we are guaranteed to have a useful trained model for data in X 1 ∪ X 2 ∪ · · · ∪ X k .

## 5 Analysis of the Learning Model

In this section, using multiscale entropies, we statistically analyze the learning model's performance. In Subsection 5.1, we prove that the multiscale entropic training mechanism achieves low chained risk . Then, in Subsection 5.2, we provide an example of the data instance distribution µ , a power-law probability distribution, with which we can bound the statistical risk based on the chained risk. Subsection 6 shows a parameterization example and analyzes its representation power.

## 5.1 Multiscale Entropic Training and Chained Risk

Let w := ( w 1 , . . . , w d ). For all 1 ≤ k ≤ d , we define w k 1 := ( w 1 , . . . , w k ) and

<!-- formula-not-decoded -->

Clearly, the loss of the model on example ( x, f ( x )) is glyph[lscript] ( w , x ) = ∑ d k =1 glyph[lscript] k ( w k 1 , x ) .

Recall that the n -tuple of training instances is denoted with s = ( x 1 , . . . , x n ). For all 1 ≤ k ≤ d , we define

<!-- formula-not-decoded -->

Based on the definition of the model in the previous section, for all 1 ≤ k ≤ d , we have

<!-- formula-not-decoded -->

and

<!-- formula-not-decoded -->

For simplicity in the notation, henceforth we assume that λ d +1 := 0. The next theorem indicates that the self-similar measure P ∗ W is the minimizing distribution of the sum of a multiscale loss and a multiscale entropy. Let

<!-- formula-not-decoded -->

where for all 1 ≤ k ≤ d ,

<!-- formula-not-decoded -->

## Theorem 2. We have

<!-- formula-not-decoded -->

Proof. We develop what we call the 'multiscale congruent technique' used in the proof of [7, Theorem 13]. Specifically, recalling the definition of congruent functionals in Definition 4, we aim to show that as a functional of P W ,

<!-- formula-not-decoded -->

This would then immediately imply (10), as setting P W = P glyph[star] W makes all entropies in the right side of (11) vanish together and .

For all 1 ≤ k ≤ d , define

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

and

We can write

<!-- formula-not-decoded -->

where (12) is based on Lemma 3, (13) is based on the chain rule of entropy (Lemma 1), and (14) is based on Lemma 2. Note that

<!-- formula-not-decoded -->

thus

<!-- formula-not-decoded -->

Let Z and ¯ Z denote the normalizing constants (partition functions) of ( Q ( d ) W d -1 1 ) λ d λ d -1 and

Q ( d -1) W d -1 1 , respectively. We have

<!-- formula-not-decoded -->

Thus,

<!-- formula-not-decoded -->

Based on (14) and (15), we deduce

<!-- formula-not-decoded -->

Iterating this argument for k = d -1 , . . . , 1, we deduce that

<!-- formula-not-decoded -->

Note that, for all 1 ≤ k ≤ d ,

<!-- formula-not-decoded -->

Thus, based on (16), we have

<!-- formula-not-decoded -->

Since, for all 1 ≤ k ≤ d , we have

<!-- formula-not-decoded -->

we can deduce that

<!-- formula-not-decoded -->

as desired.

Notice that for all 1 ≤ k ≤ d , glyph[lscript] k ( w k -1 1 , s ) is a Kolmogorov mean, the same type of which assumed in Lemma 4.

Straightforwardly, the result extends for a random training sequence (vector) S ∼ µ ⊗ n .

Corollary 1. Assume that for all 1 ≤ k ≤ d ,

<!-- formula-not-decoded -->

Let P ∗ W | S = P ∗ W 1 | S P ∗ W 2 | W 1 S . . . P ∗ W d | W d -1 1 S . Then,

<!-- formula-not-decoded -->

where ( S , W ) ∼ P S P W | S .

Assume that our hypothesis set is realizable, that is, there exist parameters ˆ w in our hypothesis index set such that ψ k ( · ) = F ( · , ˆ w k ) (recall the definition of ψ k in Definition 9). Namely, we assume that function f belongs to the hypothesis set. Consider the following definition of risk:

Definition 10 (Chained Risk) . For any fixed w , we define the chained risk as follows:

<!-- formula-not-decoded -->

The training mechanism of the learning model chooses the values of parameters w 1 , ..., w d sequentially. At the k th stage, choosing w k instead of the true target function parameter ˆ w k results in the following difference between the statistical risks:

<!-- formula-not-decoded -->

The chained risk is equal to the accumulation of these deviations of statistical risk at each of the d stages of the training mechanism. Clearly, we have L C µ (ˆ w ) = 0. Intuitively and roughly speaking, if the chained risk of w is small, then h w should be close to the target function h ˆ w = f . In the next theorem, we derive an upper bound on the expected value of the chained risk E [ L C µ ( W ) ] when the model is trained by the multiscale entropic mechanism (sampling w from P glyph[star] W | S ). Then, in the next subsection, we show a condition on the data instance distribution µ with which small chained risk implies small statistical risk.

Theorem 3. Let ( S , W ) ∼ P S P glyph[star] W | S . The average chained risk of the training mechanism P glyph[star] W | S satisfies the following inequality:

<!-- formula-not-decoded -->

Proof. Let ¯ W and ¯ S = ( ¯ X 1 , . . . , ¯ X n ) be independent copies of W and S , respectively, and assume that ¯ W and ¯ S are independent from each other. Namely, ( ¯ W , ¯ S ) ∼ P W P S . Clearly,

<!-- formula-not-decoded -->

Recall that for any fixed w = ( w 1 , . . . , w d ) and any x and all 1 ≤ k ≤ d , h k ( x ) is defined as in (7). Based on (9), we have

<!-- formula-not-decoded -->

and

<!-- formula-not-decoded -->

Since for any a, b ∈ R , it is easily seen that || a | - | b || ≤ | a -b | , we deduce that

<!-- formula-not-decoded -->

where (17) is obtained by using (7). Hence, based on Azuma-Hoeffding's inequality (Lemma 6), for any fixed w and all 1 ≤ k ≤ d , glyph[lscript] k ( w k 1 , S ) -glyph[lscript] k ( w k -1 1 ˆ w k , S ) is 4 γ k ρ k / √ n -subgaussian. Thus, glyph[lscript] k ( ¯ W k 1 , ¯ S ) -glyph[lscript] k ( ¯ W k -1 1 ˆ w k , ¯ S ) is 4 γ k ρ k / √ n -subgaussian as well. Based on Lemma 5, we can write

<!-- formula-not-decoded -->

where we define for all 1 ≤ k ≤ d ,

Therefore,

<!-- formula-not-decoded -->

where (19) is obtained by rewriting (18), (20) is based on Lemma 4, (21) is obtained based on Corollary 1 and by replacing P ∗ W | S with the conditional distribution P W | S = δ ˆ w (the Dirac measure on ˆ w ), and (22) is again based on Lemma 4 and by noting that glyph[lscript] k ( ˆ w k 1 , x ) = 0 for all 1 ≤ k ≤ d and all x ∈ X .

<!-- formula-not-decoded -->

Optimizing the bound in (23) over the values of ( λ 1 , . . . , λ d ) gives the following result:

Corollary 2. Assume that ( λ 1 , . . . , λ d ) are chosen such that for all 1 ≤ k ≤ d ,

<!-- formula-not-decoded -->

Then, the right side of (23) is minimized with respect to ( λ 1 , . . . , λ d ) . In this case, the bound simplifies to the following form:

<!-- formula-not-decoded -->

## 5.2 Bounding Statistical Risk Based on Chained Risk: An Example

The analysis, up to now, did not require any restrictions on the data instance distribution µ . In this subsection, we give an example of a condition on µ for which small chained risk implies small statistical risk.

Assume that the instance distribution µ defined on X has the following power-law probability density function with shape parameter α ≥ 1:

<!-- formula-not-decoded -->

where C ′ = ∫ X | x | -α d x . The density function q ( x ) is scale-invariant: For all x ∈ X , we have

<!-- formula-not-decoded -->

Given this assumption, in the following result we show that the chained risk can bound the statistical risk from above:

Theorem 4. If µ has a power-law probability density function with shape parameter α , then for any w , we have

<!-- formula-not-decoded -->

Proof. For any 1 ≤ k ≤ d , let ˆ h k ( x ) denote the k th level of the model given weight parameters

ˆ w and input x . Assume that ψ ˆ w k ( · ) := F ( · , ˆ w k ). For any x ∈ X k , we have

<!-- formula-not-decoded -->

where in (25), we used the fact that ψ ˆ w k is C 1 ε ( β -1) β k -1 γ k -Lipschitz (based on Theorem 1). Now, define

<!-- formula-not-decoded -->

We can observe that x ′ ∈ X k -1 and the transformation x → x ′ is a bijection between X k and X k -1 . Therefore

<!-- formula-not-decoded -->

Let X be distributed according to density q ( x ). Based on (26), we have

<!-- formula-not-decoded -->

Thus,

<!-- formula-not-decoded -->

where (28) is based on (27).

Therefore, based on Corollary 2, given sufficiently large α &gt; 1 for which

<!-- formula-not-decoded -->

the following inequality holds:

<!-- formula-not-decoded -->

Given the realizability assumption on the hypothesis set, the regular union bound applied to the empirical-risk-minimizing hypothesis yields

<!-- formula-not-decoded -->

Ignoring the effect of ( 1 -β 1 -α ( 1 + C 1 R (1 -β -1 ) )) , the following example shows that the right side of (29) can be quite smaller than the right side of (30):

Example 2. Let R/glyph[epsilon1] := ¯ R and β = ( ¯ R ) 1 /d . Recall that we have γ k = β k -d as in (6) . Assume that ρ k = ρ 0 β k for all k = 1 , . . . , d and |W 1 | = · · · = |W d | . We compute the following ratio

<!-- formula-not-decoded -->

The power of two exists to compare the bounds on the required number of samples n . For example, given ¯ R = 10 and d = 20 , we obtain Λ ≈ 0 . 2648 .

## 6 A Parameterization Example and Analysis of its Representation Power

In this section, we show with an example that a set of Lipschitz functions, i.e. bounded Lipschitz norm, can be represented with a parameterized model with bounded-norm parameters. Note that the range of f [ γ k ] , which is the domain of ψ k , is an interval subset of ( -M 1 R,M 1 R ) that includes 0, where R = εβ d .

Let Ψ( x ) be a φ 1 -Lipschitz and φ 2 -smooth function with support D Ψ = ( a 1 , a 2 ) ⊆ ( -M 1 R,M 1 R ) such that 0 ∈ D Ψ and Ψ(0) = 0. Later in this section, we will replace Ψ with each ψ k of the ladder decomposition of the diffeomorphism f in (3). For any x ∈ D Ψ , we can write

<!-- formula-not-decoded -->

where H ( x ) is the Heaviside (unit) step function. Note that due to Ψ being φ 1 -Lipschitz, for all b ∈ D Ψ , we have | Ψ ′ ( b ) | ≤ φ 1 . Moreover, since Ψ(0) = 0, we conclude that | Ψ( a 1 ) | ≤ φ 1 | a 1 | ≤ φ 1 M 1 R .

For a given integer τ ≥ 2, let a τ -width two-layer network with continuous parameters w := { w 1 , . . . , w τ , w ( c ) } be defined for any x ∈ ( -M 1 R,M 1 R ) as

<!-- formula-not-decoded -->

where for all 1 ≤ j ≤ τ , b j := ( -1 + 2 j/τ ) M 1 R,

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

We can view ¯ ψ ( τ ) w ( x ) in (32) as a Reimann sum approximation to the integral representation of Ψ( x ) in (31). Using Lemma 7, we deduce the following result:

Lemma 8. For all x ∈ D Ψ , we have

<!-- formula-not-decoded -->

Proof. Let j 1 be the smallest integer j such that b j ∈ D Ψ , and let j 2 be the largest integer j and

such that ( -1 + 2 j/τ ) M 1 R ≤ x . We have

<!-- formula-not-decoded -->

Now, we proceed to discretize the weights of the network. Let η &gt; 0 be the precision level of the weights. We discretize the weights of ¯ ψ ( τ ) w in (32) by choosing the closest real number in η Z to each weight. For any function Ψ, let its approximate τ -width two-layer network with discretized parameters at discretization η be defined as

<!-- formula-not-decoded -->

We have the following bound on the approximation error of the finite-width two-layer network: Proposition 2 (Approximation Error) . For all x ∈ D Ψ ,

<!-- formula-not-decoded -->

Proof. Based on Lemma 8, for all x ∈ D Ψ we have

<!-- formula-not-decoded -->

Thus, for all x ∈ D Ψ , we can deduce

<!-- formula-not-decoded -->

In the following proposition, we show an upper bound on the glyph[lscript] 1 -norm of the finite-width discretized network ψ ( τ,η ) w derived from function Ψ( x ) in (33).

Proposition 3 (Bounded Norm) . The glyph[lscript] 1 -norm of the network ψ ( τ,η ) w , defined as the sum of absolute values of its weights w satisfies

<!-- formula-not-decoded -->

Proof. Ψ( x ) is φ 2 -smooth, therefore Ψ ′ ( x ) is φ 2 -Lipschitz. Thus, | Ψ ′ ( x ) | is φ 2 -Lipschitz as well. Recall that | Ψ ′ ( b ) | ≤ φ 1 M 1 R for all b ∈ D Ψ . We have

<!-- formula-not-decoded -->

We now replace function Ψ in the previous arguments with ψ k for any 1 ≤ k ≤ d . Based on Theorem 1, we take φ 1 ← C 1 Rβ k -d -1 ( β -1) and φ 2 ← C 2 . Define

<!-- formula-not-decoded -->

Suppose W k , the set of weights for our learning model at level k , is

<!-- formula-not-decoded -->

For all 1 ≤ k ≤ d , in the recursive definition of the model (7), we define

<!-- formula-not-decoded -->

Therefore,

<!-- formula-not-decoded -->

Such bounded regularization on the weights of the learning model immediately implies the following property:

Proposition 4 (Bounded Output) . Let w ∈ W . Then, for all x ∈ ( -M R,M R ) , we have

<!-- formula-not-decoded -->

k 1 1 τ,η .

<!-- formula-not-decoded -->

Proof. We can write

## 7 Conclusions

In this paper, we presented an entropy-based learning model to exploit the multiscale structure of data domains and the smoothness of target functions. We first showed the definition of ladder decompositions for diffeomorphisms and studied Lipschitz continuity and smoothness of the levels of this decomposition. Then, we proved that the self-similar maximum-entropy type training achieves low chained risk. We showed that if the data distribution µ is a powerlaw distribution, then the chained risk can bound the statistical risk from above. Hence, this yields that the multiscale-entropic training mechanism achieves low statistical risk. Finally, we provided an example of a parameterized model with bounded-norm parameters.

Our proposed learning model has the following merits: It is a hierarchical learning model with interpretable levels. The training is carried out to make the mapping between the input and any level of the learning model approximate a dilation of the target function. This makes the role of each level of the learning model have an interpretation, in contrast to black-box hierarchical models. Another merit of the proposed model is the computational point of view. The amount of computation required on computing the output of the learned model given a particular data instance as input is proportionate to the complexity of that instance. Since, by assumption, data instances are distributed heterogeneously at different scales and complexities, this fact can result in significant computational savings and higher inference speed. On the other hand, the proposed learning model can provide computational savings when several different users each require learning the target function at a particular scale of data instances. Moreover, training of current machine learning models on massive datasets may take a very long time. We showed that as our training mechanism consists of different stages if for any reason this mechanism terminates after a stage, one can still guarantee a useful model with which it can accurately predict the label of data instances with norms smaller than a particular value depending on the stage. Finally, as the statistical analysis of the risk of the trained compositional model is tailored to the hierarchical training mechanism and takes its multiscale structure into account in deriving the bound on its statistical risk, the bound can be much sharper than a uniform convergence bound for the empirical-riskminimizing hypothesis.

## References

- [1] Vladimir Vapnik. The Nature of Statistical Learning Theory . Springer science &amp; business media, 1999.
- [2] Shai Shalev-Shwartz and Shai Ben-David. Understanding Machine Learning: From Theory to Algorithms . Cambridge University Press, 2014.
- [3] Weinan E. Principles of Multiscale Modeling . Cambridge University Press, 2011.
- [4] Aaron Clauset, Cosma Rohilla Shalizi, and M. E. J. Newman. Power-law distributions in empirical data. SIAM Review , 51(4):661-703, 2009.
- [5] Morris W Hirsch. Differential Topology , volume 33. Springer Science &amp; Business Media, 2012.
- [6] Peter L. Bartlett, Steven N. Evans, and Philip M. Long. Representing smooth functions as compositions of near-identity functions with implications for deep network optimization. arXiv preprint arXiv:1804.05012 , 2018.
- [7] Amir R. Asadi and Emmanuel Abbe. Chaining meets chain rule: Multilevel entropic regularization and training of neural networks. Journal of Machine Learning Research , 21(139):1-32, 2020.
- [8] David A. McAllester. PAC-Bayesian model averaging. In Proceedings of the twelfth annual conference on Computational learning theory , pages 164-170, 1999.
- [9] Daniel Russo and James Zou. How much does your data exploration overfit? controlling bias via information usage. arXiv preprint arXiv:1511.05219 , 2015.
- [10] Aolin Xu and Maxim Raginsky. Information-theoretic analysis of generalization capability of learning algorithms. In Advances in Neural Information Processing Systems , pages 2524-2533, 2017.
- [11] Yuheng Bu, Shaofeng Zou, and Venugopal V Veeravalli. Tightening mutual information based bounds on generalization error. IEEE Journal on Selected Areas in Information Theory , 2020.
- [12] J. Audibert and O. Bousquet. Combining PAC-Bayesian and generic chaining bounds. Journal of Machine Learning Research , 8(Apr):863-889, 2007.
- [13] Amir R. Asadi, Emmanuel Abbe, and Sergio Verd´ u. Chaining mutual information and tightening generalization bounds. In Advances in Neural Information Processing Systems , pages 7234-7243, 2018.
- [14] Eugenio Clerico, Amitis Shidani, George Deligiannidis, and Arnaud Doucet. Chained generalisation bounds. In Proceedings of Machine Learning Research , pages 4212 -4212, 2022.
- [15] Maxim Raginsky, Alexander Rakhlin, Matthew Tsao, Yihong Wu, and Aolin Xu. Information-theoretic analysis of stability and bias of learning algorithms. In 2016 IEEE Information Theory Workshop (ITW) , pages 26-30. IEEE, 2016.

- [16] R. M. Dudley. The sizes of compact subsets of Hilbert space and continuity of Gaussian processes. Journal of Functional Analysis , 1(3):290-330, 1967.
- [17] Amir R. Asadi and Emmanuel Abbe. Maximum multiscale entropy and neural network regularization. arXiv preprint arXiv:2006.14614 , 2020.
- [18] Alastair Fletcher and Vladimir Markovic. Decomposing diffeomorphisms of the sphere. Bulletin of the London Mathematical Society , 44(3):599-609, 2012.
- [19] Kaiming He, Xiangyu Zhang, Shaoqing Ren, and Jian Sun. Deep residual learning for image recognition. In Proceedings of the IEEE conference on computer vision and pattern recognition , pages 770-778, 2016.
- [20] Weinan E. A proposal on machine learning via dynamical systems. Communications in Mathematics and Statistics , 5(1):1-11, 2017.
- [21] Thomas M Cover and Joy A Thomas. Elements of Information Theory . John Wiley &amp; Sons, 2012.
- [22] Tim Van Erven and Peter Harremos. R´ enyi divergence and Kullback-Leibler divergence. IEEE Transactions on Information Theory , 60(7):3797-3820, 2014.
- [23] S. Boucheron, G. Lugosi, and P. Massart. Concentration Inequalities: A Nonasymptotic Theory of Independence . Oxford University Press, 2013.
- [24] Deborah Hughes-Hallett, Andrew M. Gleason, and William G. McCallum. Calculus: Single and Multivariable . John Wiley &amp; Sons, 2020.
- [25] Edwin T Jaynes. Information theory and statistical mechanics. Physical review , 106(4):620, 1957.