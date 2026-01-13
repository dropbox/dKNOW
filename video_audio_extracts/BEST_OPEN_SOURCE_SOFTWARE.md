# Best Open Source Software for Video & Audio Extraction
## Comprehensive Technology Stack

---

## ⚠️ HISTORICAL DOCUMENT - Implementation Differs

**This document was created during the planning phase (pre-implementation) and describes tools that were evaluated.**

**Actual Implementation Status** (as of Phase 3 completion):
- ✅ **100% Python-free** - All Python dependencies eliminated (Faster-Whisper, PyAnnote.audio replaced)
- ✅ **Whisper.cpp** via whisper-rs (not Faster-Whisper)
- ✅ **Custom Diarization** pipeline (WebRTC VAD + ONNX + K-means, not PyAnnote.audio)
- ✅ **All ML models** via ONNX Runtime (pure Rust inference)
- ✅ **FFmpeg scdet** for scene detection (classical, not ML-based TransNetV2)

See **README.md** and **AI_TECHNICAL_SPEC.md** for accurate current implementation details.

---

## ⚠️ Performance Benchmarks Notice

**Performance numbers listed in this document (FPS, throughput, latency, etc.) are reference values from literature and typical benchmarks.**

These serve as:
- Comparative baselines for tool selection
- Expected performance ranges
- Optimization targets

**Actual performance will vary based on**:
- Hardware configuration
- Input media characteristics
- Processing parameters
- System load

All performance claims will be validated through empirical testing using the test suite (TEST_SUITE_REPORT.md).

---

## 1. VIDEO & AUDIO PROCESSING (CPU-bound)

### 1.1 FFmpeg Ecosystem

#### FFmpeg 7.x
- **Repo**: https://github.com/FFmpeg/FFmpeg
- **Language**: C
- **License**: LGPL 2.1+ / GPL 2+
- **Purpose**: Universal video/audio encoder, decoder, transcoder
- **Why Best-in-Class**:
  - Industry standard, 20+ years of development
  - 500+ codecs supported
  - Hardware acceleration (NVDEC, VAAPI, VideoToolbox, QSV, AMF)
  - Battle-tested at scale (YouTube, Netflix, Twitch)
  - Active community, excellent documentation
- **Features**:
  - Video decoding/encoding (H.264, H.265, VP9, AV1, ProRes)
  - Audio processing (AAC, Opus, FLAC, MP3)
  - Filter graphs (resize, crop, overlay, denoise)
  - Streaming protocols (RTMP, HLS, DASH)
  - Format muxing/demuxing (MP4, MOV, MKV, WebM)
- **Performance**: 100-1000+ FPS with hardware acceleration
- **Rust Bindings**: `ffmpeg-next`, `ffmpeg-sys-next`

#### FFprobe (part of FFmpeg)
- **Purpose**: Media file metadata extraction
- **Features**:
  - Codec information (video/audio/subtitle streams)
  - Duration, bitrate, resolution, frame rate
  - Container format details
  - HDR metadata (HDR10, Dolby Vision)
  - JSON/XML output
- **Performance**: < 1 second for most files

### 1.2 Alternative Video Tools

#### GStreamer
- **Repo**: https://github.com/GStreamer/gstreamer
- **Language**: C
- **License**: LGPL 2.1
- **Purpose**: Multimedia framework with pipeline architecture
- **Pros**:
  - Plugin architecture (modular)
  - Low-latency streaming
  - Hardware acceleration
- **Cons**:
  - Steeper learning curve than FFmpeg
  - Less comprehensive codec support
- **Use Case**: Real-time video processing, camera input

#### OpenCV
- **Repo**: https://github.com/opencv/opencv
- **Language**: C++
- **License**: Apache 2.0
- **Purpose**: Computer vision library
- **Features**:
  - Video I/O (capture, write)
  - Classical CV (edge detection, feature matching)
  - Object tracking (KCF, CSRT, MedianFlow)
  - Image filtering and transformations
- **Rust Bindings**: `opencv-rust`
- **Note**: Use for CV tasks, not for general video encoding

---

## 2. SPEECH-TO-TEXT (ASR)

### 2.1 Whisper Family (RECOMMENDED)

#### Faster-Whisper
- **Repo**: https://github.com/SYSTRAN/faster-whisper
- **Language**: Python (CTranslate2 backend in C++)
- **License**: MIT
- **Models**: OpenAI Whisper (tiny to large-v3)
- **Why Best**:
  - 4x faster than original Whisper
  - Same accuracy as OpenAI Whisper
  - Lower memory footprint (8-bit quantization)
  - Batch processing support
- **Languages**: 99 languages (including code-switching)
- **Models**:
  - tiny (39M params) - 32x real-time on CPU
  - base (74M) - 16x real-time
  - small (244M) - 6x real-time
  - medium (769M) - 2x real-time
  - large-v3 (1.5B) - 1x real-time on GPU
- **Performance**:
  - GPU: 5-10x real-time (large-v3)
  - CPU: 1-3x real-time (medium)
- **WER**: 5-10% on LibriSpeech (depending on model)
- **Integration**: Python API, easily called from Rust via PyO3

#### Whisper.cpp
- **Repo**: https://github.com/ggerganov/whisper.cpp
- **Language**: C/C++
- **License**: MIT
- **Why Consider**:
  - Pure C++ (no Python dependency)
  - CoreML support (optimized for Apple Silicon)
  - Quantization (4-bit, 5-bit, 8-bit)
  - Metal acceleration (Apple GPUs)
  - CUDA support (NVIDIA GPUs)
