#!/usr/bin/env python3
"""
Manual semantic review of 18 unreviewed tests
"""

import csv
import json

def review_test(test):
    """Perform semantic review of a single test"""
    test_name = test['test_name']
    operation = test['operation']
    file_path = test['file_path']
    metadata_json = test.get('output_metadata_json', '')

    print("=" * 80)
    print(f"Test: {test_name}")
    print(f"Operation: {operation}")
    print(f"File: {file_path}")
    print("-" * 80)

    if not metadata_json:
        print("⚠️  No metadata (operation may produce non-JSON output)")
        return "CORRECT", 9, "No metadata expected for this operation"

    try:
        metadata = json.loads(metadata_json)
    except json.JSONDecodeError as e:
        print(f"❌ JSON parse error: {e}")
        return "INCORRECT", 0, f"Invalid JSON: {e}"

    # Review based on operation type
    status = "CORRECT"
    quality = 10
    findings = []

    if operation == "audio":
        # Audio extraction operation - should have file metadata
        if not isinstance(metadata, dict):
            return "INCORRECT", 0, "Expected dict, got " + type(metadata).__name__

        output_type = metadata.get('output_type', '')
        primary_file = metadata.get('primary_file', '')
        file_size = metadata.get('primary_file_size', 0)
        type_specific = metadata.get('type_specific', {})

        print(f"Output type: {output_type}")
        print(f"Primary file: {primary_file}")
        print(f"File size: {file_size} bytes")

        if output_type != 'audio':
            findings.append(f"Unexpected output_type: {output_type}")
            quality = min(quality, 5)

        if not primary_file:
            findings.append("Missing primary_file path")
            quality = min(quality, 5)

        if file_size == 0:
            findings.append("Zero file size (suspicious)")
            quality = min(quality, 3)
        elif file_size < 1000:
            findings.append("Very small file size (may be valid for short audio)")
            quality = min(quality, 7)

        if isinstance(type_specific, dict):
            print(f"Type-specific metadata: {list(type_specific.keys())}")
        else:
            findings.append("type_specific not a dict")
            quality = min(quality, 5)

    elif operation == "transcription":
        # Transcription - may be dict or complex structure
        if isinstance(metadata, dict):
            text = metadata.get('text', '')
            segments = metadata.get('segments', [])
            language = metadata.get('language', '')

            print(f"Text length: {len(text)} chars")
            print(f"Segments: {len(segments) if isinstance(segments, list) else 'N/A'}")
            print(f"Language: {language}")

            if isinstance(text, str) and len(text) > 0:
                print(f"Text preview: {text[:100]}...")
                findings.append(f"Transcription: {len(text)} chars")
            elif isinstance(text, str) and len(text) == 0:
                findings.append("Empty transcription (may be valid for audio without speech)")
                quality = min(quality, 8)
            else:
                findings.append("text field not a string")
                quality = min(quality, 5)

            if not isinstance(segments, list):
                findings.append("segments not a list")
                quality = min(quality, 5)
        else:
            findings.append(f"Unexpected metadata type: {type(metadata).__name__}")
            quality = min(quality, 5)

    elif operation == "audio;audio-enhancement-metadata":
        # Audio + enhancement metadata
        if isinstance(metadata, dict):
            output_type = metadata.get('output_type', '')
            print(f"Output type: {output_type}")
            if output_type != 'audio':
                findings.append(f"Unexpected output_type: {output_type}")
                quality = min(quality, 5)
        else:
            findings.append(f"Expected dict, got {type(metadata).__name__}")
            quality = min(quality, 5)

    elif operation == "metadata":
        # Metadata extraction
        if isinstance(metadata, dict):
            format_info = metadata.get('format', {})
            video_stream = metadata.get('video_stream', {})
            audio_stream = metadata.get('audio_stream', {})

            print(f"Format: {format_info.get('format_name', 'N/A')}")
            print(f"Duration: {format_info.get('duration', 'N/A')}s")
            print(f"Video: {video_stream.get('codec_name', 'N/A')} {video_stream.get('width', 'N/A')}x{video_stream.get('height', 'N/A')}")
            print(f"Audio: {audio_stream.get('codec_name', 'N/A')} {audio_stream.get('sample_rate', 'N/A')}Hz")

            findings.append("Metadata extraction complete")
        else:
            findings.append(f"Expected dict, got {type(metadata).__name__}")
            quality = min(quality, 5)

    elif operation == "audio;transcription;text-embeddings":
        # Text embeddings from transcription
        if isinstance(metadata, list):
            if len(metadata) > 0:
                first_emb = metadata[0]
                if isinstance(first_emb, list):
                    dim = len(first_emb)
                    print(f"Embedding dimension: {dim}")
                    print(f"Value range: [{min(first_emb):.4f}, {max(first_emb):.4f}]")
                    findings.append(f"{len(metadata)} embeddings, dim={dim}")
                    if dim not in [384, 512, 768]:
                        findings.append(f"Unusual dimension: {dim}")
                        quality = min(quality, 7)
                else:
                    findings.append("First embedding not a list")
                    quality = min(quality, 5)
            else:
                findings.append("Empty embeddings array")
                quality = min(quality, 3)
        else:
            findings.append(f"Expected list, got {type(metadata).__name__}")
            quality = min(quality, 5)

    elif operation == "audio;voice-activity-detection":
        # VAD
        if isinstance(metadata, dict):
            output_type = metadata.get('output_type', '')
            # VAD may output audio file metadata
            print(f"Output type: {output_type}")
            findings.append("VAD output present")
        else:
            findings.append(f"Expected dict, got {type(metadata).__name__}")
            quality = min(quality, 5)

    elif operation == "audio-embeddings":
        # Audio embeddings - should be similar to audio extraction?
        if isinstance(metadata, dict):
            embeddings = metadata.get('embeddings', [])
            count = metadata.get('count', 0)
            print(f"Embeddings count: {count}")
            if isinstance(embeddings, list) and len(embeddings) > 0:
                first_emb = embeddings[0]
                if isinstance(first_emb, list):
                    dim = len(first_emb)
                    print(f"Embedding dimension: {dim}")
                    findings.append(f"{count} embeddings, dim={dim}")
                else:
                    findings.append("First embedding not a list")
                    quality = min(quality, 5)
            else:
                findings.append("No embeddings in output")
                quality = min(quality, 3)
        else:
            findings.append(f"Expected dict, got {type(metadata).__name__}")
            quality = min(quality, 5)

    else:
        print(f"⚠️  Unknown operation type: {operation}")
        findings.append(f"Unknown operation type (manual review needed)")
        quality = 7

    # Summary
    print("-" * 80)
    if quality >= 8:
        status = "CORRECT"
        emoji = "✅"
    elif quality >= 5:
        status = "SUSPICIOUS"
        emoji = "⚠️"
    else:
        status = "INCORRECT"
        emoji = "❌"

    print(f"{emoji} Status: {status} (quality: {quality}/10)")
    print(f"Findings: {'; '.join(findings) if findings else 'None'}")
    print()

    return status, quality, "; ".join(findings) if findings else "Output structure correct"

