#!/bin/bash

# Run claude continue in a loop
# Press Ctrl+C to stop

echo "Starting claude continue loop..."
echo "Press Ctrl+C to stop"
echo "================================"
echo ""

iteration=1
while true; do
    echo "--- Iteration $iteration ---"
    claude --dangerously-skip-permissions -p "continue the work. Do not stop until you end your session per protocol with a proper git commit" --permission-mode acceptEdits --output-format stream-json --verbose 2>&1 | tee iteration_${iteration}.jsonl
    exit_code=$?

    echo ""
    echo "Exit code: $exit_code"

    # Stop if claude exits with error
    if [ $exit_code -ne 0 ]; then
        echo "Claude exited with error. Stopping loop."
        break
    fi

    iteration=$((iteration + 1))
    echo ""
done

echo "Loop completed after $((iteration - 1)) iterations"
