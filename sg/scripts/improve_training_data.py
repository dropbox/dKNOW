#!/usr/bin/env python3
"""
Improve training data quality with:
1. Semantic filtering of low-quality pairs
2. Query augmentation
3. BM25 hard negative mining

Usage:
    python scripts/improve_training_data.py data/training_data_extended.jsonl \
        -o data/training_improved.jsonl \
        --filter --augment --hard-negatives 3
"""

import argparse
import json
import random
import re
from pathlib import Path
from collections import defaultdict
from typing import List, Dict, Optional, Set
import hashlib

# Optional: BM25 for hard negatives
try:
    from rank_bm25 import BM25Okapi
    HAS_BM25 = True
except ImportError:
    HAS_BM25 = False
    print("Warning: rank_bm25 not installed. Hard negative mining disabled.")
    print("Install with: pip install rank-bm25")


# =============================================================================
# 1. Data Quality Filtering
# =============================================================================

BOILERPLATE_PATTERNS = [
    r"^Returns?\s+(self|the|a|an|true|false|none|ok|err)\b",
    r"^Returns?\s+whether\b",
    r"^Gets?\s+the\s+",
    r"^Sets?\s+the\s+",
    r"^Creates?\s+(a\s+)?new\s+",
    r"^Constructs?\s+(a\s+)?new\s+",
    r"^Default\s+impl",
    r"^TODO",
    r"^FIXME",
    r"^XXX",
    r"^See\s+",
    r"^Deprecated",
    r"^Internal\s+",
    r"^Helper\s+(function|method)",
    r"^Wrapper\s+(for|around)",
    r"^Same\s+as\s+",
    r"^Like\s+\[",
    r"^This\s+is\s+a\s+",
]

BOILERPLATE_REGEX = [re.compile(p, re.IGNORECASE) for p in BOILERPLATE_PATTERNS]

# Valuable patterns to KEEP even if short
VALUABLE_PATTERNS = [
    r"SAFETY",
    r"INVARIANT",
    r"panic",
    r"undefined behavior",
    r"UB",
    r"proof",
    r"verify",
    r"assert",
    r"#\[kani",
    r"requires\s*\(",
    r"ensures\s*\(",
]

VALUABLE_REGEX = [re.compile(p, re.IGNORECASE) for p in VALUABLE_PATTERNS]


def extract_identifiers(code: str) -> Set[str]:
    """Extract identifiers from code (split camelCase, snake_case)."""
    # Find word-like tokens
    tokens = re.findall(r'[a-zA-Z_][a-zA-Z0-9_]*', code)

    identifiers = set()
    for token in tokens:
        # Add original
        identifiers.add(token.lower())

        # Split camelCase
        parts = re.findall(r'[A-Z]?[a-z]+|[A-Z]+(?=[A-Z]|$)', token)
        for part in parts:
            if len(part) > 2:
                identifiers.add(part.lower())

        # Split snake_case
        for part in token.split('_'):
            if len(part) > 2:
                identifiers.add(part.lower())

    return identifiers


def is_high_quality_pair(query: str, positive: str, func_name: str) -> bool:
    """Filter low-quality training pairs."""

    # Always keep valuable patterns (verification, safety, etc.)
    for pattern in VALUABLE_REGEX:
        if pattern.search(query) or pattern.search(positive):
            return True

    # Check for boilerplate queries
    for pattern in BOILERPLATE_REGEX:
        if pattern.match(query):
            return False

    # Minimum unique words in query
    query_words = set(query.lower().split())
    stopwords = {'the', 'a', 'an', 'is', 'are', 'was', 'were', 'be', 'been',
                 'being', 'have', 'has', 'had', 'do', 'does', 'did', 'will',
                 'would', 'could', 'should', 'may', 'might', 'must', 'shall',
                 'this', 'that', 'these', 'those', 'it', 'its', 'of', 'to',
                 'in', 'for', 'on', 'with', 'at', 'by', 'from', 'as', 'into',
                 'if', 'or', 'and', 'but', 'not', 'no', 'yes'}
    meaningful_words = query_words - stopwords

    if len(meaningful_words) < 3:
        return False

    # Check semantic overlap (query should describe the code)
    code_identifiers = extract_identifiers(positive)
    func_parts = set(func_name.lower().replace('_', ' ').split())
    code_identifiers.update(func_parts)

    overlap = len(meaningful_words & code_identifiers)

    # Require at least some overlap for longer queries
    if len(meaningful_words) > 5 and overlap < 2:
        return False

    # Filter very short or very long queries
    if len(query) < 15 or len(query) > 2000:
        return False

    # Filter if query is mostly punctuation/symbols
    alpha_ratio = sum(c.isalpha() for c in query) / max(len(query), 1)
    if alpha_ratio < 0.5:
        return False

    return True


