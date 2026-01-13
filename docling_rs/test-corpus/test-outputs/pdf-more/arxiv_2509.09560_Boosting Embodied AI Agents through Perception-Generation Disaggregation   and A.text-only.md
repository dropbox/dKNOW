## Boosting Embodied AI Agents through Perception-Generation Disaggregation and Asynchronous Pipeline Execution

Shulai Zhang

Shanghai Jiao Tong University

Han Zhao

Shanghai Jiao Tong University

Haibin Lin Bytedance

## Abstract

Embodied AI systems operate in dynamic environments, requiring seamless integration of perception and generation modules to process high-frequency input and output demands. Traditional sequential computation patterns, while effective in ensuring accuracy, face significant limitations in achieving the necessary "thinking" frequency for real-world applications. In this work, we present Auras, an algorithmsystem co-designed inference framework to optimize the inference frequency of embodied AI agents. Auras disaggregates the perception and generation and provides controlled pipeline parallelism for them to achieve high and stable throughput. Faced with the data staleness problem that appears when the parallelism is increased, Auras establishes a public context for perception and generation to share, thereby promising the accuracy of embodied agents. Experimental results show that Auras improves throughput by 2 . 54 × on average while achieving 102 . 7% of the original accuracy, demonstrating its efficacy in overcoming the constraints of sequential computation and providing high throughput.

## 1 Introduction

Embodied Artificial Intelligence (AI) represents a rapidly evolving field at the intersection of intelligent manufacturing, autonomous driving, and robotics. Unlike LLM-based chatbots [13, 19, 23, 51] that operate solely in virtual environments, embodied AI agents interact with the physical world, integrating perception, decision-making, and action in real-time [8, 15, 35, 74, 86]. The ability to 'think' and act in real-time is a fundamental requirement for these applications. The cutting-edge capabilities of embodied AI are predominantly driven by generative AI algorithms. Generative models, such as LLMs [42, 52, 63, 68] and diffusionbased algorithms [15, 27, 49, 67], have emerged as critical components for their superior reasoning abilities. A defining characteristic of generative AI algorithms is their iterative

Ao Xu

Shanghai Jiao Tong University

Weihao Cui

Shanghai Jiao Tong University

Xin Liu

Bytedance

Quan Chen Shanghai Jiao Tong University

Ningxin Zheng Bytedance

Minyi Guo

Shanghai Jiao Tong University

<!-- image -->

(a) The 'thinking' process of embodied AI agents

(b) Comparison between sequential and concurrent execution

Figure 1. The general workflow of an embodied AI agent.

nature [27, 69], where multiple rounds of generation are employed to achieve high-quality outputs.

Figure 1(a) illustrates the general workflow of current embodied AI systems. These systems process human language prompts and continuous images as multi-modal inputs, utilize generative algorithms for 'thinking', and output actions for embodied backends, such as robotic actuators. Specifically, each image captured by the camera triggers a request within the embodied system. This request undergoes the thinking process to generate an action for the robotics. Any additional requests generated during this period are discarded until the current request completes the overall action generation process. Such systems are often deployed on edge devices that typically feature a consumer-level GPU (e.g., an Nvidia RTX 4090) to meet low-latency response requirements.

Withlimited resources and advanced algorithms, the 'thinking' stage suffers from the low-frequency problem. Specifically, input data streams, such as videos captured by highresolution cameras, often operate at frequencies of 30 to 120 frames per second or higher [7, 15, 59, 62]. Robotic actuators require output commands at equally high or higher frequencies (e.g., 125-200Hz [15]) to achieve smooth and precise movements. However, the 'thinking' frequency of current embodied AI agents is quite low (e.g., 3-10Hz for

Google-series embodied agents [50, 86] and 1-6Hz for OpenVLA [35]). To this end, many algorithm works [16, 74, 81, 85] seek to use smaller models for inference but the throughput is still not ideal, and it is hard to fulfill the hardware's capability. Some humanoid robots (e.g., Figure AI's Helix [4]) deploy another "fast thinking" system using an additional GPU to keep pace with the high-frequency robotic actions, without optimizing resource utilization.

For the practical value of robotic agents, improving hardware efficiency is of equal importance to increasing the thinking frequency. However, the hardware remains underutilized currently, and effective strategies remain largely unexplored. The key obstacle to making good use of current hardware is that the perception and generation are predominantly implemented in a closed-loop pattern. The sequential computation of requests hinders possible throughput improvement. There is potential to improve the thinking throughput by increasing the parallelism, but two challenges appear. First, by parallelizing the original sequential requests, concurrent requests may interfere with each other using techniques such as CUDA multi-stream [1]. This interference hinders the agent from providing stable and high throughput (e.g., the output interval has an 89.8% fluctuation on average). Second, the latency of each request may be prolonged when paralleled as depicted in Figure 1(b), causing the 'thinking' process to compute on staled data, thereby decreasing the agent's accuracy.

Our key insights are: 1) it is possible to alleviate interference while achieving high thinking throughput, by organizing concurrent requests in a regular and controlled pattern (e.g., pipeline parallelism). 2) Disaggregating the perception and generation steps helps in maintaining the thinking accuracy. After the disaggregation, the generation phase can always be computed based on the fresh data exported from the latest perception phase, thereby maintaining high accuracy.

It is non-trivial to achieve the above design. While both perception and generation are intertwined on the same device, it is difficult to optimize the throughput through software pipelining. A promising strategy is disaggregating the two stages and managing their pipelines in a unified framework. Moreover, resolving the data staleness problem requires effective algorithm adaptation in the disaggregation design. It is complex to instruct the data flow between perception and generation while maintaining accuracy.

We therefore propose Auras, an embodied AI system that enables high-frequency perception and generation without compromising agent accuracy. To increase parallelism while maintaining accuracy, we analyze the mainstream algorithms and disaggregate the perception and generation, while sharing a public context buffer between them. By interacting through the public buffer, concurrent generation phases benefit from the latest data. To achieve high throughput while not harming agent accuracy, we propose an asynchronous pipeline executor to allow pipeline parallelism for both perception and generation. The executor targets to coordinate the execution pattern across different stages and we can then achieve a sweet point between high accuracy and high frequency. This work makes three main contributions.

- 1) Detailed analysis of characteristics of prevalent embodied AI algorithms and systems. We analyze the inability to meet the high-frequency demands of real-world agents.
- 2) The co-design of algorithm and system for perception and generation disaggregation. The disaggregation design ensures that the output of agents is based on fresh input data.
- 3) The design of asynchronous pipeline executor to achieve both high throughput and accuracy. Auras therefore provide controllable and stable output frequency.

Experimental results show that Auras improves the throughput of embodied AI systems with auto-regressive models by 3 . 05 × and diffusion-based models by 2 . 28 × , while achieving almost the same agent accuracy compared to original sequential computation patterns (102 . 7% on average). These results highlight Auras' capability to enhance system performance and scalability.

## 2 Related Works

Generative AI for robotics. Recently, more and more algorithm works focus on leveraging large generative models to implement embodied AI, covering scenarios including embodied question answering [17, 45, 55], task planning [29, 31, 48, 79], action planning [33, 38, 57, 58, 66, 73] and so on. Within them, Google [11, 86] demonstrates the potential of using an end-to-end vision-language-model to do semantic reasoning and generate actions in a discrete space. Following that, representative works such as OpenVLA [35], ICRT [77] and 𝜋 0 [10] also employ LLMs to auto-regressively generate responses to interact with the environment. The agent interacts with the environment by predicting the next token. Diffusion policy [15] generates robot behavior by representing a robot's visuomotor policy as a conditional denoising diffusion process and is also widely adopted in robotic agents [18, 34, 44, 75] to generate actions. Some commercialready robots such as Helix [4] and Nvidia's GR00T N1 [5] deploy two separate systems, namely 'System 2' for slowthinking and 'System 1' for fast-thinking.

System-level optimization. There are plenty of works focusing on reducing the latency of executing generative models such as Large Language Models (LLMs). To improve the effectiveness of handling concurrent LLM requests, scheduling techniques such as batching inference requests, as explored in prior works [6, 25, 78], have significantly increased the throughput of LLM serving systems. For better resource utilization, different phases of the generation process are disaggregated and utilize separate resources. For example,

<!-- image -->

(b) Closed-loop sequential execution of agents in real-time

Figure 2. The compute pattern of embodied AI agents.

some works [28, 56, 84] disaggregate the prefill and decoding phases to eliminate the interferences between them. To mitigate the inference using multi-stream on a single device, task prioritization and CUDA stream priorities techniques are leveraged in some previous works [54, 76], and these techniques are effective for priority-sensitive tasks and requests. In providing structured parallelism for large-scale LLM inference, PipeLLM [43] and PipeInfer [12] propose pipeline parallelism in serving distributed LLM, while many frameworks [37, 83] also provide data parallelism and tensor parallelism. There are also frameworks exploring various parallelism strategies in serving diffusion models [22, 39, 71] for better throughput.

## 3 Background and Motivation

This section introduces the background of embodied AI algorithms, discusses the weaknesses of current systems, and presents the opportunities to improve thinking throughput.

## 3.1 Embodied AI Compute Pattern

Figure 2(a) shows that the computation of an embodied AI agent includes two parts: Perception and Generation . The perception module receives multiple modal data as input and encodes them into a unified latent space, namely context . In the phase of generating actions or responses, current end-toend embodied agents mainly leverage generative algorithms, because they have the ability to 'think' iteratively [15, 35, 86] and perform better in reasoning and planning. There are typically two generative algorithms as agent policies.

