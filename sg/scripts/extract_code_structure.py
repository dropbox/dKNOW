#!/usr/bin/env python3
"""
Extract training data from CODE STRUCTURE, not just comments.

Captures semantic meaning from:
- Function/method names → natural language queries
- Class/struct/trait names → concept descriptions
- Type signatures → transformation descriptions
- Trait implementations → capability descriptions
- Test names → behavior descriptions
- Module paths → organizational queries
- Formal specs (Lean) → theorem/proof structures
- Error types → failure mode queries

Usage:
    python scripts/extract_code_structure.py ~/code/myproject -o data/structure_training.jsonl
"""

import argparse
import json
import re
import sys
from collections import defaultdict
from pathlib import Path
from typing import Dict, Generator, List, Optional, Tuple

# Language file extensions - focused on systems languages + formal
LANG_EXTENSIONS = {
    # Primary targets
    ".rs": "rust",
    ".c": "c",
    ".h": "c",
    ".cpp": "cpp",
    ".cc": "cpp",
    ".cxx": "cpp",
    ".hpp": "cpp",
    ".hh": "cpp",
    ".lean": "lean",
    ".swift": "swift",
    ".m": "objc",
    ".mm": "objc",
    # Formal verification
    ".tla": "tla",
    ".smt2": "smt",
    ".z3": "smt",
    ".v": "coq",
    ".dfy": "dafny",
    ".bpl": "boogie",
    # Secondary
    ".py": "python",
    ".go": "go",
}


def camel_to_words(name: str) -> str:
    """Convert CamelCase to words."""
    # Insert space before uppercase letters
    s = re.sub(r'([a-z])([A-Z])', r'\1 \2', name)
    return s.lower()


def snake_to_words(name: str) -> str:
    """Convert snake_case to words."""
    return name.replace("_", " ").lower()


def name_to_query(name: str) -> str:
    """Convert any identifier to natural language query."""
    # Handle common prefixes
    name = re.sub(r'^(get_?|set_?|is_?|has_?|can_?|should_?)', '', name, flags=re.I)

    # Convert casing
    if "_" in name:
        query = snake_to_words(name)
    else:
        query = camel_to_words(name)

    # Clean up
    query = re.sub(r'\s+', ' ', query).strip()
    return query