- **Performance**:
  - Apple M1/M2: 10-20x real-time (medium model)
  - CPU-only: 3-5x real-time (base model)
- **Use Case**: Embedded devices, Apple Silicon optimization
- **Rust Bindings**: `whisper-rs`

#### OpenAI Whisper (Original)
- **Repo**: https://github.com/openai/whisper
- **Language**: Python (PyTorch)
- **License**: MIT
- **Why Include**:
  - Reference implementation
  - Latest research features first
  - Easy to fine-tune
- **Cons**:
  - Slower than Faster-Whisper
  - Higher memory usage
- **Use Case**: Research, experimentation, fine-tuning

### 2.2 Alternative ASR Systems

#### wav2vec 2.0
- **Repo**: https://github.com/facebookresearch/fairseq
- **Language**: Python (PyTorch)
- **License**: MIT
- **Why Consider**:
  - Best for low-resource languages
  - Self-supervised pre-training
  - State-of-the-art on some benchmarks
- **Models**: Base (~300M params), Large (~1B params)
- **Languages**: 100+ via XLSR models
- **Cons**: Less popular than Whisper, harder to deploy

#### Vosk
- **Repo**: https://github.com/alphacep/vosk-api
- **Language**: Python/C++
- **License**: Apache 2.0
- **Why Consider**:
  - Offline, no internet required
  - Small models (50MB - 1.8GB)
  - Real-time capable
  - 20+ languages
- **Cons**: Lower accuracy than Whisper
- **Use Case**: Edge devices, privacy-critical applications

#### Mozilla DeepSpeech (DEPRECATED)
- **Repo**: https://github.com/mozilla/DeepSpeech
- **Status**: No longer maintained (use Coqui STT fork)
- **Replacement**: Coqui STT (community fork)

#### Coqui STT
- **Repo**: https://github.com/coqui-ai/STT
- **Language**: Python (TensorFlow Lite)
- **License**: MPL 2.0
- **Why Consider**:
  - Open-source DeepSpeech successor
  - Small models (~900MB)
  - Real-time streaming
- **Cons**: Lower accuracy than Whisper

---

## 3. SPEAKER DIARIZATION

### 3.1 PyAnnote.audio (RECOMMENDED)
- **Repo**: https://github.com/pyannote/pyannote-audio
- **Language**: Python (PyTorch)
- **License**: MIT
- **Why Best**:
  - State-of-the-art accuracy (DER < 10%)
  - Pre-trained models (HuggingFace)
  - Active development and research
  - Pipeline architecture (VAD + embedding + clustering)
- **Features**:
  - Speaker diarization (who spoke when)
  - Speaker segmentation
  - Overlapped speech detection
  - Voice activity detection (VAD)
  - Speaker embedding extraction
- **Performance**: 2-5x real-time on GPU
- **Models**:
  - `pyannote/speaker-diarization-3.1` (latest)
  - `pyannote/segmentation-3.0`
- **DER (Diarization Error Rate)**: 8-12% on standard benchmarks

### 3.2 Alternative Diarization Tools

#### Speechbrain
- **Repo**: https://github.com/speechbrain/speechbrain
- **Language**: Python (PyTorch)
- **License**: Apache 2.0
- **Why Consider**:
  - All-in-one speech toolkit
  - Speaker recognition
  - Speech enhancement
  - Emotion recognition
  - Extensive recipes
- **Features**:
  - ECAPA-TDNN embeddings (state-of-the-art)
  - Speaker verification
  - Multi-speaker diarization
- **Use Case**: When you need multiple speech tasks

#### NVIDIA NeMo
- **Repo**: https://github.com/NVIDIA/NeMo
- **Language**: Python (PyTorch)
- **License**: Apache 2.0
- **Why Consider**:
  - GPU-optimized (TensorRT integration)
  - Production-ready (Riva deployment)
  - Multi-task (ASR + diarization + TTS)
- **Features**:
  - TitaNet speaker embeddings
  - Clustering-based diarization
  - End-to-end ASR+diarization
- **Cons**: NVIDIA ecosystem lock-in

#### Resemblyzer
- **Repo**: https://github.com/resemble-ai/Resemblyzer
- **Language**: Python (PyTorch)
- **License**: Apache 2.0
- **Purpose**: Voice encoder for speaker recognition
- **Use Case**: Extract speaker embeddings, simple diarization

---

## 4. SCENE DETECTION

### 4.1 Classical Methods

#### PySceneDetect
- **Repo**: https://github.com/Breakthrough/PySceneDetect
- **Language**: Python
- **License**: BSD-3-Clause
- **Why Best for Classical**:
  - Fast (100+ FPS on CPU)
  - Multiple algorithms (content-aware, threshold, adaptive)
  - FFmpeg integration
  - CLI + Python API
- **Algorithms**:
  - Content-aware (histogram difference)
  - Threshold-based (fade in/out)
  - Adaptive threshold
- **Performance**: 50-200 FPS (CPU)
- **Accuracy**: 85-90% F1 score
- **Use Case**: Fast, CPU-only scene detection

#### ffmpeg-scene-detection (Built-in)
- **Command**: `ffmpeg -i input.mp4 -vf "select='gt(scene,0.3)'" -vsync vfr frames%d.jpg`
- **Why Consider**: No dependencies, ultra-fast
- **Cons**: Limited configurability

### 4.2 ML-Based Methods (RECOMMENDED)