1. Auto-regressive modeling: The generation module takes the latent context as input and generates responses in an auto-regressive manner. For robotic agents, each generated token represents a discrete value of the degree of freedom of the robot (e.g., the delta movement of the mechanical arm in the x-axis). For Google Robot [11, 86], the length of

Figure 3. GPU utilization of robotic agents within a single request. The utilization is the SM Warp Occupancy profiled using Nsight system [2]. The profiled models are Diffusion Policy [15], OpenVLA [35] and TinyVLA [74] respectively.

<!-- image -->

the generated response of each observation is fixed to be 7. Some mobile manipulation robots [32, 70] have a degree of freedom of 5. There also exist other embodied agents that generate length-variable tokens [30, 41], acting as instructions to downstream action generation policies.

2. Diffusion-based modeling: Denoising Diffusion Probabilistic Models (DDPM) are a class of generative models where the output generation is modeled as a denoising process. The diffusion-based algorithms take Gaussian noisy data 𝑥 0 as input, while the context is also forwarded as the conditioning input. Through multiple iterations of denoising, a desired noise-free output 𝑥 𝑛 is formed. The output can be predicted vectors of actions (e.g., the predicted robot actions [15, 47] or trajectories [24, 44, 75]).

## 3.2 Poor Utilization with Current Pattern

Current embodied AI agents cannot make good use of the equipped hardware resources. As shown in Figure 2(b), the agent perceives and generates responses in a closed-loop sequential manner. Define the interval between two adjacent actions generated Δ 𝑡 . Suppose the observation at time slot 𝑡 -1 is 𝑂 𝑡 -1, then the corresponding response is 𝐴 𝑡 = Agent ( 𝑂 𝑡 -1 ) This execution pattern ensures the bijection of observation and response. However, in real-time execution, this execution pattern cannot fully and evenly utilize the hardware's compute ability because different compute components of an agent have different resource requirements.

Figure 3 reveals the GPU utilization of typical robotic agents running on an NVIDIA RTX 4090 GPU within a time slot with the sequential execution pattern. The average utilization of a diffusion-based agent (DP) [15] is merely 34.9%. The utilization of OpenVLA [35], an auto-regressive agent is

Figure 4. Concurrent execution of requests with pipeline degree = 3. The agent's computed output 𝐴 𝑡 is based on the staled data 𝑂 𝑡 -3.

<!-- image -->

79 . 2%, higher than Diffusion Policy. But we still observe the inefficiency within the decode iterations (lower than 75%). Wealso observe the resource requirements of perception and generation are different. As shown in Figure 3(c), the utilization of the perception phase is 74.3% and merely 36.3% for the generation, revealing the low efficiency in the closed-loop sequential execution. Therefore, it is potential to increase the parallelism in computation to fulfill hardware utilization and increase agent throughput.

## 3.3 Challenges and Opportunities

Although increasing the parallelism can potentially achieve a higher execution frequency or throughput, there are two challenges in maintaining high agent accuracy and exporting stable and equidistant action responses.

Challenge 1: Uncontrolled parallelism provides suboptimal throughput. When requests are simply computed concurrently (e.g., using CUDA multi-stream [1]), the interference across requests is severe, leading to a suboptimal throughput. According to our experimental results, by increasing the degree of parallelism while not providing structured control, the throughput can be increased by 2 . 93 × on maximum compared with sequential execution. For comparison, when pipeline parallelism is applied, the throughput is increased by 3 . 47 × . We thus demonstrate that structured pipeline parallelism can provide high and stable throughput for embodied AI. It is challenging to manage the parallelism pattern for embodied AI systems because both perception and generation within the system are required to be parallelized for optimal execution. This leads to a complex parallelism configuration space, only by finding out a reasonable configuration within it can help to achieve both high throughput and agent accuracy.

Challenge 2: Staled data degrades agent accuracy. With parallelism increased, the prolonged execution duration of each request can largely harm the precision. We illustrate this with an example depicted as Figure 4. For example,

Figure 5. Agent accuracy with various degrees of parallelism. Pipeline degree = 1 refers to sequential execution.

<!-- image -->

when requests are executed concurrently with pipeline parallelism with a pipeline degree = 3 (i.e., the number of pipeline stages), three requests are computed simultaneously. With the interval of response generated still Δ 𝑡 , each response is computed as 𝐴 𝑡 = Agent ( 𝑂 𝑡 -3 ) , indicating that two latest observations 𝑂 𝑡 -2 and 𝑂 𝑡 -1 are skipped. Thus, although the increased parallelism may help to improve utilization, the data staleness may degrade the accuracy of agents. The experimental results are shown in Figure 5. As we increase the pipeline degree, the accuracy of agents is degraded drastically.

We identify that the key reason for the accuracy drop is that agents are generating responses according to stale data when executed in parallel. If all concurrent generations leverage the latest context, we observe that the agent accuracy is improved and comparable with the accuracy in sequential execution. Therefore, leveraging fresh data in the generation phase helps to maintain high accuracy. This requires us to break the data dependency between perception and generation, so that the generation can be computed with the latest context.

## 4 Auras Methodology

Based on the two opportunities above, we propose Auras, an inference framework for embodied AI agents that boosts their execution while maintaining accuracy.

## 4.1 Overview of Auras

Auras leverages algorithm and system co-design to provide an efficient scheme for embodied agents. Figure 6 shows the systematic workflow of Auras.

Figure 6. The systematic workflow of Auras.

<!-- image -->

Auras first modifies the compute pattern of embodied AI algorithms to achieve satisfying agent accuracy when the parallelism increases. Given an embodied AI algorithm, Auras takes a thorough analysis of its compute characteristics and disaggregates the perception and generation modules within. A public context is extracted from the original compute graph to share between the two modules. In real time, the perception module updates the public context buffer and the generation module fetches from the buffer asynchronously. This disaggregation design enables the generation module to always compute based on the latest environment context information. Targeting the two mainstream categories of embodied AI algorithms: auto-regressive models and diffusionbased models, we explain how we identify the public context and maintain high accuracy in §4.2.

The execution pattern of perception and generation influences the throughput and accuracy of the agent. Compared with uncontrolled parallelism with no interference control on a single device, we claim that pipeline parallelism is more advanced in providing stable and predictable throughput and beneficial in maintaining accuracy. Thus we propose the asynchronous pipeline executor (§4.3), which enables various algorithms to access the public context asynchronously, while in a controlled and defined manner. The execution behavior of the perception and generation pipelines is defined in the execution configuration, for which Auras leverages a hierarchical tuner to achieve a sweet point between throughput and accuracy in the simulation environment.

## 4.2 Disaggregate Perception and Generation

We first have a detailed analysis of the original sequential computation to disaggregate the perception and generation. Then we introduce how to form a public context to share between perception and generation.

## 4.2.1 Compute Graph Analysis of Embodied Agents. An embodied AI algorithm consists of both perception and generation. For a request, the perception module takes multimodal inputs from the environment as an observation and encodes them into a unified context using various models

(e.g., image encoders [14, 20, 26] for vision inputs and natural language tokenizers [36, 64, 65] for text inputs). Although may comprised of multiple models, the computation of perception is finished in a one-step manner.

However, the generation phase is completed in an iterative manner. We analyze the original sequential compute graph of the generation phase for auto-regressive and diffusion algorithms, the two promising techniques in embodied AI.

For auto-regressive models, the compute graph of a request R1's generation phase R1.gen is depicted in Figure 7(a), interacting with the token embedding 𝑋 = [ 𝑋 𝑉 , 𝑋 𝐿 , 𝑋 𝐴 ] , where 𝑋 𝑉 , 𝑋 𝐿 , 𝑋 𝐴 represent the embeddings for vision observations, human language prompts and generated action tokens, respectively. The embeddings 𝑋 𝑉 and 𝑋 𝐿 are the output of the perception module and remain unchanged within the request, while only the embeddings in 𝑋 𝐴 are computed token-by-token auto-regressively. The consistency of 𝑋 𝑉 , 𝑋 𝐿 and generated 𝑋 𝐴 enables the original auto-regressive generation to be optimized to use KV-cache [37, 69] to reduce computation. The generation is thus split into the prefill phase and multiple decode phases as Equation 1, where 𝑙 refers to the total length of 𝑋 𝑉 and 𝑋 𝐿 , 𝑙 𝑎 refers to the number of action tokens and 𝑥 refers to the embedding of a single token.

<!-- formula-not-decoded -->

The compute graph of diffusion-based models follows the denoising process. For diffusion-based models, the 𝐻 𝑜 is consistent across different iterative steps as shown in Figure 7(b), which is the hidden intermediate of observations as shown in Equation 2. The 𝑥 0 is a random Gaussian noise and after 𝑛 steps of de-noising, the 𝑥 𝑛 is the final output.

<!-- formula-not-decoded -->

Figure 7 only shows the compute graphs of auto-regressive models and diffusion-based models. However, as long as the model is composed of perception and generation, we can extract the variables within which are shared by both perception and generation.

4.2.2 Identify the Public Context. As we increase the parallelism by breaking the compulsory closed-loop computation, the system can hold parallel requests (e.g., R1 and R2 in Figure 7(c) and (d)) to handle continuous observations

