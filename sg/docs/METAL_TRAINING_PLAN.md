# Metal/C++ Training Acceleration Plan

**Goal:** Port XTR training bottlenecks to C++/Metal for 10-50x speedup on Apple Silicon.

**Current State:** Training at 9 samples/s on M2, ETA ~12 days for 9M examples.

**Target:** 100+ samples/s, ETA ~1 day.

---

## Phase 1: Profile and Identify Bottlenecks

**Task:** Instrument current Python training to find exact bottlenecks.

```python
# Add to train() function:
import time
timings = {"data": 0, "forward": 0, "maxsim": 0, "loss": 0, "backward": 0}

t0 = time.time()
# ... data loading ...
timings["data"] += time.time() - t0

t0 = time.time()
# ... model forward ...
timings["forward"] += time.time() - t0

# etc.
```

**Expected bottlenecks (hypothesis):**
1. MaxSim scoring: O(B×Q×B×K) = O(24×128×24×384) = 28M ops per batch
2. Loss computation: Multiple reductions, softmax, cross-entropy
3. Backward pass: Autograd overhead
4. Data loading: Tokenization, batching (already cached)

**Deliverable:** Timing breakdown showing % time in each component.

---

## Phase 2: Design Metal Kernels

### 2.1 MaxSim Kernel (Highest Priority)

MaxSim is the core of XTR multi-vector retrieval:

```
For each query q_i and document d_j:
  score[i,j] = (1/|q_i|) * Σ_t max_k(q_i[t] · d_j[k])
```

**Current Python (slow):**
```python
def maxsim_scores(query_emb, query_mask, doc_emb, doc_mask):
    # [B, Q, D] x [B, K, D]^T -> [B, Q, B, K]
    all_sims = torch.einsum('iqd,jkd->iqjk', query_emb, doc_emb)
    # ... masking and reduction ...
```

**Proposed Metal kernel:**
```metal
kernel void maxsim_forward(
    device const float* query_emb [[buffer(0)]],  // [B, Q, D]
    device const float* doc_emb [[buffer(1)]],    // [B, K, D]
    device const int* query_mask [[buffer(2)]],   // [B, Q]
    device const int* doc_mask [[buffer(3)]],     // [B, K]
    device float* scores [[buffer(4)]],           // [B, B]
    constant uint& B [[buffer(5)]],
    constant uint& Q [[buffer(6)]],
    constant uint& K [[buffer(7)]],
    constant uint& D [[buffer(8)]],
    uint2 gid [[thread_position_in_grid]]
) {
    uint i = gid.x;  // query batch index
    uint j = gid.y;  // doc batch index

    float score = 0.0;
    int query_len = 0;

    for (uint t = 0; t < Q; t++) {
        if (query_mask[i * Q + t] == 0) continue;
        query_len++;

        float max_sim = -INFINITY;
        for (uint k = 0; k < K; k++) {
            if (doc_mask[j * K + k] == 0) continue;

            // Dot product q[i,t] · d[j,k]
            float dot = 0.0;
            for (uint d = 0; d < D; d++) {
                dot += query_emb[(i * Q + t) * D + d] *
                       doc_emb[(j * K + k) * D + d];
            }
            max_sim = max(max_sim, dot);
        }
        score += max_sim;
    }

    scores[i * B + j] = score / float(query_len);
}
```

**Optimizations:**
- Use threadgroup memory for query/doc tile caching
- Process multiple (i,j) pairs per thread
- Use SIMD for dot products (float4)
- Fuse L2 normalization into kernel

### 2.2 MNR Loss Kernel

Multiple Negatives Ranking loss with in-batch negatives:

