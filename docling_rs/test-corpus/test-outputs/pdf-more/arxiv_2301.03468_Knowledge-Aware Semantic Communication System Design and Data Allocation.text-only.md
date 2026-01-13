## Knowledge-Aware Semantic Communication System Design and Data Allocation

Sachin Kadam and Dong In Kim , Fellow, IEEE

Abstract -The recent emergence of 6G raises the challenge of increasing the transmission data rate even further in order to overcome the Shannon limit. Traditional communication methods fall short of the 6G goals, paving the way for Semantic Communication (SemCom) systems that have applications in the metaverse, healthcare, economics, etc. In SemCom systems, only the relevant keywords from the data are extracted and used for transmission. In this paper, we design an auto-encoder and auto-decoder that only transmit these keywords and, respectively, recover the data using the received keywords and the shared knowledge. This SemCom system is used in a setup in which the receiver allocates various categories of the same dataset collected from the transmitter, which differ in size and accuracy, to a number of users. This scenario is formulated using an optimization problem called the data allocation problem (DAP). We show that it is NP-complete and propose a greedy algorithm to solve it. Using simulations, we show that the proposed methods for SemCom system design outperform state-of-the-art methods in terms of average number of words per sentence for a given accuracy, and that the proposed greedy algorithm solution of the DAP performs significantly close to the optimal solution.

Index Terms -Semantic Communications, Knowledge Base, 6G, Data Allocation, Wireless Communications

## I. INTRODUCTION

As per the prediction in [2], semantic communication (SemCom) technology is identified as one of the key ingredients in 6G due to the requirement of low latency and high data rate transmissions. The recent emergence of SemCom technologies finds applications in wide range of fields such as economics [3], metaverse [4], autonomous transportation systems [5], healthcare [6], smart factories [7], and so on. In SemCom, we only transmit useful and necessary information to the recipients. The semantic extraction (SE) is a process wherein the useful and necessary features are extracted from the original raw data. For example, the essential speech features are extracted using an attention-based mechanism in [8]-[10], image features are extracted using ResNet-50 [11] in [12], etc.

In order to overcome the Shannon limit in 6G communication systems, the transmission data rate must be increased even further [13]-[15]. The Shannon channel capacity may be exceeded for a communication system that transmits semantically correct data but permits a non-zero bit error rate (BER). A formal proof for the same is provided in [16]. In a traditional communication system, the source information is transformed

A preliminary version of this paper is published in IEEE International Conference on Communications (ICC) 2023 [1].

S. Kadam and D. I. Kim are with the Department of Electrical and Computer Engineering, Sungkyunkwan University (SKKU), Suwon 16419, Republic of Korea (e-mail: sachinkadam@skku.edu, dikim@skku.ac.kr).

into bit sequences for processing. The bit sequence corresponding to the source information is precisely decoded at the receiver. In a traditional communication system, the bit/symbol transmission rate is limited by Shannon capacity. Semantic communication systems convey the semantic meaning of the source information. One significant distinction is the addition of semantic coding, which captures semantic information based on tasks or actions to be performed by the receiver. Only those semantic characteristics will be communicated, considerably reducing the number of essential communication resources. At the receiver, operations such as data reconstruction or more sophisticated tasks like image recognition and language translation might be performed. Using a semantic encoder with minimal semantic ambiguity and a semantic decoder with strong inference capabilities and a large shared knowledge base, it is proved in [17] that there is a possibility to obtain higher transmission rates in semantic communication systems than those shown by Shannon's channel capacity theorem for traditional communication systems.

During critical applications such as military operations, search operations by forest personnel in a dense forest, medical emergencies in remote areas, fire incidents in a remote agricultural land, the release of water from a nearby dam, etc., only the essential information needs to be communicated on an urgent basis. The messages could be in the form of text or audio and they come from a limited dataset. In a non-critical application, such as broadcasting a text/audio summary of commentary provided by live football commentators. Among all the words spoken by them, only a limited set of useful or important words are relevant to the game. These words are drawn from a limited dataset such as football vocabulary [18] which includes words such as goal, player names, red card, football, score, assist, half-time , etc. This limited dataset provides an opportunity, in the context of SemCom design, for a significant overhead reduction by extracting and processing only the relevant keywords. For example, an uttered commentary sentence in 2022 FIFA world cup final game is: 'Messi shoots the ball into the right-bottom of the net and it's a goal!' The extracted keywords in this example are Messi, shoots, ball, right-bottom, net, goal . Only these keywords are transmitted in place of the entire sentence, and the receiver reconstructs a meaningful sentence. The reconstructed sentence in this case is: 'Messi shoots the ball into the right-bottom of the net to score a goal.' This sentence is not exactly the same as the original sentence, but it conveys the same meaning.

The first goal of this paper is to use SemCom technology to reduce communication overhead, in the context of natural language processing (NLP) problems, while maintaining a cer-

tain minimum accuracy in wireless communication systems. The overhead reduction is performed with high accuracy in the literature [19], [20]. However, in some applications, high data rates are preferred over high accuracy. For example, consider the football commentary transmission described in the preceding paragraph. Instead of sending the entire sentence, only the essential keywords are sent, and the entire sentence is reconstructed using the received keywords. The reconstructed sentence may not be completely accurate, but it conveys the same information as the original sentence with some accuracy. We were able to reduce overhead significantly while maintaining some accuracy. This reduction in overhead results in transmission of data with a faster rate, which is a key requirement for 6G. As a result, we present the results of the trade-off between overhead reduction and accuracy. Model parameters are chosen based on the context. Instead of transmitting raw data, the transmitter is designed to transmit semantic data, which significantly reduces network data traffic. A knowledge graph (KG) is a knowledge base that integrates data using a graph-structured topology. They are used to store interconnected event descriptions. These are used to predict the missing words in the received data (keywords) to construct a meaningful sentence.

Next, we apply the designed SemCom system to a realistic problem in which the transmitter and receiver are assumed to be a cloud server and a data center, respectively. A cloud server located on a remote cloud platform has access to a large raw dataset that is stored in the cloud. A data center is a centralized information storage facility that can store, process, and distribute massive amounts of data to its users [21]. A data center requests a portion of the raw dataset in various categories. These dataset categories are based on their size and accuracy levels. For example, in the case of live streaming of sports events, the cloud server provides different quality videos of the same content to the data center, such as 480 p, 720 p, 1080 p, 2160 p, and 4 K , and the users are served based on the subscribed video quality service and fee. The proposed SemCom technology-based communication system is used to transmit these datasets to the data center. Since the data center also has access to the shared KG, these datasets can be decoded with a certain accuracy. The data center replicates these different category datasets to store them in its storage facility to serve its subscribed users. Because storage capacity is limited, the data center's challenge is to find an optimal set of dataset replications to serve its subscribers. The cloud server determines the price of each category dataset based on its size and quality (in terms of accuracy).

In our case, the data center is assumed to store the different categories of the same portion of the dataset in its storage facility. The users can access these datasets directly from the storage facility by paying a certain price. The different-quality datasets are priced differently based on their quality and sizes. For example, a highly compressed dataset is small in size but poor in accuracy, so it is less expensive, whereas a lightly compressed dataset is large in size but superior in accuracy, so it is more expensive. Every user has a budget constraint, just as the data center has a storage constraint. 1 The data center replicates these datasets based on the needs and budgets of the users. It strives to provide the highest-quality dataset possible to every user with a sufficient budget. It is a highly desirable scenario for the following two reasons: (a) the data center can maximize profits, and (b) the user is extremely satisfied with the service and can provide a high rating as feedback. Hence, we formulate an optimization problem in which the data center attempts to maximize its profit given the constraint on its storage capacity in order to serve all the subscribed users. We denote this problem as the Data Allocation Problem (DAP).

However, challenges arise in data allocation due to resource constraints, such as data storage capacity at the data center and the need to serve all subscribed users. This difficulty is most noticeable when there are a large number of subscribers. In order to serve all subscribed users, the data center is forced to allocate lower-quality datasets to some of its subscribers, despite the fact that user feedback may damage its reputation and cost it revenue.

Now, we provide the main contributions of this paper, which are as follows:

- In this research work, we designed an auto-encoderequipped transmitter and an auto-decoder-equipped receiver that only transmit the relevant keywords and, respectively, retrieve the data based on the received keywords and shared knowledge.
- We defined a new metric called Semantic Score (SS) that combines the best of two separate quantities, the BLEU score [22], and sentence similarity that employs BERT [23], to quantify the overall semantic loss between the original and reconstructed sentences at the receiver.
- The performances of our proposed scheme are analytically compared to those of the two state-of-the-art schemes in terms of accuracy versus overhead reduction trade-off.
- The SemCom system is then implemented in a realistic scenario in which a cloud server and a data center serve as transmitter and receiver, respectively. We formulated the DAP in which the data center optimally distributes different types of datasets received from the cloud server to its subscribers. We proved that the DAP is an NPcomplete problem and proposed a greedy algorithm for solving it.

1 The assumption of data center's storage constraint is due to the following reasons: Data center storage, in general, refers to the devices, equipment, and software solutions that enable data storage within a data center facility. This includes data center storage policies and procedures that govern the entire data storage and retrieval process. Furthermore, data center storage may include data center storage security and access control techniques and methodologies. Data center components require substantial infrastructure to support such large hardware and software requirements. These include power subsystems, uninterruptible power supplies, ventilation, cooling systems, fire suppression, backup generators, and connections to external networks. To provide all of this necessary support, data centers typically have a limit on total data storage. Similarly, the constraints on the budget of its associated users are because each user has a limit on how much they can earn and, as a result, a limited budget that they can allocate to various items. Finally, limited income is the root cause of budget constraints. Budget constraints are visible in the fact that users cannot simply buy everything they want and are forced to choose between alternatives based on their preferences.

- We demonstrated using simulations on the real-world dataset that the proposed methods for SemCom system design outperform state-of-the-art methods in terms of average number of words per sentence for a given accuracy and that the proposed greedy algorithm solution of the DAP performs significantly close to the optimal solution.