def filter_data(examples: List[Dict], verbose: bool = False) -> List[Dict]:
    """Filter training data for quality."""
    filtered = []
    reasons = defaultdict(int)

    for ex in examples:
        query = ex.get('query', '')
        positive = ex.get('positive', '')
        func_name = ex.get('func_name', '')

        if is_high_quality_pair(query, positive, func_name):
            filtered.append(ex)
        else:
            # Track rejection reasons for debugging
            for i, pattern in enumerate(BOILERPLATE_REGEX):
                if pattern.match(query):
                    reasons[f"boilerplate_{i}"] += 1
                    break
            else:
                reasons["other"] += 1

    if verbose:
        print(f"  Filtered: {len(examples)} -> {len(filtered)} ({len(filtered)/len(examples)*100:.1f}%)")
        print(f"  Top rejection reasons: {dict(sorted(reasons.items(), key=lambda x: -x[1])[:5])}")

    return filtered


# =============================================================================
# 2. Query Augmentation
# =============================================================================

VERB_SYNONYMS = {
    "create": ["build", "construct", "make", "initialize", "instantiate"],
    "get": ["retrieve", "fetch", "obtain", "return", "access"],
    "set": ["assign", "update", "modify", "change", "configure"],
    "find": ["search", "locate", "look up", "discover", "identify"],
    "check": ["verify", "validate", "test", "ensure", "confirm"],
    "convert": ["transform", "parse", "serialize", "encode", "decode"],
    "remove": ["delete", "drop", "clear", "erase", "destroy"],
    "add": ["insert", "append", "push", "include", "attach"],
    "compute": ["calculate", "evaluate", "determine", "derive"],
    "load": ["read", "import", "fetch", "open", "deserialize"],
    "save": ["write", "store", "persist", "export", "serialize"],
    "handle": ["process", "manage", "deal with", "take care of"],
    "parse": ["extract", "decode", "interpret", "analyze"],
    "validate": ["verify", "check", "ensure", "confirm"],
    "render": ["display", "draw", "show", "present"],
}


def augment_query(query: str, num_augments: int = 2) -> List[str]:
    """Generate query variations."""
    augmentations = []
    query_lower = query.lower()

    # 1. Verb synonym replacement
    for verb, synonyms in VERB_SYNONYMS.items():
        if query_lower.startswith(verb + " ") or query_lower.startswith(verb + "s "):
            verb_len = len(verb) + 1 if query_lower[len(verb)] == 's' else len(verb)
            for syn in random.sample(synonyms, min(2, len(synonyms))):
                suffix = "s " if query_lower[len(verb)] == 's' else " "
                augmented = syn + suffix + query[verb_len + 1:]
                augmentations.append(augmented)
            break

    # 2. Question form
    if not query.endswith("?") and len(query) < 100:
        # "Create X" -> "How to create X?"
        augmentations.append(f"How to {query_lower.rstrip('.')}?")

    # 3. Imperative to noun phrase
    words = query.split()
    if len(words) >= 2 and words[0].lower() in VERB_SYNONYMS:
        # "Create a new user" -> "User creation"
        noun_phrase = " ".join(words[1:]).rstrip('.')
        if len(noun_phrase) > 5:
            augmentations.append(f"{noun_phrase} {words[0].lower()}ing")

    # 4. Remove filler words
    fillers = {'the', 'a', 'an', 'this', 'that', 'given', 'specified', 'provided'}
    words = query.split()
    if len(words) > 4:
        filtered = [w for w in words if w.lower() not in fillers]
        if len(filtered) >= 3 and filtered != words:
            augmentations.append(" ".join(filtered))

    # 5. Simplification (first sentence only)
    if '. ' in query:
        first_sentence = query.split('. ')[0]
        if len(first_sentence) > 20:
            augmentations.append(first_sentence)

    # Deduplicate and limit
    seen = {query.lower()}
    unique_augments = []
    for aug in augmentations:
        aug_lower = aug.lower()
        if aug_lower not in seen and len(aug) > 10:
            seen.add(aug_lower)
            unique_augments.append(aug)

    return unique_augments[:num_augments]