#### TransNetV2
- **Repo**: https://github.com/soCzech/TransNetV2
- **Language**: Python (TensorFlow)
- **License**: MIT
- **Why Best for ML**:
  - State-of-the-art shot detection (97%+ F1 score)
  - 100+ FPS on GPU
  - Single-frame prediction (no temporal window needed)
  - Pre-trained on ClipShots dataset
- **Features**:
  - Shot boundary detection (cuts, fades, dissolves)
  - Per-frame predictions
  - Confidence scores
- **Performance**: 100-200 FPS on modern GPU
- **Model Size**: ~10MB
- **Use Case**: High-accuracy scene detection

#### Katna
- **Repo**: https://github.com/keplerlab/katna
- **Language**: Python (OpenCV)
- **License**: GPL 3.0
- **Features**:
  - Smart keyframe extraction
  - Scene change detection
  - Brightness filtering
- **Use Case**: Extracting best representative frames

---

## 5. OBJECT & FACE DETECTION

### 5.1 Object Detection

#### YOLOv8 (Ultralytics) - RECOMMENDED
- **Repo**: https://github.com/ultralytics/ultralytics
- **Language**: Python (PyTorch)
- **License**: AGPL 3.0 (Commercial license available)
- **Why Best**:
  - State-of-the-art speed/accuracy tradeoff
  - 80 object classes (COCO)
  - Multiple model sizes (nano to extra-large)
  - ONNX export (easy deployment)
  - Active development, excellent docs
- **Models**:
  - YOLOv8n (nano) - 300 FPS on GPU, 6MB
  - YOLOv8s (small) - 200 FPS, 22MB
  - YOLOv8m (medium) - 100 FPS, 52MB
  - YOLOv8l (large) - 60 FPS, 87MB
  - YOLOv8x (xlarge) - 30 FPS, 136MB
- **mAP**: 50-60% on COCO (depending on model size)
- **Use Case**: Real-time object detection
- **Rust Integration**: ONNX Runtime (`ort` crate)

#### YOLOv9 / YOLOv10
- **Repos**:
  - https://github.com/WongKinYiu/yolov9
  - https://github.com/THU-MIG/yolov10
- **Why Consider**: Latest YOLO versions, incremental improvements
- **Status**: Less mature than YOLOv8

#### DETR (Detection Transformer)
- **Repo**: https://github.com/facebookresearch/detr
- **Language**: Python (PyTorch)
- **License**: Apache 2.0
- **Why Consider**:
  - Transformer-based (no anchors, no NMS)
  - Better on small/occluded objects
  - Research interest
- **Cons**: Slower than YOLO (10-30 FPS)
- **Use Case**: High-accuracy detection, research

#### Detectron2 (Meta AI)
- **Repo**: https://github.com/facebookresearch/detectron2
- **Language**: Python (PyTorch)
- **License**: Apache 2.0
- **Features**:
  - Multiple architectures (Faster R-CNN, Mask R-CNN, RetinaNet)
  - Instance segmentation
  - Keypoint detection
  - Panoptic segmentation
- **Cons**: Slower than YOLO, more complex
- **Use Case**: Research, segmentation tasks

### 5.2 Face Detection

#### RetinaFace - RECOMMENDED
- **Repo**: https://github.com/serengil/retinaface
- **Language**: Python (PyTorch/TensorFlow)
- **License**: MIT
- **Why Best**:
  - State-of-the-art face detection (WIDER FACE benchmark)
  - Bounding boxes + 5 facial landmarks
  - Multi-scale detection
  - Real-time capable (30-60 FPS)
- **Features**:
  - Face detection
  - Facial landmarks (eyes, nose, mouth corners)
  - Age, gender, emotion (via DeepFace)
- **Performance**: 30-100 FPS depending on resolution

#### MTCNN (Multi-task Cascaded CNN)
- **Repo**: https://github.com/ipazc/mtcnn
- **Language**: Python (TensorFlow)
- **License**: MIT
- **Why Consider**:
  - Lightweight, fast
  - Face detection + alignment
  - 5 facial landmarks
- **Cons**: Less accurate than RetinaFace on difficult cases

#### MediaPipe Face Detection
- **Repo**: https://github.com/google/mediapipe
- **Language**: C++ (with Python bindings)
- **License**: Apache 2.0
- **Why Consider**:
  - Optimized for mobile/edge
  - Real-time (100+ FPS)
  - Face mesh (468 landmarks)
- **Use Case**: Mobile apps, webcam applications

#### DeepFace
- **Repo**: https://github.com/serengil/deepface
- **Language**: Python
- **License**: MIT
- **Features**:
  - Face detection (RetinaFace, MTCNN, etc.)
  - Face recognition
  - Age, gender, emotion, race
  - Multiple backends (VGG-Face, Facenet, ArcFace)
- **Use Case**: Complete face analysis pipeline

---

## 6. OCR (OPTICAL CHARACTER RECOGNITION)

### 6.1 PaddleOCR - RECOMMENDED
- **Repo**: https://github.com/PaddlePaddle/PaddleOCR
- **Language**: Python (PaddlePaddle)
- **License**: Apache 2.0
- **Why Best**:
  - 80+ languages supported
  - High accuracy (95%+ on clear text)
  - Text detection + recognition
  - ONNX export for deployment
  - Active development, excellent docs
- **Features**:
  - Multilingual text detection
  - Text recognition (printed + handwritten)
  - Text direction detection (horizontal, vertical)
  - Lightweight models (8-10MB)
- **Performance**: 10-50 FPS on GPU
- **Languages**: Latin, Chinese, Arabic, Korean, Japanese, etc.