The organization of the paper is as follows: A brief literature review of SemCom technologies and KGs is provided in Section II. We introduce our SemCom system model and problem formulation in Section III. The proposed SemCom system model is presented in detail in Section IV. The performance analysis of the proposed scheme in terms of accuracy versus overhead reduction trade-off and cost comparisons is provided in Section IV-B. Then we define our DAP, show that it is an NP-complete problem, and propose a greedy algorithm to solve it in Section V. We provide simulation results related to the proposed SemCom system model and the solutions of the DAP in Section VI. Finally, we conclude the paper along with a few future research directions in Section VII.

## II. RELATED WORK

The study on SemCom technologies started very recently. The following state-of-the-art survey papers provide in-depth discussions on various SemCom technologies and their applications [13]-[15], [24]-[26]. Deep learning based SemCom technologies are proposed in [19], [20]. A brief tutorial on the framework of SemCom and a method to calculate a bound on semantic data compression is provided in [27]. The SemCom technology wherein both transmitter and receiver are empowered with the capability of contextual reasoning is proposed in [28]. The SemCom technology for a system where transmitter and receiver speak different languages is designed in [29]. A multi-user task-oriented SemCom system for multi-modal data transmission is proposed in [30]. A nonlinear transform based source-channel coding approach for SemCom is proposed in [31], wherein a nonlinear transform mechanism is used to extract the source semantic features. A joint source-channel coding scheme is proposed in [32] that preserves the meaning between the transmitted sentence s and the recovered sentence ˆ s , while the two sentences may have different words and different lengths. The work in [33] introduced a reinforcement learning (RL)-powered SemCom paradigm that gives a system the ability to express semantics. In [34], a SemCom framework for textual data transmission is proposed. In this framework, semantic information is represented by a KG made up of a set of semantic triples and the receiver recovers the original text using a graph-to-text generation model. Another SemCom system based on the KG is proposed in [35]. In this system, transmitted sentences are converted into triplets using the KG, which are seen as fundamental semantic symbols for semantic extraction and restoration, and they are ordered based on semantic relevance. All of these works are focused on achieving an overhead reduction without compromising the accuracy of the received data. None of these works investigated the possibility of further overhead reduction, thereby improving transmission data rate, while sacrificing a little accuracy. This issue is addressed in this paper using a shared knowledge base.

A significant research on the usage of KGs is carried out in the field of natural language processing (NLP). The survey work in [36] provides a comprehensive study of KGs, which leverage large-scale data collections for usage in a variety of industry and academic applications. A survey paper based on KG is presented in [37]. Similarly, another survey paper on KG text generation is presented in [38]. A method to generate a summary of sentences by using a given set of keywords is proposed in [39]. Similarly, a method to generate a summary of sentences by using a knowledge base is shown in [40]. Recently, KGs are utilized in the context of SemCom design [34], [35], [41], [42]. But these works do not focus on the issue presented in this paper, which is to design a SemCom system with a significant overhead reduction with a little compromise on accuracy.

Problems similar to the DAP are well studied in the literature. A few similar problems studied are file allocation problems (FAP) in distributed systems [43], [44], data allocation in database systems (DADS) [45]-[47], data allocation over multiple channels at broadcast servers (DABS) [48], etc. The goal of FAP is to find the optimal way to allocate files in order to minimize the operating costs associated with the files while keeping the following constraints in mind: (a) the expected time to access each file is less than a given bound; and (b) the amount of storage required at each computer does not exceed the available storage capacity. The problem in DADS is similar to FAP but differs in the following aspects: First, the data objects to be allocated are unknown in advance; second, these data objects are accessible by schedules that comprise transmissions between these data objects to generate the result. The goal of the DABS is to optimally allocate data objects across the available channels in a broadcast system so that the overall cost (in terms of average expected delay) is minimized while ensuring that all nodes receive at least one copy of these data objects. However, the uniqueness of the DAP lies in its problem structure. The data objects are allocated in the DAP with the goal of maximizing the data center's profit while ensuring that the cost of each allocated data object does not exceed the user budget and that each user is assigned exactly one category of data object.

## III. SYSTEM MODEL AND PROBLEM FORMULATION

In this Section, first, we provide a brief overview of the proposed system model and later in Section III-B, we present our problem formulation. In Table I, we list the symbols and abbreviations used in this paper along with their meanings.

## A. System Model

The system model of the proposed SemCom system is shown in Fig. 1. Let X be the input text dataset with N sentences, X i be the i th , i ∈ { 1 , . . . , N } , sentence of X , and K be the shared knowledge base. First, we extract the keywords from X using K . Let the total set of keywords be Ω = ⋃ N i =1 Ω i , where Ω i denotes the set of keywords present in X i . The keyword extraction process is executed by multiplying the input sentence X i = [ ω iℓ , ℓ = 1 , . . . , L i ] , with a binary

Fig. 1: The block diagram of our proposed SemCom system model. The model in Fig. 1(a) is used for training the system parameters and the model in Fig. 1(b) is used for evaluating the SemCom system.

<!-- image -->

vector b i = [ b iℓ , ℓ = 1 , . . . , L i ] , where L i = | X i | , 2 which is defined as follows:

<!-- formula-not-decoded -->

Hence, Ω i , i ∈ { 1 , . . . , N } , is obtained by collecting the nonzero elements from X i ⊙ b i , where ⊙ is a word-wise multiplication operator. Here X i ⊙ b i ≜ [ ω iℓ b iℓ , ∀ ℓ = { 1 , . . . , L i } ] . 3 It is a simple search method, like checking whether the given English word is in the dictionary or not. For a faster and automatic keyword extraction process, refer to Rapid Automatic Keyword Extraction (RAKE) method [49]. It is an approach for extracting keywords from particular documents that is unsupervised, domain-independent, and language-independent.

Next, the i th keyword set Ω i is encoded using the autoencoder which consists of semantic and channel encoders. Let us denote S θ e and C ϕ e as the semantic and channel encoders with θ e and ϕ e as the parameter sets, respectively. After encoding Ω i , we get the following set of symbols:

<!-- formula-not-decoded -->

The encoded set of symbols ˜ Ω i is transmitted via the AWGN (additive white Gaussian noise) channel. Let h be the channel

2 |A| denotes the cardinality of set A .

3 For ease of understanding, let us consider the example discussed in Section I. Let X i be [Messi shoots the ball into the right-bottom of the net and it's a goal!]. If the set of keywords present in X i is { Messi, shoots, ball, right-bottom, net, goal } then b i = [11010010010001] . Now, X i ⊙ b i gives [ Messi, shoots, 0, ball, 0, 0, right-bottom, 0, 0, net, 0, 0, 0, goal ] . Next, Ω i is obtained by collecting the non-zero elements, i.e., Ω i = { Messi, shoots, ball, right-bottom, net, goal } .

gain and η be the noise which gets added to ˜ Ω i during transmission. So, the set of received symbols at the receiver is Ω i = h ˜ Ω i + η . After receiving, this set of symbols is decoded using the auto-decoder which consists of channel and semantic decoders. Let us denote C ϕ d and S θ d as the channel and semantic decoders with ϕ d and θ d as the parameter sets, respectively. After decoding Ω i , we get the following set of keywords:

<!-- formula-not-decoded -->

From the decoded set of keywords and the shared knowledge base K , the sentence generator at the receiver generates the sentence Y i ∈ Y , i ∈ { 1 , . . . , N } , where Y is the reconstructed text dataset.

## B. Problem Formulation

Given the limited size of knowledge base, though the accuracy of the reconstructed sentences in Y may not be sufficiently high, the useful content in those sentences is summarized and conveyed to the receiver. This novel approach saves a significant amount of overhead.

To measure the overall semantic loss between the original sentence s ∈ X and the reconstructed sentence ˆ s ∈ Y at the receiver, we define a new metric named Semantic Score (SS) which combines the best of two different quantities, viz., BLEU score (bilingual evaluation understudy [22]) and sentence similarity which uses BERT [23]. 4 Let ∆ λ ( s, ˆ s ) denote the SS between sentences s and ˆ s , which is a convex

4 The detailed explanation on SS is provided in Section III-B1.

TABLE I: List of mathematical symbols and abbreviations

| Symbol          | Meaning                                                                           |
|-----------------|-----------------------------------------------------------------------------------|
| SemCom          | Semantic Communication                                                            |
| DAP             | Data Allocation Problem                                                           |
| KG              | Knowledge Graph                                                                   |
| X               | Input text dataset                                                                |
| N               | Number of sentences in X                                                          |
| X i             | i th , i ∈ { 1 ,...,N } , sentence of X                                           |
| K               | Shared knowledge base                                                             |
| Ω i             | The set of keywords present in X i                                                |
| Ω               | Total set of keywords                                                             |
| S θ e           | Semantic encoder with θ e as the parameter set                                    |
| C ϕ e           | Channel encoder with ϕ e as the parameter set                                     |
| ˜ Ω             | The encoded set of symbols                                                        |
| h               | Channel gain                                                                      |
| AWGN            | Additive White Gaussian Noise                                                     |
| η               | AWGN noise                                                                        |
| Ω               | The set of received symbols at the receiver                                       |
| C ϕ d           | Channel decoder with ϕ d as the parameter set                                     |
| S θ d           | Semantic decoder with θ d as the parameter set                                    |
| ̂ Ω             | Decoded set of keywords                                                           |
| Y               | Set of sentences generated at the receiver                                        |
| Y               | Reconstructed text dataset                                                        |
| SS              | Semantic Score                                                                    |
| λ               | a hyper-parameter between 0 and 1                                                 |
| τ               | user defined accuracy parameter                                                   |
| BLEU            | Bilingual Evaluation Understudy                                                   |
| BERT            | Bidirectional Encoder Representations from Transformers                           |
| ∆ λ ( s, ˆ s )  | Semantic Score between sentences s and ˆ s                                        |
| BLEU ( s, ˆ s ) | BLEU score between sentences s and ˆ s                                            |
| Φ( s, ˆ s )     | sentence similarity score between sentences s and ˆ s                             |
| B               | Batch size                                                                        |
| L               | Total length of sentences                                                         |
| M               | Semantic information provided by Ω                                                |
| V               | Dimension of the encoder's output                                                 |
| ̂ M             | Recovered semantic information                                                    |
| G               | Number of data categories in data center                                          |
| z i             | Size of i th category data in data center                                         |
| c i             | Selling cost of of i th category data from data center                            |
| J               | Number of subscribed users                                                        |
| Z               | Total size constraint of data center                                              |
| b j             | Budget constraint of j th user                                                    |
| d ( z i )       | Purchase price of i th category data                                              |
| KP              | Knapsack Problem                                                                  |
| W               | Average number of words per sentence Deep learning enabled Semantic Communication |
| DeepSC JSCC     | Joint Source-Channel Coding                                                       |

combination of BLEU ( s, ˆ s ) 5 (see (6)) and Φ( s, ˆ s ) (see (8)), i.e.,

<!-- formula-not-decoded -->

where λ ∈ [0 , 1] is a parameter. Note that pure BLEU ( s, ˆ s ) and Φ( s, ˆ s ) are obtained by setting λ = 0 and λ = 1 , respectively.

There exists a trade-off between overhead reduction and the accuracy that depends on the size of the knowledge base K . For example, if the size of the set K is small, then on an average only a few keywords are extracted from the given input sentences in X , encoded and transmitted, which implies higher amount of average overhead reduction. This creates a large amount of missing information on an average, due to which accuracy of the reconstructed sentences in Y is expected to be low. On the other side, if the size of the set K is large, then on an average a significant number of keywords are extracted from the given sentences in X , encoded and transmitted, which

5 When explicitly not mentioned then BLEU 1-gram is used.

implies lower average overhead reduction. This creates a small amount of missing information on an average, due to which accuracy of the reconstructed sentences in Y is expected to be high. This phenomenon is numerically shown in Section VI-A.

So, in this paper we aim at minimizing the transmission of average number of words per sentence (equivalent to maximizing the average overhead reduction) by keeping a certain minimum accuracy information τ in the received sentence, i.e.,

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

where | Ω i | denotes the number of keywords in Ω i that corresponds to sentence X i , and τ is an user defined parameter.

1) Semantic Score (SS): Now, we describe the semantic score used in the problem formulation. An attempt was made earlier in [50] where features of BLEU score and sentence similarity score are integrated using a multiple linear regression model. It determines sentence lexical matches, lexical semantic similarity between non-matching words, and sentence lengths. Motivated by this work, we propose the idea of SS (see (4)). The BLEU score cannot handle word synonyms, but it is a fast, low-cost algorithm that is language-independent and corresponds with human judgment. The sentence similarity score using BERT vectors is slow, has comparable ratings to the BLEU, but it also handles synonyms. A brief description of the BLEU score and sentence similarity score is provided in the next two paragraphs.

