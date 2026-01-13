
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'docling' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'docling'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'docling' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('convert', 'convert', [CompletionResultType]::ParameterValue, 'Convert a document to markdown, JSON, or YAML')
            [CompletionResult]::new('batch', 'batch', [CompletionResultType]::ParameterValue, 'Convert multiple documents in batch (streaming mode)')
            [CompletionResult]::new('benchmark', 'benchmark', [CompletionResultType]::ParameterValue, 'Benchmark document conversion performance')
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate shell completion scripts')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'docling;convert' {
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Output file path (default: stdout)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output file path (default: stdout)')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format')
            [CompletionResult]::new('-b', '-b', [CompletionResultType]::ParameterName, 'Backend to use for conversion')
            [CompletionResult]::new('--backend', '--backend', [CompletionResultType]::ParameterName, 'Backend to use for conversion')
            [CompletionResult]::new('--compact', '--compact', [CompletionResultType]::ParameterName, 'Compact JSON output (no pretty-printing, only affects JSON format)')
            [CompletionResult]::new('--ocr', '--ocr', [CompletionResultType]::ParameterName, 'Enable OCR for scanned documents (PDF only, requires Python backend)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'docling;batch' {
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Output directory for converted files (required for batch mode)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output directory for converted files (required for batch mode)')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format')
            [CompletionResult]::new('--max-file-size', '--max-file-size', [CompletionResultType]::ParameterName, 'Maximum file size in bytes (skip files larger than this)')
            [CompletionResult]::new('--continue-on-error', '--continue-on-error', [CompletionResultType]::ParameterName, 'Continue processing on errors (default: stop on first error)')
            [CompletionResult]::new('--ocr', '--ocr', [CompletionResultType]::ParameterName, 'Enable OCR for scanned documents (PDF only, requires Python backend)')
            [CompletionResult]::new('--compact', '--compact', [CompletionResultType]::ParameterName, 'Compact JSON output (no pretty-printing, only affects JSON format)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'docling;benchmark' {
            [CompletionResult]::new('-n', '-n', [CompletionResultType]::ParameterName, 'Number of iterations to run for each file')
            [CompletionResult]::new('--iterations', '--iterations', [CompletionResultType]::ParameterName, 'Number of iterations to run for each file')
            [CompletionResult]::new('-w', '-w', [CompletionResultType]::ParameterName, 'Warmup iterations (results discarded)')
            [CompletionResult]::new('--warmup', '--warmup', [CompletionResultType]::ParameterName, 'Warmup iterations (results discarded)')
            [CompletionResult]::new('-f', '-f', [CompletionResultType]::ParameterName, 'Output format')
            [CompletionResult]::new('--format', '--format', [CompletionResultType]::ParameterName, 'Output format')
            [CompletionResult]::new('-o', '-o', [CompletionResultType]::ParameterName, 'Output file path (default: stdout)')
            [CompletionResult]::new('--output', '--output', [CompletionResultType]::ParameterName, 'Output file path (default: stdout)')
            [CompletionResult]::new('--ocr', '--ocr', [CompletionResultType]::ParameterName, 'Enable OCR for scanned documents (PDF only)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'docling;completion' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            break
        }
        'docling;help' {
            [CompletionResult]::new('convert', 'convert', [CompletionResultType]::ParameterValue, 'Convert a document to markdown, JSON, or YAML')
            [CompletionResult]::new('batch', 'batch', [CompletionResultType]::ParameterValue, 'Convert multiple documents in batch (streaming mode)')
            [CompletionResult]::new('benchmark', 'benchmark', [CompletionResultType]::ParameterValue, 'Benchmark document conversion performance')
            [CompletionResult]::new('completion', 'completion', [CompletionResultType]::ParameterValue, 'Generate shell completion scripts')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'docling;help;convert' {
            break
        }
        'docling;help;batch' {
            break
        }
        'docling;help;benchmark' {
            break
        }
        'docling;help;completion' {
            break
        }
        'docling;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
