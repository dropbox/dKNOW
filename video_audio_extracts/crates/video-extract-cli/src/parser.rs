//! Operation string parser with parallel syntax support
//!
//! Parses operation strings like "[keyframes,audio];[obj-detect,transcription]"
//! into structured groups for pipeline building.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Empty operation string")]
    EmptyInput,

    #[error("Empty operation group at position {position}: {group}")]
    EmptyGroup { position: usize, group: String },

    #[error("Mismatched brackets in: {input}")]
    MismatchedBrackets { input: String },

    #[error("Invalid syntax: {message}")]
    InvalidSyntax { message: String },
}

/// Parse operation string with parallel syntax
///
/// # Syntax
///
/// - **Semicolons (`;`)**: Separate sequential stages
/// - **Brackets (`[...]`)**: Indicate parallel operations within a stage
/// - **Commas (`,`)**: Separate operations within a parallel group
/// - **No brackets**: Sequential operations (backward compatible)
///
/// # Examples
///
/// Sequential (backward compatible):
/// ```text
/// "audio,transcription" → vec![vec!["audio"], vec!["transcription"]]
/// ```
///
/// Parallel:
/// ```text
/// "[audio,keyframes]" → vec![vec!["audio", "keyframes"]]
/// ```
///
/// Mixed (sequential then parallel):
/// ```text
/// "audio;[transcription,diarization]" → vec![vec!["audio"], vec!["transcription", "diarization"]]
/// ```
///
/// # Returns
///
/// `Vec<Vec<String>>` where:
/// - Outer Vec: sequential stages
/// - Inner Vec: parallel operations at that stage (single element = sequential)
///
/// # Errors
///
/// Returns `ParseError` if:
/// - Input is empty
/// - Brackets are mismatched
/// - Operation groups are empty
/// - Invalid syntax
pub fn parse_ops_string(input: &str) -> Result<Vec<Vec<String>>, ParseError> {
    let input = input.trim();

    if input.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    // Validate bracket matching first
    validate_brackets(input)?;

    // Check if input contains semicolons - if not, this is backward-compatible mode
    // "audio,transcription" should be treated as "audio;transcription" (sequential)
    let has_semicolons = input.contains(';');
    let has_brackets = input.contains('[') || input.contains(']');

    if !has_semicolons && !has_brackets {
        // Backward compatible mode: comma-separated means sequential
        // "audio,transcription" → [[audio], [transcription]]
        let ops: Vec<String> = input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if ops.is_empty() {
            return Err(ParseError::EmptyInput);
        }

        // Each operation is its own sequential stage
        return Ok(ops.into_iter().map(|op| vec![op]).collect());
    }

    // New syntax: semicolons separate stages, brackets indicate parallel
    // Split by semicolons to get stage groups
    let stage_groups: Vec<&str> = input.split(';').map(|s| s.trim()).collect();

    // Pre-allocate result Vec with exact capacity from stage_groups length
    let mut result = Vec::with_capacity(stage_groups.len());

    for (idx, group) in stage_groups.iter().enumerate() {
        if group.is_empty() {
            return Err(ParseError::EmptyGroup {
                position: idx,
                group: group.to_string(),
            });
        }

        let ops = parse_group(group)?;

        if ops.is_empty() {
            return Err(ParseError::EmptyGroup {
                position: idx,
                group: group.to_string(),
            });
        }

        result.push(ops);
    }

    Ok(result)
}