### 6.2 EasyOCR
- **Repo**: https://github.com/JaidedAI/EasyOCR
- **Language**: Python (PyTorch)
- **License**: Apache 2.0
- **Why Consider**:
  - Simple API (3 lines of code)
  - 80+ languages
  - Good accuracy
- **Cons**: Slower than PaddleOCR
- **Use Case**: Quick prototyping

### 6.3 Tesseract OCR
- **Repo**: https://github.com/tesseract-ocr/tesseract
- **Language**: C++
- **License**: Apache 2.0
- **Why Consider**:
  - Industry standard (Google-backed)
  - 100+ languages
  - Long history (30+ years)
- **Cons**:
  - Lower accuracy than deep learning methods
  - Slower than modern alternatives
- **Use Case**: Legacy systems, CPU-only environments

### 6.4 TrOCR (Transformer OCR)
- **Repo**: https://github.com/microsoft/unilm/tree/master/trocr
- **Language**: Python (PyTorch)
- **License**: MIT
- **Why Consider**:
  - Transformer-based (state-of-the-art)
  - Best for handwriting
  - HuggingFace integration
- **Cons**: Slower than CNN-based methods
- **Use Case**: Handwriting recognition, high-accuracy needs

---

## 7. EMBEDDINGS (MULTIMODAL)

### 7.1 Vision-Language Models

#### CLIP (OpenAI)
- **Repo**: https://github.com/openai/CLIP
- **Language**: Python (PyTorch)
- **License**: MIT
- **Why Best for Vision-Language**:
  - Zero-shot image classification
  - Text-to-image search
  - Image-to-text search
  - Strong generalization
- **Models**:
  - ViT-B/32 (149M params) - Fast
  - ViT-L/14 (428M params) - Accurate
- **Embedding Dim**: 512 (ViT-B/32), 768 (ViT-L/14)
- **Performance**: 100-300 images/sec on GPU
- **Use Case**: Multi-modal search, zero-shot classification
- **ONNX Support**: Yes (via HuggingFace)

#### OpenCLIP (LAION)
- **Repo**: https://github.com/mlfoundations/open_clip
- **Language**: Python (PyTorch)
- **License**: Various (depends on model)
- **Why Consider**:
  - Open-source CLIP training code
  - More model variants
  - Better performance on some tasks
- **Models**: ViT, ConvNext, EVA variants

### 7.2 Vision-Only Models

#### DINOv2 (Meta AI) - RECOMMENDED for Vision
- **Repo**: https://github.com/facebookresearch/dinov2
- **Language**: Python (PyTorch)
- **License**: Apache 2.0
- **Why Best for Pure Vision**:
  - Self-supervised learning (no labels needed)
  - Best feature extractor for images
  - Dense feature maps (useful for segmentation)
  - Outperforms CLIP on vision-only tasks
- **Models**:
  - ViT-S/14 (22M params)
  - ViT-B/14 (87M params)
  - ViT-L/14 (304M params)
  - ViT-g/14 (1.1B params)
- **Embedding Dim**: 384 (S), 768 (B), 1024 (L/g)
- **Use Case**: Image similarity, clustering, retrieval

#### ResNet / EfficientNet (Pre-trained)
- **Repos**:
  - https://github.com/pytorch/vision (torchvision models)
  - https://github.com/rwightman/pytorch-image-models (timm)
- **Why Consider**:
  - Classic architectures, widely used
  - Fast inference
  - Pre-trained on ImageNet
- **Cons**: Less powerful than transformers

### 7.3 Text Embeddings

#### Sentence-Transformers - RECOMMENDED
- **Repo**: https://github.com/UKPLab/sentence-transformers
- **Language**: Python (PyTorch)
- **License**: Apache 2.0
- **Why Best**:
  - 100+ pre-trained models
  - Optimized for semantic similarity
  - Multiple languages
  - Fast inference
- **Popular Models**:
  - `all-MiniLM-L6-v2` (22M params, 384-dim) - Fast, good quality
  - `all-mpnet-base-v2` (110M params, 768-dim) - Best quality
  - `multi-qa-MiniLM-L6-cos-v1` - Optimized for Q&A
- **Performance**: 1000+ sentences/sec on GPU
- **Use Case**: Semantic search, clustering, retrieval

#### OpenAI text-embedding-ada-002 (API)
- **Type**: API-only (not open-source)
- **Why Consider**: High quality, 1536-dim embeddings
- **Cons**: Requires API calls, cost

#### BGE (BAAI General Embedding)
- **Repo**: https://github.com/FlagOpen/FlagEmbedding
- **License**: MIT
- **Why Consider**:
  - State-of-the-art on MTEB benchmark
  - Multiple sizes (small to large)
  - Bilingual (English + Chinese)

### 7.4 Audio-Language Models

#### CLAP (Contrastive Language-Audio Pretraining)
- **Repo**: https://github.com/LAION-AI/CLAP
- **Language**: Python (PyTorch)
- **License**: MIT
- **Why Best for Audio**:
  - Audio-text multimodal embeddings
  - Zero-shot audio classification
  - Text-to-audio search
  - Trained on LAION-Audio-630K
- **Embedding Dim**: 512
- **Use Case**: Audio similarity, sound search
- **Performance**: Real-time capable

#### AudioCLIP
- **Repo**: https://github.com/AndreyGuzhov/AudioCLIP
- **License**: MIT
- **Why Consider**:
  - Extends CLIP to audio
  - Image-audio-text tri-modal

### 7.5 Audio-Only Embeddings

#### wav2vec 2.0 (Facebook AI)
- **Repo**: https://github.com/facebookresearch/fairseq/tree/main/examples/wav2vec
- **License**: MIT
- **Use Case**: Speech embeddings, speaker verification

