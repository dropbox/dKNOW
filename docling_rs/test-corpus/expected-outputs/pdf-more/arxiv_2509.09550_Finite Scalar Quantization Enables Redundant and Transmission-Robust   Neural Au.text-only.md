## FINITE SCALAR QUANTIZATION ENABLES REDUNDANT AND TRANSMISSION-ROBUST NEURAL AUDIO COMPRESSION AT LOW BIT-RATES

Harry Julian, Rachel Beeson, Lohith Konathala, Johanna Ulin, Jiameng Gao

## Neuphonic

of quantized residuals; mechanisms to model the hierarchical nature of RVQ codes commonly rely on two separate transformers that operate globally and locally [8].

Finite Scalar Quantization (FSQ) [9], a method that uses a simple fixed-grid for partitioning the codebook, constructs a single codebook by quantizing each output vector dimension, treating each dimension as an implicit codebook, rather than quantizing an entire latent vector as a whole. Using FSQ results in almost complete codebook utilization, requires no auxiliary losses to train and affords simpler downstream architectures due to the usage of a single codebook, rather than multiple recursively dependent codes needing to be predicted per timestep.

Through experimentation with our codec, NeuCodec, we show that FSQ-based codecs also exhibit an additional perturbation robustness property in their code sequences. First, we introduce NeuCodec, our FSQ-based codec model. Second, via an encoder distillation experiment with NeuCodec, we show that two encoders can learn to encode the same audio in very different code sequences given a fixed quantizer and decoder, yet the sequence can be reconstructed to a similar perceptual fidelity from both sequences; analyzing the differences between the representations suggests the learned encoding is localized and has redundancy baked-in. Third, via a perturbation experiment where we simulate transmission of codes from various FSQ and RVQ codecs through a noisy channel, we show that FSQ-based codecs exhibit better performance under reasonably large levels of perturbation. We offer explanations for this phenomenon and speculate on future applications of FSQ-based codecs in light of this property.

## 2. BACKGROUND

RVQdiscretizes an embedding space through first performing Vector Quantization [10] over a finite codebook, after which discretization errors (e.g. the distance between the scalar vector and the nearest neighbor codeword embedding) are obtained and discretized again, a process that continues for a predetermined number of codebooks. This means that scalar embeddings can be accurately represented through a hierarchical sequence of discretized tokens, all contained within a finite vocabulary.

FSQ creates discretized tokens from a continuous scalar space by projecting the latent space of the encoder space down to a much lower dimension and quantizing each dimension in the space to a number of scalar levels. Tokens are then obtained by enumerating through the discretized levels in each dimension. The encoder output is projected into a space where each dimension 'd' is bounded between [ -1 , 1] and then discretized to one of n equidistant values. The codebook size C is given by Eq. 1.

## ABSTRACT

Neural Audio Codecs (NACs) have become increasingly adopted in speech processing tasks due to their excellent rate-distortion performance and compatibility with Large Language Models (LLMs) as discrete feature representations for audio generation. While most existing codecs rely on Residual Vector Quantization (RVQ), Finite Scalar Quantization (FSQ) has recently emerged as a compelling alternative that simplifies training and natively supports single codebooks. We introduce NeuCodec, an FSQ-based NAC, and show that FSQ encodes baked-in redundancy which produces an encoding which is robust when transmitted through noisy channels. First, through an encoder distillation experiment, we show that two different encoders can learn to encode identical audio into vastly different code sequences whilst maintaining comparable reconstruction quality with the same quantizer and decoder. Second, we demonstrate that FSQ has vastly superior bit-level perturbation robustness by comparing the performance of RVQ and FSQ codecs when simulating the transmission of code sequences through a noisy channel.

Index Terms -Audio Compression, Neural Compression, Neural Audio Codec, Residual Vector Quantization, Finite Scalar Quantization.

## 1. INTRODUCTION

Recently, Neural Audio Codecs (NACs) have gained widespread usage in speech processing, due to their ability to compress speech into ultra-low bitrate discrete code sequences whilst maintaining high perceptual quality when reconstructing these sequences back into waveforms [1].