def augment_data(examples: List[Dict], augment_prob: float = 0.3,
                 num_augments: int = 2, verbose: bool = False) -> List[Dict]:
    """Augment training data with query variations."""
    augmented = []
    num_added = 0

    for ex in examples:
        augmented.append(ex)  # Always keep original

        if random.random() < augment_prob:
            query = ex.get('query', '')
            for aug_query in augment_query(query, num_augments):
                augmented.append({
                    **ex,
                    'query': aug_query,
                    'is_augmented': True,
                })
                num_added += 1

    if verbose:
        print(f"  Augmented: {len(examples)} -> {len(augmented)} (+{num_added} variations)")

    return augmented


# =============================================================================
# 3. BM25 Hard Negative Mining
# =============================================================================

def tokenize_for_bm25(text: str) -> List[str]:
    """Simple tokenization for BM25."""
    # Lowercase and split on non-alphanumeric
    tokens = re.findall(r'[a-zA-Z0-9]+', text.lower())
    # Filter very short tokens
    return [t for t in tokens if len(t) > 2]


def mine_hard_negatives(examples: List[Dict], k_negatives: int = 3,
                        verbose: bool = False) -> List[Dict]:
    """Add BM25 hard negatives to each example."""
    if not HAS_BM25:
        print("  Skipping hard negative mining (rank_bm25 not installed)")
        return examples

    if verbose:
        print(f"  Building BM25 index over {len(examples)} documents...")

    # Build corpus of all positives (code)
    corpus = [ex.get('positive', '') for ex in examples]
    tokenized_corpus = [tokenize_for_bm25(doc) for doc in corpus]

    # Filter empty documents
    valid_indices = [i for i, tokens in enumerate(tokenized_corpus) if len(tokens) > 0]
    if len(valid_indices) < len(examples):
        print(f"  Warning: {len(examples) - len(valid_indices)} empty documents filtered")

    tokenized_corpus = [tokenized_corpus[i] for i in valid_indices]
    index_map = {new_idx: old_idx for new_idx, old_idx in enumerate(valid_indices)}

    bm25 = BM25Okapi(tokenized_corpus)

    if verbose:
        print(f"  Mining hard negatives...")

    augmented = []
    for i, ex in enumerate(examples):
        query = ex.get('query', '')
        func_name = ex.get('func_name', '')
        query_tokens = tokenize_for_bm25(query)

        if len(query_tokens) == 0:
            augmented.append(ex)
            continue

        # Get BM25 scores
        scores = bm25.get_scores(query_tokens)

        # Get top candidates (excluding self)
        top_indices = scores.argsort()[::-1]

        hard_negs = []
        for idx in top_indices:
            original_idx = index_map[idx]
            # Skip self
            if original_idx == i:
                continue
            # Skip same function name (likely similar/duplicate)
            if examples[original_idx].get('func_name', '') == func_name:
                continue

            hard_negs.append(examples[original_idx]['positive'])
            if len(hard_negs) >= k_negatives:
                break

        augmented.append({
            **ex,
            'hard_negatives': hard_negs,
        })

        if verbose and (i + 1) % 10000 == 0:
            print(f"    Processed {i + 1}/{len(examples)}")

    if verbose:
        avg_negs = sum(len(ex.get('hard_negatives', [])) for ex in augmented) / len(augmented)
        print(f"  Added hard negatives: avg {avg_negs:.1f} per example")

    return augmented


# =============================================================================
# 4. Language Balancing
# =============================================================================