```metal
kernel void mnr_loss_forward(
    device const float* scores [[buffer(0)]],  // [B, B]
    device float* loss [[buffer(1)]],          // scalar
    device float* gradients [[buffer(2)]],     // [B, B]
    constant float& scale [[buffer(3)]],
    constant uint& B [[buffer(4)]],
    uint tid [[thread_position_in_grid]]
) {
    // Each thread handles one row (one query)
    uint i = tid;
    if (i >= B) return;

    // Compute softmax denominator
    float max_val = -INFINITY;
    for (uint j = 0; j < B; j++) {
        max_val = max(max_val, scores[i * B + j] * scale);
    }

    float sum_exp = 0.0;
    for (uint j = 0; j < B; j++) {
        sum_exp += exp(scores[i * B + j] * scale - max_val);
    }

    // Loss = -log(softmax[i,i])
    float log_softmax_ii = scores[i * B + i] * scale - max_val - log(sum_exp);

    // Atomic add to total loss
    atomic_fetch_add_explicit((device atomic_float*)loss,
                              -log_softmax_ii / float(B),
                              memory_order_relaxed);

    // Compute gradients for backward pass
    for (uint j = 0; j < B; j++) {
        float softmax_ij = exp(scores[i * B + j] * scale - max_val) / sum_exp;
        float target = (i == j) ? 1.0 : 0.0;
        gradients[i * B + j] = scale * (softmax_ij - target) / float(B);
    }
}
```

### 2.3 Margin Loss Kernel

Simple pairwise margin loss:

```metal
kernel void margin_loss_forward(
    device const float* scores [[buffer(0)]],  // [B, B]
    device float* loss [[buffer(1)]],
    device float* gradients [[buffer(2)]],
    constant float& margin [[buffer(3)]],
    constant uint& B [[buffer(4)]],
    uint2 gid [[thread_position_in_grid]]
) {
    uint i = gid.x;  // query index
    uint j = gid.y;  // negative doc index

    if (i >= B || j >= B || i == j) return;

    float pos_score = scores[i * B + i];
    float neg_score = scores[i * B + j];
    float loss_ij = max(0.0f, margin - pos_score + neg_score);

    if (loss_ij > 0) {
        atomic_fetch_add_explicit((device atomic_float*)loss,
                                  loss_ij / float(B * (B - 1)),
                                  memory_order_relaxed);
        // Gradient: -1 for positive, +1 for negative
        atomic_fetch_add_explicit((device atomic_float*)&gradients[i * B + i],
                                  -1.0f / float(B * (B - 1)),
                                  memory_order_relaxed);
        atomic_fetch_add_explicit((device atomic_float*)&gradients[i * B + j],
                                  1.0f / float(B * (B - 1)),
                                  memory_order_relaxed);
    }
}
```

---

## Phase 3: C++ Integration Layer

### 3.1 Project Structure

```
sg/
├── crates/
│   └── sg-metal-train/        # New Rust crate
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs         # Rust bindings
│       │   └── ffi.rs         # C FFI
│       ├── cpp/
│       │   ├── metal_ops.h
│       │   ├── metal_ops.mm   # Objective-C++ Metal wrapper
│       │   └── kernels/
│       │       ├── maxsim.metal
│       │       ├── mnr_loss.metal
│       │       └── margin_loss.metal
│       └── build.rs           # Compile Metal shaders
└── scripts/
    └── train_xtr_metal.py     # Python training with Metal ops
```

### 3.2 Rust FFI (sg-metal-train/src/ffi.rs)

```rust
use std::ffi::c_void;

#[repr(C)]
pub struct MetalContext {
    device: *mut c_void,
    command_queue: *mut c_void,
    maxsim_pipeline: *mut c_void,
    mnr_loss_pipeline: *mut c_void,
    margin_loss_pipeline: *mut c_void,
}

#[no_mangle]
pub extern "C" fn metal_init() -> *mut MetalContext {
    // Initialize Metal device and compile shaders
}

#[no_mangle]
pub extern "C" fn metal_maxsim_forward(
    ctx: *mut MetalContext,
    query_emb: *const f32,  // [B, Q, D]
    doc_emb: *const f32,    // [B, K, D]
    query_mask: *const i32, // [B, Q]
    doc_mask: *const i32,   // [B, K]
    scores: *mut f32,       // [B, B] output
    b: u32, q: u32, k: u32, d: u32,
) -> i32 {
    // Dispatch Metal kernel
}

#[no_mangle]
pub extern "C" fn metal_mnr_loss_forward(
    ctx: *mut MetalContext,
    scores: *const f32,     // [B, B]
    loss: *mut f32,         // scalar output
    gradients: *mut f32,    // [B, B] output
    scale: f32,
    b: u32,
) -> i32 {
    // Dispatch Metal kernel
}
```

### 3.3 Python Bindings (via ctypes or PyO3)