def extract_rust_structure(content: str, file_path: str) -> List[Dict]:
    """Extract structural elements from Rust code, including Verus verification patterns."""
    pairs = []
    lines = content.split("\n")

    # Check if this is verification code
    is_verus = "verus!" in content or "proof fn" in content or "spec fn" in content
    is_kani = "#[kani::proof]" in content or "kani::" in content
    is_creusot = "#[requires" in content or "#[ensures" in content or "#[invariant" in content
    is_verification = is_verus or is_kani or is_creusot

    # Track context
    current_impl = None
    current_struct = None

    for i, line in enumerate(lines):
        # Struct definitions
        match = re.match(r'\s*(?:pub\s+)?struct\s+(\w+)(?:<[^>]*>)?', line)
        if match:
            struct_name = match.group(1)
            current_struct = struct_name
            query = name_to_query(struct_name)

            # Get struct body
            code = extract_block(lines, i, "{", "}")
            if len(query) > 5 and len(code) > 30:
                pairs.append({
                    "query": f"{query} struct",
                    "positive": code,
                    "language": "rust",
                    "source": "struct_def",
                })
                # Also add field-based query
                fields = re.findall(r'(\w+)\s*:', code)
                if fields:
                    field_query = " ".join(name_to_query(f) for f in fields[:5])
                    pairs.append({
                        "query": f"struct with {field_query}",
                        "positive": code,
                        "language": "rust",
                        "source": "struct_fields",
                    })

        # Enum definitions
        match = re.match(r'\s*(?:pub\s+)?enum\s+(\w+)(?:<[^>]*>)?', line)
        if match:
            enum_name = match.group(1)
            query = name_to_query(enum_name)
            code = extract_block(lines, i, "{", "}")
            if len(query) > 5 and len(code) > 30:
                pairs.append({
                    "query": f"{query} enum",
                    "positive": code,
                    "language": "rust",
                    "source": "enum_def",
                })
                # Variant-based query
                variants = re.findall(r'^\s*(\w+)(?:\s*\{|\s*\(|\s*,|\s*$)', code, re.MULTILINE)
                if variants:
                    variant_query = " or ".join(name_to_query(v) for v in variants[:4])
                    pairs.append({
                        "query": variant_query,
                        "positive": code,
                        "language": "rust",
                        "source": "enum_variants",
                    })

        # Trait definitions
        match = re.match(r'\s*(?:pub\s+)?trait\s+(\w+)(?:<[^>]*>)?', line)
        if match:
            trait_name = match.group(1)
            query = name_to_query(trait_name)
            code = extract_block(lines, i, "{", "}")
            if len(query) > 3 and len(code) > 30:
                pairs.append({
                    "query": f"{query} trait",
                    "positive": code,
                    "language": "rust",
                    "source": "trait_def",
                })

        # Impl blocks
        match = re.match(r'\s*impl(?:<[^>]*>)?\s+(\w+)(?:<[^>]*>)?\s+for\s+(\w+)', line)
        if match:
            trait_name = match.group(1)
            type_name = match.group(2)
            current_impl = (trait_name, type_name)
            code = extract_block(lines, i, "{", "}")
            query = f"implement {name_to_query(trait_name)} for {name_to_query(type_name)}"
            if len(code) > 30:
                pairs.append({
                    "query": query,
                    "positive": code,
                    "language": "rust",
                    "source": "impl_trait",
                })

        # Regular impl blocks
        match = re.match(r'\s*impl(?:<[^>]*>)?\s+(\w+)(?:<[^>]*>)?\s*\{', line)
        if match and "for" not in line:
            type_name = match.group(1)
            current_impl = (None, type_name)

        # Verus proof/spec functions
        if is_verus:
            # proof fn
            match = re.match(r'\s*(?:pub\s+)?proof\s+fn\s+(\w+)(?:<[^>]*>)?\s*\(([^)]*)\)', line)
            if match:
                fn_name = match.group(1)
                query = name_to_query(fn_name)
                code = extract_verus_block(lines, i)

                if len(query) > 3 and len(code) > 30:
                    pairs.append({
                        "query": f"proof {query}",
                        "positive": code,
                        "language": "rust",
                        "source": "verus_proof",
                    })
                    # Extract requires/ensures
                    requires = extract_verus_contracts(code, "requires")
                    ensures = extract_verus_contracts(code, "ensures")
                    if requires:
                        pairs.append({
                            "query": f"precondition {' '.join(requires[:3])}",
                            "positive": code,
                            "language": "rust",
                            "source": "verus_requires",
                        })
                    if ensures:
                        pairs.append({
                            "query": f"postcondition {' '.join(ensures[:3])}",
                            "positive": code,
                            "language": "rust",
                            "source": "verus_ensures",
                        })

            # spec fn
            match = re.match(r'\s*(?:pub\s+)?(?:open\s+|closed\s+)?spec\s+fn\s+(\w+)(?:<[^>]*>)?\s*\(([^)]*)\)(?:\s*->\s*([^{]+))?', line)
            if match:
                fn_name = match.group(1)
                return_type = match.group(3)
                query = name_to_query(fn_name)
                code = extract_verus_block(lines, i)

                if len(query) > 3 and len(code) > 20:
                    pairs.append({
                        "query": f"spec {query}",
                        "positive": code,
                        "language": "rust",
                        "source": "verus_spec",
                    })

        # Kani proof harnesses
        if is_kani and re.match(r'\s*#\[kani::proof\]', line):
            # Next line should be fn
            if i + 1 < len(lines):
                next_line = lines[i + 1]
                match = re.match(r'\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)', next_line)
                if match:
                    fn_name = match.group(1)
                    query = name_to_query(fn_name)
                    code = extract_block(lines, i, "{", "}")

                    if len(query) > 3:
                        pairs.append({
                            "query": f"proof harness {query}",
                            "positive": code,
                            "language": "rust",
                            "source": "kani_proof",
                        })

        # Creusot contracts (requires/ensures/invariant)
        if is_creusot:
            # Extract functions with contracts
            if re.match(r'\s*#\[(requires|ensures)\(', line):
                # Look for the function this applies to
                contract_lines = [line]
                for j in range(i + 1, min(i + 10, len(lines))):
                    next_line = lines[j]
                    if re.match(r'\s*#\[(requires|ensures|invariant)\(', next_line):
                        contract_lines.append(next_line)
                    elif re.match(r'\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)', next_line):
                        fn_match = re.match(r'\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)', next_line)
                        if fn_match:
                            fn_name = fn_match.group(1)
                            query = name_to_query(fn_name)
                            code = "\n".join(contract_lines) + "\n" + extract_block(lines, j, "{", "}")

                            if len(query) > 3 and len(code) > 30:
                                pairs.append({
                                    "query": f"verified {query}",
                                    "positive": code,
                                    "language": "rust",
                                    "source": "creusot_contract",
                                })
                                # Also extract the contract conditions
                                for cl in contract_lines:
                                    if "requires" in cl:
                                        pairs.append({
                                            "query": f"precondition for {query}",
                                            "positive": code,
                                            "language": "rust",
                                            "source": "creusot_requires",
                                        })
                                    if "ensures" in cl:
                                        pairs.append({
                                            "query": f"postcondition for {query}",
                                            "positive": code,
                                            "language": "rust",
                                            "source": "creusot_ensures",
                                        })
                        break
                    else:
                        break

        # Function definitions
        match = re.match(r'\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)(?:<[^>]*>)?\s*\(([^)]*)\)(?:\s*->\s*([^{]+))?', line)
        if match:
            fn_name = match.group(1)
            params = match.group(2)
            return_type = match.group(3)

            # Skip trivial functions
            if fn_name in ["new", "default", "clone", "drop", "main"]:
                continue

            query = name_to_query(fn_name)
            code = extract_block(lines, i, "{", "}")

            if len(query) > 5 and len(code) > 30:
                # Basic function query
                pairs.append({
                    "query": query,
                    "positive": code,
                    "language": "rust",
                    "source": "function",
                })

                # Type signature query
                if return_type:
                    return_type = return_type.strip()
                    type_query = f"function returning {name_to_query(return_type)}"
                    pairs.append({
                        "query": type_query,
                        "positive": code,
                        "language": "rust",
                        "source": "function_signature",
                    })

                # Context-aware query (if in impl block)
                if current_impl:
                    trait_name, type_name = current_impl
                    if trait_name:
                        ctx_query = f"{name_to_query(type_name)} {query}"
                    else:
                        ctx_query = f"{name_to_query(type_name)} {query}"
                    pairs.append({
                        "query": ctx_query,
                        "positive": code,
                        "language": "rust",
                        "source": "method",
                    })

        # Test functions
        if re.match(r'\s*#\[test\]', line):
            # Next line should be fn
            if i + 1 < len(lines):
                next_line = lines[i + 1]
                match = re.match(r'\s*(?:async\s+)?fn\s+(\w+)', next_line)
                if match:
                    test_name = match.group(1)
                    test_name = re.sub(r'^test_?', '', test_name)
                    query = name_to_query(test_name)
                    code = extract_block(lines, i + 1, "{", "}")
                    if len(query) > 5:
                        pairs.append({
                            "query": f"test {query}",
                            "positive": code,
                            "language": "rust",
                            "source": "test",
                        })

        # Error types
        match = re.match(r'\s*(?:pub\s+)?(?:struct|enum)\s+(\w*Error\w*|\w*Err\w*)', line)
        if match:
            error_name = match.group(1)
            query = name_to_query(error_name)
            code = extract_block(lines, i, "{", "}")
            if len(code) > 20:
                pairs.append({
                    "query": f"error {query}",
                    "positive": code,
                    "language": "rust",
                    "source": "error_type",
                })

    return pairs


def extract_python_structure(content: str, file_path: str) -> List[Dict]:
    """Extract structural elements from Python code."""
    pairs = []
    lines = content.split("\n")

    current_class = None

    for i, line in enumerate(lines):
        # Class definitions
        match = re.match(r'^class\s+(\w+)(?:\([^)]*\))?:', line)
        if match:
            class_name = match.group(1)
            current_class = class_name
            query = name_to_query(class_name)
            code = extract_python_block(lines, i)

            if len(query) > 5 and len(code) > 30:
                pairs.append({
                    "query": f"{query} class",
                    "positive": code,
                    "language": "python",
                    "source": "class_def",
                })

        # Function/method definitions
        match = re.match(r'^(\s*)(?:async\s+)?def\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*([^:]+))?:', line)
        if match:
            indent = match.group(1)
            fn_name = match.group(2)
            params = match.group(3)
            return_type = match.group(4)

            # Skip magic methods except important ones
            if fn_name.startswith("__") and fn_name not in ["__init__", "__call__", "__iter__", "__next__"]:
                continue

            query = name_to_query(fn_name)
            code = extract_python_block(lines, i)

            if len(query) > 3 and len(code) > 30:
                # Basic query
                pairs.append({
                    "query": query,
                    "positive": code,
                    "language": "python",
                    "source": "function",
                })

                # Type hint query
                if return_type:
                    type_query = f"function returning {name_to_query(return_type.strip())}"
                    pairs.append({
                        "query": type_query,
                        "positive": code,
                        "language": "python",
                        "source": "function_typed",
                    })

                # Method context
                if current_class and indent:
                    ctx_query = f"{name_to_query(current_class)} {query}"
                    pairs.append({
                        "query": ctx_query,
                        "positive": code,
                        "language": "python",
                        "source": "method",
                    })

        # Dataclass/NamedTuple
        if "@dataclass" in line or "NamedTuple" in line:
            if i + 1 < len(lines):
                next_match = re.match(r'^class\s+(\w+)', lines[i + 1])
                if next_match:
                    class_name = next_match.group(1)
                    query = name_to_query(class_name)
                    code = extract_python_block(lines, i)
                    pairs.append({
                        "query": f"{query} data structure",
                        "positive": code,
                        "language": "python",
                        "source": "dataclass",
                    })

        # Test functions
        match = re.match(r'^(\s*)def\s+(test_\w+)\s*\(', line)
        if match:
            test_name = match.group(2)
            test_name = re.sub(r'^test_?', '', test_name)
            query = name_to_query(test_name)
            code = extract_python_block(lines, i)
            if len(query) > 5:
                pairs.append({
                    "query": f"test {query}",
                    "positive": code,
                    "language": "python",
                    "source": "test",
                })

    return pairs