The autoencoding task used to train NACs embeds a compressed latent representation of speech features into discrete sequences of codes, which are useful for training autoregressive transformers to complete downstream audio tasks such as Text-to-Speech (TTS) [2], Automatic Speech Recognition (ASR) [3] and Full Duplex Speech Modeling [4]; they can also be used as a domain-specific tokenized vocabulary that Large Language Models (LLMs) can be adapted to use for audio generation [5].

Conventionally, the most widely used NACs have utilized Residual Vector Quantization (RVQ) [6], where at each encoder output timestep, the encoded feature representation is quantized by a toplevel 'coarse' codebook, and additional codebooks quantize the residual error from each prior quantization operation. Although effective, RVQ presents training challenges, as propagating gradients to the codeword vectors to align them with the unquantized encoder outputs necessitates the use of auxiliary loss functions. This creates a delicate optimization problem that often leads to codebook collapse [7] where only a subset of codewords is used. Additionally, RVQ also requires a comparatively complicated downstream modeling setup, as the sequence length is expanded by the number

C

=

d

∏

i

=1

Importantly, this implies that the output of the encoder is projected and quantized into a vector that can be mapped to a discrete set of values. Therefore, codebooks of the same size with the same n values for each dimension will result in the same partitioning of the bounded quantization space. As a decoder operates on this quantization space, it means that two encoders that learn a similar partitioning of the fixed quantization space could utilize the same decoders without retraining.

For the experiment in Section 5 we make comparisons between our codecs and other NACs. For RVQ, we use Encodec [11] and Descript Audio Codec (DAC) [7], both of which are mainly composed of convolutions. For FSQ, we use our own models as well as Stable Codec [12], a large transformer based codec with 1B parameters.

## 3. NEUCODEC

NeuCodec is primarily based on XCodec2 [5], an ultra-low bitrate audio codec designed for downstream modeling in LLM-based TTS. The encoder takes raw waveforms as an input and consists of a pretrained frozen semantic encoder and a trainable acoustic encoder. The semantic encoder is Wav2Vec2-BERT-large [13] which was pretrained on 4.5 million hours of unsupervised speech. The acoustic encoder is derived from the encoder of BigCodec [14] which is a stack of Residual CNNs with Snake activation functions [15]. The discrete bottleneck of the codec is an FSQ module with a projection dimension of 8 and a codebook size of 2 16 . The decoder is a standard transformer decoder, which is used to directly predict magnitude and phase for a Vocos [16] head that generates a waveform.

The base model was trained for 800k steps following the approach of XCodec2 on one 8xH100 node with an effective batch size of 96 across GPUs. During training, each batch item is randomly cut into a 6 second segment (or padded if shorter). Training data are described in Table 1. The datasets used were selected as they are licensed for commercial usage (in contrast to the original XCodec2). Evaluation of all trained models is presented in Table 2.

Additionally, we froze the weights of the encoder and quantizer and trained a new 24kHz upsampling decoder by increasing the hoplength from 320 to 480 to enable 16kHz to 24kHz upsampling. The model was trained for 200k steps using the same compute configuration. A 24khz subset of the data was used to train the upsampling decoder.

Table 1 . NeuCodec Training Data Sources.

| Dataset                  | Hours   | Subset   | License   |
|--------------------------|---------|----------|-----------|
| Emilia-YODAS [17]        | 110,000 | 16       | CC-BY     |
| MLS [18]                 | 45,000  | 16       | CC-BY     |
| LibriTTS-R [19]          | 585     | 16/24    | CC-BY     |
| Fleurs-R [20]            | 692     | 16/24    | CC-BY     |
| Common Voice Subset [21] | 9,283   | 16       | CC0       |
| HUI [22]                 | 326     | 16/24    | CC0       |
| Proprietary              | 1000    | 16/24    | -         |
| Total 16kHz              | 166,930 |          |           |
| Total 24kHz              | 2,603   |          |           |

n

i

(1)

## 4. ENCODER DISTILLATION

XCodec2 was originally designed as a feature representation for TTS. Its asymmetric configuration of an encoder that largely outsizes the decoder in parameter size and compute complexity, enables a trade-off of enhanced compression performance and slow encoding speeds with fast decoding speeds at inference time. As a trained TTS model is decode-heavy, this offsets most of the computation to training time where code sequences need to be generated beforehand. We set out to distill NeuCodec for low latency usage in encode-heavy paradigms (e.g. ASR).