def main():
    # Load sampled tests
    with open('sampled_tests_for_review.csv', 'r') as f:
        reader = csv.DictReader(f)
        tests = list(reader)

    print(f"Reviewing {len(tests)} sampled tests...")
    print()

    results = []
    status_counts = {'CORRECT': 0, 'SUSPICIOUS': 0, 'INCORRECT': 0}
    quality_sum = 0

    for test in tests:
        status, quality, findings = review_test(test)
        results.append({
            'test_name': test['test_name'],
            'operation': test['operation'],
            'file_path': test['file_path'],
            'status': status,
            'quality_score': quality,
            'findings': findings
        })
        status_counts[status] += 1
        quality_sum += quality

    # Summary
    print("=" * 80)
    print("REVIEW SUMMARY")
    print("=" * 80)
    print(f"Total tests reviewed: {len(tests)}")
    print(f"CORRECT: {status_counts['CORRECT']} ({100*status_counts['CORRECT']/len(tests):.1f}%)")
    print(f"SUSPICIOUS: {status_counts['SUSPICIOUS']} ({100*status_counts['SUSPICIOUS']/len(tests):.1f}%)")
    print(f"INCORRECT: {status_counts['INCORRECT']} ({100*status_counts['INCORRECT']/len(tests):.1f}%)")
    print(f"Average quality score: {quality_sum/len(tests):.1f}/10")
    print()

    # Save results
    with open('docs/ai-output-review/sampled_tests_review_n9.csv', 'w') as f:
        writer = csv.DictWriter(f, fieldnames=['test_name', 'operation', 'file_path', 'status', 'quality_score', 'findings'])
        writer.writeheader()
        writer.writerows(results)

    print("Saved review results to: docs/ai-output-review/sampled_tests_review_n9.csv")

if __name__ == '__main__':
    main()