Figure 7. The compute graphs of the generation module. With sequential computation, the variables that request R1 use are consistent within the request, but are considered volatile when requests are concurrent ((a) and (b)). With the public context, generation steps from parallel requests (R1 and R2) can share the same latest public context for computation ((c) and (d)).

<!-- image -->

instead of supporting only one request at a time. The parallel requests can then see each other's contexts within their lifecycles. It is beneficial for each request to discard its private consistent context, and look for the most timely context to generate responses. Therefore, we propose to establish a public context for parallel requests to use and fetch the latest environment information. It is thus crucial to identify the volatile context within each request that can be updated by the public one.

For auto-regressive models, the input in each generation step is annotated as volatile as colored gray in Equation 1, indicating that the KV-cache can not be shared across concurrent requests. Thus, as shown in Figure 7(c), the whole context [ 𝑋 𝑉 , 𝑋 𝐿 , 𝑋 𝐴 ] is regarded as the public context. In detail, to form the public context, the 𝑋 𝑉 and 𝑋 𝐿 are derived from the latest output of the perception module and the 𝑋 𝐴 are updated by the concurrent requests: each request leverages a prefill from [ 𝑋 𝑉 , 𝑋 𝐿 , 𝑋 𝐴 1 → 𝑖 ] to update the 𝑋 𝐴 𝑖 + 1 . The public context is therefore updated once new output from the perception module is calculated or new action tokens are generated.

For diffusion-based models, the 𝐻 𝑜 is volatile as annotated in Equation 2 and is considered as the public context. Then different iterative steps in the generation phase of concurrent requests can all access the same public context. The public context is derived from the perception module as the conditioning data for the generation policy network.

In summary, we regard the output of the perception module as the public context for diffusion-based models, and regard the output of perception module along with the generated action token embeddings as the public context for auto-regressive models. Due to the diversity of generative algorithms, we cannot simply consider the output of the perception module as the public context.

4.2.3 Generation based on the Public Context. With parallel requests sharing the public context to generate output action tokens, there is also a chance that parallel requests share the computation. We explain how Auras optimizes the generation by showcasing a scenario in which two parallel requests R1 and R2 are both in the generation phase of an auto-regressive model.

Suppose the computation of the 𝑖 -th iteration of R1 and the 𝑗 -th iteration of R2 are to be executed at the same time, represented by Generate ( 𝑥 ≤ 𝑙 + 𝑖 ) and Generate ( 𝑥 ≤ 𝑙 + 𝑗 ) , respectively. Then these computations can be merged into a single computation on the public context: the result of prefilling the first 𝑙 + 𝑖 tokens can be extracted from prefilling the first 𝑙 + 𝑗 tokens when 𝑖 &lt; 𝑗 . This holds true when the transformer applies causal masking, because causal masking ensures that the computation of the hidden state ℎ 𝑙 + 𝑖 (for token 𝑥 𝑙 + 𝑖 ) depends only on tokens 𝑥 1 , 𝑥 2 , · · · , 𝑥 𝑙 + 𝑖 , and not on tokens 𝑥 𝑙 + 𝑖 + 1 , · · · , 𝑥 𝑙 + 𝑗 . Therefore, the hidden state ℎ 𝑙 + 𝑖 is identical whether prefilling up to 𝑙 + 𝑖 tokens or 𝑙 + 𝑗 tokens.

Thus, for auto-regressive models, when different iterative steps are overlapped and share the public context, only a merged unified computation as Generate ( 𝑋 ) is required to update the action token embeddings 𝑋 𝐴 . The merging of diverse iterative steps can significantly reduce computation for auto-regressive models. Since the denoised data is not identical across different requests in diffusion-based models, the computation merge is not applied to diffusion policies.

4.2.4 Effectiveness of the Public Context. Figure 8 reveals the real execution process w/wo our public context design, using OpenVLA [35] on the Simpler simulation environment [40], with the task of Pick-Coke-Can . The action window is 80 frames and the original sequential execution finishes the task at the 69th frame, while the algorithm with context update finishes the task at the 49th frame, indicating shorter steps to finish the task. We observe that the sequential computation pattern encounters an obstacle of the gripper hanging within the 10th frame to the 34th frame (the gripper keeps closed and the robotic arm keeps stale). This is because the original sequential computation does not incorporate the 𝑋 𝐴 into computation and thus may hang when output actions are similar. With 𝑋 𝐴 also considered as a part of the public buffer, all concurrent requests can see the latest generated tokens, which we claim is beneficial in dynamic environments.

Within all the tasks that are judged as successful, Auras can reduce the number of frames by 20.5% compared with

Figure 8. Behaviors of OpenVLA on the Pick-Coke-Can task. (a) The original sequential execution; (b) Execution with integrating the latest public context into the computation.

<!-- image -->

sequential execution in the Pick-Coke-Can scenario. Note that we do not modify any model weights through finetuning.

## 4.3 Asynchronous Pipeline Executor

Since we have disaggregated the perception and generation, we must instruct the data flow between those two components and decide how the public context is shared between them to maintain accuracy. Besides, we also aim to provide a high and stable throughput.

The core concept of balancing throughput and accuracy is to establish deterministic frames and execute by frames. Therefore, the asynchronous pipeline executor enables the pipeline parallelism of both perception and generation and organizes different stages of perception and generation into a frame . We first determine the interaction logic of perception and generation on the public context, by providing a public context buffer design. Then we employ a hierarchical tuning process that can automatically determine the optimal pipeline configuration for both high agent accuracy and throughput.

4.3.1 Interact with the Public Context Buffer. We first establish a public context buffer to store the public context specific to each frame. Then the executor decides how the public context is shared between perception and generation. As shown in Figure 9(a), within each frame, the perception module puts the current computed context into the buffer to keep it always containing the latest environment information. Meanwhile, for the generation, it is allowed to fetch and compute on the public context of a past frame with an offset fetch\_offset . This design helps to increase the diversity of parallelism between perception and generation and is effective for some auto-regressive models (§6.4). We apply the double-buffering design to avoid the locking and unlocking overhead on the public context buffer. Because the context's size is negligible compared with the total device memory, the overhead of this double-buffering design is trivial.

When fetch\_offset is 0, which means perception and generation are operating on the same public context within a frame, they have to be executed in sequence because of the data dependency. This ensures that within a frame, the generation receives the latest context information because no actions are taken to change the environment within the frame. When the fetch\_offset is set to -1, the generation within a frame fetches and computes on the public context that is put by the perception module in the last frame. In this scenario, the perception and generation can run in parallel. A negative fetch\_offset indicates that perception and generation asynchronously access the public context buffer. Different algorithms have different affinities to this offset parameter. It is set to 0 for all diffusion-based policies and -1 for all auto-regressive policies.

4.3.2 Hierarchical Tuning for Pipeline Parallelism. The complete configuration space for pipelining is large because we have not only the number of pipeline stages and the method to split the model to decide, but also need to consider both perception and generation. We therefore leverage a hierarchical tuning process to determine how these two modules are pipelined. We first determine the number of pipeline stages and then finetune the partitioning method, considering both perception and generation.

Determine the number of pipeline stages. We first determine the number of pipeline stages for perception and generation because they drastically influence the throughput as well as agent accuracy. Specifically, the pp\_perception is the pipeline degree of the perception module. Any input observation has to experience pp\_perception frames to put results to the public context buffer. The pp\_generation is the pipeline degree of the generation module. The pp\_generation is identical to the number of frames that a complete generation phase would experience. Thus, within each frame, there are pp\_perception stages for perception and pp\_generation stages for generation from different requests, and these stages are computed concurrently. All generation stages can access

Figure 9. Illustration of the asynchronous pipeline executor. (a) Different patterns of accessing the public context buffer. (b) A configuration of pipelines for perception and generation. The complete compute process of the request R1 is marked red. In the generation phase of R1, the four generation stages compute on different public contexts (with different depths of color) because they are in different frames. The offset for generation to fetch the context buffer is -1 in this illustration.

<!-- image -->

the public context buffer and take the latest public context as input as shown in Figure 9(b).

Auras performs a grid search for these two parameters for pipelining to determine the optimal configuration that meets the requirements of users. Define 𝐿 as the number of frames that a request needs to experience. Then the request R1 shown in Figure 9(b) experiences 𝐿 = pp\_perception + pp\_generation = 6 frames to complete output. We establish an upper bound for 𝐿 in the grid search to prevent searching on extremely poor configurations.

Fine-tune the partitioning of computations. The partitioning of the perception and generation module in the grid search follows the uniform distribution, where each computation stage of the perception phase consists of identical computation flops, and so does the generation phase. This ensures that the concurrent computation within a frame can be overlapped as much as possible, which maximizes the throughput. Within the generation of each request, the iterative steps are uniformly distributed to the multiple stages. Suppose the number of iterations for generation of request R1 in Figure 9(b) is 100, then every 25 iterations are computed on the public context of frame 𝑡 -4, 𝑡 -3, 𝑡 -2 and 𝑡 -1 respectively. We find that by adjusting the mapping of computation onto multiple stages, both the throughput and accuracy of agents are impacted. It is feasible to sacrifice a certain amount of throughput and obtain additional accuracy gains.

After the pipeline degree of perception and generation is determined, Auras applies fine-tuning on the mapping of the computation to different pipeline stages. Specifically,

Table 1. Hardware and software specifications.

