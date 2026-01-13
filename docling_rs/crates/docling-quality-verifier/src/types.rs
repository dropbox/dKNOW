//! Types for quality verification

use serde::{Deserialize, Serialize};

/// Quality category for classification of findings
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QualityCategory {
    /// Missing content (sections, pages, elements)
    #[default]
    Completeness,
    /// Incorrect content (wrong text, numbers, data)
    Accuracy,
    /// Wrong document structure (hierarchy, organization)
    Structure,
    /// Poor formatting (tables, lists, code blocks)
    Formatting,
    /// Missing or incorrect metadata (title, author, dates)
    Metadata,
}

impl std::fmt::Display for QualityCategory {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Completeness => write!(f, "completeness"),
            Self::Accuracy => write!(f, "accuracy"),
            Self::Structure => write!(f, "structure"),
            Self::Formatting => write!(f, "formatting"),
            Self::Metadata => write!(f, "metadata"),
        }
    }
}

impl std::str::FromStr for QualityCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "completeness" => Ok(Self::Completeness),
            "accuracy" => Ok(Self::Accuracy),
            "structure" => Ok(Self::Structure),
            "formatting" => Ok(Self::Formatting),
            "metadata" => Ok(Self::Metadata),
            _ => Err(format!(
                "unknown quality category: '{s}' (expected: completeness, accuracy, structure, formatting, metadata)"
            )),
        }
    }
}

/// Severity level for quality findings
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Critical issue (major content missing, unusable output)
    Critical,
    /// Major issue (significant content problems)
    Major,
    /// Minor issue (small formatting differences)
    Minor,
    /// Informational (acceptable differences)
    #[default]
    Info,
}

impl std::fmt::Display for Severity {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Critical => write!(f, "critical"),
            Self::Major => write!(f, "major"),
            Self::Minor => write!(f, "minor"),
            Self::Info => write!(f, "info"),
        }
    }
}

impl std::str::FromStr for Severity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "critical" => Ok(Self::Critical),
            "major" => Ok(Self::Major),
            "minor" => Ok(Self::Minor),
            "info" | "informational" => Ok(Self::Info),
            _ => Err(format!(
                "unknown severity: '{s}' (expected: critical, major, minor, info)"
            )),
        }
    }
}

/// Individual quality finding
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QualityFinding {
    /// Category of the finding
    pub category: QualityCategory,
    /// Severity level
    pub severity: Severity,
    /// Human-readable description of the issue
    pub description: String,
    /// Optional location in document (e.g., "Page 2", "Table 3")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// Category-specific quality score (0-100)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CategoryScores {
    /// Completeness score (0-100)
    pub completeness: u8,
    /// Accuracy score (0-100)
    pub accuracy: u8,
    /// Structure score (0-100)
    pub structure: u8,
    /// Formatting score (0-100)
    pub formatting: u8,
    /// Metadata score (0-100)
    pub metadata: u8,
}

/// Overall quality assessment report
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityReport {
    /// Overall quality score (0.0-1.0)
    pub score: f64,
    /// Pass/fail based on configured threshold
    pub passed: bool,
    /// Detailed findings by category
    pub findings: Vec<QualityFinding>,
    /// Category-specific scores
    pub category_scores: CategoryScores,
    /// LLM reasoning (if `detailed_diagnostics` enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
}

/// Visual quality report from LLM vision comparison
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VisualQualityReport {
    /// Overall visual quality score (0.0-1.0)
    pub overall_score: f64,
    /// Layout quality score (0.0-1.0)
    pub layout_score: f64,
    /// Formatting quality score (0.0-1.0)
    pub formatting_score: f64,
    /// Tables quality score (0.0-1.0)
    pub tables_score: f64,
    /// Completeness quality score (0.0-1.0)
    pub completeness_score: f64,
    /// Structure quality score (0.0-1.0)
    pub structure_score: f64,
    /// List of specific issues found
    pub issues: Vec<String>,
    /// List of things done well
    pub strengths: Vec<String>,
}

impl QualityReport {
    /// Create a new quality report from LLM response
    #[inline]
    #[must_use = "creates quality report from LLM response"]
    pub fn new(
        score: f64,
        threshold: f64,
        category_scores: CategoryScores,
        findings: Vec<QualityFinding>,
        reasoning: Option<String>,
    ) -> Self {
        Self {
            score,
            passed: score >= threshold,
            findings,
            category_scores,
            reasoning,
        }
    }

    /// Get highest severity finding
    #[inline]
    #[must_use = "returns highest severity finding"]
    pub fn highest_severity(&self) -> Option<&Severity> {
        self.findings.iter().map(|f| &f.severity).min()
    }

    /// Filter findings by severity
    #[inline]
    #[must_use = "filters findings by severity level"]
    pub fn findings_by_severity(&self, severity: Severity) -> Vec<&QualityFinding> {
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .collect()
    }

