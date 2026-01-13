# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_docling_global_optspecs
	string join \n h/help V/version
end

function __fish_docling_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_docling_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_docling_using_subcommand
	set -l cmd (__fish_docling_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c docling -n "__fish_docling_needs_command" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c docling -n "__fish_docling_needs_command" -s V -l version -d 'Print version'
complete -c docling -n "__fish_docling_needs_command" -f -a "convert" -d 'Convert a document to markdown, JSON, or YAML'
complete -c docling -n "__fish_docling_needs_command" -f -a "batch" -d 'Convert multiple documents in batch (streaming mode)'
complete -c docling -n "__fish_docling_needs_command" -f -a "benchmark" -d 'Benchmark document conversion performance'
complete -c docling -n "__fish_docling_needs_command" -f -a "completion" -d 'Generate shell completion scripts'
complete -c docling -n "__fish_docling_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c docling -n "__fish_docling_using_subcommand convert" -s o -l output -d 'Output file path (default: stdout)' -r -F
complete -c docling -n "__fish_docling_using_subcommand convert" -s f -l format -d 'Output format' -r -f -a "markdown\t'Markdown output (default)'
json\t'JSON output'
yaml\t'YAML output'"
complete -c docling -n "__fish_docling_using_subcommand convert" -s b -l backend -d 'Backend to use for conversion' -r -f -a "rust\t'Use Rust backend (supported formats only)'
python\t'Use Python backend (all formats)'
auto\t'Auto-select backend (default: Rust if supported, else Python)'"
complete -c docling -n "__fish_docling_using_subcommand convert" -l compact -d 'Compact JSON output (no pretty-printing, only affects JSON format)'
complete -c docling -n "__fish_docling_using_subcommand convert" -l ocr -d 'Enable OCR for scanned documents (PDF only, requires Python backend)'
complete -c docling -n "__fish_docling_using_subcommand convert" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c docling -n "__fish_docling_using_subcommand batch" -s o -l output -d 'Output directory for converted files (required for batch mode)' -r -F
complete -c docling -n "__fish_docling_using_subcommand batch" -s f -l format -d 'Output format' -r -f -a "markdown\t'Markdown output (default)'
json\t'JSON output'
yaml\t'YAML output'"
complete -c docling -n "__fish_docling_using_subcommand batch" -l max-file-size -d 'Maximum file size in bytes (skip files larger than this)' -r
complete -c docling -n "__fish_docling_using_subcommand batch" -l continue-on-error -d 'Continue processing on errors (default: stop on first error)'
complete -c docling -n "__fish_docling_using_subcommand batch" -l ocr -d 'Enable OCR for scanned documents (PDF only, requires Python backend)'
complete -c docling -n "__fish_docling_using_subcommand batch" -l compact -d 'Compact JSON output (no pretty-printing, only affects JSON format)'
complete -c docling -n "__fish_docling_using_subcommand batch" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c docling -n "__fish_docling_using_subcommand benchmark" -s n -l iterations -d 'Number of iterations to run for each file' -r
complete -c docling -n "__fish_docling_using_subcommand benchmark" -s w -l warmup -d 'Warmup iterations (results discarded)' -r
complete -c docling -n "__fish_docling_using_subcommand benchmark" -s f -l format -d 'Output format' -r -f -a "text\t'Human-readable text'
json\t'JSON format'
csv\t'CSV format'
markdown\t'Markdown table'"
complete -c docling -n "__fish_docling_using_subcommand benchmark" -s o -l output -d 'Output file path (default: stdout)' -r -F
complete -c docling -n "__fish_docling_using_subcommand benchmark" -l ocr -d 'Enable OCR for scanned documents (PDF only)'
complete -c docling -n "__fish_docling_using_subcommand benchmark" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c docling -n "__fish_docling_using_subcommand completion" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c docling -n "__fish_docling_using_subcommand help; and not __fish_seen_subcommand_from convert batch benchmark completion help" -f -a "convert" -d 'Convert a document to markdown, JSON, or YAML'
complete -c docling -n "__fish_docling_using_subcommand help; and not __fish_seen_subcommand_from convert batch benchmark completion help" -f -a "batch" -d 'Convert multiple documents in batch (streaming mode)'
complete -c docling -n "__fish_docling_using_subcommand help; and not __fish_seen_subcommand_from convert batch benchmark completion help" -f -a "benchmark" -d 'Benchmark document conversion performance'
complete -c docling -n "__fish_docling_using_subcommand help; and not __fish_seen_subcommand_from convert batch benchmark completion help" -f -a "completion" -d 'Generate shell completion scripts'
complete -c docling -n "__fish_docling_using_subcommand help; and not __fish_seen_subcommand_from convert batch benchmark completion help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
