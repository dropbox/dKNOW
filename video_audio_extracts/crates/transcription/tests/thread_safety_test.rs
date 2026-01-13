/// Reproducible test case demonstrating whisper-rs thread safety issue
///
/// **Issue**: whisper-rs declares `unsafe impl Send + Sync for WhisperInnerContext`
/// but the underlying whisper.cpp C++ library is NOT thread-safe.
///
/// **Expected behavior**: WhisperContext.create_state() should be safe to call
/// concurrently from multiple threads (as guaranteed by Send + Sync).
///
/// **Actual behavior**: Concurrent calls to create_state() cause race conditions,
/// deadlocks, or hangs.
///
/// **Workaround**: Wrap WhisperContext in Mutex to serialize access.
///
/// This test demonstrates the issue by attempting to create multiple whisper states
/// concurrently without a Mutex. The test will likely hang or fail.
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use whisper_rs::{WhisperContext, WhisperContextParameters};

/// Test audio file - small WAV file for quick testing
const TEST_AUDIO: &str = "./test_media_generated/test_audio_5min_sine.wav";

/// Whisper model path - adjust to your model location
const MODEL_PATH: &str = "./models/ggml-tiny.en.bin";

#[test]
#[ignore] // Run with: cargo test --test whisper_thread_safety_test -- --ignored
fn test_whisper_context_concurrent_create_state_without_mutex() {
    // Load Whisper model once
    let model_path = PathBuf::from(MODEL_PATH);

    if !model_path.exists() {
        eprintln!("⚠️  Model not found at {:?}", model_path);
        eprintln!("   Download from: https://huggingface.co/ggerganov/whisper.cpp");
        panic!("Model file required for test");
    }

    let audio_path = PathBuf::from(TEST_AUDIO);
    if !audio_path.exists() {
        panic!("Test audio file not found at {:?}", audio_path);
    }

    println!("Loading Whisper model from {:?}...", model_path);

    let ctx_params = WhisperContextParameters::default();
    let context = WhisperContext::new_with_params(&model_path.to_string_lossy(), ctx_params)
        .expect("Failed to load Whisper model");

    // Share context across threads using Arc
    // According to whisper-rs documentation, WhisperContext implements Send + Sync,
    // so this SHOULD be safe. But it's NOT.
    let context = Arc::new(context);

    println!("Spawning 4 threads to concurrently create whisper states...");
    println!("⚠️  THIS WILL LIKELY HANG OR CRASH due to race conditions in whisper.cpp");

    let handles: Vec<_> = (0..4)
        .map(|i| {
            let ctx: Arc<WhisperContext> = Arc::clone(&context);
            thread::spawn(move || {
                println!("  Thread {}: Calling create_state()...", i);

                // This call is NOT thread-safe despite Send + Sync implementation
                let state = ctx.create_state().expect("Failed to create state");

                println!("  Thread {}: Successfully created state", i);

                // Return the state to prevent early drop
                state
            })
        })
        .collect();

    println!("Waiting for all threads to complete...");

    for (i, handle) in handles.into_iter().enumerate() {
        match handle.join() {
            Ok(_state) => println!("  Thread {}: Completed successfully", i),
            Err(e) => panic!("  Thread {}: Panicked: {:?}", i, e),
        }
    }

    println!("✅ All threads completed (unexpected - this usually hangs)");
}

#[test]
#[ignore] // Run with: cargo test --test whisper_thread_safety_test -- --ignored
fn test_whisper_context_concurrent_create_state_with_mutex() {
    // Load Whisper model once
    let model_path = PathBuf::from(MODEL_PATH);

    if !model_path.exists() {
        eprintln!("⚠️  Model not found at {:?}", model_path);
        eprintln!("   Download from: https://huggingface.co/ggerganov/whisper.cpp");
        panic!("Model file required for test");
    }

    let audio_path = PathBuf::from(TEST_AUDIO);
    if !audio_path.exists() {
        panic!("Test audio file not found at {:?}", audio_path);
    }

    println!("Loading Whisper model from {:?}...", model_path);

    let ctx_params = WhisperContextParameters::default();
    let context = WhisperContext::new_with_params(&model_path.to_string_lossy(), ctx_params)
        .expect("Failed to load Whisper model");

    // WORKAROUND: Wrap in Mutex to serialize access
    let context = Arc::new(Mutex::new(context));

    println!("Spawning 4 threads to concurrently create whisper states (WITH Mutex)...");

    let handles: Vec<_> = (0..4)
        .map(|i| {
            let ctx: Arc<Mutex<WhisperContext>> = Arc::clone(&context);
            thread::spawn(move || {
                println!("  Thread {}: Acquiring mutex lock...", i);

                // Acquire lock - serializes all access to WhisperContext
                let guard = ctx.lock().expect("Failed to lock");

                println!("  Thread {}: Calling create_state()...", i);
                let state = guard.create_state().expect("Failed to create state");

                // Release lock before doing inference (optional but recommended)
                drop(guard);

                println!("  Thread {}: Successfully created state", i);

                // Return the state to prevent early drop
                state
            })
        })
        .collect();

    println!("Waiting for all threads to complete...");

    for (i, handle) in handles.into_iter().enumerate() {
        match handle.join() {
            Ok(_state) => println!("  Thread {}: Completed successfully", i),
            Err(e) => panic!("  Thread {}: Panicked: {:?}", i, e),
        }
    }

    println!("✅ All threads completed successfully (Mutex prevents race conditions)");
}

#[test]
#[ignore]
fn test_recommended_usage_pattern() {
    // This test documents the CORRECT usage pattern for whisper-rs in concurrent contexts

    let model_path = PathBuf::from(MODEL_PATH);
    if !model_path.exists() {
        eprintln!("⚠️  Model not found - skipping test");
        return;
    }

    println!("=== RECOMMENDED PATTERN for whisper-rs concurrency ===\n");

    // Step 1: Load model with Mutex wrapper
    println!("1. Load WhisperContext and wrap in Arc<Mutex<...>>");
    let ctx_params = WhisperContextParameters::default();
    let context = WhisperContext::new_with_params(&model_path.to_string_lossy(), ctx_params)
        .expect("Failed to load model");
    let context = Arc::new(Mutex::new(context));

    // Step 2: Share Arc<Mutex<WhisperContext>> across threads/tasks
    println!("2. Clone Arc for each thread/task\n");

    // Step 3: In each thread, acquire lock, create state, release lock
    println!("3. In each thread:");
    println!("   a) Acquire Mutex lock");
    println!("   b) Call create_state()");
    println!("   c) Release lock (drop guard)");
    println!("   d) Perform inference with state\n");

    let ctx_clone: Arc<Mutex<WhisperContext>> = Arc::clone(&context);
    let handle = thread::spawn(move || {
        // Acquire lock
        let guard = ctx_clone.lock().expect("Failed to lock");

        // Create state while holding lock
        let state = guard.create_state().expect("Failed to create state");

        // Release lock before long-running inference
        drop(guard);

        // State can now be used for inference without holding the lock
        // (inference itself may have its own internal locking in whisper.cpp)
        println!("   ✅ State created successfully");

        state
    });

    handle.join().expect("Thread panicked");

    println!("\n✅ Pattern demonstrated successfully");
}