## 4.1. Training

We modify the encoder architecture of NeuCodec, whilst mirroring the joint semantic and acoustic encoder paradigm used in the original model. We swap the BigCodec acoustic encoder with the L3AC Encoder [23] (60% of the original size) and swap Wav2VecBERT2.0 with DistillHubert [24] (4% of the original size). Although the change in parameter count for the acoustic encoder is modest, it is mainly motivated by the fact that the BigCodec encoder has an abnormally high ratio of Multiply-Accumulate Operations (MACs) to parameters due to its deep stacking of convolutions [25].

Fig. 1 . Distillation Training. Blocks labelled with snowflakes are frozen during training, with the remaining non-loss blocks being trained.

<!-- image -->

For distillation training, we add the distillation loss in Eq. 2 to push the encoded representations of the teacher and student encoders to be more similar:

<!-- formula-not-decoded -->

where h represents the pre-quantization encoder outputs of each respective encoder. This approach requires the output shapes of the student encoder to be the same as those of the teacher.

The distillation loss is added to the original loss function used to train XCodec2, which is comprised of a multi-resolution melspectrogram loss [11] an average of the losses predicted by the Spectrogram Discriminator [12] and the HiFiGAN multi-period discriminator [26], a discriminator feature matching loss and an L2 semantic reconstruction loss to make the final loss eq 3:

<!-- formula-not-decoded -->

In training, the weights of the FSQ bottleneck and decoder are frozen. We use the 16kHz decoder to train the model, as it allows for use of a far larger pool of data. We train the student model for 400k steps on a single 8xH100 node with an effective batch size of 192, using the same dataset as NeuCodec. The distillation loss was activated after 20k steps, as activation at the beginning of training led the model to diverge due to large initial magnitudes of the loss.

Performance is evaluated using a subset of CMU-Arctic [27], where 100 utterances were randomly selected from each of the 18 speaker's data. The results are presented in Table 2 with a parameter breakdown of each model. Performance is measured via the Word-Error-Rate (WER) and Character-Error-Rate (CER) with transcriptions from whisper-large-v3 [28], in addition to ShortTerm Objective Intelligibility (STOI) [29], Perceptual Evaluation of Speech Quality (PESQ) [30] and Real-Time-Factor of the Encoder (encRTF).

The evaluation shows that there is a limited difference in performance between the encoders when autoencoding; NeuCodec with the 16kHz decoder performs slightly better in terms of WER and CER, whereas the distilled model performs slightly better in both STOI and PESQ, which could possibly be attributed to its much larger batch-size during training. Note, the distilled encoder is also 6x faster and 15x smaller than the original encoder.

## 4.2. Code-Level Analysis

Given the similar performance between the encoders, we investigate how similar their encodings are using intermediate outputs from the performance comparison. Comparing code sequences for each utterance element-wise, only 2% of the codes match between sequences, while Mean Cosine Similarity between quantizer output projections is 0 . 73 . In the implicit codebook, element-wise accuracy was 53% between sequences. A subset of codebook confusion matrices are presented in Figure 2; these show that there is reasonable incorrect classification, though notably this is primarily between neighboring levels in each implicit codebook. As 93% of level predictions are either correct or within a single level of the correct code, it appears that a shift by a single level is permissible for decent reconstruction and that there could be some locality to the representation in the implicit codebooks. These results show that even without learning a higher degree of code or implicit codebook correspondence, comparable reconstruction performance can still be achieved as output projections remain similar.

## 5. BIT-LEVEL PERTURBATION EXPERIMENT

When signals are transmitted through a medium, the data that is sent may be different from what is received due to signal interference or noise. How catastrophic the perturbation of a single bit is to the received signal depends on the encoding of said signal. The code indices of our quantized encoder outputs can be viewed as a bit-level digitally encoded signal, e.g. if each codebook is of size 1024 ( 2 10 ) each code index can be represented as 10 bits.

Our analysis of the output codes of our separately trained encoders shows a high level of code-level disagreement, meaning their respective bit-strings will differ substantially whilst the reconstructions will remain perceptually similar. Because there appears to be local redundancy between neighboring codes, a single perturbation in a bit-string would merely shift the code to a neighboring, perceptually similar point in the quantization space. This suggests that such perturbations would result in only limited signal degradation.