def extract_lean_structure(content: str, file_path: str) -> List[Dict]:
    """Extract structural elements from Lean 4 code with proof emphasis."""
    pairs = []
    lines = content.split("\n")

    for i, line in enumerate(lines):
        # Theorem definitions with full proof
        match = re.match(r'^theorem\s+(\w+)\s*(.*?)(?::\s*(.*))?$', line)
        if match:
            thm_name = match.group(1)
            params = match.group(2) or ""
            type_sig = match.group(3) or ""
            query = name_to_query(thm_name)
            code = extract_lean_proof_block(lines, i)

            if len(query) > 3 and len(code) > 20:
                pairs.append({
                    "query": f"theorem {query}",
                    "positive": code,
                    "language": "lean",
                    "source": "theorem",
                })
                # Type signature query
                if type_sig:
                    sig_query = clean_lean_signature(type_sig)
                    if len(sig_query) > 5:
                        pairs.append({
                            "query": f"prove {sig_query}",
                            "positive": code,
                            "language": "lean",
                            "source": "theorem_sig",
                        })
                # Proof tactics query (what tactics are used)
                tactics = extract_lean_tactics(code)
                if tactics:
                    pairs.append({
                        "query": f"proof using {' '.join(tactics[:4])}",
                        "positive": code,
                        "language": "lean",
                        "source": "proof_tactics",
                    })

        # Lemma definitions
        match = re.match(r'^lemma\s+(\w+)\s*(.*?)(?::\s*(.*))?$', line)
        if match:
            lemma_name = match.group(1)
            type_sig = match.group(3) or ""
            query = name_to_query(lemma_name)
            code = extract_lean_proof_block(lines, i)
            if len(query) > 3 and len(code) > 20:
                pairs.append({
                    "query": f"lemma {query}",
                    "positive": code,
                    "language": "lean",
                    "source": "lemma",
                })
                if type_sig:
                    sig_query = clean_lean_signature(type_sig)
                    if len(sig_query) > 5:
                        pairs.append({
                            "query": f"prove {sig_query}",
                            "positive": code,
                            "language": "lean",
                            "source": "lemma_sig",
                        })

        # Definition
        match = re.match(r'^def\s+(\w+)\s*(.*?)(?::\s*(.*))?(?:\s*:=|\s*where)?', line)
        if match:
            def_name = match.group(1)
            type_sig = match.group(3) or ""
            query = name_to_query(def_name)
            code = extract_lean_block(lines, i)
            if len(query) > 3:
                pairs.append({
                    "query": f"define {query}",
                    "positive": code,
                    "language": "lean",
                    "source": "definition",
                })
                if type_sig:
                    pairs.append({
                        "query": f"function {clean_lean_signature(type_sig)}",
                        "positive": code,
                        "language": "lean",
                        "source": "definition_sig",
                    })

        # Structure definitions
        match = re.match(r'^structure\s+(\w+)\s*', line)
        if match:
            struct_name = match.group(1)
            query = name_to_query(struct_name)
            code = extract_lean_block(lines, i)
            if len(query) > 3:
                pairs.append({
                    "query": f"{query} structure",
                    "positive": code,
                    "language": "lean",
                    "source": "structure",
                })
                # Extract field names
                fields = re.findall(r'(\w+)\s*:', code)
                if fields:
                    pairs.append({
                        "query": f"structure with {' '.join(name_to_query(f) for f in fields[:5])}",
                        "positive": code,
                        "language": "lean",
                        "source": "structure_fields",
                    })

        # Inductive type definitions
        match = re.match(r'^inductive\s+(\w+)', line)
        if match:
            ind_name = match.group(1)
            query = name_to_query(ind_name)
            code = extract_lean_block(lines, i)
            if len(query) > 3:
                pairs.append({
                    "query": f"inductive {query}",
                    "positive": code,
                    "language": "lean",
                    "source": "inductive",
                })
                # Constructors
                constructors = re.findall(r'\|\s*(\w+)', code)
                if constructors:
                    pairs.append({
                        "query": f"type with constructors {' '.join(name_to_query(c) for c in constructors[:4])}",
                        "positive": code,
                        "language": "lean",
                        "source": "inductive_constructors",
                    })

        # Class definitions
        match = re.match(r'^class\s+(\w+)', line)
        if match:
            class_name = match.group(1)
            query = name_to_query(class_name)
            code = extract_lean_block(lines, i)
            if len(query) > 3:
                pairs.append({
                    "query": f"{query} typeclass",
                    "positive": code,
                    "language": "lean",
                    "source": "class",
                })

        # Instance definitions
        match = re.match(r'^instance\s*(?:\[.*?\])?\s*:?\s*(\w+)(?:\s+(\w+))?', line)
        if match:
            first = match.group(1)
            second = match.group(2)
            if second:
                query = f"{name_to_query(second)} is {name_to_query(first)}"
            else:
                query = f"instance {name_to_query(first)}"
            code = extract_lean_block(lines, i)
            if len(code) > 20:
                pairs.append({
                    "query": query,
                    "positive": code,
                    "language": "lean",
                    "source": "instance",
                })

        # Axiom
        match = re.match(r'^axiom\s+(\w+)\s*:', line)
        if match:
            axiom_name = match.group(1)
            query = name_to_query(axiom_name)
            code = extract_lean_block(lines, i)
            pairs.append({
                "query": f"axiom {query}",
                "positive": code,
                "language": "lean",
                "source": "axiom",
            })

    return pairs


def extract_lean_proof_block(lines: List[str], start: int) -> str:
    """Extract a Lean proof block including the full proof."""
    result = []
    depth = 0
    in_proof = False

    for i in range(start, min(start + 100, len(lines))):
        line = lines[i]
        result.append(line)

        # Track proof blocks
        if ":=" in line or "by" in line or "where" in line:
            in_proof = True

        # Track indentation/structure
        if in_proof:
            stripped = line.strip()
            if stripped == "" and i > start + 1:
                # Check if next non-empty line is a new definition
                for j in range(i + 1, min(i + 5, len(lines))):
                    next_line = lines[j].strip()
                    if next_line and re.match(r'^(theorem|lemma|def|structure|instance|class|inductive|axiom)\s', next_line):
                        return "\n".join(result[:-1])
                    if next_line:
                        break

    return "\n".join(result)