First, let us define the quantity BLEU score (bilingual evaluation understudy [22]) to compare the similarities between two sentences quantitatively. The BLEU ( s, ˆ s ) ∈ [0 , 1] score between transmitted sentence s and reconstructed sentence ˆ s is computed as follows:

<!-- formula-not-decoded -->

where p n denotes the modified n -gram precision function up to length W , w n denotes the weights, and brevity penalty (BP) is given by the following expression:

<!-- formula-not-decoded -->

where ℓ c is the length of the candidate translation and ℓ r is the effective reference corpus length [22].

Next, sentence similarity score Φ( s, ˆ s ) is defined as follows:

<!-- formula-not-decoded -->

where β , representing BERT [23], is a massive pre-trained model, which uses word embeddings, with billions of parameters used to extract semantic information. The Φ( s, ˆ s ) is a number between 0 and 1, indicating how similar the reconstructed sentence is to the transmitted sentence, with 1 indicating the highest similarity and 0 indicating no similarity between s and ˆ s . Word embeddings are vectors that have been mapped to words to assist computers in interpreting language.

For example, the words cat or dog are difficult for a computer to understand, their vector form is more appropriate. One assumption of embedding mapping is that related words should be near each other. Note that word embedding vectors with contextualized embeddings have distinct embeddings for the same word depending on its context. Hence, these embeddings are proposed to be used as sentence-level embeddings.

2) Shared Knowledge Base: In this paper, we generate the shared knowledge base K by using the keywords from a limited dataset Ω which consists of only the relevant words of a particular event, like that of a football game in our case. We assume that both transmitter and receiver have access to K . During the feature extraction process, in every sentence, the words w ∈ Ω are encoded into their corresponding symbols and transmitted to the receiver in their corresponding time slots. At other time slots, a common symbol is transmitted. By utilizing K , the receiver reconstructs the sentence based on the received words in that sentence. To improve the accuracy of the reconstructed sentences, we can increase the size of K by adding more keywords from the vocabulary generated using X . This result is shown using simulations in Section VI-A.

## IV. PROPOSED SEMCOM SYSTEM MODEL

The detailed architecture of the semantic and channel encoder/decoder models is shown in Fig. 2. The transmitter comprises of a semantic encoder that extracts semantic characteristics from the texts to be broadcast and a channel encoder that generates symbols to assist further transmission. The conceptual encoder has many Transformer encoder layers [51], whereas the channel encoder has dense layers with various units. In the model, the AWGN channel is considered as one layer. As a result, the receiver is composed of a channel decoder for symbol identification and a semantic decoder for text estimation, with the channel decoder consisting of dense layers with varied units and the semantic decoder consisting of several Transformer decoder layers [51].

Let ϵ ≜ 1 -SS . During training, each step attempts to minimize ϵ using gradient descent with mini-batch until the stop condition is fulfilled, the maximum number of iterations is achieved, or none of the terms in the loss function are reduced anymore. Unlike separate semantic and channel coding, where the channel encoder/decoder deals with digital bits rather than semantic information, joint semantic-channel coding can maintain semantic information when compressing data [32].

## A. Training the System Model

The training of the SemCom model shown in Fig. 1(a) is carried out keeping the SS in mind. A mini-batch of sentences S ∈ R B × L , are converted to sets of keywords Ω ∈ R B × L , where B and L = ∑ B i =1 L i are batch size and total length of the sentences, respectively, is transmitted to the semantic encoder. These sentences can be represented as a dense word vector R ∈ R B × L × R which are obtained after passing through the embedding layer, where R is the dimension of the word vector. This word vector R is then passed to the Transformer encoder, primary component of Semantic Encoder, to acquire the semantic information M ∈ R B × L × V , provided by Ω , where V is the dimension of the encoder's output. Then, to account for the effects of the physical channel noise, M is encoded into symbols ˜ Ω , where ˜ Ω ∈ R B × NL × 2 , which constitutes the channel encoder which is implemented using dense layer followed by reshape layer. Next, the receiver receives distorted symbols Ω after travelling through the channel. The channel decoder layer, implemented using reshape layer followed by dense layer, decodes distorted symbols Ω received at the receiver, where ̂ M ∈ R B × L × V is the recovered semantic information of the sources. The semantic decoder layer then estimates the transmitted sentences ̂ Ω with the help of sentence generator and the shared knowledge K . Finally, the stochastic gradient descent (SGD) method is used to optimize the network, using the error ϵ .

## B. Performance Analysis

Now, we analytically compare the performances of our proposed scheme with those of the DeepSC scheme [19] and the adaptive scheme [29].

1) Accuracy versus Overhead Reduction Trade-off: Let X i , i = { 1 , . . . , N } , denote the i th sentence in the dataset X and ω iℓ , ℓ = { 1 , . . . , | X i |} , denote the ℓ th word in the sentence X i , then we can write,

<!-- formula-not-decoded -->

In our proposed scheme, as described in the system model (see Section III-A), we only transmit the extracted keywords before encoding. Hence, the total number of words present in the dataset N 0 and the total number of keywords to be transmitted N τ , respectively, are

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

where Ω i ( τ ) , i = { 1 , . . . , N } , is the set of keywords present in the sentence X i for a given accuracy τ . Let n 0 denote the fixed number of symbols used to represent a word in the DeepSC scheme [19]. So, the total number of symbols used for communicating the whole data in the DeepSC scheme, Ψ 0 , and the proposed scheme, Ψ τ , respectively, are as follows:

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Let p iℓ denote the probability of occurrence of the word ω iℓ , i.e.,

<!-- formula-not-decoded -->

Now, based on the value p iℓ , in the adaptive scheme the number of symbols used to encode the word ω iℓ is chosen using the following equation:

<!-- formula-not-decoded -->

Fig. 2: The architecture of the semantic encoder/decoder and channel encoder/decoder models of the proposed SemCom system model.

<!-- image -->

where n min &lt; n 0 denotes the minimum number of possible symbols. Hence, the total number of symbols used in the adaptive scheme is

<!-- formula-not-decoded -->

Let α d τ , α a τ , and α p τ represent the product of the average number of symbols used for each word and the fraction of total words transmitted required to achieve the accuracy τ in DeepSC [19], adaptive scheme [29], and proposed scheme, respectively. Since all words are transmitted in both DeepSC and adaptive schemes, the α τ values for these schemes are as follows:

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

where n 0 ( τ ) and ̂ Ψ( τ ) /N 0 are the average number of symbols required to achieve accuracy τ in DeepSC and adaptive schemes, respectively. Note that n 0 (1) = n 0 and ̂ Ψ(1) = ̂ Ψ . In case of the proposed scheme, only a fraction of all the words are transmitted. It uses the same number of symbols as that of the DeepSC scheme, that is n 0 ( τ ) , but it is still able to achieve better results due to the transmission of only the keywords. Hence, the value of α τ for the proposed scheme is

<!-- formula-not-decoded -->

The accuracy vs. overhead reduction trade-off can be compared among the proposed scheme, DeepSC, and adaptive scheme by measuring the α τ values obtained by each of these schemes. We numerically compare these values, and the results are shown in Fig. 7.

2) Cost Comparisons: Now, we analyse the average costs incurred for transmission of a sentence in various schemes. Let t ω (in µs ) be the time to check word ω whether it is a keyword or not from the knowledge base K . Let the cost equivalent of spending time t ω on such operation is c ( ω ) . So the average cost spent on keyword extraction process can be found as