To study the robustness of the encoded sequences from both RVQ and FSQ models, we simulate the transmission of code sequences through a binary symmetric channel, where each code se-

Fig. 2 . A subset of implicit codebook confusion matrices between Original and Distilled Encoder level predictions. Q i refers to the index of the implicit codebook in the quantization vector.

<!-- image -->

quence is converted into a bit-string and each bit is transmitted incorrectly with a probability P flip . For each codec, we encode all of Librispeech test-clean [31]. Whilst encoding the data, we transform the integer values that correspond to individual codes in each sequence into bits using the maximum size of the codebook. Individual integer bit-strings are then concatenated into a single flat sequence, and bits are randomly perturbed (by flipping the binary value) at a given probability P flip across a range of values { 0.001, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5 } . We then map the bit-string back to integers, reshape the flat sequence of integers into the shape of the original code sequence, and then reconstruct it.

Multiple RVQ and FSQ-based codecs are compared, as described in Table 3. StableCodec uses a modified formulation of FSQ that enables arbitrary post-hoc FSQ bottlenecks to be applied to the model; we apply a bottleneck to make the codebook a power of 2, where the quantizer levels are set to { 8, 8, 8, 8, 4, 4 } with a 2 16 codebook size equivalent to NeuCodec that nicely fits into the bit-flipping paradigm.

Performance is measured via four metrics: STOI, PESQ, Scale-Invariant Signal-to-Distortion Ratio (SI-SDR) [32] and MelSpectrogram Mean Squared Error between original and generated spectrograms.

As shown in Fig. 3, FSQ-based codecs maintain relatively stable performance under increasing perturbations, whereas RVQ codecs experience a sharp decline once more than 1% of bits are altered. Notably, the STOI scores for all FSQ codecs remain high for a longer range of perturbations, indicating that - although speech quality degrades - the intelligibility remains relatively robust, even with up to 10% of bits altered in NeuCodec.

## 6. DISCUSSION

As shown in our distillation experiment, when encoder outputs and code sequences change, the reconstruction quality can remain the

Table 2 . Encoder/decoder parameter breakdown and performance comparison on CMU-Arctic subset.

| Encoder   | Decoder   |   Acoustic (M) |   Semantic (M) |   Total (M) |   WER(%) |   CER (%) |   STOI |   PESQ |   encRTF |
|-----------|-----------|----------------|----------------|-------------|----------|-----------|--------|--------|----------|
| NeuCodec  | 16kHz     |             35 |            600 |         635 |      2.3 |       0.9 |   0.9  |   2.06 |    0.018 |
| NeuCodec  | 24kHz     |             35 |            600 |         635 |      2.6 |       1.1 |   0.9  |   2.04 |    0.018 |
| Distilled | 16kHz     |             21 |             21 |          42 |      2.8 |       1.2 |   0.91 |   2.11 |    0.003 |
| Distilled | 24kHz     |             21 |             21 |          42 |      2.8 |       1.4 |   0.91 |   2.12 |    0.003 |

Table 3 . Compared FSQ and RVQ Models.

| Model            |   Rate (kHz) | Quantizer   |   Codebooks |
|------------------|--------------|-------------|-------------|
| NeuCodec         |           24 | FSQ         |           1 |
| Distill-NeuCodec |           24 | FSQ         |           1 |
| StableCodec [12] |           16 | FSQ         |           1 |
| DAC [7]          |           24 | RVQ         |           6 |
| Encodec [11]     |           24 | RVQ         |          12 |

same while using same decoder. Our analysis indicates that both (1) the encoding has baked-in redundancy and (2) codes that point to different local regions in the space index similar acoustic features. Since FSQ encourages the encoder to distribute information across all codewords, as long as the codebook is large enough, redundancy becomes a feature of FSQ, as a redundant representation will be created as information spreads into all codewords regardless of the actual dimensionality of the data. Additionally, even when intentionally perturbing the code sequences, the reconstruction quality remains high compared to RVQ codecs. With FSQ, perturbations in the code indices will result in predictable size changes in embedding space. In contrast, other methods of vector quantization impose no such constraints, hence perturbing their code indices can result in arbitrarily-sized changes in the embedding space. These aspects of FSQ result in a robust method of quantization with inherent redundancy and locality in representation space.