def extract_lean_tactics(code: str) -> List[str]:
    """Extract tactic names used in a Lean proof."""
    # Common Lean 4 tactics
    tactic_pattern = r'\b(simp|rfl|exact|apply|intro|cases|induction|constructor|rw|have|let|show|calc|trivial|assumption|contradiction|exfalso|funext|ext|congr|ring|linarith|omega|decide|native_decide|norm_num|positivity|nlinarith|polyrith|field_simp|push_neg|by_contra|by_cases|split|left|right|use|obtain|rcases|rintro|refine|convert|specialize|revert|clear|rename|subst|injection|generalize|change|unfold|dsimp|norm_cast|push_cast|lift|swap|rotate|focus|all_goals|any_goals|first|try|repeat|iterate)\b'

    tactics = re.findall(tactic_pattern, code)
    # Unique, preserve order
    seen = set()
    unique = []
    for t in tactics:
        if t not in seen:
            seen.add(t)
            unique.append(t)
    return unique


def extract_verus_block(lines: List[str], start: int) -> str:
    """Extract a Verus proof/spec function block."""
    result = []
    depth = 0
    started = False

    for i in range(start, min(start + 100, len(lines))):
        line = lines[i]
        result.append(line)

        # Track braces for block detection
        depth += line.count("{") - line.count("}")

        if "{" in line:
            started = True

        # End at closing brace at same level
        if started and depth <= 0:
            break

        # Also check for next function definition (Verus functions may not use braces)
        if i > start and re.match(r'\s*(?:pub\s+)?(?:proof|spec|fn|struct|enum|impl|trait)\s', line):
            result.pop()
            break

    return "\n".join(result)


def extract_verus_contracts(code: str, contract_type: str) -> List[str]:
    """Extract requires/ensures conditions from Verus code."""
    conditions = []

    # Pattern for contract blocks
    pattern = rf'{contract_type}\s*([^,{{}}]+(?:,\s*[^,{{}}]+)*)'

    for match in re.finditer(pattern, code, re.MULTILINE):
        cond = match.group(1).strip()
        # Split multiple conditions
        for c in cond.split(","):
            c = c.strip()
            if c and len(c) > 3:
                # Clean up and simplify
                c = re.sub(r'\s+', ' ', c)
                conditions.append(c[:100])  # Truncate long conditions

    return conditions


def extract_tla_structure(content: str, file_path: str) -> List[Dict]:
    """Extract structural elements from TLA+ specifications."""
    pairs = []
    lines = content.split("\n")

    # Module name
    match = re.search(r'^----\s*MODULE\s+(\w+)\s*----', content, re.MULTILINE)
    if match:
        module_name = match.group(1)
        pairs.append({
            "query": f"{name_to_query(module_name)} specification",
            "positive": content[:3000],
            "language": "tla",
            "source": "module",
        })

    for i, line in enumerate(lines):
        # Operators/definitions
        match = re.match(r'^(\w+)\s*(?:\([^)]*\))?\s*==', line)
        if match:
            op_name = match.group(1)
            if op_name not in ["VARIABLES", "CONSTANTS", "ASSUME"]:
                query = name_to_query(op_name)
                code = extract_tla_block(lines, i)
                if len(query) > 3 and len(code) > 20:
                    pairs.append({
                        "query": f"operator {query}",
                        "positive": code,
                        "language": "tla",
                        "source": "operator",
                    })

        # Invariants (often named *Inv or *Invariant)
        if re.match(r'^(\w*Inv\w*|\w*Invariant\w*)\s*==', line, re.I):
            match = re.match(r'^(\w+)', line)
            if match:
                inv_name = match.group(1)
                query = name_to_query(inv_name)
                code = extract_tla_block(lines, i)
                pairs.append({
                    "query": f"invariant {query}",
                    "positive": code,
                    "language": "tla",
                    "source": "invariant",
                })

        # Type invariants
        if re.match(r'^TypeInvariant\s*==|^TypeOK\s*==', line, re.I):
            code = extract_tla_block(lines, i)
            pairs.append({
                "query": "type invariant specification",
                "positive": code,
                "language": "tla",
                "source": "type_invariant",
            })

        # Init and Next
        if re.match(r'^Init\s*==', line):
            code = extract_tla_block(lines, i)
            pairs.append({
                "query": "initial state specification",
                "positive": code,
                "language": "tla",
                "source": "init",
            })

        if re.match(r'^Next\s*==', line):
            code = extract_tla_block(lines, i)
            pairs.append({
                "query": "next state transition",
                "positive": code,
                "language": "tla",
                "source": "next",
            })

        # Safety/Liveness properties
        if re.match(r'^(\w*Safety\w*|\w*Liveness\w*)\s*==', line, re.I):
            match = re.match(r'^(\w+)', line)
            if match:
                prop_name = match.group(1)
                query = name_to_query(prop_name)
                code = extract_tla_block(lines, i)
                pairs.append({
                    "query": f"property {query}",
                    "positive": code,
                    "language": "tla",
                    "source": "property",
                })

    return pairs


def extract_tla_block(lines: List[str], start: int) -> str:
    """Extract a TLA+ definition block."""
    result = [lines[start]]

    for i in range(start + 1, min(start + 50, len(lines))):
        line = lines[i]

        # TLA+ definitions end at next definition or blank line followed by definition
        if re.match(r'^(\w+)\s*(?:\([^)]*\))?\s*==', line):
            break
        if re.match(r'^----', line):
            break

        result.append(line)

        # Stop at blank line if followed by another definition
        if line.strip() == "" and i + 1 < len(lines):
            next_line = lines[i + 1]
            if re.match(r'^(\w+)\s*(?:\([^)]*\))?\s*==', next_line):
                break

    return "\n".join(result)


def extract_smt_structure(content: str, file_path: str) -> List[Dict]:
    """Extract structural elements from SMT-LIB/Z3 code."""
    pairs = []

    # Define-fun
    for match in re.finditer(r'\(define-fun\s+(\w+)\s+\(([^)]*)\)\s+(\w+)', content):
        fn_name = match.group(1)
        params = match.group(2)
        ret_type = match.group(3)
        query = name_to_query(fn_name)

        # Extract full definition
        start = match.start()
        depth = 0
        end = start
        for j, c in enumerate(content[start:]):
            if c == '(':
                depth += 1
            elif c == ')':
                depth -= 1
                if depth == 0:
                    end = start + j + 1
                    break

        code = content[start:end]
        if len(query) > 3:
            pairs.append({
                "query": f"define {query}",
                "positive": code,
                "language": "smt",
                "source": "define_fun",
            })

    # Declare-fun
    for match in re.finditer(r'\(declare-fun\s+(\w+)', content):
        fn_name = match.group(1)
        query = name_to_query(fn_name)
        if len(query) > 3:
            # Get the line
            line_start = content.rfind('\n', 0, match.start()) + 1
            line_end = content.find('\n', match.end())
            line = content[line_start:line_end if line_end > 0 else len(content)]
            pairs.append({
                "query": f"declare {query}",
                "positive": line,
                "language": "smt",
                "source": "declare_fun",
            })

    # Assert statements
    for match in re.finditer(r'\(assert\s+', content):
        start = match.start()
        depth = 0
        end = start
        for j, c in enumerate(content[start:]):
            if c == '(':
                depth += 1
            elif c == ')':
                depth -= 1
                if depth == 0:
                    end = start + j + 1
                    break

        code = content[start:end]
        if len(code) > 20 and len(code) < 2000:
            # Try to extract meaningful query from the assertion
            inner = code[8:-1].strip()  # Remove (assert and )
            pairs.append({
                "query": "assert constraint",
                "positive": code,
                "language": "smt",
                "source": "assert",
            })

    return pairs