|           | System Overview          |
|-----------|--------------------------|
| CPU       | AMD EPYC 7763 64-core    |
| GPU       | Nvidia RTX3090 & RTX4090 |
| Runtimes  | CUDA 12.4                |
| Libraries | PyTorch 2.2.1            |

the distribution is controlled by a skewness weight 𝛼 , and the amount of computation in the 𝑖 -th stage is multiplied by the weight 𝑒 𝑖𝛼 and then normalized with other stages. When 𝛼 &gt; 0, more computations are executed by the later stages, which endows the whole generation process to incorporate more fresh context data. For example, when 𝛼 = 0 . 5, 46 iterative steps are computed on the public context within frame 𝑡 -1, indicating leveraging more on the freshest data.

## 5 Implementation

We implement Auras with 4,000+ LoC of Python. It receives the PyTorch models wrapped in torch.Module and automatically slice models into multiple stages. For transformer models that are not implemented in PyTorch, Auras converts it into x-transformers [72]. On Nvidia GPUs, since the compute pattern of perception and generation in embodied AI is fixed, we capture kernel launches with CUDA graph [60] and all computations on GPU within a frame are launched as CUDA graphs. Each CUDA graph is bonded to a specific CUDA stream[1] to support concurrency within the frames.

Optimizations. Auras applies multiple optimizations for the execution and resource management within frames.

1. Memory management: When the model size is too large for the GPU's device memory to hold, we apply the weight offloading feature to reside some model parameters on the CPU at runtime. Auras leverages HuggingFace's accelerate [21] library to automate the weight offloading procedure.
2. Batched execution: When multiple concurrent compute graphs are identical within a frame, we batch the input to allow the framework to use more efficient kernels for computation.
3. Graph launch sequence: Since a CUDA graph may be launched multiple times within a frame for the generation phase, in order to overlap the launch overhead of CUDA graphs on CPU, Auras alternately launch CUDA graphs from different streams.

## 6 Evaluation

In this section, we evaluate Auras in improving the throughput of embodied AI systems while maintaining accuracy.

## 6.1 Experimental Setup

The system configuration is shown in Table 1. To evaluate the effectiveness of Auras, we use a suite of embodied AI agents that employ either auto-regressive models or diffusion

Figure 10. The accuracy and throughput of embodied robotic agents. Accuracy : The S1-S3 represents the average success rate in three scenarios: Pick-Coke-Can , Move-Near and Open-Drawer for OpenVLA, and Sweep-wo-Exceeding , Scene-Understanding and Visual-Manipulation for RT2 ∗ . For Diffusion Policy models, S1-S3 represents the result on models that perform 40 , 100 , 160 diffusion steps on the Push-T task, respectively. Throughput : We anchor DP and DP-plus to 100 diffusion steps [15] and anchor DP-CNN and DP-CNN-plus to 40 diffusion steps for faster inference.

<!-- image -->

Table 2. Agent models in evaluation. ( ★ represents that the agent generates with the auto-regressive policy and ◦ represents the diffusion policy.)

| Agent              | Perception                | Generation    | Open-sourced          |
|--------------------|---------------------------|---------------|-----------------------|
| OpenVLA ★ [35]     | DinoV2 [53] + SigLIP [80] | OpenVLA       | Yes                   |
| RT2 ∗ ★ [86]       | ViT-large [14]            | LLama2 [3]    | Adapted in LLaRa [41] |
| DP ◦ [15]          | ResNet18 [26]             | Transformer-S | Yes                   |
| DP-CNN ◦ [15]      | ResNet18                  | UNet-S [61]   | Yes                   |
| DP-plus ◦ [15]     | 4 × ResNet18              | Transformer-L | Yes                   |
| DP-CNN-plus ◦ [15] | 4 × ResNet18              | UNet-L        | Yes                   |
| TinyVLA ◦ [74]     | Pythia [9]                | Transformer   | No                    |

models, as shown in Table 2. This table also shows the specific perception and generation components for each model.

Agents and simulation environments. We directly adopt the released parameters of the open-sourced models and run their trained tasks on their default simulation platforms. Specifically, OpenVLA is composed of two vision encoders (0.7B) and a 7B LLM backend and is tested on Simpler [40] with three scenarios, including 756 tasks in total. The original RT2 [86] is a 560B model and is not released. We thus adopt RT2 ★ , which is a released 7B model trained in LLaRa [41] in RT2's training style. It uses VIMA [32] and has 840 tasks. DP and DP-CNN are tested on the Push-T task, with 40MB and 505MB parameters separately. The plus versions of Diffusion Policy are tested on more complicated tasks in robomimic [46] that two robotic arms and four cameras are adopted. The total model sizes are also expanded to 182MB and 527MB for DP-plus and DP-CNN-plus, separately.

Compare targets. We compare Auras with three baseline execution modes: sequential, decoupled, and parallel execution modes. Sequential execution (SEQ) is the conventional pattern in existing embodied AI systems, where all incoming requests strictly follow the perception-generation workflow in a fully serial manner. With decoupled execution (DEC), perception and generation are fully separated into two different streams. During the generation phase, the system automatically fetches the latest perception results. Parallel execution (PAR) enhances system throughput by employing multiple workers, with each worker assigned an isolated thread and a CUDA stream. Each request is then assigned to an idle worker in real time. The request interval is aligned with the frame interval using Auras, representing the same input frequency.

## 6.2 Overall Performance

In this subsection, we evaluate the accuracy and throughput of the agents with different execution modes.

Accuracy. Figure 10(a) shows the success rate of each model under different execution modes. The sequential mode naturally achieves high accuracy, as the perception step is always up-to-date before generation, so does decoupled execution mode. In contrast, the parallel mode suffers from a notable drop in success rate, on average by 80 . 22%, because individual requests take longer to process, causing each worker to rely on older (less fresh) perception results. Additionally, we observe that the accuracy drop problem is especially severe when the task is complicated. Compared with DP and DP-CNN, although the models are scaled up,

DP-plus and DP-CNN-plus still suffer a complete disability (the accuracy drops to 0) in the parallel mode.

With Auras, we resolve the data staleness problem by leveraging the public context design. This ensures that each generation phase benefits from the most recent perception results. As a result, Auras maintains a success rate that closely matches that of sequential execution (102 . 7% on average across 6 different models).

Throughput. Figure 10(b) presents the throughput of different models and execution modes when running on Nvidia RTX4090 and RTX 3090 GPUs, respectively. Auras increases the execution frequency by 2 . 29 × , 3 . 01 × and 1 . 49 × compared with SEQ, DEC and PAR on average. Compared with the original 6Hz thinking frequency of OpenVLA, Auras increases the frequency to 17Hz.

Compared to the sequential mode, the throughput under the decoupled execution mode decreases by 22 . 5% on average. This is because separating perception and generation into different streams can lead to interference, reducing throughput. Additionally, in this execution mode, the perception component runs continuously, but the generation step does not consistently utilize its results. This leads to redundant computations and wastage of computational resources, further exacerbating the inefficiencies of this approach.

Deploying multiple workers in parallel yields an increase in throughput of 1 . 06 × -2 . 93 × , benefiting from the increased parallelism. Diffusion-based models see the most pronounced gains (up to 2 . 93 × ). However, the overall throughput improvement is bounded by potential bottlenecks in GPU capacity and resource contention. Notably, for the two autoregressive models, OpenVLA and RT2 ∗ , the throughput gains are marginal, only achieving 1 . 06 × -1 . 29 × .

By disaggregating perception and generation, Auras outperforms the sequential approach across all tested models, boosting throughput by 1 . 32 × -3 . 48 × on RTX 4090 and 1 . 18 × -3 . 08 × on RTX 3090. The efficient pipelines adopted in the asynchronous pipeline executor of Auras contribute to such throughput improvement. Besides, the public context design enables auto-regressive models to share the generation across requests, further reducing the computation burden and improving the throughput.

## 6.3 Effectiveness of Pipeline Parallelism

In this section, we conduct experiments to evaluate the effectiveness of pipeline parallelism in increasing the system throughput, compared with unstructured parallelism using multi-stream as shown in Figure 11.

Weobserve that although the throughput is increased with the number of worker streams in the parallel mode, there is still a gap compared with the pipeline parallelism achieved by Auras's asynchronous pipeline executor. For the two auto-regressive models, OpenVLA and RT2 ∗ , the throughput is only improved by a modest 1 . 11 × in the parallel mode,

Figure 11. Throughput of agents under various parallelisms.

<!-- image -->

while Auras improves the throughput by 2 . 20 × -3 . 29 × . For diffusion models, the throughput increases with the number of worker streams because of the comparably small model sizes. However, the gain diminishes as the parallelism degree reaches 16. Auras easily surpasses the maximum achievable throughput by an additional 1 . 07 × -1 . 19 × . We conclude that the benefits of pipeline parallelism over unstructured parallelism using multi-stream come from three aspects.

Lowinterference between requests. By separating perception and generation into well-organized stages, Auras reduces interference between parallel requests. The interference can be evaluated by the job completion time (JCT) of each request. When we impose the same input frequency of observations to parallel workers as Auras, requests are queued and the JCT increases drastically. If we impose just-infit input frequency, we observe that the JCT of each request is also longer than the JCT in the sequential mode. For example, the JCT of OpenVLA is 153ms in the sequential mode, while in the parallel mode the JCT is 303ms when the number of workers is 2, which means the parallel two requests on the two workers are almost interwoven completely.