<!-- formula-not-decoded -->

Next, we compute the costs involved in the transmission process. 6 Let ¯ c ( ξ ) denote the cost of spending resources for the transmission of symbol ξ . The total number of symbols used in the schemes DeepSC, proposed, and adaptive, respectively, can be computed by using (11a), (11b), and (14). So the average costs incurred for the proposed ( C p t ), DeepSC ( C d t ), and adaptive ( C a t ) schemes, respectively, in the transmission process are

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

In the proposed scheme, after recovering the set of keywords, the receiver has to reconstruct the sentences for the meaningful recovery. Let ˆ c ( x ) be the cost of reconstructing sentence X at the receiver. So the average cost spent on sentence reconstruction process can be found as

<!-- formula-not-decoded -->

The average costs incurred for end-to-end transmission of a sentence for DeepSC, adaptive, and proposed schemes are C d t , C a t , C p = C k + C p t + C r , respectively.

Remark 1. For communication systems with large computing and storage capabilities, pre-processing and post-processing operations like keyword extraction and sentence generation, respectively, incur a marginal cost. Also, in the case of poor channel conditions and highly congested networks, the amount of information (symbols) to be transmitted becomes a crucial factor for efficient data communication. In these scenarios, our proposed scheme outperforms DeepSC [19] and adaptive [29] schemes in terms of average costs incurred, i.e., C p &lt; C a t &lt; C d t .

6 These costs include transmission power, encoding and decoding processes, etc.

Fig. 3: This figure shows the setup used to describe the data allocation problem (DAP).

<!-- image -->

## V. DATA ALLOCATION PROBLEM

Let us assume that the transmitter and receiver shown in Fig. 1 are a cloud server and a data center, respectively. The original copy of the dataset X is stored in the cloud, and a data center requests a portion of it from the cloud server, say X ⊂ X . The cloud server uses SemCom technology to communicate the requested portion of data to the data center, as described in Section III-A. The data center obtains each copy of X τ , for some specific values of τ ∈ [0 , 1] , by tuning the parameter τ . It can only make a limited number of copies of these data due to storage constraints. This data center serves a number of users, each of whom has a budget constraint. Assume that the users do not have sufficient memory to store the data. They use data stored in the data center. Once data portions are allocated, users can access them whenever they need. Next, we formulate an optimization problem, which we call Data Allocation Problem (DAP), to maximize the profit for this data center given its storage constraint and the budget constraints of its associated users.

The setup used to describe the DAP is shown in Fig. 3. 7 Let G be the number of data categories that the data center has

7 Real-world examples of the setup shown in Fig. 3 are as follows: (a) Microsoft Azure provides storage facilities for the subscribed users in its data center [52]. The storage options include premium and standard quality options to optimize costs and workload performance. Microsoft Azure guarantees submillisecond latency in its storage facility for high throughput and transactionintensive workloads. The prices quoted for using the storage facility are determined by the users' location [53]. For example, prices per month of usage for users in the Korean Central Region are $0 . 81 , $1 . 62 , $3 . 24 , $5 . 28 , . . . , with storage sizes of 4 GB, 8 GB, 16 GB, 32 GB, . . . , respectively. (b) Amazon Web Services offers its subscribers an easy-to-use, scalable, highperformance block-storage service called Elastic Block Store (EBS) [54]. The quoted prices for using the EBS facility are decided by the user's location [55]. For example, for users in the Asia-Pacific (Seoul) region, the prices per month of usage are $0 . 37 , $0 . 72 , $1 . 46 , $2 . 92 , . . . , for data storage sizes of 4 GB, 8 GB, 16 GB, 32 GB, . . . , respectively.

based on the accuracy τ . They are indexed by i = 1 , . . . , G , such that τ ∈ [ τ min , τ max ] , τ min &lt; τ max , is one-to-one corresponding to i ∈ { 1 , . . . , G } . The data center can produce m i copies of X i , i = 1 , . . . , G . Each copy of data is z i in size, and its selling cost is c i and they are related by z i 1 &lt; z i 2 and c i 1 &lt; c i 2 for i 1 &lt; i 2 , ∀ i 1 ∈ { 1 , . . . , G -1 } , ∀ i 2 ∈ { 2 , . . . , G } . These constraints indicate that the sizes and costs of different categories of data increase as τ increases, which is shown using the indices i ∈ { 1 , . . . , G } . Now, we would carefully incorporate the results obtained from (5a) and (5b) in terms of size and accuracy, so as to make the DAP meaningful from the perspective of SemCom developed in this paper.

Assume the data center serves J users, with each user having a budget constraint b j ≥ c 1 , ∀ j ∈ { 1 , . . . , J } . 8 Let U i,j represent an indicator variable that returns 1 when b j ≥ c i , which means that the i th category data can be provided to user j when its cost is not more than the user budget and 0 otherwise, i.e., for a given i ∈ { 1 , . . . , G } , j ∈ { 1 , . . . , J } ,

<!-- formula-not-decoded -->

Similarly, let V i,j represent an indicator random variable that returns 1 when the i th category data is actually provided to user j and 0 otherwise, i.e.,

<!-- formula-not-decoded -->

So, the value of m i can be obtained as follows:

<!-- formula-not-decoded -->

The value of m i computed using (22) shows that it also denotes the number of users who are provided with i th , i ∈ { 1 , . . . , G } , category data.

Now we consider the purchase price of the data from the cloud server. The limited backhaul capacity between the cloud server and data center constrains the rate of data transfer between them. Due to this, the size of the data, and hence the use of SemCom technology, plays an important role. The purchase prices d ( z i ) , i ∈ { 1 , . . . , G } , of different categories of data from the cloud server are based on their sizes, i.e., d ( z i 1 ) &lt; d ( z i 2 ) for i 1 &lt; i 2 , ∀ i 1 ∈ { 1 , . . . , G -1 } , ∀ i 2 ∈ { 2 , . . . , G } . Based on this information, an optimization problem, which we call DAP, to maximize the profit of the data center is formulated as follows:

8 This constraint ensures that every user is eligible to access at least one category of data.

<!-- formula-not-decoded -->

The constraint (23b) ensures that the total size of all copies of allocated data is within the limit of the maximum permissible size Z at the data center. Similarly, the constraint (23c) indicates that all data portions are assigned to users while making sure that the cost of every allocated data portion is not more than the user budget (see (20)), whereas the constraint (23d) indicates that each user j ∈ { 1 , . . . , J } is allocated exactly one category of data. The last constraint (23e) indicates that the variables V i,j , ∀ i ∈{ 1 , . . . , G } , j ∈{ 1 , . . . , J } , are binary. This makes our optimization problem, DAP, defined in (23a)-(23e) as a type of binary integer programming.

In general, the integer programming problems are shown to be NP-complete [56]. We show that the DAP belongs to the class of NP-complete problems by reducing the well known knapsack problem (KP) [57] to it.

## Theorem 1. The DAP is NP-complete.

<!-- formula-not-decoded -->

## A. Greedy Algorithm

In Theorem 1, we have shown that the DAP belongs to the class of NP-complete problems. Now, we present a greedy algorithm to solve the DAP. First, we identify the condition under which the solution is feasible. From the DAP formulation, it is clear that there is a limit to the number of users that the data center can serve. In the worst-case scenario, all users could be assigned the least desirable data category, i = 1 . The total data size in this case is z 1 times J and is limited by Z . Hence the condition for the solution to exist is:

<!-- formula-not-decoded -->

Given that the primary goal of the DAP is to maximize profit for the data center while ensuring data allocations to all users, the proposed algorithm allocates the best possible category data to each user based on their budget. This is accomplished by determining k ( j ) ∈ { 1 , . . . , G } , ∀ j ∈ { 1 , . . . , J } , such that c k ( j ) ≤ b j &lt; c k ( j )+1 (which is same as finding i such that U i,j = 1 and U i +1 ,j = 0 ), and allocating the data category i = k ( j ) for j th user. This gives V i,j = V k ( j ) ,j = 1 , ∀ j ∈ { 1 , . . . , J } . Next, the algorithm computes the total size due to this allocation policy, i.e., Z = ∑ J j =1 z k ( j ) V k ( j ) ,j . If Z ≤ Z , then we have found the solution, V ⋆ , of the DAP and it is as follows:

<!-- formula-not-decoded -->

And the profit is P = ∑ G i =1 ( c i ∑ J j =1 V ⋆ i,j -d ( z i ) ) . But if Z &gt; Z , which implies the violation of the constraint (23b), then the algorithm updates the data allocation policy in the following way. It finds the smallest argument k ( j ′ ) which minimizes the ratio r k ( j ) = ( c k ( j ) /z k ( j ) ) , ∀ j ∈ { 1 , . . . , J } , and does the following updates using it: V k ( j ′ ) ,j ′ = 0 , V k ( j ′ ) -1 ,j ′ = 1 , Z → Z -z k ( j ′ ) + z k ( j ′ ) -1 , k ( j ′ ) → k ( j ′ ) -1 . 9 The algorithm again compares Z , computed with updated value of k ( j ′ ) , and Z . This process continues until it encounters Z ≤ Z and the solution is V ⋆ = V . The detailed algorithm is provided in Algorithm 1.