## 7. CONCLUSION

In conclusion, we found that FSQ biases NACs to learn discrete audio encodings that have in-built redundancy and a code-level perturbation robustness that could be advantageous for designing futre low bit-rate neural compressors that are resilient to noise in transmission. Future work should assess (1) the usefulness of this property in low-latency FSQ codecs aimed at widespread deployment in transmission use-cases and (2) if the formulation of FSQ can be altered to either improve robustness properties further or to allow for direct controllability of the extent of redundancy.

## 8. REFERENCES

- [1] Yiwei Guo et al., 'Recent advances in discrete speech tokens: A review,' arXiv preprint arXiv:2502.06490 , 2025.
- [2] Dan Lyth and Simon King, 'Natural language guidance of high-fidelity text-to-speech with synthetic annotations,' arXiv preprint arXiv:2402.01912 , 2024.
- [3] Kunal Dhawan et al, 'Codec-asr: Training performant automatic speech recognition systems with discrete speech representations,' in Interspeech 2024 , 2024.
- [4] Alexandre D´ efossez et al., 'Moshi: a speech-text foundation model for real-time dialogue,' arXiv preprint arXiv:2410.00037 , 2024.
- [5] Zhen Ye et al., 'Llasa: Scaling train-time and inference-time compute for llama-based speech synthesis,' arXiv preprint arXiv:2502.04128 , 2025.
- [6] Neil Zeghidour, Alejandro Luebs, Ahmed Omran, Jan Skoglund, and Marco Tagliasacchi, 'Soundstream: An end-toend neural audio codec,' IEEE/ACM Transactions on Audio, Speech, and Language Processing , vol. 30, pp. 495-507, 2022.
- [7] Rithesh Kumar, Prem Seetharaman, Alejandro Luebs, Ishaan Kumar, and Kundan Kumar, 'High-fidelity audio compression with improved rvqgan,' in Advances in Neural Information Processing Systems , 2023, vol. 36.
- [8] Sanyuan Chen et al., 'Neural codec language models are zeroshot text to speech synthesizers,' IEEE Transactions on Audio, Speech and Language Processing , vol. 33, pp. 705-718, 2025.
- [9] Fabian Mentzer, David Minnen, Eirikur Agustsson, and Michael Tschannen, 'Finite scalar quantization: VQ-VAE made simple,' in The Twelfth International Conference on Learning Representations , 2024.
- [10] Aaron van den Oord, Oriol Vinyals, and koray kavukcuoglu, 'Neural discrete representation learning,' in Advances in Neural Information Processing Systems , 2017, vol. 30.
- [11] Alexandre D´ efossez, Jade Copet, Gabriel Synnaeve, and Yossi Adi, 'High fidelity neural audio compression,' Transactions on Machine Learning Research , 2023.
- [12] Julian D Parker, Anton Smirnov, Jordi Pons, CJ Carr, Zack Zukowski, Zach Evans, and Xubo Liu, 'Scaling transformers for low-bitrate high-quality speech coding,' in The Thirteenth International Conference on Learning Representations , 2025.
- [13] Seamless Communication et al., 'Seamless: Multilingual expressive and streaming speech translation,' arXiv preprint arXiv:2313.05187 , 2023.
- [14] Detai Xin, Xu Tan, Shinnosuke Takamichi, and Hiroshi Saruwatari, 'Bigcodec: Pushing the limits of low-bitrate neural speech codec,' arXiv preprint arXiv:2409.05377 , 2024.
- [15] Liu Ziyin, Tilman Hartwig, and Masahito Ueda, 'Neural networks fail to learn periodic functions and how to fix it,' in Advances in Neural Information Processing Systems , 2020, vol. 33.
- [16] Hubert Siuzdak, 'Vocos: Closing the gap between timedomain and fourier-based neural vocoders for high-quality audio synthesis,' in The Twelfth International Conference on Learning Representations , 2024.
- [17] Haorui He et al., 'Emilia: An extensive, multilingual, and diverse speech dataset for large-scale speech generation,' in 2024 IEEE Spoken Language Technology Workshop (SLT) , 2024, pp. 885-890.
- [18] Vineel Pratap, Qiantong Xu, Anuroop Sriram, Gabriel Synnaeve, and Ronan Collobert, 'Mls: A large-scale multilingual dataset for speech research,' in Interspeech 2020 , 2020.
- [19] Yuma Koizumi et al., 'Libritts-r: A restored multi-speaker text-to-speech corpus,' in Interspeech 2023 , 2023, pp. 54965500.
- [20] Min Ma et al., 'FLEURS-R: A Restored Multilingual Speech Corpus for Generation Tasks,' in Interspeech 2024 , 2024, pp. 1835-1839.
- [21] Rosana et al. Ardila, 'Common voice: A massivelymultilingual speech corpus,' in Proceedings of the Twelfth Language Resources and Evaluation Conference , Marseille, France, May 2020, pp. 4218-4222, European Language Resources Association.
- [22] Pascal Puchtler, Johannes Wirth, and Ren´ e Peinl, 'Hui-audiocorpus-german: A high quality tts dataset,' in KI 2021: Advances in Artificial Intelligence , Cham, 2021, pp. 204-216, Springer International Publishing.
- [23] Linwei Zhai, Han Ding, Cui Zhao, fei wang, Ge Wang, Wang Zhi, and Wei Xi, 'L3ac: Towards a lightweight and lossless audio codec,' arXiv preprint arXiv:2504.04949 , 2025.
- [24] Heng-Jui Chang, Shu-wen Yang, and Hung-yi Lee, 'Distilhubert: Speech representation learning by layer-wise distillation of hidden-unit bert,' in ICASSP 2022 - 2022 IEEE International Conference on Acoustics, Speech and Signal Processing (ICASSP) , 2022, pp. 7087-7091.
- [25] Haibin Wu, Naoyuki Kanda, Sefik Emre Eskimez, and Jinyu Li, 'TS3-Codec: Transformer-Based Simple Streaming Single Codec,' in Interspeech 2025 , 2025, pp. 604-608.
- [26] Jungil Kong, Jaehyeon Kim, and Jaekyoung Bae, 'Hifi-gan: Generative adversarial networks for efficient and high fidelity speech synthesis,' in Advances in Neural Information Processing Systems , 2020, vol. 33, pp. 17022-17033.
- [27] John Kominek and Alan W. Black, 'CMU ARCTIC databases for speech synthesis,' Technical Report CMU-LTI-03-177, Language Technologies Institute, Carnegie Mellon University, 2003.
- [28] Alec Radford, Jong Wook Kim, Tao Xu, Greg Brockman, Christine Mcleavey, and Ilya Sutskever, 'Robust speech recognition via large-scale weak supervision,' 2023, vol. 202 of Proceedings of Machine Learning Research , pp. 28492-28518.
- [29] Cees H. Taal, Richard C. Hendriks, Richard Heusdens, and Jesper Jensen, 'A short-time objective intelligibility measure for time-frequency weighted noisy speech,' in Proceedings of the IEEE International Conference on Acoustics, Speech, and Signal Processing (ICASSP) , 2010, pp. 4214-4217.
- [30] Antony W. Rix, John G. Beerends, Martin P. Hollier, and Andries P. Hekstra, 'Perceptual evaluation of speech quality (PESQ)-a new method for speech quality assessment of telephone networks and codecs,' Proceedings of the IEEE International Conference on Acoustics, Speech, and Signal Processing (ICASSP) , vol. 2, pp. 749-752, 2001.
- [31] Vassil Panayotov, Guoguo Chen, Daniel Povey, and Sanjeev Khudanpur, 'Librispeech: An asr corpus based on public domain audio books,' in 2015 IEEE International Conference on Acoustics, Speech and Signal Processing (ICASSP) , 2015, pp. 5206-5210.
- [32] Jonathan Le Roux, Scott Wisdom, Hakan Erdogan, and John R. Hershey, 'Sdr - half-baked or well done?,' in ICASSP 2019 2019 IEEE International Conference on Acoustics, Speech and Signal Processing (ICASSP) , 2019, pp. 626-630.

Fig. 3 . Perturbation Robustness Across conditions for all NACs.

<!-- image -->