/// Validate that brackets are properly matched
fn validate_brackets(input: &str) -> Result<(), ParseError> {
    let mut depth = 0;
    let mut max_depth = 0;

    for ch in input.chars() {
        match ch {
            '[' => {
                depth += 1;
                max_depth = max_depth.max(depth);
            }
            ']' => {
                depth -= 1;
                if depth < 0 {
                    return Err(ParseError::MismatchedBrackets {
                        input: input.to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    if depth != 0 {
        return Err(ParseError::MismatchedBrackets {
            input: input.to_string(),
        });
    }

    // Reject nested brackets (max depth > 1)
    if max_depth > 1 {
        return Err(ParseError::InvalidSyntax {
            message: format!("Nested brackets are not supported: {}", input),
        });
    }

    Ok(())
}

/// Parse a single group (either bracketed or not)
fn parse_group(group: &str) -> Result<Vec<String>, ParseError> {
    let group = group.trim();

    if group.is_empty() {
        return Ok(Vec::new());
    }

    // Check if this is a bracketed group (parallel)
    if group.starts_with('[') && group.ends_with(']') {
        // Parallel operations
        let inner = &group[1..group.len() - 1].trim();

        if inner.is_empty() {
            return Err(ParseError::InvalidSyntax {
                message: "Empty brackets []".to_string(),
            });
        }

        // Split by commas
        let ops: Vec<String> = inner
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if ops.is_empty() {
            return Err(ParseError::InvalidSyntax {
                message: "Empty brackets []".to_string(),
            });
        }

        Ok(ops)
    } else if group.starts_with('[') || group.ends_with(']') {
        // Mismatched brackets (shouldn't happen if validate_brackets passed)
        Err(ParseError::MismatchedBrackets {
            input: group.to_string(),
        })
    } else {
        // No brackets - single operation (since backward compat handled in parse_ops_string)
        // When semicolons are used, each group is a single operation unless bracketed
        Ok(vec![group.to_string()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_sequential() {
        let result = parse_ops_string("audio,transcription").unwrap();
        assert_eq!(result, vec![vec!["audio"], vec!["transcription"]]);
    }

    #[test]
    fn test_simple_parallel() {
        let result = parse_ops_string("[audio,keyframes]").unwrap();
        assert_eq!(result, vec![vec!["audio", "keyframes"]]);
    }

    #[test]
    fn test_mixed_sequential_then_parallel() {
        let result = parse_ops_string("audio;[transcription,diarization]").unwrap();
        assert_eq!(
            result,
            vec![vec!["audio"], vec!["transcription", "diarization"]]
        );
    }

    #[test]
    fn test_parallel_then_sequential() {
        let result = parse_ops_string("[keyframes,audio];obj-detect").unwrap();
        assert_eq!(result, vec![vec!["keyframes", "audio"], vec!["obj-detect"]]);
    }

    #[test]
    fn test_multiple_parallel_groups() {
        let result = parse_ops_string("[keyframes,audio];[obj-detect,transcription]").unwrap();
        assert_eq!(
            result,
            vec![
                vec!["keyframes", "audio"],
                vec!["obj-detect", "transcription"]
            ]
        );
    }

    #[test]
    fn test_single_operation() {
        let result = parse_ops_string("audio").unwrap();
        assert_eq!(result, vec![vec!["audio"]]);
    }

    #[test]
    fn test_single_operation_in_brackets() {
        let result = parse_ops_string("[audio]").unwrap();
        assert_eq!(result, vec![vec!["audio"]]);
    }

    #[test]
    fn test_three_parallel_operations() {
        let result = parse_ops_string("[keyframes,audio,scene-detection]").unwrap();
        assert_eq!(result, vec![vec!["keyframes", "audio", "scene-detection"]]);
    }

    #[test]
    fn test_complex_pipeline() {
        let result = parse_ops_string(
            "[keyframes,audio,scene-detection];[obj-detect,transcription,vision-embeddings]",
        )
        .unwrap();
        assert_eq!(
            result,
            vec![
                vec!["keyframes", "audio", "scene-detection"],
                vec!["obj-detect", "transcription", "vision-embeddings"]
            ]
        );
    }

    #[test]
    fn test_whitespace_handling() {
        let result = parse_ops_string("  [ audio , keyframes ]  ; [ obj-detect ]  ").unwrap();
        assert_eq!(result, vec![vec!["audio", "keyframes"], vec!["obj-detect"]]);
    }

    #[test]
    fn test_empty_input() {
        let result = parse_ops_string("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::EmptyInput));
    }

    #[test]
    fn test_empty_brackets() {
        let result = parse_ops_string("[]");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::InvalidSyntax { .. }
        ));
    }

    #[test]
    fn test_mismatched_brackets_open() {
        let result = parse_ops_string("[audio");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::MismatchedBrackets { .. }
        ));
    }

    #[test]
    fn test_mismatched_brackets_close() {
        let result = parse_ops_string("audio]");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ParseError::MismatchedBrackets { .. }
        ));
    }

    #[test]
    fn test_mismatched_brackets_nested() {
        let result = parse_ops_string("[[audio]]");
        assert!(result.is_err());
        // Will fail during bracket validation or group parsing
    }

    #[test]
    fn test_empty_group() {
        let result = parse_ops_string("audio;;transcription");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::EmptyGroup { .. }));
    }

    #[test]
    fn test_backward_compatible_three_ops() {
        let result = parse_ops_string("audio,transcription,diarization").unwrap();
        // Each comma-separated op becomes its own sequential stage
        assert_eq!(
            result,
            vec![vec!["audio"], vec!["transcription"], vec!["diarization"]]
        );
    }

    #[test]
    fn test_trailing_comma_ignored() {
        let result = parse_ops_string("[audio,keyframes,]").unwrap();
        assert_eq!(result, vec![vec!["audio", "keyframes"]]);
    }

    #[test]
    fn test_hyphens_in_names() {
        let result = parse_ops_string("[object-detection,face-detection]").unwrap();
        assert_eq!(result, vec![vec!["object-detection", "face-detection"]]);
    }
}