```python
# scripts/metal_ops.py
import ctypes
import numpy as np
from pathlib import Path

# Load the compiled library
_lib = ctypes.CDLL(str(Path(__file__).parent.parent / "target/release/libsg_metal_train.dylib"))

class MetalTrainer:
    def __init__(self):
        self.ctx = _lib.metal_init()

    def maxsim_forward(self, query_emb, doc_emb, query_mask, doc_mask):
        """
        Args:
            query_emb: torch.Tensor [B, Q, D]
            doc_emb: torch.Tensor [B, K, D]
        Returns:
            scores: torch.Tensor [B, B]
        """
        B, Q, D = query_emb.shape
        K = doc_emb.shape[1]

        # Ensure contiguous numpy arrays
        q_ptr = query_emb.detach().cpu().numpy().ctypes.data_as(ctypes.POINTER(ctypes.c_float))
        d_ptr = doc_emb.detach().cpu().numpy().ctypes.data_as(ctypes.POINTER(ctypes.c_float))
        qm_ptr = query_mask.cpu().numpy().ctypes.data_as(ctypes.POINTER(ctypes.c_int))
        dm_ptr = doc_mask.cpu().numpy().ctypes.data_as(ctypes.POINTER(ctypes.c_int))

        scores = np.zeros((B, B), dtype=np.float32)
        s_ptr = scores.ctypes.data_as(ctypes.POINTER(ctypes.c_float))

        _lib.metal_maxsim_forward(self.ctx, q_ptr, d_ptr, qm_ptr, dm_ptr, s_ptr, B, Q, K, D)

        return torch.from_numpy(scores).to(query_emb.device)
```

---

## Phase 4: PyTorch Custom Autograd Integration

To use Metal ops in training with automatic differentiation:

```python
# scripts/metal_autograd.py
import torch
from metal_ops import MetalTrainer

_trainer = MetalTrainer()

class MetalMaxSimFunction(torch.autograd.Function):
    @staticmethod
    def forward(ctx, query_emb, doc_emb, query_mask, doc_mask):
        scores = _trainer.maxsim_forward(query_emb, doc_emb, query_mask, doc_mask)
        ctx.save_for_backward(query_emb, doc_emb, query_mask, doc_mask)
        return scores

    @staticmethod
    def backward(ctx, grad_scores):
        query_emb, doc_emb, query_mask, doc_mask = ctx.saved_tensors
        # Call Metal backward kernel
        grad_query, grad_doc = _trainer.maxsim_backward(
            grad_scores, query_emb, doc_emb, query_mask, doc_mask
        )
        return grad_query, grad_doc, None, None

class MetalMNRLossFunction(torch.autograd.Function):
    @staticmethod
    def forward(ctx, scores, scale):
        loss, gradients = _trainer.mnr_loss_forward(scores, scale)
        ctx.save_for_backward(gradients)
        ctx.scale = scale
        return loss

    @staticmethod
    def backward(ctx, grad_loss):
        gradients, = ctx.saved_tensors
        return gradients * grad_loss, None

# Drop-in replacements
def metal_maxsim_scores(query_emb, query_mask, doc_emb, doc_mask):
    return MetalMaxSimFunction.apply(query_emb, doc_emb, query_mask, doc_mask)

def metal_mnr_loss(scores, scale=20.0):
    return MetalMNRLossFunction.apply(scores, scale)
```

---

## Phase 5: Implementation Steps (Estimated: 50-60 AI Commits)

### Phase 5.1: Profiling & Baseline (3 commits)
1. [ ] Instrument training code with detailed timers
2. [ ] Run profiling, generate baseline metrics
3. [ ] Document bottleneck analysis with data

### Phase 5.2: Project Setup (4 commits)
4. [ ] Create sg-metal-train crate structure + Cargo.toml
5. [ ] Set up build.rs for Metal shader compilation
6. [ ] Implement Metal context initialization (device, queue)
7. [ ] Verify Metal setup with simple kernel test

### Phase 5.3: Testing Infrastructure (5 commits)
8. [ ] Create test harness for Python↔Metal comparison
9. [ ] Generate golden outputs from Python implementation
10. [ ] Implement numerical tolerance checker (atol=1e-5, rtol=1e-4)
11. [ ] Create edge case test data (empty masks, B=1, max lengths)
12. [ ] Set up property-based test framework (hypothesis)