Low memory consumption. We observe that for the two auto-regressive models, OpenVLA and RT2 ∗ , the maximum number of workers we can deploy simultaneously is limited to 3 because of the limited VRAM on both the Nvidia RTX 4090 and RTX 3090 GPUs (24GiB). Although there is no need to replicate the model weights for multiple workers, each worker has to maintain its own CUDA graph for the agent model because the input and output buffers for different workers can not be shared. In contrast, Auras only requires managing the CUDA graphs within a single frame and reusing them in different frames, thereby reducing the memory footprint.

Figure 12. Throughput and accuracy tradeoff. The benchmark used in diffusion policy is Push-T and the benchmark used in the auto-regressive model is Pick-Coke-Can .

<!-- image -->

Merged computation within frames. An additional optimization that helps Auras to beat unstructured parallelism is the merging of computation based on the public context. It is applied in auto-regressive models. In transformer-based generative models that employ causal masking, the computation for the hidden state of the 𝑖 -th token is independent of any tokens beyond 𝑖 . This property allows a single large prefill to replace multiple smaller ones, substantially cutting down the number of repeated operations.

## 6.4 Effectiveness of Pipeline Tuning

The asynchronous pipeline executor aims to find a sweet point between high throughput and accuracy. This subsection examines the effectiveness of the hierarchical tuning process in achieving a satisfying pipeline configuration.

Tuning the number of pipeline stages. Figure 12 reveals the segments of the grid search when exploring the optimal number of pipeline stages. As shown in Figure 12(a) and (b), wefirst adjust the pp\_generation and fix the pp\_perception = 1. We observe that for diffusion policy, the agent accuracy decreases but the throughput increases with pp\_generation . When pp\_generation is fixed to 1, increasing pp\_perception causes a severe accuracy drop for diffusion policy as shown in Figure 12(c).

For auto-regressive models, the throughput is sensitive to neither pp\_generation nor pp\_perception alone, but the accuracy is affected by the summation 𝐿 = pp\_perception + pp\_generation . As shown in Figure 12(b) and (d), when 𝐿 = 3, OpenVLA has a noticeable accuracy improvement. The

Figure 13. Accuracy and throughput with different partitioning skewness weight for generation ( pp\_generation = 5) on the Push-T task.

<!-- image -->

non-monotonic relationship between pipeline parameters and accuracy holds for other auto-regressive models in our experiment, necessitating the complete grid search to find the optimal numbers of pipeline stages.

Fine-tuning the skewness of partition distribution. Figure 13 shows the agent accuracy and throughput of diffusion policy with different skewness when partitioning the generation phase. When the distribution is uniform, the overall throughput is the highest. However, with the increase of the skewness parameter 𝛼 , we notice an increasing trend in accuracy. This is because to generate a complete response, 64 of the 100 diffusion steps are generated according to the latest public context when 𝛼 = 1, while the partitioning using uniform distribution only endows 20/100 diffusion steps computed on the latest data.

The effectiveness of fine-tuning the partitioning is nonnegligible. As shown in Figure 13(a), compared with the uniform distribution partitioning that has a 11 . 84 FPS, a proper distribution ( 𝛼 = 1) can also maintain a 10 . 25 FPS throughput and improves the accuracy by 23 . 98%. As for the auto-regressive models, if the computation of generation is merged within each frame as introduced in §4.2, adjusting the skewness of partitioning would not take effect.

## 6.5 Scaling the Model

To work in diverse and complicated scenarios, embodied AI algorithms have to be adaptive to different inputs, outputs, and model structures. The model provided by algorithm developers may vary drastically. For example, model structures can vary in parameter sizes and iterative steps. In this subsection, we use experimental results to show the performance of Auras scaling to different scenarios.

Various parameter sizes. Wescale the parameters of two representative models: the auto-regressive OpenVLA and the diffusion-based DP-plus. We measure the throughput speedup of Auras relative to the sequential baseline, as shown in Figure 14. We regard the original model size as

Figure 14. Speedup of Auras compared with the sequential execution with different model parameter size.

<!-- image -->

1, then apply scale factors of {-4, -2, 2, 4} to the perception and generation modules for exponential scaling, respectively. For instance, a scale factor of -2 for perception means its parameter size is 2 -2 times the original size.

Specifically, for OpenVLA, the speedup ranges from 1 . 53 × to 2 . 63 × . For DP-plus, the speedup ranges from 1 . 93 × to 3 . 93 × . Notably, both models share a similar trend: The speedup is proportional to the generation module's size and inversely proportional to the perception module's size. This is because that perception module is less likely to be pipelined because it is typically finished in a one-step manner. However, the generation module is executed iteratively naturally. Thus, increasing the model size of the generation module increases the possibility of pipelining.

Various iterative steps. The number of iterative steps, determined by the algorithm, directly influences the agent's accuracy. For example, more output tokens for an autoregressive model can be used to control more robotic arms, more degrees of freedom, or be applied in action ensembling [82]. More iteration steps in diffusion indicate more fine-grained generation and higher accuracy. We evaluate the throughput speedup of Auras with various iterative steps as shown in Figure 15.

For auto-regressive models, the speedup of Auras is proportional to the length of output tokens. The core reason is that the throughput of sequential execution is inversely proportional to the number of iterative (decode) steps, while Auras maintains a stable throughput because all decodings are integrated into one large prefill within a frame. The duration increase of a prefill with tens of token length growth is negligible when the input length is large, which is the case of large vision-language models. For instance, the length of embedded tokens encoded by a ViT-large of a 336px × 336px image is 576, which is 10 × larger than the output token length.

For diffusion models, we observe a minor speedup degradation when the number of iterative steps is small for the

Figure 15. Speedup of Auras with different numbers of iteration steps in the generation phase.

<!-- image -->

Figure 16. Throughput of Auras on TinyVLA with different configurations.

<!-- image -->

DP model. This is because when generation shrinks, the perception ratio expands, thereby reducing the opportunity for efficient overlapping across iterative generations.

## 6.6 A Case Study in Auras

The open-sourced models listed in Table 2 regard pre-trained vision encoders (e.g., ResNets [26] and vision transformers [20]) to output intermediate embeddings. We also see a bunch of works integrating MLLMs to encode the images and prompts as the perception module [47, 74]. The action policies in these algorithms are also diffusion. The characteristic of the LLM-as-encoder model is that the computation within the perception module is comparably large.

We showcase the profiling result of TinyVLA [74] to exhibit how Auras finds configurations that meet the throughput requirement of users. As shown in Figure 16, with different offsets for the generation to fetch from the public context buffer, the difference in throughput is not prominent. Then Auras applies the grid search to find configurations that meet the throughput and accuracy requirement of users. We assume a 2 × higher throughput is required and the fetch\_offset is set to -1, then the boxed configurations would be forwarded to the simulation platform to further test the accuracy.

## 6.7 Startup Time

The overhead introduced by Auras is predominantly confined to the startup phase of the system. For auto-regressive models, Auras initiates by performing a serial execution to generate the initial public context. Specifically, the startup

time is 154ms for OpenVLA and 218ms for RT2 ∗ . For diffusion models, the overhead is associated with the pipeline fill time. which are 40 ms and 67 ms for the DP and DP-plus models, respectively. The millisecond level of startup time is negligible in operating a real-world task.

## 7 Conclusion

In this paper, we addressed the limitations of traditional sequential computation patterns in embodied AI systems, particularly their inability to achieve the high "thinking" frequency required for real-world applications. To overcome these challenges, we introduced Auras, an algorithm-system co-designed inference framework to optimize the inference frequency of embodied AI systems. Auras disaggregates the perception and generation, and provides controlled pipeline parallelism for them. Experimental results demonstrated that Auras improves throughput by 2.54 × on average while achieving 102.7% of the original accuracy, demonstrating its effectiveness and scalability.

## References