| 1:     | Input: c i , z i ,d ( z i ) ,U i,j , i ∈ { 1 ,...,G } , j ∈ { 1 , . . . ,J } , G,J,Z   |
|--------|----------------------------------------------------------------------------------------|
| 2:     | if J ≤ Z/z (1) then                                                                    |
| 3:     | Initialize j = 1 , r (1) = ∞ , i = 2 , V = 0 G × J .                                   |
| 4:     | while i ≤ G do                                                                         |
| 5:     | r i ← c i /z i , i ← i +1 .                                                            |
| 6:     | end while                                                                              |
| 7:     | while j ≤ J do                                                                         |
| 8:     | Find i such that U i,j = 1 and U i +1 ,j = 0 .                                         |
| 9: 10: | k ( j ) ← i , V k ( j ) ,j ← 1 , j ← j +1 . end while                                  |
| 11:    | Compute Z = ∑ J j =1 z k ( j ) (Note: V k ( j ) ,j = 1 , ∀ j ∈ { 1 , . . . ,J } ).     |
| 12:    | while (1) do                                                                           |
| 13:    | if Z ≤ Z then                                                                          |
| 14:    | End the algorithm and output V ⋆ = V .                                                 |
| 15:    | else                                                                                   |
| 16:    | Compute k ( j ′ ) = argmin k ( j ) ,j ∈{ 1 ,...,J } r k ( j ) .                        |
| 17:    | V k ( j ′ ) ,j ′ ← 0 , V k ( j ′ ) - 1 ,j ′ ← 1 ,                                      |
| 18:    | Z ← Z - z k ( j ′ ) + z k ( j ′ ) - 1 , k ( j ′ ) ← k ( j ′ ) - 1 .                    |
| 19:    | end if                                                                                 |
| 20:    | end while                                                                              |
| 21:    | else                                                                                   |
| 22:    | End the algorithm and display 'No feasible solution'.                                  |
| 23:    | end if Output: V ⋆ of size G × J , and the profit: P =                                 |
| 24:    | ∑ G i =1 ( c i ∑ J j =1 V ⋆ i,j - d ( z i ) ) .                                        |

9 This approach ensures the smallest possible reduction in selling cost from P to (P -c k ( j ′ ) + c k ( j ′ ) -1 ) and/or the largest possible data size reduction from Z to ( Z -z k ( j ′ ) + z k ( j ′ ) -1 ) , which aids in satisfying the constraint (23b). If only the selling cost is considered in place of the ratio, which is the case in most greedy algorithms, the algorithm ignores the impact of data sizes on the DAP. We call this algorithm as greedy-cost algorithm and show, using simulations, in Section VI-B that the proposed greedy algorithm outperforms the greedy-cost algorithm.

TABLE II: Simulation hyper-parameters

| Number of matches used in training   | 1580   |
|--------------------------------------|--------|
| Number of matches used in evaluation | 340    |
| Number of epochs during training     | 10     |
| SNR                                  | 6 dB   |
| λ                                    | 0.3    |
| Learning rate                        | 0.001  |
| Dropout rate                         | 0.1    |
| Batch Size                           | 64     |
| Channel                              | AWGN   |
| Standard Deviation σ in AWGN         | 0.02   |

TABLE III: Simulation settings for SemCom Encoder/Decoder layers

|             | Layer                                 | Units                 | Activation       |
|-------------|---------------------------------------|-----------------------|------------------|
| Transmitter | Transformer Encoder (3) Dense Reshape | 128 (8 heads) 256 16  | Linear Relu Relu |
| Channel     | AWGN                                  | None                  | None             |
| Receiver    | Reshape Dense Transformer Decoder (3) | 256 128 128 (8 heads) | Relu Relu Linear |

## B. Computational Complexity of the Greedy Algorithm

Now, we find the computational complexity of the proposed greedy algorithm, if the solution exists. First, we compute the values of r i , ∀ i ∈ { 1 , . . . , G } , using a loop described in lines 4-7 of Algorithm 1. This computation results in the time complexity of O ( G ) . Similarly, we find that the computational complexity of the loop described in lines 8-13 is O ( J ) . Next, in the loop described in lines 15-24, we compute the argument minimizer in line 19 whose computational complexity is O ( J ) , and this loop, in the worst case, executes till all k ( j ) , j ∈ { 1 , . . . , J } , become 1. This happens after O ( G ) times execution of the loop. Thus the computational complexity of the loop described in lines 15-24 is O ( GJ ) . Hence, the total computational complexity of the proposed greedy algorithm in Algorithm 1 is O ( G + J + GJ ) .

The computational complexity of finding the solution for the DAP using the brute-force search method is O (2 GJ ) , since it uses all the possible binary matrices of size G × J , sequentially, to compute the solution.

Remark 2. The proposed greedy algorithm is highly efficient in terms of the computational complexity w.r.t. the brute-force search method, i.e., O ( G + J + GJ ) &lt;&lt; O (2 GJ ) .

The comparison study of the numerical solutions of the DAP using the proposed greedy algorithm and Gurobi software [58] as a solver is shown in Section VI-B.

## VI. SIMULATION RESULTS

In this section, we first provide the simulation results related to the designed SemCom system in Section VI-A and then provide the results related to the DAP in Section VI-B.

## A. The performance of SemCom System Model

First, we evaluate the performance of the text data transmission in terms of accuracy using BLEU score [22]. 10 In our work, we use the dataset provided in [59]. We parse the

10 We defined the BLEU score in (6).

football commentary data of 1920 matches from the website goal.com. The considered football matches are from Union of European Football Associations (UEFA) Champions League, UEFA Europa League, and Premier League between 2016 and 2020. The simulations are performed in a computer with NVIDIA GeForce RTX 3090 GPU and Intel Core i9-10980XE CPU with 256GB RAM.

The simulation hyper-parameters used for plots in this section are shown in Table II. The simulation settings of the proposed SemCom system architecture (see Fig. 2) consist of three transformer encoder and decoder layers with eight heads each. The dense layers in the transmitter and receiver are 256 units and 128 units, respectively. Similarly, the reshape layers in the transmitter and receiver are 16 units and 256 units, respectively. The linear activation functions are used in the encoders and decoders, whereas Relu activation functions are used in the rest of the layers. These settings are also listed in Table III.

Let ρ be the fraction of the total vocabulary V , which contains all the dataset words, to be added to K . ρ = 0 indicates that no additional vocabulary is added and the system is evaluated only with the initial keyword set Ω 0 . Based on the way of adding the vocabulary words to Ω 0 , we propose two types of schemes. In the first type, ρ | V | vocabulary words are uniformly chosen at random from V and added to K . In the second type, the words in V are first arranged in the decreasing order of the frequency of appearances in the dataset, and then the first ρ | V | vocabulary words are added to K . We call these schemes as 'RANDOM' and 'ORDERED', respectively.

The accuracy performances of both the schemes and a deep learning based SemCom system method named DeepSC [19], in terms of BLEU score vs. ρ , are shown in Fig. 4. From the plot we can infer that even with ρ = 0 , the initial keyword set can produce a BLEU score of 0.55 (for 1-gram). This shows that the context-related keywords produce good results. Also, we see that as we add more vocabulary words to Ω 0 , the BLEU score increases. For the same value of ρ and n , the ORDERED scheme performs better than the RANDOM scheme because of the addition of high frequency words. And, in terms of different n -grams, BLEU score decreases as n increases, which is an expected result. In comparison to the DeepSC scheme, the proposed schemes perform poorly in terms of accuracy but outperform it in terms of overhead reduction, as shown below.

Next, we evaluate the performance of the proposed schemes, in terms of the transmission of average number of words per sentence, with respect to DeepSC [19] and the results are shown in Fig. 5a. Let W denote the average number of words per sentence. From the plot we observe that both the schemes outperform DeepSC. Among the proposed schemes, for a given ρ the RANDOM scheme outperforms the ORDERED scheme. This is because, in the ORDERED scheme high frequency words are added which increases the number of words to be encoded in the input data as compared with the RANDOM scheme.

Now, we solve the optimization problem presented in (5a) and (5b), with λ = 0 , using both the proposed schemes. For this purpose, we evaluate W vs. τ and the results are shown

Fig. 4: This plot shows the BLEU score vs. ρ for different values of n -grams, where n = { 1 , 2 , 3 , 4 } , for the proposed schemes and the DeepSC scheme [19].

<!-- image -->

Fig. 5: These plots show the average number of words per sentence vs. ρ in the left plot and vs. τ in the right plot, respectively, for the proposed schemes and the DeepSC scheme [19].

<!-- image -->

in Fig. 5b. From the plot we observe that both the schemes outperform DeepSC. Also, we see that the performance of both the schemes is same for a given accuracy threshold τ . This is because, as shown in Fig. 4, for a given value of ρ ∈ (0 , 1) , the ORDERED scheme outperforms the RANDOM scheme in terms of accuracy, whereas in Fig. 5a, the RANDOM scheme outperforms the ORDERED scheme in terms of overhead reduction. Hence, we can choose any one of the proposed methods to solve the optimization problem.

Now, we evaluate the performance of the proposed schemes, in terms of the semantic score (see (4)), with respect to DeepSC [19] and Joint Source-Channel Coding (JSCC) schemes [32], and the results are shown in Fig. 6a. The trends are similar to that of Fig. 4, and the proposed schemes outperform JSCC when ρ approaches 0.8. Next, we compare the performance of the proposed scheme (with ρ = 0 . 8 ) in terms of different SNR values and the results are shown in Fig. 6b. As expected, the semantic score increases as SNR increases due to a reduction in the noise effect for all the schemes but saturates for higher SNR values. When compared with DeepSC, the performance is slightly poor, but the proposed scheme follows the trends of DeepSC. But in comparison to JSCC, the proposed scheme outperforms it for all SNR values.

Now, we compare the performance of our scheme with those of the DeepSC [19] and adaptive [29] schemes. Recall from

Fig. 6: These plots show the semantic score (SS) vs. ρ in the left plot and vs. SNR (in dB) in the right plot, respectively, for the proposed schemes, DeepSC scheme [19], and JSCC [32].

<!-- image -->

Fig. 7: The plot in left shows the total number of symbols used in each of the schemes with respect to ρ . We use n min = 1 , n 0 = 4 . The plot in right shows the α τ values in each of the schemes with respect to the given accuracy τ .

<!-- image -->

Section IV-B that in the DeepSC and proposed schemes, an average n 0 number of symbols are used for every word during encoding, and the adaptive scheme proposed in [29] uses an adaptive method for choosing the number of symbols for every word that depends on the size of that word (see (13)). We show the comparisons among the proposed method and the schemes proposed in [19], [29] in terms of ̂ Ψ , Ψ 0 , and Ψ τ , in Fig. 7a. From this plot, we observe that the proposed scheme transmits a significantly smaller number of symbols compared with both schemes, from ρ = 0 to ρ = 0 . 6 . Next, we compare the performance of our scheme in terms of α d τ , α a τ , and α p τ vs. the accuracy parameter τ . The comparisons are shown in Fig. 7b. From this plot, we observe that the proposed scheme outperforms both schemes for accuracy levels up to 82% .