### Phase 5.4: MaxSim Kernel (12 commits)
13. [ ] MaxSim forward kernel - basic implementation
14. [ ] MaxSim forward - unit tests vs Python golden outputs
15. [ ] MaxSim forward - edge case tests
16. [ ] MaxSim forward - optimize with threadgroup tiling
17. [ ] MaxSim forward - optimize with SIMD float4
18. [ ] MaxSim forward - benchmark and validate speedup
19. [ ] MaxSim backward kernel - gradient w.r.t. query_emb
20. [ ] MaxSim backward kernel - gradient w.r.t. doc_emb
21. [ ] MaxSim backward - unit tests vs torch.autograd.gradcheck
22. [ ] MaxSim backward - numerical gradient verification
23. [ ] MaxSim backward - edge case gradient tests
24. [ ] MaxSim - integration test (forward + backward matches PyTorch)

### Phase 5.5: MNR Loss Kernel (8 commits)
25. [ ] MNR loss forward kernel
26. [ ] MNR loss forward - unit tests vs Python
27. [ ] MNR loss forward - numerical stability tests (large scores)
28. [ ] MNR loss backward kernel
29. [ ] MNR loss backward - gradient tests
30. [ ] MNR loss backward - numerical gradient verification
31. [ ] MNR loss - edge cases (B=1, identical scores)
32. [ ] MNR loss - integration test

### Phase 5.6: Margin Loss Kernel (6 commits)
33. [ ] Margin loss forward kernel
34. [ ] Margin loss forward - unit tests
35. [ ] Margin loss backward kernel
36. [ ] Margin loss backward - gradient tests
37. [ ] Margin loss - edge cases (all violations, no violations)
38. [ ] Margin loss - integration test

### Phase 5.7: C++/Rust Integration (6 commits)
39. [ ] Rust FFI definitions (metal_ops.rs)
40. [ ] Objective-C++ Metal wrapper (metal_ops.mm)
41. [ ] Buffer management and memory pooling
42. [ ] Error handling and validation
43. [ ] Integration tests for FFI layer
44. [ ] Memory leak tests (instruments)

### Phase 5.8: Python Bindings (5 commits)
45. [ ] ctypes bindings for all kernels
46. [ ] PyTorch tensor ↔ Metal buffer conversion
47. [ ] Python unit tests for bindings
48. [ ] Benchmark Python overhead
49. [ ] Documentation and type hints

### Phase 5.9: PyTorch Autograd Integration (6 commits)
50. [ ] MetalMaxSimFunction (forward + backward)
51. [ ] MetalMNRLossFunction (forward + backward)
52. [ ] MetalMarginLossFunction (forward + backward)
53. [ ] torch.autograd.gradcheck tests for all functions
54. [ ] Drop-in replacement test (swap in training loop)
55. [ ] End-to-end training step comparison (loss, gradients)

### Phase 5.10: Optimization & Validation (5 commits)
56. [ ] Fuse L2 normalization into MaxSim kernel
57. [ ] Memory optimization (buffer reuse, pooling)
58. [ ] Full training run comparison (100 steps, compare losses)
59. [ ] Performance benchmarks and documentation
60. [ ] Final cleanup and CI integration

---

## Commit Estimate Summary

| Phase | Commits | Description |
|-------|---------|-------------|
| Profiling | 3 | Baseline metrics |
| Setup | 4 | Crate structure, Metal init |
| Test Infra | 5 | Harness, golden outputs, edge cases |
| MaxSim | 12 | Forward, backward, tests, optimization |
| MNR Loss | 8 | Forward, backward, tests |
| Margin Loss | 6 | Forward, backward, tests |
| C++/Rust | 6 | FFI, wrapper, memory |
| Python | 5 | Bindings, conversion |
| Autograd | 6 | PyTorch integration |
| Optimization | 5 | Fusing, polish |
| **Total** | **60** | ~12 hours AI work |

**Risk buffer:** Add 10-15 commits for debugging Metal shader issues, MPS quirks, and numerical precision problems.

**Realistic total: 60-75 commits**

---

## Phase 6: Expected Performance

### Theoretical Analysis

**MaxSim kernel:**
- Current: PyTorch einsum on MPS, ~100ms per batch
- Metal optimized: ~5-10ms per batch (10-20x speedup)
- Reason: Custom kernel avoids intermediate tensor allocation