def extract_coq_structure(content: str, file_path: str) -> List[Dict]:
    """Extract Coq theorem/lemma → proof pairs (spec-implementation)."""
    pairs = []
    lines = content.split("\n")

    i = 0
    while i < len(lines):
        line = lines[i]

        # Theorem/Lemma definitions
        match = re.match(r'^(Theorem|Lemma|Proposition|Corollary)\s+(\w+)\s*(.*)$', line)
        if match:
            kind = match.group(1).lower()
            name = match.group(2)
            rest = match.group(3)

            # Collect the full statement (may span multiple lines until ".")
            stmt_lines = [line]
            j = i + 1
            while j < len(lines) and not stmt_lines[-1].rstrip().endswith('.'):
                stmt_lines.append(lines[j])
                j += 1

            statement = "\n".join(stmt_lines)

            # Now find the proof (Proof. ... Qed.)
            proof_lines = []
            in_proof = False
            for k in range(j, min(j + 200, len(lines))):
                pline = lines[k]
                if re.match(r'^\s*Proof\.', pline):
                    in_proof = True
                if in_proof:
                    proof_lines.append(pline)
                if re.match(r'^\s*(Qed|Defined|Admitted)\s*\.', pline):
                    break

            proof = "\n".join(proof_lines)
            full_code = statement + "\n" + proof

            query = name_to_query(name)
            if len(query) > 3 and len(full_code) > 50:
                # Spec (statement) → Implementation (proof)
                pairs.append({
                    "query": f"{kind} {query}",
                    "positive": full_code,
                    "language": "coq",
                    "source": f"coq_{kind}",
                })

                # Also: statement as query → proof as positive
                if len(statement) < 500 and len(proof) > 20:
                    # Clean up statement for query
                    stmt_query = re.sub(r'\s+', ' ', statement).strip()
                    stmt_query = stmt_query[:200]  # Truncate
                    pairs.append({
                        "query": f"prove {stmt_query}",
                        "positive": full_code,
                        "language": "coq",
                        "source": "coq_spec_proof",
                    })

            i = j + len(proof_lines)
            continue

        # Definition
        match = re.match(r'^Definition\s+(\w+)', line)
        if match:
            def_name = match.group(1)
            # Collect until "."
            def_lines = [line]
            j = i + 1
            while j < len(lines) and not def_lines[-1].rstrip().endswith('.'):
                def_lines.append(lines[j])
                j += 1

            code = "\n".join(def_lines)
            query = name_to_query(def_name)
            if len(query) > 3 and len(code) > 20:
                pairs.append({
                    "query": f"define {query}",
                    "positive": code,
                    "language": "coq",
                    "source": "coq_definition",
                })
            i = j
            continue

        # Inductive type
        match = re.match(r'^Inductive\s+(\w+)', line)
        if match:
            ind_name = match.group(1)
            ind_lines = [line]
            j = i + 1
            while j < len(lines) and not ind_lines[-1].rstrip().endswith('.'):
                ind_lines.append(lines[j])
                j += 1

            code = "\n".join(ind_lines)
            query = name_to_query(ind_name)
            if len(query) > 3:
                pairs.append({
                    "query": f"inductive type {query}",
                    "positive": code,
                    "language": "coq",
                    "source": "coq_inductive",
                })
            i = j
            continue

        i += 1

    return pairs


def extract_dafny_structure(content: str, file_path: str) -> List[Dict]:
    """Extract Dafny method contracts → implementation pairs (spec-implementation)."""
    pairs = []
    lines = content.split("\n")

    i = 0
    while i < len(lines):
        line = lines[i]

        # Method/function with contracts
        match = re.match(r'^(\s*)(?:ghost\s+)?(method|function|lemma)\s+(\w+)', line)
        if match:
            indent = match.group(1)
            kind = match.group(2)
            name = match.group(3)

            # Collect signature and contracts
            sig_lines = [line]
            requires_clauses = []
            ensures_clauses = []
            decreases_clauses = []

            j = i + 1
            while j < len(lines):
                next_line = lines[j]

                # Check for contract keywords
                if re.match(r'\s*requires\s+', next_line):
                    requires_clauses.append(next_line.strip())
                    sig_lines.append(next_line)
                elif re.match(r'\s*ensures\s+', next_line):
                    ensures_clauses.append(next_line.strip())
                    sig_lines.append(next_line)
                elif re.match(r'\s*decreases\s+', next_line):
                    decreases_clauses.append(next_line.strip())
                    sig_lines.append(next_line)
                elif re.match(r'\s*modifies\s+', next_line):
                    sig_lines.append(next_line)
                elif re.match(r'\s*\{', next_line):
                    # Start of body
                    break
                elif next_line.strip() == '' or re.match(r'\s*//', next_line):
                    sig_lines.append(next_line)
                else:
                    break
                j += 1

            # Extract body
            body_lines = []
            if j < len(lines) and '{' in lines[j]:
                depth = 0
                for k in range(j, min(j + 200, len(lines))):
                    body_line = lines[k]
                    body_lines.append(body_line)
                    depth += body_line.count('{') - body_line.count('}')
                    if depth <= 0 and '{' in lines[j]:
                        break

            signature = "\n".join(sig_lines)
            body = "\n".join(body_lines)
            full_code = signature + "\n" + body

            query = name_to_query(name)
            if len(query) > 3 and len(full_code) > 50:
                pairs.append({
                    "query": f"{kind} {query}",
                    "positive": full_code,
                    "language": "dafny",
                    "source": f"dafny_{kind}",
                })

                # Contract-based queries
                if requires_clauses:
                    req_query = " ".join(requires_clauses)[:150]
                    pairs.append({
                        "query": f"requires {req_query}",
                        "positive": full_code,
                        "language": "dafny",
                        "source": "dafny_requires",
                    })

                if ensures_clauses:
                    ens_query = " ".join(ensures_clauses)[:150]
                    pairs.append({
                        "query": f"ensures {ens_query}",
                        "positive": full_code,
                        "language": "dafny",
                        "source": "dafny_ensures",
                    })

                # Spec (requires + ensures) → Implementation
                if requires_clauses or ensures_clauses:
                    spec_query = f"{kind} {query}"
                    if requires_clauses:
                        spec_query += f" requires {requires_clauses[0][:50]}"
                    if ensures_clauses:
                        spec_query += f" ensures {ensures_clauses[0][:50]}"
                    pairs.append({
                        "query": spec_query[:200],
                        "positive": full_code,
                        "language": "dafny",
                        "source": "dafny_spec_impl",
                    })

            i = j + len(body_lines)
            continue

        # Datatype
        match = re.match(r'^datatype\s+(\w+)', line)
        if match:
            dt_name = match.group(1)
            dt_lines = [line]
            j = i + 1
            # Collect until we hit a blank line or new definition
            while j < len(lines):
                next_line = lines[j]
                if next_line.strip() == '' or re.match(r'^(method|function|lemma|datatype|class)', next_line):
                    break
                dt_lines.append(next_line)
                j += 1

            code = "\n".join(dt_lines)
            query = name_to_query(dt_name)
            if len(query) > 3:
                pairs.append({
                    "query": f"datatype {query}",
                    "positive": code,
                    "language": "dafny",
                    "source": "dafny_datatype",
                })
            i = j
            continue

        i += 1

    return pairs