#### VGGish (Google)
- **Repo**: https://github.com/tensorflow/models/tree/master/research/audioset/vggish
- **License**: Apache 2.0
- **Use Case**: Audio event embeddings (AudioSet)

---

## 8. AUDIO SOURCE SEPARATION

### 8.1 Demucs (Meta AI) - RECOMMENDED
- **Repo**: https://github.com/facebookresearch/demucs
- **Language**: Python (PyTorch)
- **License**: MIT
- **Why Best**:
  - State-of-the-art music separation
  - 4 stems (vocals, drums, bass, other)
  - Real-time capable (Hybrid Transformer v4)
  - Pretrained models
- **Models**:
  - HTDemucs (Hybrid Transformer) - Latest, best quality
  - MDX (Music Demixing Challenge) - Competition winner
- **Performance**: 1x real-time on GPU (HTDemucs)
- **Quality**: SDR (Signal-to-Distortion Ratio) 7-10 dB
- **Use Case**: Music separation, karaoke, remixing

### 8.2 Spleeter (Deezer)
- **Repo**: https://github.com/deezer/spleeter
- **Language**: Python (TensorFlow)
- **License**: MIT
- **Why Consider**:
  - Fast (10x real-time on GPU)
  - Production-ready
  - Pre-trained models (2, 4, 5 stems)
- **Cons**: Lower quality than Demucs
- **Use Case**: Real-time applications, low-latency needs

### 8.3 Open-Unmix
- **Repo**: https://github.com/sigsep/open-unmix-pytorch
- **Language**: Python (PyTorch)
- **License**: MIT
- **Why Consider**:
  - Open-source baseline
  - Research-friendly
- **Cons**: Lower quality than Demucs/Spleeter

---

## 9. AUDIO CLASSIFICATION & EVENT DETECTION

### 9.1 PANNs (Pre-trained Audio Neural Networks) - RECOMMENDED
- **Repo**: https://github.com/qiuqiangkong/audioset_tagging_cnn
- **Language**: Python (PyTorch)
- **License**: MIT
- **Why Best**:
  - Trained on AudioSet (2M+ audio clips, 527 classes)
  - High accuracy (mAP 0.43)
  - Multiple architectures (CNN14, ResNet, EfficientNet)
  - Fast inference
- **Classes**: 527 (music, speech, laughter, applause, animals, etc.)
- **Performance**: 100+ clips/sec on GPU
- **Use Case**: Audio event detection, sound classification

### 9.2 YAMNet (Google)
- **Repo**: https://github.com/tensorflow/models/tree/master/research/audioset/yamnet
- **Language**: Python (TensorFlow)
- **License**: Apache 2.0
- **Why Consider**:
  - Lightweight (3.7M params)
  - 521 AudioSet classes
  - TensorFlow Hub integration
- **Cons**: Lower accuracy than PANNs
- **Use Case**: Mobile, edge devices

### 9.3 BEATs (Microsoft)
- **Repo**: https://github.com/microsoft/unilm/tree/master/beats
- **License**: MIT
- **Why Consider**:
  - State-of-the-art on AudioSet
  - Self-supervised pre-training
- **Status**: Research, less production-ready

---

## 10. VECTOR DATABASES

### 10.1 Qdrant - RECOMMENDED
- **Repo**: https://github.com/qdrant/qdrant
- **Language**: Rust
- **License**: Apache 2.0
- **Why Best**:
  - Rust-native (blazing fast, memory safe)
  - Advanced filtering (metadata + vector search)
  - Hybrid search (dense + sparse vectors)
  - Distributed architecture
  - gRPC + REST APIs
  - Active development
- **Features**:
  - Billion-scale vectors
  - Multiple distance metrics (cosine, euclidean, dot product)
  - Quantization (scalar, product)
  - Payload indexing (filter before search)
- **Performance**: 10-100ms query latency (millions of vectors)
- **Rust Client**: Official `qdrant-client` crate

### 10.2 Milvus
- **Repo**: https://github.com/milvus-io/milvus
- **Language**: Go, C++
- **License**: Apache 2.0
- **Why Consider**:
  - Battle-tested at scale (billions of vectors)
  - GPU indexing support (IVF_GPU)
  - Rich ecosystem (Attu UI, Feder visualizer)
  - Multiple index types (IVF, HNSW, DiskANN)
- **Cons**: More complex setup than Qdrant
- **Use Case**: Massive scale (10B+ vectors)

### 10.3 Weaviate
- **Repo**: https://github.com/weaviate/weaviate
- **Language**: Go
- **License**: BSD-3-Clause
- **Why Consider**:
  - GraphQL API
  - Built-in ML modules (transformers, GPT)
  - Hybrid search (BM25 + vector)
  - Multi-tenancy
- **Cons**: More opinionated than Qdrant/Milvus

### 10.4 Chroma
- **Repo**: https://github.com/chroma-core/chroma
- **Language**: Python
- **License**: Apache 2.0
- **Why Consider**:
  - Lightweight, embeddable
  - LangChain integration
  - Simple API
- **Cons**: Less mature, smaller scale

### 10.5 pgvector (PostgreSQL Extension)
- **Repo**: https://github.com/pgvector/pgvector
- **Language**: C
- **License**: PostgreSQL License
- **Why Consider**:
  - No additional infrastructure (uses Postgres)
  - ACID transactions
  - Familiar SQL interface
- **Cons**: Slower than specialized vector DBs

---