**Loss computation:**
- Current: Multiple PyTorch ops, ~20ms per batch
- Metal fused: ~1-2ms per batch (10-20x speedup)
- Reason: Single kernel, no Python overhead

**Overall:**
- Current: ~0.38 batch/s (9 samples/s)
- Target: ~4-8 batch/s (100-200 samples/s)
- Training time: 12 days → 12-24 hours

### Memory Benefits
- No intermediate tensor allocation for [B,Q,B,K] similarity matrix
- In-place gradient computation
- Reduced Python/PyTorch overhead

---

## Alternative: MLX Framework

Apple's MLX framework might be simpler than raw Metal:

```python
import mlx.core as mx
import mlx.nn as nn

def mlx_maxsim(query_emb, doc_emb, query_mask, doc_mask):
    # MLX operations are lazy and compiled to Metal
    B, Q, D = query_emb.shape
    K = doc_emb.shape[1]

    # Normalize
    query_emb = query_emb / mx.linalg.norm(query_emb, axis=-1, keepdims=True)
    doc_emb = doc_emb / mx.linalg.norm(doc_emb, axis=-1, keepdims=True)

    # Similarity: [B, Q, B, K]
    sims = mx.einsum('iqd,jkd->iqjk', query_emb, doc_emb)

    # Mask and max
    sims = mx.where(doc_mask[None, None, :, :], sims, -1e9)
    max_sims = mx.max(sims, axis=-1)  # [B, Q, B]

    # Mask query and mean
    max_sims = mx.where(query_mask[:, :, None], max_sims, 0)
    scores = mx.sum(max_sims, axis=1) / mx.sum(query_mask, axis=1, keepdims=True)

    return scores

# MLX handles compilation and optimization automatically
```

**Pros:** Easier to implement, automatic differentiation
**Cons:** Less control, may not match hand-tuned Metal

---

## Files to Create

1. `crates/sg-metal-train/Cargo.toml`
2. `crates/sg-metal-train/src/lib.rs`
3. `crates/sg-metal-train/cpp/metal_ops.mm`
4. `crates/sg-metal-train/cpp/kernels/maxsim.metal`
5. `crates/sg-metal-train/cpp/kernels/mnr_loss.metal`
6. `crates/sg-metal-train/cpp/kernels/margin_loss.metal`
7. `scripts/metal_ops.py`
8. `scripts/metal_autograd.py`
9. `scripts/train_xtr_metal.py`

---

## Testing Methodology

### Test 1: Golden Output Comparison
```python
# Generate golden outputs from Python
def generate_golden_outputs():
    torch.manual_seed(42)
    test_cases = []

    for B in [1, 4, 16, 24]:
        for Q in [32, 64, 128]:
            for K in [64, 128, 384]:
                query_emb = torch.randn(B, Q, 768)
                doc_emb = torch.randn(B, K, 768)
                query_mask = torch.ones(B, Q)
                doc_mask = torch.ones(B, K)

                # Add some padding
                query_mask[:, -Q//4:] = 0
                doc_mask[:, -K//4:] = 0

                scores = python_maxsim_scores(query_emb, query_mask, doc_emb, doc_mask)

                test_cases.append({
                    'inputs': (query_emb, doc_emb, query_mask, doc_mask),
                    'expected_scores': scores,
                    'config': {'B': B, 'Q': Q, 'K': K}
                })

    torch.save(test_cases, 'tests/golden_maxsim.pt')
```

### Test 2: Numerical Gradient Verification
```python
def test_maxsim_gradients():
    query_emb = torch.randn(4, 32, 768, requires_grad=True, dtype=torch.float64)
    doc_emb = torch.randn(4, 64, 768, requires_grad=True, dtype=torch.float64)
    query_mask = torch.ones(4, 32)
    doc_mask = torch.ones(4, 64)

    # Use float64 for numerical gradient checking
    def func(q, d):
        return metal_maxsim_scores(q, query_mask, d, doc_mask).sum()

    assert torch.autograd.gradcheck(func, (query_emb, doc_emb), eps=1e-6, atol=1e-4, rtol=1e-3)
```