def balance_languages(examples: List[Dict], max_per_language: Optional[int] = None,
                      min_per_language: int = 100, verbose: bool = False) -> List[Dict]:
    """Balance dataset across languages."""
    by_language = defaultdict(list)
    for ex in examples:
        lang = ex.get('language', 'unknown')
        by_language[lang].append(ex)

    if verbose:
        print(f"  Language distribution before balancing:")
        for lang, exs in sorted(by_language.items(), key=lambda x: -len(x[1])):
            print(f"    {lang}: {len(exs)}")

    balanced = []
    for lang, exs in by_language.items():
        if len(exs) < min_per_language:
            if verbose:
                print(f"  Dropping {lang} ({len(exs)} < {min_per_language})")
            continue

        if max_per_language and len(exs) > max_per_language:
            # Randomly sample
            exs = random.sample(exs, max_per_language)

        balanced.extend(exs)

    random.shuffle(balanced)

    if verbose:
        print(f"  After balancing: {len(examples)} -> {len(balanced)}")

    return balanced


# =============================================================================
# Main
# =============================================================================

def main():
    parser = argparse.ArgumentParser(description='Improve training data quality')
    parser.add_argument('input', type=Path, help='Input JSONL file')
    parser.add_argument('-o', '--output', type=Path, required=True, help='Output JSONL file')
    parser.add_argument('--filter', action='store_true', help='Apply quality filtering')
    parser.add_argument('--augment', action='store_true', help='Apply query augmentation')
    parser.add_argument('--augment-prob', type=float, default=0.3, help='Augmentation probability')
    parser.add_argument('--hard-negatives', type=int, default=0, help='Number of BM25 hard negatives (0 to disable)')
    parser.add_argument('--balance', action='store_true', help='Balance languages')
    parser.add_argument('--max-per-language', type=int, default=None, help='Max examples per language')
    parser.add_argument('--validation-split', type=float, default=0.0, help='Fraction to hold out for validation')
    parser.add_argument('-v', '--verbose', action='store_true')
    args = parser.parse_args()

    # Load data
    print(f"Loading {args.input}...")
    examples = []
    with open(args.input) as f:
        for line in f:
            if line.strip():
                examples.append(json.loads(line))
    print(f"  Loaded {len(examples)} examples")

    # Apply improvements
    if args.filter:
        print("Applying quality filtering...")
        examples = filter_data(examples, verbose=args.verbose)

    if args.augment:
        print("Applying query augmentation...")
        examples = augment_data(examples, augment_prob=args.augment_prob, verbose=args.verbose)

    if args.hard_negatives > 0:
        print(f"Mining {args.hard_negatives} hard negatives per example...")
        examples = mine_hard_negatives(examples, k_negatives=args.hard_negatives, verbose=args.verbose)

    if args.balance:
        print("Balancing languages...")
        examples = balance_languages(examples, max_per_language=args.max_per_language, verbose=args.verbose)

    # Validation split
    if args.validation_split > 0:
        random.shuffle(examples)
        val_size = int(len(examples) * args.validation_split)
        val_examples = examples[:val_size]
        train_examples = examples[val_size:]

        val_path = args.output.with_suffix('.val.jsonl')
        print(f"Writing {len(val_examples)} validation examples to {val_path}")
        with open(val_path, 'w') as f:
            for ex in val_examples:
                f.write(json.dumps(ex) + '\n')

        examples = train_examples

    # Deduplicate by (func_name, query[:100])
    print("Deduplicating...")
    seen = set()
    unique = []
    for ex in examples:
        key = (ex.get('func_name', ''), ex.get('query', '')[:100])
        if key not in seen:
            seen.add(key)
            unique.append(ex)

    print(f"  {len(examples)} -> {len(unique)} unique examples")
    examples = unique

    # Write output
    print(f"Writing {len(examples)} examples to {args.output}")
    with open(args.output, 'w') as f:
        for ex in examples:
            f.write(json.dumps(ex) + '\n')

    # Stats
    by_lang = defaultdict(int)
    has_hard_negs = 0
    is_augmented = 0
    for ex in examples:
        by_lang[ex.get('language', 'unknown')] += 1
        if ex.get('hard_negatives'):
            has_hard_negs += 1
        if ex.get('is_augmented'):
            is_augmented += 1

    print(f"\nFinal statistics:")
    print(f"  Total: {len(examples)}")
    print(f"  With hard negatives: {has_hard_negs}")
    print(f"  Augmented: {is_augmented}")
    print(f"  By language:")
    for lang, count in sorted(by_lang.items(), key=lambda x: -x[1]):
        print(f"    {lang}: {count}")


if __name__ == '__main__':
    main()