## 11. SEARCH ENGINES (FULL-TEXT)

### 11.1 Tantivy - RECOMMENDED for Rust
- **Repo**: https://github.com/quickwit-oss/tantivy
- **Language**: Rust
- **License**: MIT
- **Why Best for Rust**:
  - Lucene-inspired, but pure Rust
  - Blazing fast indexing (1M+ docs/sec)
  - Low memory footprint
  - Excellent for embedding in Rust apps
- **Features**:
  - Full-text search (BM25)
  - Faceted search
  - Phrase queries, fuzzy search
  - Real-time indexing
- **Performance**: <10ms query latency (millions of docs)

### 11.2 Meilisearch
- **Repo**: https://github.com/meilisearch/meilisearch
- **Language**: Rust
- **License**: MIT
- **Why Consider**:
  - Typo-tolerant search (out of the box)
  - Instant search (as-you-type)
  - Simple RESTful API
  - Faceted search, filtering
  - Multi-tenancy
- **Use Case**: User-facing search (web apps)
- **Cons**: Not as low-level as Tantivy

### 11.3 Elasticsearch
- **Repo**: https://github.com/elastic/elasticsearch
- **Language**: Java
- **License**: SSPL (not OSI-approved) / Elastic License
- **Why Consider**:
  - Industry standard
  - Distributed by default
  - Rich ecosystem (Kibana, Logstash)
- **Cons**:
  - Heavy (Java, high memory usage)
  - Licensing concerns (SSPL)
- **Rust Client**: `elasticsearch` crate

### 11.4 Apache Solr
- **Repo**: https://github.com/apache/solr
- **Language**: Java
- **License**: Apache 2.0
- **Why Consider**:
  - Mature, feature-rich
  - True open-source (Apache 2.0)
- **Cons**: Losing popularity to Elasticsearch

### 11.5 OpenSearch (Elasticsearch fork)
- **Repo**: https://github.com/opensearch-project/OpenSearch
- **Language**: Java
- **License**: Apache 2.0
- **Why Consider**:
  - AWS-backed Elasticsearch fork
  - True open-source
  - AWS OpenSearch Service
- **Use Case**: If you need Elasticsearch but want OSI license

---

## 12. DATABASES

### 12.1 Metadata & Structured Data

#### PostgreSQL 16+ - RECOMMENDED
- **Repo**: https://github.com/postgres/postgres
- **License**: PostgreSQL License (permissive)
- **Why Best**:
  - Industry standard, rock-solid reliability
  - JSONB (flexible schema)
  - Full-text search (built-in)
  - Extensions (TimescaleDB, pgvector, PostGIS)
  - Advanced indexing (GIN, GIST, BRIN)
- **Rust Client**: `sqlx`, `tokio-postgres`, `diesel`
- **Use Case**: Metadata, timelines, job state

#### SurrealDB
- **Repo**: https://github.com/surrealdb/surrealdb
- **Language**: Rust
- **License**: BSL 1.1 (converts to Apache 2.0 after 4 years)
- **Why Consider**:
  - Rust-native
  - Multi-model (document, graph, key-value)
  - Real-time subscriptions
  - Embedded or distributed
- **Cons**: Newer, less battle-tested
- **Use Case**: Modern apps, Rust-first stack

#### SQLite (Embedded)
- **Repo**: https://github.com/sqlite/sqlite
- **License**: Public Domain
- **Why Consider**:
  - Embedded, zero-config
  - Single file database
  - ACID transactions
- **Rust Client**: `rusqlite`, `sqlx`
- **Use Case**: Embedded apps, local storage

### 12.2 Key-Value Stores

#### Redis
- **Repo**: https://github.com/redis/redis
- **License**: BSD-3-Clause (core), RSALv2/SSPL (modules)
- **Why Best**:
  - In-memory, ultra-fast (sub-millisecond)
  - Data structures (strings, lists, sets, hashes, sorted sets)
  - Pub/sub messaging
  - Persistence (RDB snapshots, AOF log)
- **Rust Client**: `redis-rs`
- **Use Case**: Cache, job queue, session storage

#### Redb (Embedded)
- **Repo**: https://github.com/cberner/redb
- **Language**: Rust
- **License**: MIT
- **Why Consider**:
  - Pure Rust, embedded KV store
  - ACID transactions
  - No external dependencies (not even C libraries)
  - Inspired by LMDB
- **Use Case**: Rust apps needing embedded storage

#### RocksDB
- **Repo**: https://github.com/facebook/rocksdb
- **Language**: C++
- **License**: Apache 2.0 / GPL 2.0
- **Why Consider**:
  - Optimized for SSDs
  - Used in production (Meta, LinkedIn)
  - High write throughput
- **Rust Bindings**: `rocksdb`

---

## 13. MESSAGE QUEUES & EVENT STREAMING

### 13.1 NATS - RECOMMENDED for Rust
- **Repo**: https://github.com/nats-io/nats-server
- **Language**: Go
- **License**: Apache 2.0
- **Why Best for Rust**:
  - Excellent Rust client (`async-nats`)
  - Lightweight, simple
  - High throughput (millions of messages/sec)
  - Multiple patterns (pub/sub, request/reply, queue groups)
  - JetStream (persistence, exactly-once delivery)
- **Features**:
  - At-most-once, at-least-once, exactly-once delivery
  - Distributed architecture
  - Message replay
- **Use Case**: Job queues, event streaming