## B. Solutions of the Data Allocation Problem

Now, we present the simulation results related to the solution of the DAP defined in (23a)-(23e). We solve the DAP using three methods: Optimal , Greedy , and Greedy-cost . We refer to the solutions obtained by the Gurobi software [58] and the greedy algorithm (see Algorithm 1) as Optimal and Greedy, respectively. Similarly, the solution obtained by the algorithm, which is the same as that of the greedy algorithm, except that the argument minimizer minimizes the cost c i instead of the ratio c i /z i , ∀ i ∈ { 1 , . . . , G } , in line 19 of the proposed Algorithm 1 is called greedy-cost. The primary goal of using the greedy-cost algorithm is to demonstrate numerically that maximizing profit by greedily changing only the costs does not produce better results than the proposed greedy algorithm. In our simulations, we have assumed that the

Fig. 8: The plot in left shows the total profit gained using all three algorithms with respect to the number of subscribed users J . The maximum profit is observed for J = 2100 , and the profit computed by the proposed greedy algorithm (respectively, greedycost algorithm) at the same value of J is 90 . 54% (respectively, 77 . 76% ) of the optimal maximum profit. The plot in right shows the total profit gained using all three algorithms with respect to the discount factor. The average fall of the profit with each discount factor for the optimal, greedy, and greedy-cost algorithms is 0 . 653% , 0 . 666% , and 0 . 706% , respectively. This shows that the discount factor does not affect the profit significantly. Hence, there is a room to attract more subscribers without loosing the significant profit. We use J = 1500 in this case.

<!-- image -->

data center has purchased a standard persistent disk (PD) from Google cloud [60] and has a memory capacity of Z = 64 TB, minimum and maximum data sizes are 10GB and 100GB, respectively, and G = 20 . The costs and sizes are chosen uniformly at RANDOM from [0 , 1] and [10 , 100] , respectively, and the number of iterations in our simulations is 25.

First, we compute the total profit gained by the data center with respect to the number of users it serves, and the results obtained by all three methods are shown in Fig. 8a. From the plot, we can observe that for a set of a smaller number of users, in particular from J = 500 to J = 1200 in our case, the profit computed by all three methods is the same. This is because every method is successful in allocating the best possible category data to every user without violating the size constraint (23b), due to the small number of users. As the number of users increases the profit obtained by Optimal starts outperforming both greedy algorithms. The proposed greedy algorithm solution closely follows the optimal solution, but the greedy-cost algorithm solution starts moving away significantly from the optimal solution. This is because the greedycost algorithm only accounts for the cost maximization without bothering about the data sizes, which results in violation of the size constraint (23b) more often than the proposed greedy algorithm, which accounts for both the costs and the data sizes. The plot in Fig. 8a also shows that the profit increases initially for all three algorithms, then reaches its maximum value and begins to decrease again.

Next, we evaluate the performance of the algorithms in terms of profit gained with respect to the discount factor, which is the percentage discount given by the data center to its users in comparison to the purchase price of the data to attract more new subscribers. The results are shown in Fig. 8b. As expected, the optimal solution outperforms both greedy algorithms, and also, the proposed greedy algorithm outperforms the greedycost algorithm, for all discount factors. Also, as the amount of discount increases, the profit for a fixed number of users

Fig. 9: This plot shows the average rating given by users for the data center's service using each of the algorithms w.r.t. the number of users. For J = 2100 users, where profit is maximized, the average ratings are 3.27, 3.18, and 3.02 for services provided using the optimal, greedy, and greedy-cost algorithm solutions, respectively. This result demonstrates that the average rating provided for the optimal solution is not significantly higher compared to that of the proposed greedy algorithm solution.

<!-- image -->

reduces, which is also along expected lines.

Now, we evaluate the users' satisfaction with the service provided by the data center by using their ratings. The ratings are provided by users using one of the numbers between 1 and 5, where 1 and 5 signify the worst and best user experiences, respectively. Let ¯ i ( j ) , ˜ i ( j ) ∈ { 1 , . . . , G } , be the quantities such that U ¯ i ( j ) ,j = 1 and U ¯ i ( j )+1 ,j = 0 , and V ⋆ ˜ i ( j ) ,j = 1 , j ∈ { 1 , . . . , J } . Let SL( j ) denote the satisfaction level of the users j ∈ { 1 , . . . , J } . We define satisfaction level as SL( j ) = ¯ i ( j ) -˜ i ( j ) , j ∈ { 1 , . . . , J } . For the purpose of evaluation, we assume that the ratings and satisfaction levels are related as shown in Table IV. 11 The plot in Fig. 9 shows the average rating provided by the subscribed users for the services provided by all three algorithms. From the plot we observe that all users provide the rating of 5 to the service when number of subscribers is low, i.e., 500 ≤ J ≤ 1100 in our example. This is due to the lower number of subscribers, which resulted in the best possible category of data allocation based on each subscriber's budget. However, as J increases beyond 1100 , every algorithm starts allocating lower level categories of data to some of the subscribers so that the size constraint (23b) is not violated. Hence, there is a fall in the average rating.

The histograms of the ratings provided by the users are shown in Fig. 10. These plots support the observation that when J is small, the size constraint (23b) is easily satisfied,

11 The relation between satisfaction level of user j and user j th ratings is studied in different contexts, such as video streaming [61], multimedia communications [62], information retrieval [63], audio transmission [64], speech transmission [65], m-Commerce [66], etc. The satisfaction level measures quality of service (QoS) provided by the data center, whereas user ratings measure quality of experience (QoE) experienced by the users. The values of satisfaction levels, scaled to 0-20, and their corresponding user ratings, scaled to 1-5, shown in Table III, are compatible with those of the work presented in [67]. A user gives a rating of 5 for a service in which the data center provides the same quality content that the user expects. Then the user tends to provide poor ratings as the data center provides poor-quality products.

Fig. 10: These plots show the histogram of the ratings provided by the users for each of the algorithms. The left figure shows the results for a small number of users, J = 1200 , whereas the right figure shows the results for a large, J = 4000 , number of users.

| Satisfaction Level   |   0 |   1-2 |   3-5 |   6-10 |   11-20 |
|----------------------|-----|-------|-------|--------|---------|
| User Rating          |   5 |     4 |     3 |      2 |       1 |

TABLE IV: Relation between the satisfaction levels and user ratings

<!-- image -->

and thus the best possible category of data is allocated to each user. Conversely, when J is large, to satisfy the size constraint (23b) algorithms tend to allocate a lower category of data to users, resulting in lower ratings.

## VII. CONCLUSIONS AND FUTURE WORK

In this paper, we first extracted relevant keywords from the dataset using the shared knowledge base. Then, using the received keywords and the shared knowledge, we designed an auto-encoder and auto-decoder that only transmit these keywords and, respectively, recover the data. We provided analytical comparisons of the proposed scheme versus the DeepSC [19] and the adaptive [29] schemes in terms of accuracy vs. overhead reduction trade-off and cost comparisons. We computed the accuracy of the reconstructed sentences at the receiver quantitatively. We demonstrated through simulations that the proposed methods outperform a state-of-theart method in terms of the average number of words per sentence. The designed SemCom system is then applied to a realistic scenario in which a cloud server and a data center serve as transmitter and receiver, respectively. We formulated a data allocation problem (DAP), in which the data center optimally allocates various categories of datasets received from the cloud server to its subscribers. We proved that the DAP belongs to a class of NP-complete problems and proposed a greedy algorithm for solving it. Furthermore, we have numerically demonstrated that the solutions of the proposed greedy algorithm, in terms of profits, are 90% of the optimal solutions.

In this paper, we focused solely on the text dataset; however, similar SemCom system design approaches can be proposed in the future for other types of datasets such as images, audio, and video. Also, a real-time DAP can be formulated by considering the dynamic storage facility using cache memories at the data center in place of a static storage facility. Another direction for future research is to address the problem of the dynamic arrival and departure of subscribers for a service at the data center and analyze how the data center handles it. Finally, another open problem is to design an approximation algorithm with a provable approximation ratio for the DAP.

## APPENDIX A PROOF OF THEOREM 1

The decision version of the DAP is as follows: 'Given a number L , does there exist a binary matrix V , of size G × J , that satisfy the constraints (23b)-(23d) such that ∑ G i =1 ( c i ∑ J j =1 V i,j -d ( z i ) ) ≥ L '? Given V , we can check in polynomial time whether it satisfies (23b)-(23d) and whether ∑ G i =1 ( c i ∑ J j =1 V i,j -d ( z i ) ) ≥ L . Thus the DAP is in class NP [68]. By using (22), we simplify the expressions (23a) and (23b) as following:

<!-- formula-not-decoded -->

We now show that the DAP is NP-complete by reducing the knapsack problem (KP), which has been shown to be NPcomplete [68], to it.

Let us consider the following KP: We want to pack n different types of items in a knapsack which can withstand a maximum weight of W . Each item of type i ∈ { 1 , . . . , n } is categorised with two parameters: a weight w i and a value v i . The goal of the KP is to find a set of items that produce the maximum possible value, with the restriction that the total weight of the set should not exceed W . It is also written as follows:

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

where x i , i ∈ { 1 , . . . , n } , denote the number of items of type i . The decision version of the KP is as follows: 'Given a number L , is it possible to achieve ∑ n i =1 v i x i ≥ L without exceeding the total weight constraint W '? Let us denote the decision version inequalities of both the problems as following:

<!-- formula-not-decoded -->

<!-- formula-not-decoded -->