def extract_boogie_structure(content: str, file_path: str) -> List[Dict]:
    """Extract Boogie procedure specs → implementation pairs."""
    pairs = []
    lines = content.split("\n")

    i = 0
    while i < len(lines):
        line = lines[i]

        # Procedure with pre/post conditions
        match = re.match(r'^procedure\s+(\w+)', line)
        if match:
            proc_name = match.group(1)

            # Collect signature and contracts
            proc_lines = [line]
            requires_clauses = []
            ensures_clauses = []
            modifies_clauses = []

            j = i + 1
            while j < len(lines):
                next_line = lines[j]

                if re.match(r'\s*requires\s+', next_line):
                    requires_clauses.append(next_line.strip())
                    proc_lines.append(next_line)
                elif re.match(r'\s*ensures\s+', next_line):
                    ensures_clauses.append(next_line.strip())
                    proc_lines.append(next_line)
                elif re.match(r'\s*modifies\s+', next_line):
                    modifies_clauses.append(next_line.strip())
                    proc_lines.append(next_line)
                elif re.match(r'\s*\{', next_line):
                    break
                elif next_line.strip() == '' or re.match(r'\s*//', next_line):
                    proc_lines.append(next_line)
                else:
                    break
                j += 1

            # Extract body
            body_lines = []
            if j < len(lines) and '{' in lines[j]:
                depth = 0
                for k in range(j, min(j + 200, len(lines))):
                    body_line = lines[k]
                    body_lines.append(body_line)
                    depth += body_line.count('{') - body_line.count('}')
                    if depth <= 0:
                        break

            signature = "\n".join(proc_lines)
            body = "\n".join(body_lines)
            full_code = signature + "\n" + body

            query = name_to_query(proc_name)
            if len(query) > 3 and len(full_code) > 50:
                pairs.append({
                    "query": f"procedure {query}",
                    "positive": full_code,
                    "language": "boogie",
                    "source": "boogie_procedure",
                })

                # Spec-based queries
                if requires_clauses or ensures_clauses:
                    spec_parts = []
                    if requires_clauses:
                        spec_parts.append(f"requires {requires_clauses[0][:50]}")
                    if ensures_clauses:
                        spec_parts.append(f"ensures {ensures_clauses[0][:50]}")
                    spec_query = f"procedure {query} " + " ".join(spec_parts)
                    pairs.append({
                        "query": spec_query[:200],
                        "positive": full_code,
                        "language": "boogie",
                        "source": "boogie_spec_impl",
                    })

            i = j + len(body_lines)
            continue

        # Function (pure)
        match = re.match(r'^function\s+(\w+)', line)
        if match:
            fn_name = match.group(1)
            fn_lines = [line]
            j = i + 1
            # Functions are usually single line or until }
            while j < len(lines):
                fn_lines.append(lines[j])
                if '}' in lines[j]:
                    break
                j += 1

            code = "\n".join(fn_lines)
            query = name_to_query(fn_name)
            if len(query) > 3:
                pairs.append({
                    "query": f"function {query}",
                    "positive": code,
                    "language": "boogie",
                    "source": "boogie_function",
                })
            i = j + 1
            continue

        i += 1

    return pairs


def clean_lean_signature(sig: str) -> str:
    """Clean Lean type signature into readable query."""
    # Remove type annotations
    sig = re.sub(r'\{[^}]*\}', '', sig)
    sig = re.sub(r'\[[^\]]*\]', '', sig)
    # Simplify arrows
    sig = sig.replace('→', 'implies')
    sig = sig.replace('->', 'implies')
    # Clean up
    sig = re.sub(r'\s+', ' ', sig).strip()
    return sig


def extract_block(lines: List[str], start: int, open_char: str, close_char: str) -> str:
    """Extract a brace-delimited block."""
    result = []
    depth = 0
    started = False

    for i in range(start, min(start + 100, len(lines))):
        line = lines[i]
        result.append(line)

        depth += line.count(open_char) - line.count(close_char)

        if open_char in line:
            started = True

        if started and depth <= 0:
            break

    return "\n".join(result)


def extract_python_block(lines: List[str], start: int) -> str:
    """Extract an indentation-delimited Python block."""
    if start >= len(lines):
        return ""

    first_line = lines[start]
    base_indent = len(first_line) - len(first_line.lstrip())

    result = [first_line]

    for i in range(start + 1, min(start + 100, len(lines))):
        line = lines[i]

        # Empty lines are OK
        if not line.strip():
            result.append(line)
            continue

        # Check indent
        current_indent = len(line) - len(line.lstrip())
        if current_indent <= base_indent and line.strip():
            break

        result.append(line)

    return "\n".join(result)


def extract_lean_block(lines: List[str], start: int) -> str:
    """Extract a Lean definition block."""
    result = []

    for i in range(start, min(start + 50, len(lines))):
        line = lines[i]
        result.append(line)

        # Lean blocks often end with empty line or next definition
        if i > start and line.strip() == "":
            break
        if i > start and re.match(r'^(theorem|lemma|def|structure|instance|class)\s', line):
            result.pop()
            break

    return "\n".join(result)


def extract_filename_pairs(file_path: Path, content: str, language: str, repo_root: Path = None) -> List[Dict]:
    """Extract training pairs from file name and full path semantics."""
    pairs = []

    # Get filename without extension
    name = file_path.stem

    # Skip generic names
    if name.lower() in ["main", "index", "mod", "lib", "init", "__init__", "test", "setup", "utils", "helpers"]:
        return []

    # Get relative path from repo root
    if repo_root:
        try:
            rel_path = file_path.relative_to(repo_root)
        except ValueError:
            rel_path = file_path
    else:
        rel_path = file_path

    # Build full path query: crates/sg-core/src/embedder.rs → "crates sg core src embedder"
    path_parts = [p for p in rel_path.parts[:-1] if p not in ["src", "lib", "pkg", "internal", "."]]
    path_query = " ".join(name_to_query(p) for p in path_parts)
    file_query = name_to_query(name)

    # Use first ~50 lines as the "positive" (file overview)
    lines = content.split("\n")[:60]
    code_preview = "\n".join(lines)

    if len(code_preview) > 100:
        # Full path query
        full_query = f"{path_query} {file_query}".strip()
        if len(full_query) > 5:
            pairs.append({
                "query": full_query,
                "positive": code_preview,
                "language": language,
                "source": "filepath",
                "path": str(rel_path),
            })

        # Also just the filename for shorter queries
        if len(file_query) > 5 and file_query != full_query:
            pairs.append({
                "query": file_query,
                "positive": code_preview,
                "language": language,
                "source": "filename",
                "path": str(rel_path),
            })

    return pairs