### 13.2 Apache Kafka
- **Repo**: https://github.com/apache/kafka
- **Language**: Java, Scala
- **License**: Apache 2.0
- **Why Consider**:
  - Industry standard for event streaming
  - Battle-tested at scale (LinkedIn, Netflix)
  - Strong durability guarantees
  - Rich ecosystem (Connect, Streams)
- **Rust Client**: `rdkafka` (librdkafka bindings)
- **Cons**: Complex setup, heavy
- **Use Case**: Large-scale event streaming, log aggregation

### 13.3 RabbitMQ
- **Repo**: https://github.com/rabbitmq/rabbitmq-server
- **Language**: Erlang
- **License**: MPL 2.0
- **Why Consider**:
  - Mature, reliable
  - Multiple messaging patterns
  - AMQP protocol
- **Rust Client**: `lapin`
- **Cons**: Slower than NATS/Kafka

### 13.4 Redis Streams
- **Part of Redis**
- **Why Consider**:
  - Simple (same Redis instance)
  - Append-only log
  - Consumer groups
- **Cons**: Not as feature-rich as Kafka

---

## 14. OBJECT STORAGE

### 14.1 MinIO - RECOMMENDED for Self-Hosted
- **Repo**: https://github.com/minio/minio
- **Language**: Go
- **License**: AGPL 3.0
- **Why Best**:
  - S3-compatible API
  - High performance (100+ GB/s throughput)
  - Multi-cloud gateway
  - Erasure coding (reliability)
  - Encryption at rest
- **Rust Client**: `s3` crate (works with any S3-compatible storage)
- **Use Case**: Self-hosted object storage

### 14.2 Amazon S3 (Cloud)
- **Type**: Managed service
- **Why Consider**: Industry standard, 99.999999999% durability
- **Rust SDK**: `aws-sdk-s3`

### 14.3 Ceph
- **Repo**: https://github.com/ceph/ceph
- **Language**: C++
- **License**: LGPL 2.1
- **Why Consider**:
  - Open-source, distributed storage
  - S3-compatible (RadosGW)
  - Block, file, object storage
- **Cons**: Complex setup

### 14.4 SeaweedFS
- **Repo**: https://github.com/seaweedfs/seaweedfs
- **Language**: Go
- **License**: Apache 2.0
- **Why Consider**:
  - Fast, simple
  - S3-compatible
  - Optimized for small files
- **Use Case**: Alternative to MinIO for small file storage

---

## 15. ORCHESTRATION & INFRASTRUCTURE

### 15.1 Container Orchestration

#### Kubernetes
- **Repo**: https://github.com/kubernetes/kubernetes
- **License**: Apache 2.0
- **Why Industry Standard**:
  - Container orchestration (Docker, containerd)
  - Auto-scaling (HPA, VPA)
  - Service mesh (Istio, Linkerd)
  - Rich ecosystem
- **Use Case**: Production deployments, microservices

### 15.2 Model Serving

#### KServe (formerly KFServing)
- **Repo**: https://github.com/kserve/kserve
- **License**: Apache 2.0
- **Why Best for ML**:
  - ML model serving on Kubernetes
  - Multiple frameworks (TensorFlow, PyTorch, ONNX, TensorRT)
  - Auto-scaling (scale to zero)
  - Canary deployments, A/B testing
- **Use Case**: Serving ML models at scale

#### Triton Inference Server (NVIDIA)
- **Repo**: https://github.com/triton-inference-server/server
- **License**: BSD-3-Clause
- **Why Consider**:
  - NVIDIA-optimized
  - Dynamic batching
  - Multi-model serving
  - Multiple backends (TensorRT, ONNX, PyTorch)
- **Use Case**: GPU-bound inference

#### TorchServe (PyTorch)
- **Repo**: https://github.com/pytorch/serve
- **License**: Apache 2.0
- **Why Consider**:
  - Official PyTorch serving
  - Easy deployment
- **Cons**: Less feature-rich than Triton/KServe

---

## 16. OBSERVABILITY & MONITORING

### 16.1 Metrics

#### Prometheus
- **Repo**: https://github.com/prometheus/prometheus
- **License**: Apache 2.0
- **Why Best**:
  - Industry standard for metrics
  - Time-series database
  - Powerful query language (PromQL)
  - Service discovery
- **Rust Client**: `prometheus` crate
- **Use Case**: Application metrics, alerting

#### Grafana
- **Repo**: https://github.com/grafana/grafana
- **License**: AGPL 3.0
- **Why Best**:
  - Visualization platform
  - Dashboards for Prometheus, Loki, etc.
  - Alerting
- **Use Case**: Monitoring dashboards

### 16.2 Tracing

#### OpenTelemetry
- **Repo**: https://github.com/open-telemetry/opentelemetry-rust
- **License**: Apache 2.0
- **Why Best**:
  - Vendor-neutral standard
  - Traces, metrics, logs
  - Instrumentation libraries
- **Rust Crates**: `opentelemetry`, `tracing-opentelemetry`
- **Use Case**: Distributed tracing

#### Jaeger
- **Repo**: https://github.com/jaegertracing/jaeger
- **License**: Apache 2.0
- **Why Best**:
  - Distributed tracing backend
  - OpenTelemetry compatible
  - UI for trace visualization
- **Use Case**: Performance debugging

### 16.3 Logging

#### Grafana Loki
- **Repo**: https://github.com/grafana/loki
- **License**: AGPL 3.0
- **Why Best**:
  - Like Prometheus, but for logs
  - Label-based indexing (cost-efficient)
  - Grafana integration
- **Use Case**: Centralized logging

---

## 17. RUST ML & DATA SCIENCE ECOSYSTEM

