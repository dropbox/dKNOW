#!/usr/bin/env python3
"""
LLM-as-Judge Relevance Scoring Pipeline

Scores (query, code) pairs using Claude Haiku for semantic relevance.
This enables training on true semantic understanding rather than vocabulary matching.

Usage:
    python scripts/score_relevance.py \
        --input data/training_improved.jsonl \
        --output data/scored_training.jsonl \
        --sample 10000

Cost estimate: ~$3-5 per 10K pairs with Haiku
"""

import argparse
import json
import os
import random
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Optional

import anthropic

SCORING_PROMPT = """You are evaluating code search relevance. Given a search query and code snippet, score how well the code matches what the user is looking for.

Query: {query}

Code ({language}):
```{language}
{code}
```

Score relevance from 1-5:
1 = Completely irrelevant (different topic entirely)
2 = Slightly related (same domain but wrong functionality)
3 = Partially relevant (related but not quite what user wants)
4 = Good match (mostly what user wants, minor gaps)
5 = Excellent match (exactly or very close to what user wants)

Consider:
- Does the code DO what the query describes?
- Would a developer searching for this query be satisfied with this result?
- Ignore variable names - focus on functionality

Respond with JSON only:
{{"score": N, "reasoning": "one sentence explanation"}}"""

QUERY_VARIATION_PROMPT = """Generate 3 semantic variations of this code search query. Each variation should describe the SAME functionality but use different words/phrasing.

Original query: {query}

For context, this query describes code that: {code_summary}

Generate variations that:
1. Use synonyms and different phrasing
2. Describe the functionality, not implementation
3. Avoid technical jargon when possible
4. Sound like what a developer might actually search for

Respond with JSON array only:
["variation 1", "variation 2", "variation 3"]"""


class RelevanceScorer:
    def __init__(self, model: str = "claude-3-haiku-20240307"):
        api_key = os.environ.get("ANTHROPIC_API_KEY")
        if not api_key:
            raise ValueError("ANTHROPIC_API_KEY environment variable required")
        self.client = anthropic.Anthropic(api_key=api_key)
        self.model = model
        self.total_tokens = 0
        self.total_cost = 0.0

    def score_pair(self, query: str, code: str, language: str = "rust") -> dict:
        """Score a single (query, code) pair for relevance."""
        prompt = SCORING_PROMPT.format(
            query=query,
            code=code[:4000],  # Truncate very long code
            language=language,
        )

        try:
            response = self.client.messages.create(
                model=self.model,
                max_tokens=150,
                messages=[{"role": "user", "content": prompt}],
            )

            # Track usage
            self.total_tokens += response.usage.input_tokens + response.usage.output_tokens

            # Parse response
            text = response.content[0].text.strip()
            # Handle potential markdown code blocks
            if text.startswith("```"):
                text = text.split("\n", 1)[1].rsplit("```", 1)[0]

            result = json.loads(text)
            return {
                "score": int(result["score"]),
                "reasoning": result.get("reasoning", ""),
            }

        except json.JSONDecodeError as e:
            print(f"Warning: Failed to parse JSON response: {text[:100]}", file=sys.stderr)
            return {"score": 3, "reasoning": "parse_error"}
        except Exception as e:
            print(f"Warning: API error: {e}", file=sys.stderr)
            return {"score": 0, "reasoning": f"error: {str(e)[:50]}"}

    def generate_query_variations(self, query: str, code: str) -> list[str]:
        """Generate semantic variations of a query."""
        # Create a brief code summary (first 500 chars)
        code_summary = code[:500].replace("\n", " ")

        prompt = QUERY_VARIATION_PROMPT.format(
            query=query,
            code_summary=code_summary,
        )

        try:
            response = self.client.messages.create(
                model=self.model,
                max_tokens=200,
                messages=[{"role": "user", "content": prompt}],
            )

            self.total_tokens += response.usage.input_tokens + response.usage.output_tokens

            text = response.content[0].text.strip()
            if text.startswith("```"):
                text = text.split("\n", 1)[1].rsplit("```", 1)[0]

            variations = json.loads(text)
            if isinstance(variations, list):
                return [v for v in variations if isinstance(v, str)][:3]
            return []

        except Exception as e:
            print(f"Warning: Variation generation failed: {e}", file=sys.stderr)
            return []

    def estimate_cost(self):
        """Estimate cost based on token usage (Haiku pricing)."""
        # Haiku: $0.25/1M input, $1.25/1M output (approx)
        # Rough estimate: $0.50/1M tokens average
        return self.total_tokens * 0.5 / 1_000_000