def get_module_path(file_path: Path, repo_root: Path) -> str:
    """Get module path for context (e.g., 'sg_core::embedder')."""
    try:
        rel = file_path.relative_to(repo_root)
    except ValueError:
        return ""

    parts = []
    for p in rel.parts:
        if p in ["src", "lib", "mod.rs", "__init__.py"]:
            continue
        if p.endswith(".rs") or p.endswith(".py"):
            p = p.rsplit(".", 1)[0]
        parts.append(p)

    return "::".join(parts) if parts else ""


def process_file(file_path: Path) -> List[Dict]:
    """Process a single file and extract structural training pairs."""
    suffix = file_path.suffix.lower()
    language = LANG_EXTENSIONS.get(suffix)

    if not language:
        return []

    try:
        content = file_path.read_text(encoding="utf-8", errors="ignore")
    except Exception:
        return []

    if len(content) < 100:
        return []

    pairs = []

    # Extract from filename/path
    pairs.extend(extract_filename_pairs(file_path, content, language))

    # Extract from code structure
    if language == "rust":
        pairs.extend(extract_rust_structure(content, str(file_path)))
    elif language == "python":
        pairs.extend(extract_python_structure(content, str(file_path)))
    elif language == "lean":
        pairs.extend(extract_lean_structure(content, str(file_path)))
    elif language in ["cpp", "c"]:
        pairs.extend(extract_cpp_structure(content, str(file_path)))
    elif language == "swift":
        pairs.extend(extract_swift_structure(content, str(file_path)))
    elif language == "objc":
        pairs.extend(extract_objc_structure(content, str(file_path)))
    elif language == "tla":
        pairs.extend(extract_tla_structure(content, str(file_path)))
    elif language == "smt":
        pairs.extend(extract_smt_structure(content, str(file_path)))
    elif language == "coq":
        pairs.extend(extract_coq_structure(content, str(file_path)))
    elif language == "dafny":
        pairs.extend(extract_dafny_structure(content, str(file_path)))
    elif language == "boogie":
        pairs.extend(extract_boogie_structure(content, str(file_path)))
    else:
        # Generic extraction for other languages
        pairs.extend(extract_generic_structure(content, language, str(file_path)))

    return pairs


def extract_cpp_structure(content: str, file_path: str) -> List[Dict]:
    """Extract structural elements from C/C++ code."""
    pairs = []
    lines = content.split("\n")

    for i, line in enumerate(lines):
        # Class definitions
        match = re.match(r'\s*(?:template\s*<[^>]*>\s*)?class\s+(\w+)', line)
        if match:
            class_name = match.group(1)
            query = name_to_query(class_name)
            code = extract_block(lines, i, "{", "}")
            if len(query) > 3 and len(code) > 30:
                pairs.append({
                    "query": f"{query} class",
                    "positive": code,
                    "language": "cpp",
                    "source": "class_def",
                })

        # Struct definitions
        match = re.match(r'\s*(?:typedef\s+)?struct\s+(\w+)', line)
        if match:
            struct_name = match.group(1)
            query = name_to_query(struct_name)
            code = extract_block(lines, i, "{", "}")
            if len(query) > 3 and len(code) > 30:
                pairs.append({
                    "query": f"{query} struct",
                    "positive": code,
                    "language": "cpp",
                    "source": "struct_def",
                })

        # Function definitions (C-style and method-style)
        match = re.match(r'\s*(?:static\s+)?(?:inline\s+)?(?:virtual\s+)?(?:const\s+)?(\w+(?:\s*[*&]\s*)?)\s+(\w+)\s*\([^)]*\)(?:\s*const)?(?:\s*override)?(?:\s*=\s*0)?(?:\s*\{)?', line)
        if match:
            return_type = match.group(1)
            fn_name = match.group(2)

            if fn_name in ["main", "if", "while", "for", "switch", "return"]:
                continue

            query = name_to_query(fn_name)
            code = extract_block(lines, i, "{", "}")

            if len(query) > 3 and len(code) > 30:
                pairs.append({
                    "query": query,
                    "positive": code,
                    "language": "cpp",
                    "source": "function",
                })

        # Namespace
        match = re.match(r'\s*namespace\s+(\w+)', line)
        if match:
            ns_name = match.group(1)
            query = name_to_query(ns_name)
            code = extract_block(lines, i, "{", "}")
            if len(query) > 3:
                pairs.append({
                    "query": f"{query} namespace",
                    "positive": code[:2000],  # Namespaces can be huge
                    "language": "cpp",
                    "source": "namespace",
                })

        # Enum
        match = re.match(r'\s*enum\s+(?:class\s+)?(\w+)', line)
        if match:
            enum_name = match.group(1)
            query = name_to_query(enum_name)
            code = extract_block(lines, i, "{", "}")
            if len(query) > 3:
                pairs.append({
                    "query": f"{query} enum",
                    "positive": code,
                    "language": "cpp",
                    "source": "enum_def",
                })

    return pairs


def extract_swift_structure(content: str, file_path: str) -> List[Dict]:
    """Extract structural elements from Swift code."""
    pairs = []
    lines = content.split("\n")

    for i, line in enumerate(lines):
        # Class definitions
        match = re.match(r'\s*(?:public\s+|private\s+|internal\s+)?(?:final\s+)?class\s+(\w+)', line)
        if match:
            class_name = match.group(1)
            query = name_to_query(class_name)
            code = extract_block(lines, i, "{", "}")
            if len(query) > 3 and len(code) > 30:
                pairs.append({
                    "query": f"{query} class",
                    "positive": code,
                    "language": "swift",
                    "source": "class_def",
                })

        # Struct definitions
        match = re.match(r'\s*(?:public\s+|private\s+)?struct\s+(\w+)', line)
        if match:
            struct_name = match.group(1)
            query = name_to_query(struct_name)
            code = extract_block(lines, i, "{", "}")
            if len(query) > 3 and len(code) > 30:
                pairs.append({
                    "query": f"{query} struct",
                    "positive": code,
                    "language": "swift",
                    "source": "struct_def",
                })

        # Protocol definitions
        match = re.match(r'\s*(?:public\s+)?protocol\s+(\w+)', line)
        if match:
            proto_name = match.group(1)
            query = name_to_query(proto_name)
            code = extract_block(lines, i, "{", "}")
            if len(query) > 3:
                pairs.append({
                    "query": f"{query} protocol",
                    "positive": code,
                    "language": "swift",
                    "source": "protocol_def",
                })

        # Function definitions
        match = re.match(r'\s*(?:public\s+|private\s+|internal\s+)?(?:static\s+)?(?:override\s+)?func\s+(\w+)\s*(?:<[^>]*>)?\s*\([^)]*\)(?:\s*->\s*(\w+))?', line)
        if match:
            fn_name = match.group(1)
            return_type = match.group(2)

            query = name_to_query(fn_name)
            code = extract_block(lines, i, "{", "}")

            if len(query) > 3 and len(code) > 30:
                pairs.append({
                    "query": query,
                    "positive": code,
                    "language": "swift",
                    "source": "function",
                })
                if return_type:
                    pairs.append({
                        "query": f"function returning {name_to_query(return_type)}",
                        "positive": code,
                        "language": "swift",
                        "source": "function_signature",
                    })

        # Enum
        match = re.match(r'\s*(?:public\s+)?enum\s+(\w+)', line)
        if match:
            enum_name = match.group(1)
            query = name_to_query(enum_name)
            code = extract_block(lines, i, "{", "}")
            if len(query) > 3:
                pairs.append({
                    "query": f"{query} enum",
                    "positive": code,
                    "language": "swift",
                    "source": "enum_def",
                })

    return pairs