### Test 3: Edge Cases
```python
EDGE_CASES = [
    # Single batch
    {'B': 1, 'Q': 64, 'K': 128, 'name': 'single_batch'},
    # All padding
    {'B': 4, 'Q': 64, 'K': 128, 'query_mask_ratio': 0.1, 'name': 'mostly_padding'},
    # Maximum sizes
    {'B': 32, 'Q': 512, 'K': 512, 'name': 'large_batch'},
    # Identical embeddings (should give score ~1.0 on diagonal)
    {'B': 4, 'identical': True, 'name': 'identical_embs'},
    # Zero embeddings
    {'B': 4, 'zero_query': True, 'name': 'zero_query'},
]
```

### Test 4: Full Training Step Comparison
```python
def test_training_step_equivalence():
    """Run 10 training steps with both implementations, compare."""

    # Same random seed
    torch.manual_seed(42)
    model_python = load_model()

    torch.manual_seed(42)
    model_metal = load_model()

    # Same data
    batch = load_test_batch()

    for step in range(10):
        # Python path
        loss_py, grads_py = python_training_step(model_python, batch)

        # Metal path
        loss_metal, grads_metal = metal_training_step(model_metal, batch)

        # Compare
        assert torch.allclose(loss_py, loss_metal, atol=1e-5, rtol=1e-4), \
            f"Step {step}: loss mismatch {loss_py} vs {loss_metal}"

        for name in grads_py:
            assert torch.allclose(grads_py[name], grads_metal[name], atol=1e-4, rtol=1e-3), \
                f"Step {step}: gradient mismatch for {name}"
```

### Test 5: Property-Based Testing (Hypothesis)
```python
from hypothesis import given, strategies as st

@given(
    B=st.integers(1, 16),
    Q=st.integers(16, 256),
    K=st.integers(16, 256),
)
def test_maxsim_properties(B, Q, K):
    query_emb = torch.randn(B, Q, 768)
    doc_emb = torch.randn(B, K, 768)
    query_mask = torch.ones(B, Q)
    doc_mask = torch.ones(B, K)

    scores_py = python_maxsim(query_emb, query_mask, doc_emb, doc_mask)
    scores_metal = metal_maxsim(query_emb, query_mask, doc_emb, doc_mask)

    # Property 1: Same shape
    assert scores_py.shape == scores_metal.shape == (B, B)

    # Property 2: Numerically close
    assert torch.allclose(scores_py, scores_metal, atol=1e-5, rtol=1e-4)

    # Property 3: Diagonal should be highest (for normalized embeddings)
    # (This is a semantic property, not strict)
```

### Test 6: Performance Regression
```python
def test_performance_regression():
    """Ensure Metal is actually faster."""
    B, Q, K, D = 24, 128, 384, 768
    query_emb = torch.randn(B, Q, D)
    doc_emb = torch.randn(B, K, D)

    # Warmup
    for _ in range(10):
        metal_maxsim(query_emb, doc_emb)

    # Benchmark
    import time

    t0 = time.time()
    for _ in range(100):
        metal_maxsim(query_emb, doc_emb)
    metal_time = (time.time() - t0) / 100

    t0 = time.time()
    for _ in range(100):
        python_maxsim(query_emb, doc_emb)
    python_time = (time.time() - t0) / 100

    speedup = python_time / metal_time
    print(f"Speedup: {speedup:.1f}x (Metal: {metal_time*1000:.1f}ms, Python: {python_time*1000:.1f}ms)")

    assert speedup >= 5.0, f"Expected 5x+ speedup, got {speedup:.1f}x"
```

---

## Success Criteria

1. **Correctness:** Metal ops produce identical results to PyTorch (atol=1e-5, rtol=1e-4)
2. **Gradients:** All gradients pass torch.autograd.gradcheck
3. **Speed:** 10x+ speedup on MaxSim computation
4. **Training:** Full 9M example training completes in <48 hours
5. **Quality:** Model achieves P@1 >= 0.93 on code search eval
6. **Tests:** 100% pass rate on all test suites before merge

---

## References

- [Metal Shading Language Specification](https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf)
- [Metal Best Practices Guide](https://developer.apple.com/library/archive/documentation/3DDrawing/Conceptual/MTLBestPracticesGuide/)
- [PyTorch Custom C++ and CUDA Extensions](https://pytorch.org/tutorials/advanced/cpp_extension.html)
- [MLX Documentation](https://ml-explore.github.io/mlx/)
- [ColBERT MaxSim Implementation](https://github.com/stanford-futuredata/ColBERT)