- [1] 2012. Nvidia CUDA Stream Management. https://docs.nvidia.com/ cuda/cuda-runtime-api/group\_\_CUDART\_\_STREAM.html .
- [2] 2018. Nvidia Nsight Systems. https://developer.nvidia.com/nsightsystems .
- [3] 2023. Llama 2: Open Foundation and Fine-Tuned Chat Models. arXiv:2307.09288 [cs.CL] https://arxiv.org/abs/2307.09288
- [4] 2025. Helix: A Vision-Language-Action Model for Generalist Humanoid Control. https://www.figure.ai/news/helix .
- [5] 2025. NVIDIA Isaac GR00T. https://developer.nvidia.com/isaac/gr00t .
- [6] Amey Agrawal, Ashish Panwar, Jayashree Mohan, Nipun Kwatra, Bhargav S. Gulavani, and Ramachandran Ramjee. 2023. SARATHI: Efficient LLM Inference by Piggybacking Decodes with Chunked Prefills. (2023). https://www.microsoft.com/enus/research/publication/sarathi-efficient-llm-inference-bypiggybacking-decodes-with-chunked-prefills/
- [7] João Pedro Araújo, Jiaman Li, Karthik Vetrivel, Rishi Agarwal, Jiajun Wu, Deepak Gopinath, Alexander William Clegg, and Karen Liu. 2023. CIRCLE: Capture in Rich Contextual Environments. In Proceedings of the IEEE/CVF Conference on Computer Vision and Pattern Recognition (CVPR) . 21211-21221.
- [8] Suneel Belkhale, Tianli Ding, Ted Xiao, Pierre Sermanet, Quon Vuong, Jonathan Tompson, Yevgen Chebotar, Debidatta Dwibedi, and Dorsa Sadigh. 2024. RT-H: Action Hierarchies using Language. In https://arxiv.org/abs/2403.01823 .
- [9] Stella Biderman, Hailey Schoelkopf, Quentin Gregory Anthony, Herbie Bradley, Kyle O'Brien, Eric Hallahan, Mohammad Aflah Khan, Shivanshu Purohit, USVSN Sai Prashanth, Edward Raff, et al. 2023. Pythia: A suite for analyzing large language models across training and scaling. In International Conference on Machine Learning . PMLR, 2397-2430.
- [10] Kevin Black, Noah Brown, Danny Driess, Adnan Esmail, Michael Equi, Chelsea Finn, Niccolo Fusai, Lachy Groom, Karol Hausman, Brian Ichter, et al. 2024. 𝜋 0: A Vision-Language-Action Flow Model for General Robot Control.
- [11] Anthony Brohan, Noah Brown, Justice Carbajal, Yevgen Chebotar, Joseph Dabis, Chelsea Finn, Keerthana Gopalakrishnan, Karol Hausman, Alex Herzog, Jasmine Hsu, et al. 2022. Rt-1: Robotics transformer for real-world control at scale. arXiv preprint arXiv:2212.06817 (2022).
- [12] Branden Butler, Sixing Yu, Arya Mazaheri, and Ali Jannesari. 2024. PipeInfer: Accelerating LLM Inference using Asynchronous Pipelined Speculation. In SC24: International Conference for High Performance Computing, Networking, Storage and Analysis . 1-19. doi: 10.1109/ SC41406.2024.00046
- [13] Mark Chen, Jerry Tworek, Heewoo Jun, Qiming Yuan, Henrique Ponde de Oliveira Pinto, Jared Kaplan, Harri Edwards, Yuri Burda, Nicholas Joseph, Greg Brockman, Alex Ray, Raul Puri, Gretchen Krueger, Michael Petrov, Heidy Khlaaf, Girish Sastry, Pamela Mishkin, Brooke Chan, Scott Gray, Nick Ryder, Mikhail Pavlov, Alethea Power, Lukasz Kaiser, Mohammad Bavarian, Clemens Winter, Philippe Tillet, Felipe Petroski Such, Dave Cummings, Matthias Plappert, Fotios Chantzis, Elizabeth Barnes, Ariel Herbert-Voss, William Hebgen Guss, Alex Nichol, Alex Paino, Nikolas Tezak, Jie Tang, Igor Babuschkin, Suchir Balaji, Shantanu Jain, William Saunders, Christopher Hesse, Andrew N. Carr, Jan Leike, Josh Achiam, Vedant Misra, Evan Morikawa, Alec Radford, Matthew Knight, Miles Brundage, Mira Murati, Katie Mayer, Peter Welinder, Bob McGrew, Dario Amodei, Sam McCandlish, Ilya Sutskever, and Wojciech Zaremba. 2021. Evaluating Large Language Models Trained on Code. arXiv:2107.03374 [cs.LG] https: //arxiv.org/abs/2107.03374
- [14] Xi Chen and Xiao Wang. 2022. PaLI: Scaling Language-Image Learning in 100+ Languages. In Conference on Neural Information Processing Systems (NeurIPS) .
- [15] Cheng Chi, Zhenjia Xu, Siyuan Feng, Eric Cousineau, Yilun Du, Benjamin Burchfiel, Russ Tedrake, and Shuran Song. 2023. Diffusion policy: Visuomotor policy learning via action diffusion. The International Journal of Robotics Research (2023), 02783649241273668.
- [16] Xiangxiang Chu, Limeng Qiao, Xinyu Zhang, Shuang Xu, Fei Wei, Yang Yang, Xiaofei Sun, Yiming Hu, Xinyang Lin, Bo Zhang, and Chunhua Shen. 2024. MobileVLM V2: Faster and Stronger Baseline for Vision Language Model. arXiv:2402.03766 [cs.CV] https://arxiv. org/abs/2402.03766
- [17] Abhishek Das, Samyak Datta, Georgia Gkioxari, Stefan Lee, Devi Parikh, and Dhruv Batra. 2018. Embodied Question Answering. In 2018 IEEE/CVF Conference on Computer Vision and Pattern Recognition . 1-10. doi: 10.1109/CVPR.2018.00008
- [18] Sudeep Dasari, Oier Mees, Sebastian Zhao, Mohan Kumar Srirama, and Sergey Levine. 2024. The Ingredients for Robotic Diffusion Transformers. arXiv:2410.10088 [cs.RO] https://arxiv.org/abs/2410.10088
- [19] Jacob Devlin, Ming-Wei Chang, Kenton Lee, and Kristina Toutanova. 2019. BERT: Pre-training of Deep Bidirectional Transformers for Language Understanding. In Proceedings of the 2019 Conference of the North American Chapter of the Association for Computational Linguistics: Human Language Technologies, Volume 1 (Long and Short Papers) . 4171-4186. doi: 10.18653/v1/N19-1423
- [20] Alexey Dosovitskiy. 2020. An image is worth 16x16 words: Transformers for image recognition at scale. arXiv preprint arXiv:2010.11929 (2020).
- [21] Hugging Face. 2023. Loading Big Models into Memory. https: //huggingface.co/docs/accelerate/usage\_guides/big\_modeling . https: //huggingface.co/docs/accelerate/usage\_guides/big\_modeling
- [22] Jiarui Fang, Jinzhe Pan, Xibo Sun, Aoyu Li, and Jiannan Wang. 2024. xDiT: an Inference Engine for Diffusion Transformers (DiTs) with Massive Parallelism. arXiv preprint arXiv:2411.01738 (2024).
- [23] Google. 2023. Bard, an experiment by Google. https://bard.google. com/
- [24] Tianpei Gu, Guangyi Chen, Junlong Li, Chunze Lin, Yongming Rao, Jie Zhou, and Jiwen Lu. 2022. Stochastic trajectory prediction via motion indeterminacy diffusion. In Proceedings of the IEEE/CVF Conference on Computer Vision and Pattern Recognition . 17113-17122.
- [25] Ozgur Guldogan, Jackson Kunde, Kangwook Lee, and Ramtin Pedarsani. 2024. Multi-Bin Batching for Increasing LLM Inference Throughput. arXiv:2412.04504 [cs.CL] https://arxiv.org/abs/2412.

04504