def extract_objc_structure(content: str, file_path: str) -> List[Dict]:
    """Extract structural elements from Objective-C code."""
    pairs = []
    lines = content.split("\n")

    for i, line in enumerate(lines):
        # Interface definitions
        match = re.match(r'\s*@interface\s+(\w+)', line)
        if match:
            class_name = match.group(1)
            query = name_to_query(class_name)
            # Find @end
            code_lines = [line]
            for j in range(i + 1, min(i + 200, len(lines))):
                code_lines.append(lines[j])
                if lines[j].strip().startswith("@end"):
                    break
            code = "\n".join(code_lines)
            if len(query) > 3:
                pairs.append({
                    "query": f"{query} interface",
                    "positive": code,
                    "language": "objc",
                    "source": "interface_def",
                })

        # Implementation
        match = re.match(r'\s*@implementation\s+(\w+)', line)
        if match:
            class_name = match.group(1)
            query = name_to_query(class_name)
            code_lines = [line]
            for j in range(i + 1, min(i + 500, len(lines))):
                code_lines.append(lines[j])
                if lines[j].strip().startswith("@end"):
                    break
            code = "\n".join(code_lines)
            if len(query) > 3:
                pairs.append({
                    "query": f"{query} implementation",
                    "positive": code[:3000],
                    "language": "objc",
                    "source": "implementation",
                })

        # Method definitions
        match = re.match(r'\s*[-+]\s*\([^)]+\)\s*(\w+)', line)
        if match:
            method_name = match.group(1)
            query = name_to_query(method_name)
            code = extract_block(lines, i, "{", "}")
            if len(query) > 3 and len(code) > 30:
                pairs.append({
                    "query": query,
                    "positive": code,
                    "language": "objc",
                    "source": "method",
                })

        # Protocol
        match = re.match(r'\s*@protocol\s+(\w+)', line)
        if match:
            proto_name = match.group(1)
            query = name_to_query(proto_name)
            code_lines = [line]
            for j in range(i + 1, min(i + 100, len(lines))):
                code_lines.append(lines[j])
                if lines[j].strip().startswith("@end"):
                    break
            code = "\n".join(code_lines)
            if len(query) > 3:
                pairs.append({
                    "query": f"{query} protocol",
                    "positive": code,
                    "language": "objc",
                    "source": "protocol_def",
                })

    return pairs


def extract_generic_structure(content: str, language: str, file_path: str) -> List[Dict]:
    """Generic structure extraction for other languages."""
    pairs = []
    lines = content.split("\n")

    # Function patterns by language
    fn_patterns = {
        "go": r'func\s+(?:\([^)]*\)\s*)?(\w+)\s*\(',
        "python": r'def\s+(\w+)\s*\(',
    }

    pattern = fn_patterns.get(language)
    if not pattern:
        return pairs

    for i, line in enumerate(lines):
        match = re.search(pattern, line)
        if match:
            fn_name = match.group(1)
            if fn_name in ["main", "init", "setup"]:
                continue

            query = name_to_query(fn_name)
            if language == "python":
                code = extract_python_block(lines, i)
            else:
                code = extract_block(lines, i, "{", "}")

            if len(query) > 5 and len(code) > 30:
                pairs.append({
                    "query": query,
                    "positive": code,
                    "language": language,
                    "source": "function",
                })

    return pairs


def is_quality_pair(pair: Dict) -> bool:
    """Check if a pair meets quality standards."""
    query = pair["query"]
    code = pair["positive"]

    if len(query) < 8 or len(query) > 150:
        return False
    if len(code) < 40 or len(code) > 4000:
        return False

    # Skip single words
    if len(query.split()) < 2:
        return False

    return True


def process_directory(
    root: Path,
    max_files: int = 100000,
) -> Generator[Dict, None, None]:
    """Process all files in a directory tree."""
    file_count = 0

    for file_path in root.rglob("*"):
        if file_count >= max_files:
            break

        if not file_path.is_file():
            continue

        # Skip hidden, vendor, test fixtures
        parts = file_path.parts
        if any(p.startswith(".") or p in ["vendor", "node_modules", "target", "__pycache__", "venv"] for p in parts):
            continue

        pairs = process_file(file_path)
        for pair in pairs:
            if is_quality_pair(pair):
                pair["file"] = str(file_path.relative_to(root))
                yield pair

        file_count += 1
        if file_count % 1000 == 0:
            print(f"  Processed {file_count} files...")


def main():
    parser = argparse.ArgumentParser(description="Extract code structure training data")
    parser.add_argument("directories", nargs="+", help="Directories to process")
    parser.add_argument("-o", "--output", required=True, help="Output JSONL file")
    parser.add_argument("--max-files", type=int, default=100000, help="Max files per directory")
    args = parser.parse_args()

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    all_pairs = []
    lang_counts = defaultdict(int)
    source_counts = defaultdict(int)

    for dir_path in args.directories:
        root = Path(dir_path).expanduser()
        if not root.exists():
            print(f"Skipping {root} (not found)")
            continue

        print(f"\nProcessing {root}...")

        for pair in process_directory(root, args.max_files):
            all_pairs.append(pair)
            lang_counts[pair["language"]] += 1
            source_counts[pair["source"]] += 1

    print(f"\nWriting {len(all_pairs)} pairs to {output_path}")

    with output_path.open("w") as f:
        for pair in all_pairs:
            f.write(json.dumps(pair) + "\n")

    print("\nBy language:")
    for lang, count in sorted(lang_counts.items(), key=lambda x: -x[1]):
        print(f"  {lang}: {count}")

    print("\nBy source:")
    for source, count in sorted(source_counts.items(), key=lambda x: -x[1]):
        print(f"  {source}: {count}")

    print(f"\nTotal: {len(all_pairs)} training pairs")


if __name__ == "__main__":
    main()