### 17.1 ONNX Runtime (Rust Bindings)
- **Repo**: https://github.com/pykeio/ort
- **Crate**: `ort`
- **License**: MIT/Apache 2.0
- **Why Best for Rust**:
  - Official ONNX Runtime Rust bindings
  - Cross-platform (CUDA, ROCm, CoreML, DirectML)
  - Production-ready
- **Backends**:
  - CPU (default)
  - CUDA (NVIDIA)
  - TensorRT (NVIDIA, optimized)
  - ROCm (AMD)
  - CoreML (Apple)
  - DirectML (Windows)

### 17.2 PyO3
- **Repo**: https://github.com/PyO3/pyo3
- **Crate**: `pyo3`
- **License**: MIT/Apache 2.0
- **Why Critical**:
  - Rust ↔ Python interop
  - Call Python ML libraries from Rust
  - Create Python extensions in Rust
- **Use Case**: Bridging Rust orchestration with Python ML models

### 17.3 ndarray
- **Repo**: https://github.com/rust-ndarray/ndarray
- **Crate**: `ndarray`
- **License**: MIT/Apache 2.0
- **Purpose**: N-dimensional arrays (like NumPy)
- **Use Case**: Tensor operations, image processing

### 17.4 image
- **Repo**: https://github.com/image-rs/image
- **Crate**: `image`
- **License**: MIT
- **Purpose**: Image encoding/decoding, manipulation
- **Formats**: PNG, JPEG, GIF, WebP, TIFF, BMP

### 17.5 imageproc
- **Repo**: https://github.com/image-rs/imageproc
- **Crate**: `imageproc`
- **License**: MIT
- **Purpose**: Image processing operations
- **Features**: Filters, transformations, drawing

---

## 18. SPECIALIZED TOOLS

### 18.1 Video Quality Assessment

#### VMAF (Netflix)
- **Repo**: https://github.com/Netflix/vmaf
- **License**: BSD+Patent
- **Purpose**: Perceptual video quality metric
- **Use Case**: Evaluate transcoding quality

### 18.2 Audio Processing

#### librosa (Python)
- **Repo**: https://github.com/librosa/librosa
- **License**: ISC
- **Purpose**: Audio analysis (Python)
- **Features**: Spectrogram, MFCC, beat tracking

#### aubio
- **Repo**: https://github.com/aubio/aubio
- **Language**: C
- **License**: GPL 3.0
- **Purpose**: Audio segmentation (onset detection, pitch, tempo)

### 18.3 Caption/Subtitle Tools

#### pysubs2
- **Repo**: https://github.com/tkarabela/pysubs2
- **Language**: Python
- **License**: MIT
- **Purpose**: Subtitle parsing (SRT, ASS, SSA, WebVTT)

#### webvtt-py
- **Repo**: https://github.com/glut23/webvtt-py
- **Language**: Python
- **License**: MIT
- **Purpose**: WebVTT parsing and manipulation

---

## 19. SUMMARY: RECOMMENDED STACK

### Core Processing (Rust/C++)
1. **FFmpeg 7** - Video/audio encoding/decoding
2. **Rust** - Orchestration, CPU processing
3. **ONNX Runtime** - ML inference (GPU)
4. **PyO3** - Rust ↔ Python bridge

### ML Models (GPU)
1. **Faster-Whisper** - Speech-to-text (best quality/speed)
2. **PyAnnote.audio** - Speaker diarization
3. **YOLOv8** - Object detection
4. **RetinaFace** - Face detection
5. **PaddleOCR** - Text recognition
6. **TransNetV2** - Scene detection (ML)
7. **CLIP/DINOv2** - Visual embeddings
8. **Sentence-Transformers** - Text embeddings
9. **CLAP** - Audio embeddings
10. **Demucs** - Audio source separation
11. **PANNs** - Audio event detection

### Storage & Indexing
1. **MinIO** - Object storage (S3-compatible)
2. **Qdrant** - Vector database (Rust-native)
3. **Tantivy/Meilisearch** - Full-text search (Rust)
4. **PostgreSQL 16** - Metadata database
5. **Redis** - Cache layer

### Infrastructure
1. **NATS** - Message queue (Rust-friendly)
2. **Kubernetes** - Container orchestration
3. **KServe** - ML model serving
4. **Prometheus + Grafana** - Monitoring
5. **OpenTelemetry + Jaeger** - Tracing

### Estimated Performance
- **Throughput**: 2-10x real-time (10-min video in 1-5 min)
- **Scalability**: 500-1000+ concurrent jobs (100 GPU nodes)
- **Accuracy**: 90-95%+ on core tasks (transcription, detection)

---

## 20. LICENSING SUMMARY

### Fully Open Source (Permissive)
- **MIT**: Whisper, CLIP, Faster-Whisper, PyAnnote, Sentence-Transformers, Redis, Tantivy, Meilisearch
- **Apache 2.0**: YOLOv8, DINOv2, PaddleOCR, PostgreSQL, Qdrant, NATS, Kafka

### Copyleft
- **GPL 3.0**: Aubio, some FFmpeg builds
- **AGPL 3.0**: MinIO, Grafana

### Special Licenses
- **YOLOv8**: AGPL 3.0 (commercial license available)
- **FFmpeg**: LGPL 2.1+ (or GPL if compiled with GPL components)

**Recommendation**: Prefer MIT/Apache 2.0 for commercial projects. Be aware of AGPL (requires releasing modified source if distributed).

---

This comprehensive list provides the **absolute best open-source software** for building a world-class video and audio extraction system. All tools are production-ready, actively maintained, and have strong communities.