- [26] Kaiming He, Xiangyu Zhang, Shaoqing Ren, and Jian Sun. 2016. Deep residual learning for image recognition. In Proceedings of the IEEE conference on computer vision and pattern recognition . 770-778.
- [27] Jonathan Ho, Ajay Jain, and Pieter Abbeel. 2020. Denoising diffusion probabilistic models. In Proceedings of the 34th International Conference on Neural Information Processing Systems (NIPS '20) . Article 574, 12 pages.
- [28] Cunchen Hu, Heyang Huang, Junhao Hu, Jiang Xu, Xusheng Chen, Tao Xie, Chenxi Wang, Sa Wang, Yungang Bao, Ninghui Sun, and Yizhou Shan. 2024. MemServe: Context Caching for Disaggregated LLM Serving with Elastic Memory Pool. ArXiv abs/2406.17565 (2024). https://api.semanticscholar.org/CorpusID:270711067
- [29] Wenlong Huang, P. Abbeel, Deepak Pathak, and Igor Mordatch. 2022. Language Models as Zero-Shot Planners: Extracting Actionable Knowledge for Embodied Agents. ArXiv abs/2201.07207 (2022). https://api.semanticscholar.org/CorpusID:246035276
- [30] Wenlong Huang, Chen Wang, Yunzhu Li, Ruohan Zhang, and Li FeiFei. 2024. Rekep: Spatio-temporal reasoning of relational keypoint constraints for robotic manipulation. arXiv preprint arXiv:2409.01652 (2024).
- [31] Wenlong Huang, F. Xia, Ted Xiao, Harris Chan, Jacky Liang, Peter R. Florence, Andy Zeng, Jonathan Tompson, Igor Mordatch, Yevgen Chebotar, Pierre Sermanet, Noah Brown, Tomas Jackson, Linda Luu, Sergey Levine, Karol Hausman, and Brian Ichter. 2022. Inner Monologue: Embodied Reasoning through Planning with Language Models. ArXiv abs/2207.05608 (2022). https://api.semanticscholar.org/CorpusID: 250451569
- [32] Yunfan Jiang, Agrim Gupta, Zichen Zhang, Guanzhi Wang, Yongqiang Dou, Yanjun Chen, Li Fei-Fei, Anima Anandkumar, Yuke Zhu, and Linxi Fan. 2023. VIMA: General Robot Manipulation with Multimodal Prompts. In Fortieth International Conference on Machine Learning .
- [33] Shiyu Jin, Jinxuan Xu, Yutian Lei, and Liangjun Zhang. 2024. Reasoning Grasping via Multimodal Large Language Model. ArXiv abs/2402.06798 (2024). https://api.semanticscholar.org/CorpusID: 267627619
- [34] Ivan Kapelyukh, Vitalis Vosylius, and Edward Johns. 2023. DALLE-Bot: Introducing Web-Scale Diffusion Models to Robotics. IEEE Robotics and Automation Letters 8, 7 (2023), 3956-3963. doi: 10.1109/ LRA.2023.3272516
- [35] Moo Jin Kim, Karl Pertsch, Siddharth Karamcheti, Ted Xiao, Ashwin Balakrishna, Suraj Nair, Rafael Rafailov, Ethan Foster, Grace Lam, Pannag Sanketi, Quan Vuong, Thomas Kollar, Benjamin Burchfiel, Russ Tedrake, Dorsa Sadigh, Sergey Levine, Percy Liang, and Chelsea Finn. 2024. OpenVLA: An Open-Source Vision-Language-Action Model. arXiv preprint arXiv:2406.09246 (2024).
- [36] Taku Kudo. 2018. Subword Regularization: Improving Neural Network Translation Models with Multiple Subword Candidates. In Proceedings of the 56th Annual Meeting of the Association for Computational Linguistics (Volume 1: Long Papers) , Iryna Gurevych and Yusuke Miyao (Eds.). Association for Computational Linguistics, Melbourne, Australia, 66-75. doi: 10.18653/v1/P18-1007
- [37] Woosuk Kwon, Zhuohan Li, Siyuan Zhuang, Ying Sheng, Lianmin Zheng, Cody Hao Yu, Joseph E. Gonzalez, Haotong Zhang, and Ion Stoica. 2023. Efficient Memory Management for Large Language Model Serving with PagedAttention. Proceedings of the 29th Symposium on Operating Systems Principles (2023). https://api.semanticscholar.org/ CorpusID:261697361
- [38] Kailin Li, Jingbo Wang, Lixin Yang, Cewu Lu, and Bo Dai. 2024. SemGrasp: Semantic Grasp Generation via Language Aligned Discretization. In Computer Vision - ECCV 2024: 18th European Conference, Milan, Italy, September 29-October 4, 2024, Proceedings, Part II (Milan, Italy). Springer-Verlag, Berlin, Heidelberg, 109-127. doi: 10.1007/978-3-03172627-9\_7
- [39] Muyang Li, Tianle Cai, Jiaxin Cao, Qinsheng Zhang, Han Cai, Junjie Bai, Yangqing Jia, Kai Li, and Song Han. 2024. Distrifusion: Distributed parallel inference for high-resolution diffusion models. In Proceedings of the IEEE/CVF Conference on Computer Vision and Pattern Recognition . 7183-7193.
- [40] Xuanlin Li, Kyle Hsu, Jiayuan Gu, Karl Pertsch, Oier Mees, Homer Rich Walke, Chuyuan Fu, Ishikaa Lunawat, Isabel Sieh, Sean Kirmani, Sergey Levine, Jiajun Wu, Chelsea Finn, Hao Su, Quan Vuong, and Ted Xiao. 2024. Evaluating Real-World Robot Manipulation Policies in Simulation. arXiv preprint arXiv:2405.05941 (2024).
- [41] Xiang Li, Cristina Mata, Jongwoo Park, Kumara Kahatapitiya, Yoo Sung Jang, Jinghuan Shang, Kanchana Ranasinghe, Ryan Burgert, Mu Cai, Yong Jae Lee, et al. 2024. LLaRA: Supercharging Robot Learning Data for Vision-Language Policy. CoRR (2024).
- [42] Meta LLaMA. 2023. LLaMA 3. https://github.com/meta-llama/llama3 . https://github.com/meta-llama/llama3
- [43] Ruilong Ma, Jingyu Wang, Qi Qi, Xiang Yang, Haifeng Sun, Zirui Zhuang, and Jianxin Liao. 2023. Poster: PipeLLM: Pipeline LLM Inference on Heterogeneous Devices with Sequence Slicing. In Proceedings of the ACM SIGCOMM 2023 Conference (New York, NY, USA) (ACM SIGCOMM '23) . Association for Computing Machinery, New York, NY, USA, 1126-1128. doi: 10.1145/3603269.3610856
- [44] Xiao Ma, Sumit Patidar, Iain Haughton, and Stephen James. 2024. Hierarchical Diffusion Policy for Kinematics-Aware Multi-Task Robotic Manipulation. In Proceedings of the IEEE/CVF Conference on Computer Vision and Pattern Recognition (CVPR) . 18081-18090.
- [45] Arjun Majumdar, Anurag Ajay, Xiaohan Zhang, Pranav Putta, Sriram Yenamandra, Mikael Henaff, Sneha Silwal, Paul Mcvay, Oleksandr Maksymets, Sergio Arnaud, Karmesh Yadav, Qiyang Li, Ben Newman, Mohit Sharma, Vincent-Pierre Berges, Shiqi Zhang, Pulkit Agrawal, Yonatan Bisk, Dhruv Batra, Mrinal Kalakrishnan, Franziska Meier, Chris Paxton, Alexander Sax, and Aravind Rajeswaran. 2024. OpenEQA: Embodied Question Answering in the Era of Foundation Models. 2024 IEEE/CVF Conference on Computer Vision and Pattern Recognition (CVPR) (2024), 16488-16498. https://api.semanticscholar. org/CorpusID:268066655
- [46] Ajay Mandlekar, Danfei Xu, Josiah Wong, Soroush Nasiriany, Chen Wang, Rohun Kulkarni, Li Fei-Fei, Silvio Savarese, Yuke Zhu, and Roberto Martín-Martín. 2021. What Matters in Learning from Offline Human Demonstrations for Robot Manipulation. In arXiv preprint arXiv:2108.03298 .
- [47] Oier Mees, Dibya Ghosh, Karl Pertsch, Kevin Black, Homer Rich Walke, Sudeep Dasari, Joey Hejna, Tobias Kreiman, Charles Xu, Jianlan Luo, et al. 2024. Octo: An Open-Source Generalist Robot Policy. In First Workshop on Vision-Language Models for Navigation and Manipulation at ICRA 2024 .
- [48] Yao Mu, Qinglong Zhang, Mengkang Hu, Wenhai Wang, Mingyu Ding, Jun Jin, Bin Wang, Jifeng Dai, Yu Qiao, and Ping Luo. 2024. EmbodiedGPT: vision-language pre-training via embodied chain of thought. In Proceedings of the 37th International Conference on Neural Information Processing Systems (New Orleans, LA, USA) (NIPS '23) . Curran Associates Inc., Red Hook, NY, USA, Article 1090, 14 pages.
- [49] Alexander Quinn Nichol and Prafulla Dhariwal. 2021. Improved Denoising Diffusion Probabilistic Models. In Proceedings of the 38th International Conference on Machine Learning . 8162-8171. https: //proceedings.mlr.press/v139/nichol21a.html
- [50] Abby O'Neill, Abdul Rehman, Abhinav Gupta, Abhiram Maddukuri, Abhishek Gupta, Abhishek Padalkar, Abraham Lee, Acorn Pooley, Agrim Gupta, Ajay Mandlekar, et al. 2023. Open xembodiment: Robotic learning datasets and rt-x models. arXiv preprint arXiv:2310.08864 (2023).
- [51] OpenAI. 2022. Introducing ChatGPT. https://openai.com/blog/chatgpt
- [52] OpenAI. 2023. GPT-4 Technical Report. https://openai.com/index/gpt4-research/

- [53] Maxime Oquab, Timothée Darcet, Theo Moutakanni, Huy V. Vo, Marc Szafraniec, Vasil Khalidov, Pierre Fernandez, Daniel Haziza, Francisco Massa, Alaaeldin El-Nouby, Russell Howes, Po-Yao Huang, Hu Xu, Vasu Sharma, Shang-Wen Li, Wojciech Galuba, Mike Rabbat, Mido Assran, Nicolas Ballas, Gabriel Synnaeve, Ishan Misra, Herve Jegou, Julien Mairal, Patrick Labatut, Armand Joulin, and Piotr Bojanowski. 2023. DINOv2: Learning Robust Visual Features without Supervision.
- [54] Weiguang Pang, Xiantong Luo, Kailun Chen, Dong Ji, Lei Qiao, and Wang Yi. 2023. Efficient CUDA stream management for multi-DNN real-time inference on embedded GPUs. Journal of Systems Architecture 139 (2023), 102888.
- [55] Bhrij Patel, Vishnu Sashank Dorbala, Amrit Singh Bedi, and Dinesh Manocha. 2024. Multi-LLM QA with Embodied Exploration. arXiv:2406.10918 [cs.LG] https://arxiv.org/abs/2406.10918
- [56] Pratyush Patel, Esha Choukse, Chaojie Zhang, Aashaka Shah, Íñigo Goiri, Saeed Maleki, and Ricardo Bianchini. 2024. Splitwise: Efficient Generative LLM Inference Using Phase Splitting. In 2024 ACM/IEEE 51st Annual International Symposium on Computer Architecture (ISCA) . 118-132. doi: 10.1109/ISCA59077.2024.00019
- [57] Rui Qian, Shuangrui Ding, Xiaoyi Dong, Pan Zhang, Yuhang Zang, Yuhang Cao, Dahua Lin, and Jiaqi Wang. 2025. Dispider: Enabling Video LLMs with Active Real-Time Interaction via Disentangled Perception, Decision, and Reaction. arXiv preprint arXiv:2501.03218 (2025).
- [58] Ilija Radosavovic, Bike Zhang, Baifeng Shi, Jathushan Rajasegaran, Sarthak Kamat, Trevor Darrell, Koushil Sreenath, and Jitendra Malik. 2024. Humanoid locomotion as next token prediction. arXiv preprint arXiv:2402.19469 (2024).
- [59] Sai Ramana Kiran Pinnama Raju, Rishabh Singh, Manoj Velmurugan, and Nitin J. Sanket. 2025. EdgeFlowNet: 100FPS@1W Dense Optical Flow for Tiny Mobile Robots. IEEE Robotics and Automation Letters 10, 1 (2025), 128-135. doi: 10.1109/LRA.2024.3496336
- [60] P. Ramarao. 2018. CUDA 10 Features Revealed: Turing, CUDA Graphs, and More. (Sept. 2018). https://devblogs.nvidia.com/cuda-10-featuresrevealed/
- [61] Olaf Ronneberger, Philipp Fischer, and Thomas Brox. 2015. U-Net: Convolutional Networks for Biomedical Image Segmentation. In Medical Image Computing and Computer-Assisted Intervention - MICCAI 2015 . 234-241.
- [62] Nitin J. Sanket, Chethan M. Parameshwara, Chahat Deep Singh, Ashwin V. Kuruttukulam, Cornelia Fermuller, Davide Scaramuzza, and Yiannis Aloimonos. 2019. EVDodge: Embodied AI For High-Speed Dodging On A Quadrotor Using Event Cameras.
- [63] P. Schmid. 2023. Fine-tune FLAN-T5 XL/XXL using DeepSpeed &amp; Hugging Face Transformers. https://www.philschmid.de/fine-tuneflan-t5-deepspeed . https://www.philschmid.de/fine-tune-flan-t5deepspeed Online.
- [64] Mike Schuster and Kaisuke Nakajima. 2012. Japanese and Korean voice search. In 2012 IEEE International Conference on Acoustics, Speech and Signal Processing (ICASSP) . 5149-5152. doi: 10.1109/ICASSP.2012. 6289079
- [65] Rico Sennrich, Barry Haddow, and Alexandra Birch. 2016. Neural Machine Translation of Rare Words with Subword Units. In Proceedings of the 54th Annual Meeting of the Association for Computational Linguistics (Volume 1: Long Papers) , Katrin Erk and Noah A. Smith (Eds.). Association for Computational Linguistics, Berlin, Germany, 1715-1725. doi: 10.18653/v1/P16-1162
- [66] Bokui (William) Shen, Ge Yang, Alan Yu, Jan Rang Wong, Leslie Pack Kaelbling, and Phillip Isola. 2023. Distilled Feature Fields Enable FewShot Language-Guided Manipulation. In Conference on Robot Learning . https://api.semanticscholar.org/CorpusID:260926035
- [67] Jascha Sohl-Dickstein, Eric A. Weiss, Niru Maheswaranathan, and Surya Ganguli. 2015. Deep unsupervised learning using nonequilibrium thermodynamics. In Proceedings of the 32nd International Conference on International Conference on Machine Learning - Volume 37 (Lille, France) (ICML'15) . JMLR.org, 2256-2265.
- [68] HugoTouvron, Thibaut Lavril, Gautier Izacard, Xavier Martinet, MarieAnne Lachaux, Timothée Lacroix, Baptiste Rozière, Naman Goyal, Eric Hambro, Faisal Azhar, Aurelien Rodriguez, Armand Joulin, Edouard Grave, and Guillaume Lample. 2023. LLaMA: Open and Efficient Foundation Language Models. arXiv:2302.13971 [cs.CL] https://arxiv. org/abs/2302.13971
- [69] Ashish Vaswani, Noam Shazeer, Niki Parmar, Jakob Uszkoreit, Llion Jones, Aidan N. Gomez, Łukasz Kaiser, and Illia Polosukhin. 2017. Attention is all you need. In Proceedings of the 31st International Conference on Neural Information Processing Systems (NIPS'17) . 6000-6010.
- [70] Homer Rich Walke, Kevin Black, Tony Z Zhao, Quan Vuong, Chongyi Zheng, Philippe Hansen-Estruch, Andre Wang He, Vivek Myers, Moo Jin Kim, Max Du, et al. 2023. Bridgedata v2: A dataset for robot learning at scale. In Conference on Robot Learning . PMLR, 1723-1736.
- [71] Jiannan Wang, Jiarui Fang, Aoyu Li, and PengCheng Yang. 2024. PipeFusion: Displaced Patch Pipeline Parallelism for Inference of Diffusion Transformer Models. arXiv preprint arXiv:2405.14430 (2024).
- [72] P. Wang. 2023. X-transformers. https://github.com/lucidrains/xtransformers . https://github.com/lucidrains/x-transformers GitHub repository.
- [73] Zihao Wang, Shaofei Cai, Guanzhou Chen, Anji Liu, Xiaojian Ma, Yitao Liang, and Team CraftJarvis. 2024. Describe, explain, plan and select: interactive planning with large language models enables openworld multi-task agents. In Proceedings of the 37th International Conference on Neural Information Processing Systems (NIPS '23) . Article 1480, 37 pages.
- [74] Junjie Wen, Yichen Zhu, Jinming Li, Minjie Zhu, Kun Wu, Zhiyuan Xu, Ning Liu, Ran Cheng, Chaomin Shen, Yaxin Peng, et al. 2024. Tinyvla: Towards fast, data-efficient vision-language-action models for robotic manipulation. arXiv preprint arXiv:2409.12514 (2024).
- [75] Zhou Xian, Nikolaos Gkanatsios, Theophile Gervet, Tsung-Wei Ke, and Katerina Fragkiadaki. 2023. ChainedDiffuser: Unifying Trajectory Diffusion and Keypose Prediction for Robotic Manipulation. In 7th Annual Conference on Robot Learning . https://openreview.net/forum? id=W0zgY2mBTA8
- [76] Yecheng Xiang and Hyoseung Kim. 2019. Pipelined data-parallel CPU/GPU scheduling for multi-DNN real-time inference. In 2019 IEEE Real-Time Systems Symposium (RTSS) . IEEE, 392-405.
- [77] Yida Yin, Zekai Wang, Yuvan Sharma, Dantong Niu, Trevor Darrell, and Roei Herzig. 2024. In-Context Learning Enables Robot Action Prediction in LLMs. arXiv preprint arXiv:2410.12782 (2024).
- [78] Gyeong-In Yu, Joo Seong Jeong, Geon-Woo Kim, Soojeong Kim, and Byung-Gon Chun. 2022. Orca: A Distributed Serving System for Transformer-Based Generative Models. In 16th USENIX Symposium on Operating Systems Design and Implementation (OSDI 22) . USENIX Association, Carlsbad, CA, 521-538. https://www.usenix.org/conference/ osdi22/presentation/yu
- [79] Andy Zeng, Maria Attarian, Brian Ichter, Krzysztof Marcin Choromanski, Adrian Wong, Stefan Welker, Federico Tombari, Aveek Purohit, Michael S. Ryoo, Vikas Sindhwani, Johnny Lee, Vincent Vanhoucke, and Pete Florence. 2023. Socratic Models: Composing Zero-Shot Multimodal Reasoning with Language. In The Eleventh International Conference on Learning Representations, ICLR 2023, Kigali, Rwanda, May 1-5, 2023 . OpenReview.net. https://openreview.net/forum?id= G2Q2Mh3avow
- [80] Xiaohua Zhai, Basil Mustafa, Alexander Kolesnikov, and Lucas Beyer. 2023. Sigmoid Loss for Language Image Pre-Training. arXiv:2303.15343 [cs.CV] https://arxiv.org/abs/2303.15343

- [81] Han Zhao, Min Zhang, Wei Zhao, Pengxiang Ding, Siteng Huang, and Donglin Wang. 2025. Cobra: Extending Mamba to Multi-Modal Large Language Model for Efficient Inference. arXiv:2403.14520 [cs.CV] https://arxiv.org/abs/2403.14520
- [82] Tony Z. Zhao, Vikash Kumar, Sergey Levine, and Chelsea Finn. 2023. Learning Fine-Grained Bimanual Manipulation with Low-Cost Hardware. https://arxiv.org/abs/2304.13705
- [83] Lianmin Zheng, Liangsheng Yin, Zhiqiang Xie, Chuyue Sun, Jeff Huang, Cody Hao Yu, Shiyi Cao, Christos Kozyrakis, Ion Stoica, Joseph E Gonzalez, et al. 2024. Sglang: Efficient execution of structured language model programs. arXiv preprint arXiv:2312.07104 (2024).
- [84] Yinmin Zhong, Shengyu Liu, Junda Chen, Jianbo Hu, Yibo Zhu, Xuanzhe Liu, Xin Jin, and Hao Zhang. 2024. DistServe: Disaggregating Prefill and Decoding for Goodput-optimized Large Language Model Serving. In 18th USENIX Symposium on Operating Systems Design and Implementation (OSDI 24) . USENIX Association, Santa Clara, CA, 193210. https://www.usenix.org/conference/osdi24/presentation/zhongyinmin
- [85] Yichen Zhu, Minjie Zhu, Ning Liu, Zhicai Ou, Xiaofeng Mou, and Jian Tang. 2024. LLaVA-Phi: Efficient Multi-Modal Assistant with Small Language Model. arXiv:2401.02330 [cs.CV] https://arxiv.org/abs/2401. 02330
- [86] Brianna Zitkovich, Tianhe Yu, Sichun Xu, Peng Xu, Ted Xiao, Fei Xia, Jialin Wu, Paul Wohlhart, Stefan Welker, Ayzaan Wahid, et al. 2023. Rt-2: Vision-language-action models transfer web knowledge to robotic control. In Conference on Robot Learning . PMLR, 2165-2183.