Now, we show that the KP is polynomial-time reducible to DAP, i.e., KP &lt; p DAP. Consider the instance of the KP stated in the previous paragraph. From this instance, we construct the following instance of the DAP. In this instance, for every i ∈ { 1 , . . . , G } , the selling costs are equivalent to the values, i.e., c i = v i ; data sizes are equivalent to the weights, i.e., z i = w i ; number of users with data category i is equivalent to the number of items of type i , i.e., m i = x i ; and the constants Z = W and G = n . In this instance, we assume that every user is allocated only one category of data that satisfies the constraint (23d). Also, we assume that the budget of every user

is higher than the maximum possible selling cost of all categories of data, i.e., b j ≥ c G , ∀ j ∈ { 1 , . . . , J } . This assumption leads to U i,j = 1 , ∀ i ∈ { 1 , . . . , G } , j ∈ { 1 , . . . , J } (see (20)). Now, we see that the constraint (23c), i.e., ∑ G i =1 ∑ J j =1 V i,j = ∑ J j =1 ∑ G i =1 V i,j = ∑ J j =1 1 = J is automatically satisfied due to (23d), i.e., ∑ G i =1 V i,j = 1 , ∀ j ∈ { 1 , . . . , J } .

Given this instance, we ask: does there exist a binary matrix V that satisfy the constraints (23b)-(23d) and D 1 ? We claim that the answer is yes if and only if the answer to the question in the KP instance is yes. The necessity part is proved as follows. If the answer to the above question is yes then there exists a binary matrix V that satisfy the constraints (23b)(23d), (22), and D 1 . From the assumptions we made in the previous paragraph, i.e., for every i ∈ { 1 , . . . , G } , c i = v i , z i = w i , m i = x i , Z = W and G = n , we see that the constraint (27b) and D 2 are satisfied since (26b) and D 1 are satisfied, respectively. This proves necessity.

To prove sufficiency, suppose the answer to the question in the KP is yes. Now, let us assume that for every i ∈ { 1 , . . . , n } , v i = c i , w i = z i , x i = m i = ∑ J j =1 V i,j , W = Z , and n = G . Now we see that the constraint (23b) and D 1 are satisfied since (27b) and D 2 are satisfied, respectively. The constraints (23c) and (23d) are satisfied due to the construction of the instance in the DAP. This proves sufficiency and the result follows.

## REFERENCES

- [1] S. Kadam and D. I. Kim, 'Knowledge-Aware Semantic Communication System Design,' in ICC 2023 -IEEE International Conference on Communications , pp. 6102-6107, 2023.
- [2] N. Rajatheva, I. Atzeni, E. Bj¨ ornson, A. Bourdoux, S. Buzzi, J.-B. Dor´ e, S. Erkucuk, M. Fuentes, K. Guan, Y. Hu, et al. , 'White paper on broadband connectivity in 6G,' 6G Research Visions , vol. 10, 2020.
- [3] Z. Q. Liew, H. Du, W. Y. B. Lim, Z. Xiong, D. Niyato, C. Miao, and D. I. Kim, 'Economics of Semantic Communication System: An Auction Approach,' arXiv preprint arXiv:2208.05040 , 2022.
- [4] L. Ismail, D. Niyato, S. Sun, D. I. Kim, M. Erol-Kantarci, and C. Miao, 'Semantic Information Market For The Metaverse: An Auction Based Approach,' arXiv preprint arXiv:2204.04878 , 2022.
- [5] W. Yang, Z. Q. Liew, W. Y. B. Lim, Z. Xiong, D. Niyato, X. Chi, X. Cao, and K. B. Letaief, 'Semantic communication meets edge intelligence,' arXiv preprint arXiv:2202.06471 , 2022.
- [6] S. Ghodratnama, M. Zakershahrak, and A. Beheshti, 'Summary2vec: Learning semantic representation of summaries for healthcare analytics,' in 2021 International Joint Conference on Neural Networks (IJCNN) , pp. 1-8, IEEE, 2021.
- [7] X. Luo, H.-H. Chen, and Q. Guo, 'Semantic communications: Overview, open issues, and future research directions,' IEEE Wireless Communications , 2022.
- [8] Z. Weng and Z. Qin, 'Semantic communication systems for speech transmission,' IEEE Journal on Selected Areas in Communications , vol. 39, no. 8, pp. 2434-2444, 2021.
- [9] Z. Weng, Z. Qin, and G. Y. Li, 'Semantic communications for speech signals,' in ICC 2021-IEEE International Conference on Communications , pp. 1-6, IEEE, 2021.
- [10] H. Tong, Z. Yang, S. Wang, Y. Hu, W. Saad, and C. Yin, 'Federated learning based audio semantic communication over wireless networks,' in 2021 IEEE Global Communications Conference (GLOBECOM) , pp. 1-6, IEEE, 2021.
- [11] K. He, X. Zhang, S. Ren, and J. Sun, 'Deep residual learning for image recognition,' in Proceedings of the IEEE conference on computer vision and pattern recognition , pp. 770-778, 2016.
- [12] Y. Zhang, W. Xu, H. Gao, and F. Wang, 'Multi-user semantic communications for cooperative object identification,' in 2022 IEEE International Conference on Communications Workshops (ICC Workshops) , pp. 157162, IEEE, 2022.
- [13] W. Yang, H. Du, Z. Q. Liew, W. Y. B. Lim, Z. Xiong, D. Niyato, X. Chi, X. S. Shen, and C. Miao, 'Semantic Communications for Future Internet: Fundamentals, Applications, and Challenges,' IEEE Communications Surveys &amp; Tutorials , 2022.
- [14] Z. Qin, X. Tao, J. Lu, and G. Y. Li, 'Semantic communications: Principles and challenges,' arXiv preprint arXiv:2201.01389 , 2021.
- [15] E. C. Strinati and S. Barbarossa, '6G networks: Beyond Shannon towards semantic and goal-oriented communications,' Computer Networks , vol. 190, p. 107930, 2021.
- [16] S. Ma, Y. Wu, H. Qi, H. Li, G. Shi, Y. Liang, and N. Al-Dhahir, 'A Theory for Semantic Communications,' arXiv preprint arXiv:2303.05181 , 2023.
- [17] J. Bao, P. Basu, M. Dean, C. Partridge, A. Swami, W. Leland, and J. A. Hendler, 'Towards a Theory of Semantic Communication,' in 2011 IEEE Network Science Workshop , pp. 110-117, IEEE, 2011.
- [18] 'Football/ soccer english vocabulary, https://www.vocabulary.cl/english/football-soccer.htm.'
- [19] H. Xie, Z. Qin, G. Y. Li, and B.-H. Juang, 'Deep learning enabled semantic communication systems,' IEEE Transactions on Signal Processing , vol. 69, pp. 2663-2675, 2021.
- [20] H. Xie and Z. Qin, 'A lite distributed semantic communication system for internet of things,' IEEE Journal on Selected Areas in Communications , vol. 39, no. 1, pp. 142-153, 2020.
- [21] 'Top 11 BEST Data Center Companies, Datacenter Services in 2022, https://www.softwaretestinghelp.com/data-center-companies/,' Last Updated: December 5, 2022.
- [22] K. Papineni, S. Roukos, T. Ward, and W.-J. Zhu, 'BLEU: A method for automatic evaluation of machine translation,' in Proceedings of the 40th annual meeting of the Association for Computational Linguistics , pp. 311-318, 2002.
- [23] J. Devlin, M.-W. Chang, K. Lee, and K. Toutanova, 'BERT: Pre-training of Deep Bidirectional Transformers for Language Understanding,' in Proceedings of the 2019 Conference of the North American Chapter of the Association for Computational Linguistics: Human Language Technologies, Volume 1 (Long and Short Papers) , pp. 4171-4186, 2019.
- [24] Q. Lan, D. Wen, Z. Zhang, Q. Zeng, X. Chen, P. Popovski, and K. Huang, 'What is semantic communication? A view on conveying meaning in the era of machine intelligence,' Journal of Communications and Information Networks , vol. 6, no. 4, pp. 336-371, 2021.
- [25] D. G¨ und¨ uz, Z. Qin, I. E. Aguerri, H. S. Dhillon, Z. Yang, A. Yener, K. K. Wong, and C.-B. Chae, 'Beyond Transmitting Bits: Context, Semantics, and Task-Oriented Communications,' IEEE Journal on Selected Areas in Communications , vol. 41, no. 1, pp. 5-41, 2023.
- [26] C. Chaccour, W. Saad, M. Debbah, Z. Han, and H. V. Poor, 'Less data, more knowledge: Building next generation semantic communication networks,' arXiv preprint arXiv:2211.14343 , 2022.
- [27] K. Niu, J. Dai, S. Yao, S. Wang, Z. Si, X. Qin, and P. Zhang, 'Towards Semantic Communications: A Paradigm Shift,' arXiv preprint arXiv:2203.06692 , 2022.
- [28] H. Seo, J. Park, M. Bennis, and M. Debbah, 'Semantics-native communication with contextual reasoning,' arXiv preprint arXiv:2108.05681 , 2021.
- [29] M. Sana and E. C. Strinati, 'Learning semantics: An opportunity for effective 6G communications,' in 2022 IEEE 19th Annual Consumer Communications &amp; Networking Conference (CCNC) , pp. 631-636, IEEE, 2022.
- [30] H. Xie, Z. Qin, and G. Y. Li, 'Task-oriented multi-user semantic communications for VQA,' IEEE Wireless Communications Letters , vol. 11, no. 3, pp. 553-557, 2021.
- [31] J. Dai, S. Wang, K. Tan, Z. Si, X. Qin, K. Niu, and P. Zhang, 'Nonlinear Transform Source-Channel Coding for Semantic Communications,' IEEE Journal on Selected Areas in Communications , vol. 40, no. 8, pp. 2300-2316, 2022.
- [32] N. Farsad, M. Rao, and A. Goldsmith, 'Deep Learning for Joint SourceChannel Coding of Text,' in 2018 IEEE international conference on acoustics, speech and signal processing (ICASSP) , pp. 2326-2330, IEEE, 2018.
- [33] K. Lu, Q. Zhou, R. Li, Z. Zhao, X. Chen, J. Wu, and H. Zhang, 'Rethinking modern communication from semantic coding to semantic communication,' IEEE Wireless Communications , 2022.
- [34] Y. Wang, M. Chen, T. Luo, W. Saad, D. Niyato, H. V. Poor, and S. Cui, 'Performance optimization for semantic communications: An attentionbased reinforcement learning approach,' IEEE Journal on Selected Areas in Communications , vol. 40, no. 9, pp. 2598-2613, 2022.
- [35] S. Jiang, Y. Liu, Y. Zhang, P. Luo, K. Cao, J. Xiong, H. Zhao, and J. Wei, 'Reliable semantic communication system enabled by knowledge graph,' Entropy , vol. 24, no. 6, p. 846, 2022.