    /// Filter findings by category
    #[inline]
    #[must_use = "filters findings by category"]
    pub fn findings_by_category(&self, category: QualityCategory) -> Vec<&QualityFinding> {
        self.findings
            .iter()
            .filter(|f| f.category == category)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_report_passed() {
        let report = QualityReport::new(
            0.90,
            0.85,
            CategoryScores {
                completeness: 95,
                accuracy: 90,
                structure: 85,
                formatting: 90,
                metadata: 100,
            },
            vec![],
            None,
        );

        assert!(report.passed);
        assert_eq!(report.score, 0.90);
    }

    #[test]
    fn test_quality_report_failed() {
        let report = QualityReport::new(
            0.75,
            0.85,
            CategoryScores {
                completeness: 70,
                accuracy: 80,
                structure: 75,
                formatting: 70,
                metadata: 80,
            },
            vec![QualityFinding {
                category: QualityCategory::Completeness,
                severity: Severity::Major,
                description: "Missing table content".to_string(),
                location: Some("Page 2".to_string()),
            }],
            None,
        );

        assert!(!report.passed);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.highest_severity(), Some(&Severity::Major));
    }

    #[test]
    fn test_findings_filtering() {
        let findings = vec![
            QualityFinding {
                category: QualityCategory::Completeness,
                severity: Severity::Critical,
                description: "Missing section".to_string(),
                location: None,
            },
            QualityFinding {
                category: QualityCategory::Formatting,
                severity: Severity::Minor,
                description: "Table spacing".to_string(),
                location: Some("Table 1".to_string()),
            },
        ];

        let report = QualityReport::new(
            0.80,
            0.85,
            CategoryScores {
                completeness: 70,
                accuracy: 90,
                structure: 85,
                formatting: 80,
                metadata: 90,
            },
            findings,
            None,
        );

        assert_eq!(report.findings_by_severity(Severity::Critical).len(), 1);
        assert_eq!(report.findings_by_severity(Severity::Minor).len(), 1);
        assert_eq!(
            report
                .findings_by_category(QualityCategory::Completeness)
                .len(),
            1
        );
    }

    #[test]
    fn test_quality_category_display() {
        assert_eq!(format!("{}", QualityCategory::Completeness), "completeness");
        assert_eq!(format!("{}", QualityCategory::Accuracy), "accuracy");
        assert_eq!(format!("{}", QualityCategory::Structure), "structure");
        assert_eq!(format!("{}", QualityCategory::Formatting), "formatting");
        assert_eq!(format!("{}", QualityCategory::Metadata), "metadata");
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", Severity::Critical), "critical");
        assert_eq!(format!("{}", Severity::Major), "major");
        assert_eq!(format!("{}", Severity::Minor), "minor");
        assert_eq!(format!("{}", Severity::Info), "info");
    }

    #[test]
    fn test_quality_category_from_str() {
        use std::str::FromStr;

        assert_eq!(
            QualityCategory::from_str("completeness").unwrap(),
            QualityCategory::Completeness
        );
        assert_eq!(
            QualityCategory::from_str("ACCURACY").unwrap(),
            QualityCategory::Accuracy
        );
        assert_eq!(
            QualityCategory::from_str("Structure").unwrap(),
            QualityCategory::Structure
        );
        assert_eq!(
            QualityCategory::from_str("formatting").unwrap(),
            QualityCategory::Formatting
        );
        assert_eq!(
            QualityCategory::from_str("metadata").unwrap(),
            QualityCategory::Metadata
        );
        assert!(QualityCategory::from_str("invalid").is_err());
    }

    #[test]
    fn test_quality_category_roundtrip() {
        use std::str::FromStr;

        for cat in [
            QualityCategory::Completeness,
            QualityCategory::Accuracy,
            QualityCategory::Structure,
            QualityCategory::Formatting,
            QualityCategory::Metadata,
        ] {
            let s = cat.to_string();
            let parsed = QualityCategory::from_str(&s).unwrap();
            assert_eq!(cat, parsed, "roundtrip failed for {s}");
        }
    }

    #[test]
    fn test_severity_from_str() {
        use std::str::FromStr;

        assert_eq!(Severity::from_str("critical").unwrap(), Severity::Critical);
        assert_eq!(Severity::from_str("MAJOR").unwrap(), Severity::Major);
        assert_eq!(Severity::from_str("Minor").unwrap(), Severity::Minor);
        assert_eq!(Severity::from_str("info").unwrap(), Severity::Info);
        assert_eq!(Severity::from_str("informational").unwrap(), Severity::Info);
        assert!(Severity::from_str("invalid").is_err());
    }

    #[test]
    fn test_severity_roundtrip() {
        use std::str::FromStr;

        for sev in [
            Severity::Critical,
            Severity::Major,
            Severity::Minor,
            Severity::Info,
        ] {
            let s = sev.to_string();
            let parsed = Severity::from_str(&s).unwrap();
            assert_eq!(sev, parsed, "roundtrip failed for {s}");
        }
    }
}