def load_training_data(path: Path, sample_size: Optional[int] = None) -> list[dict]:
    """Load training data from JSONL file."""
    examples = []
    with open(path) as f:
        for line in f:
            if line.strip():
                examples.append(json.loads(line))

    if sample_size and sample_size < len(examples):
        random.seed(42)
        examples = random.sample(examples, sample_size)

    return examples


def process_example(scorer: RelevanceScorer, example: dict, generate_variations: bool) -> list[dict]:
    """Process a single example, optionally generating query variations."""
    results = []

    query = example["query"]
    code = example["positive"]
    language = example.get("language", "rust")

    # Score original pair
    score_result = scorer.score_pair(query, code, language)
    results.append({
        **example,
        "llm_score": score_result["score"],
        "score_reasoning": score_result["reasoning"],
        "is_variation": False,
    })

    # Generate and score variations if requested
    if generate_variations and score_result["score"] >= 4:
        variations = scorer.generate_query_variations(query, code)
        for var_query in variations:
            var_score = scorer.score_pair(var_query, code, language)
            results.append({
                "query": var_query,
                "positive": code,
                "file_path": example.get("file_path", ""),
                "func_name": example.get("func_name", ""),
                "language": language,
                "original_query": query,
                "llm_score": var_score["score"],
                "score_reasoning": var_score["reasoning"],
                "is_variation": True,
            })

    return results


def main():
    parser = argparse.ArgumentParser(description="Score training pairs with LLM judge")
    parser.add_argument("--input", required=True, help="Input JSONL file")
    parser.add_argument("--output", required=True, help="Output JSONL file")
    parser.add_argument("--sample", type=int, help="Sample N examples (default: all)")
    parser.add_argument("--variations", action="store_true", help="Generate query variations")
    parser.add_argument("--workers", type=int, default=4, help="Parallel workers")
    parser.add_argument("--model", default="claude-3-haiku-20240307", help="Model to use")
    parser.add_argument("--resume", action="store_true", help="Resume from existing output")
    args = parser.parse_args()

    input_path = Path(args.input)
    output_path = Path(args.output)

    if not input_path.exists():
        print(f"Error: Input file not found: {input_path}", file=sys.stderr)
        sys.exit(1)

    # Load existing progress if resuming
    processed_queries = set()
    if args.resume and output_path.exists():
        with open(output_path) as f:
            for line in f:
                if line.strip():
                    ex = json.loads(line)
                    processed_queries.add(ex["query"])
        print(f"Resuming: {len(processed_queries)} already processed")

    # Load input data
    print(f"Loading data from {input_path}...")
    examples = load_training_data(input_path, args.sample)
    print(f"Loaded {len(examples)} examples")

    # Filter already processed
    if processed_queries:
        examples = [ex for ex in examples if ex["query"] not in processed_queries]
        print(f"Remaining to process: {len(examples)}")

    if not examples:
        print("Nothing to process!")
        return

    # Initialize scorer
    scorer = RelevanceScorer(model=args.model)

    # Process examples
    output_mode = "a" if args.resume else "w"
    processed = 0
    start_time = time.time()

    with open(output_path, output_mode) as out_f:
        # Sequential processing (parallel has rate limit issues)
        for i, example in enumerate(examples):
            try:
                results = process_example(scorer, example, args.variations)
                for result in results:
                    out_f.write(json.dumps(result) + "\n")
                processed += 1

                # Progress update every 50 examples
                if (i + 1) % 50 == 0:
                    elapsed = time.time() - start_time
                    rate = processed / elapsed if elapsed > 0 else 0
                    est_cost = scorer.estimate_cost()
                    print(
                        f"Progress: {processed}/{len(examples)} "
                        f"({rate:.1f}/s, ~${est_cost:.2f})"
                    )
                    out_f.flush()

            except KeyboardInterrupt:
                print("\nInterrupted! Progress saved.")
                break
            except Exception as e:
                print(f"Error processing example {i}: {e}", file=sys.stderr)
                continue

    # Final stats
    elapsed = time.time() - start_time
    est_cost = scorer.estimate_cost()
    print(f"\nComplete!")
    print(f"Processed: {processed} examples")
    print(f"Time: {elapsed:.1f}s ({processed/elapsed:.1f} examples/s)")
    print(f"Tokens: {scorer.total_tokens:,}")
    print(f"Estimated cost: ${est_cost:.2f}")
    print(f"Output: {output_path}")


if __name__ == "__main__":
    main()