- [36] A. Hogan, E. Blomqvist, M. Cochez, C. d'Amato, G. d. Melo, C. Gutierrez, S. Kirrane, J. E. L. Gayo, R. Navigli, S. Neumaier, et al. , 'Knowledge Graphs,' ACM Computing Surveys (CSUR) , vol. 54, no. 4, pp. 137, 2021.
- [37] S. Ji, S. Pan, E. Cambria, P. Marttinen, and S. Y. Philip, 'A survey on knowledge graphs: Representation, acquisition, and applications,' IEEE Transactions on Neural Networks and Learning Systems , vol. 33, no. 2, pp. 494-514, 2021.
- [38] W. Yu, C. Zhu, Z. Li, Z. Hu, Q. Wang, H. Ji, and M. Jiang, 'A survey of knowledge-enhanced text generation,' ACM Computing Surveys (CSUR) , 2022.
- [39] H. Li, J. Zhu, J. Zhang, C. Zong, and X. He, 'Keywords-guided abstractive sentence summarization,' Proceedings of the AAAI conference on artificial intelligence , vol. 34, no. 05, pp. 8196-8203, 2020.
- [40] L. Huang, L. Wu, and L. Wang, 'Knowledge Graph-Augmented Abstractive Summarization with Semantic-Driven Cloze Reward,' in Proceedings of the 58th Annual Meeting of the Association for Computational Linguistics , pp. 5094-5107, 2020.
- [41] F. Zhou, Y. Li, X. Zhang, Q. Wu, X. Lei, and R. Q. Hu, 'Cognitive semantic communication systems driven by knowledge graph,' arXiv preprint arXiv:2202.11958 , 2022.
- [42] J. Liang, Y. Xiao, Y. Li, G. Shi, and M. Bennis, 'Life-long learning for reasoning-based semantic communication,' arXiv preprint arXiv:2202.01952 , 2022.
- [43] K. M. Chandy and J. Hewes, 'File Allocation in Distributed Systems,' in Proceedings of the 1976 ACM SIGMETRICS conference on Computer performance modeling measurement and evaluation , pp. 10-13, 1976.
- [44] W. W. Chu, 'Optimal File Allocation in a Multiple Computer System,' IEEE Transactions on Computers , vol. 100, no. 10, pp. 885-889, 1969.
- [45] P. M. Apers, 'Data Allocation in Distributed Database Systems ,' ACM Transactions on Database Systems (TODS) , vol. 13, no. 3, pp. 263-304, 1988.
- [46] Y.-K. Kwok, K. Karlapalem, I. Ahmad, and N. M. Pun, 'Design and Evaluation of Data Allocation Algorithms for Distributed Multimedia Database Systems,' IEEE Journal on Selected Areas in Communications , vol. 14, no. 7, pp. 1332-1348, 1996.
- [47] R. Karimi Adl and S. M. T. Rouhani Rankoohi, 'A New Ant Colony Optimization based Algorithm for Data Allocation Problem in Distributed Databases,' Knowledge and Information Systems , vol. 20, pp. 349-373, 2009.
- [48] W. G. Yee, S. B. Navathe, E. Omiecinski, and C. Jermaine, 'Efficient Data Allocation over Multiple Channels at Broadcast Servers,' IEEE Transactions on Computers , vol. 51, no. 10, pp. 1231-1236, 2002.
- [49] S. Rose, D. Engel, N. Cramer, and W. Cowley, 'Automatic keyword extraction from individual documents,' Text mining: applications and theory , pp. 1-20, 2010.
- [50] N. Malandrakis, E. Iosif, and A. Potamianos, 'DeepPurple: Estimating Sentence Semantic Similarity using N-gram Regression Models and Web Snippets,' in SEM 2012: The First Joint Conference on Lexical and Computational Semantics-Volume 1: Proceedings of the main conference and the shared task, and Volume 2: Proceedings of the Sixth International Workshop on Semantic Evaluation , pp. 565-570, 2012.
- [51] A. Vaswani, N. Shazeer, N. Parmar, J. Uszkoreit, L. Jones, A. N. Gomez, Ł. Kaiser, and I. Polosukhin, 'Attention Is All You Need,' Advances in neural information processing systems , vol. 30, 2017.
- [52] 'Microsoft Azure Disk Storage, https://azure.microsoft.com/enin/products/storage/disks/.'
- [53] 'Microsoft Azure Managed Disks Pricing, https://azure.microsoft.com/en-in/pricing/details/managed-disks/.'
- [54] 'Amazon Elastic Block Store, https://aws.amazon.com/ebs/?nc2=h ql prod st ebs.'
- [55] 'Amazon Elastic Block Store Pricing, https://aws.amazon.com/ebs/pricing/?did=ap card&amp;trk=ap card.'
- [56] R. M. Karp, 'Reducibility among combinatorial problems,' in Complexity of computer computations , pp. 85-103, Springer, 1972.
- [57] S. Martello and P. Toth, Knapsack Problems: Algorithms and Computer Implementations . John Wiley &amp; Sons, Inc., 1990.
- [58] Gurobi, 'Gurobi Optimization Solver,' 2022. https://www.gurobi.com/.
- [59] R. Zhang and C. Eickhoff, 'SOCCER: An Information-Sparse Discourse State Tracking Collection in the Sports Commentary Domain,' in Proceedings of the 2021 Conference of the North American Chapter of the Association for Computational Linguistics: Human Language Technologies , pp. 4325-4333, 2021.
- [60] 'Data center storage options, https://cloud.google.com/compute/docs/disks.'
- [61] S. Karim, H. He, A. A. Laghari, and H. Madiha, 'Quality of Service (QoS): Measurements of Video Streaming,' International Journal of Computer Science Issues (IJCSI) , vol. 16, no. 6, pp. 1-9, 2019.
- [62] G. Ghinea and J. P. Thomas, 'An Approach Towards Mapping Quality of Perception to Quality of Service in Multimedia Communications,' in 1999 IEEE Third Workshop on Multimedia Signal Processing (Cat. No. 99TH8451) , pp. 497-501, IEEE, 1999.
- [63] A. Al-Maskari and M. Sanderson, 'A Review of Factors Influencing User Satisfaction in Information Retrieval,' Journal of the American Society for Information Science and Technology , vol. 61, no. 5, pp. 859868, 2010.
- [64] M. Wilson and A. Sasse, 'Investigating the Impact of Audio Degradations on Users: Subjective vs Objective Assessment Methods,' Objective Assessment Methods in Proceedings of OZCHI , 2000.
- [65] A. Watson and M. A. Sasse, 'The Good, the Bad, and the Muffled: the Impact of Different Degradations on Internet Speech,' in Proceedings of the eighth ACM international conference on Multimedia , pp. 269-276, 2000.
- [66] G. Ghinea and M. C. Angelides, 'A User Perspective of Quality of Service in m-Commerce,' Multimedia Tools and Applications , vol. 22, pp. 187-206, 2004.
- [67] J. Kawalek, 'A User Perspective for QoS Management,' in Proceedings of the QoS Workshop aligned with the 3rd International Conference on Intelligence in Broadband Services and Network (IS&amp;N 95), Crete, Greece , vol. 16, 1995.
- [68] J. Kleinberg and E. Tardos, Algorithm Design . Pearson Education India, 2006.

<!-- image -->

Sachin Kadam received the B.Eng. degree in electronics and communication engineering from the People's Education Society Institute of Technology, Bengaluru, India, in 2007, the M.Tech. degree in electrical engineering from the Indian Institute of Technology (IIT) Kanpur, India, in 2012, and the Ph.D. degree from IIT Bombay, India, in 2020. He is currently a Postdoctoral Researcher with Prof. Dong In Kim with Sungkyunkwan University, Suwon-si, Gyeonggi-do, Republic of Korea. His research interests include the design and analysis of wireless and

M2M networks, semantic communications, differential privacy, and learning. He was a recipient of Scholarship Foundation for Excellence, California, USA, during the B.Eng. degree.

<!-- image -->

Dong In Kim (Fellow, IEEE) received the Ph.D. degree in electrical engineering from the University of Southern California, Los Angeles, CA, USA, in 1990. He was a Tenured Professor with the School of Engineering Science, Simon Fraser University, Burnaby, BC, Canada. He is currently a Distinguished Professor with the College of Information and Communication Engineering, Sungkyunkwan University, Suwon, South Korea. He is a Fellow of the Korean Academy of Science and Technology and a Member of the National Academy of Engineering of Korea. He was the first recipient of the NRF of Korea Engineering Research Center in Wireless Communications for RF Energy Harvesting from 2014 to 2021. He received several research awards, including the 2023 IEEE ComSoc Best Survey Paper Award and the 2022 IEEE Best Land Transportation Paper Award. He was selected the 2019 recipient of the IEEE ComSoc Joseph LoCicero Award for Exemplary Service to Publications. He was the General Chair of the IEEE ICC 2022, Seoul. Since 2001, he has been serving as an Editor, an Editor at Large, and an Area Editor of Wireless Communications I for IEEE Transactions on Communications. From 2002 to 2011, he served as an Editor and a Founding Area Editor of Cross-Layer Design and Optimization for IEEE Transactions on Wireless Communications. From 2008 to 2011, he served as the Co-Editor- in-Chief for the IEEE/KICS Journal of Communications and Networks. He served as the Founding Editorin-Chief for the IEEE Wireless Communications Letters from 2012 to 2015. He has been listed as a 2020/2022 Highly Cited Researcher by Clarivate Analytics.