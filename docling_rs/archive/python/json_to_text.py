#!/usr/bin/env python3
"""
Convert Claude's stream-json output to human-readable text.
Reads JSON lines from stdin, outputs formatted text to stdout.
"""

import sys
import json
from datetime import datetime

# ANSI color codes
BLUE = '\033[94m'
GREEN = '\033[92m'
YELLOW = '\033[93m'
RED = '\033[91m'
CYAN = '\033[96m'
MAGENTA = '\033[95m'
BOLD = '\033[1m'
DIM = '\033[2m'
RESET = '\033[0m'

def timestamp():
    """Get current timestamp for display"""
    return datetime.now().strftime('%H:%M:%S')

def clean_output(text):
    """Remove system noise from output"""
    lines = text.strip().split('\n')
    filtered = []
    skip_mode = False

    for line in lines:
        # Skip Co-Authored-By
        if 'Co-Authored-By:' in line or '🤖 Generated with' in line:
            continue
        # Skip system reminders
        if '<system-reminder>' in line:
            skip_mode = True
            continue
        if '</system-reminder>' in line:
            skip_mode = False
            continue
        if skip_mode:
            continue
        # Skip malware check reminders
        if 'you should consider whether it would be considered malware' in line.lower():
            continue
        filtered.append(line)

    return '\n'.join(filtered)

def format_tool_output(content, tool_name, is_error=False):
    """Format tool output intelligently"""
    text = clean_output(content)
    if not text.strip():
        return None

    lines = text.split('\n')

    # For errors, show more context
    if is_error:
        preview_lines = lines[:15]
        if len(lines) > 15:
            preview_lines.append(f"{DIM}... ({len(lines) - 15} more lines){RESET}")
        return preview_lines

    # For successful tools, show smart preview
    if tool_name == 'Bash':
        # Show first few lines and last line
        if len(lines) <= 3:
            return lines
        else:
            result = lines[:2]
            if len(lines) > 3:
                result.append(f"{DIM}... ({len(lines) - 3} more lines){RESET}")
                result.append(lines[-1])
            return result

    elif tool_name == 'Read':
        # Just show how many lines read
        return [f"{DIM}({len(lines)} lines read){RESET}"]

    elif tool_name in ['Write', 'Edit']:
        # Just confirm success
        return None

    elif tool_name in ['Grep', 'Glob']:
        # Show first few matches
        if len(lines) <= 5:
            return lines
        else:
            result = lines[:5]
            result.append(f"{DIM}... ({len(lines) - 5} more matches){RESET}")
            return result

    else:
        # Generic: show first line
        if len(lines) <= 2:
            return lines
        return [lines[0], f"{DIM}... ({len(lines) - 1} more lines){RESET}"]

class MessageFormatter:
    def __init__(self):
        self.pending_tools = []
        self.last_was_text = False

    def flush_tools(self):
        """Output any pending tools"""
        if self.pending_tools:
            for tool in self.pending_tools:
                print(tool)
            self.pending_tools = []

    def format_text_message(self, text):
        """Format Claude's main messages"""
        if not text.strip():
            return

        # Flush any pending tools first
        self.flush_tools()

        # Add spacing if last output was also text
        if self.last_was_text:
            print()

        # Clean up the text
        text = text.strip()

        # Split into paragraphs
        paragraphs = text.split('\n\n')

        for i, para in enumerate(paragraphs):
            para = para.strip()
            if not para:
                continue

            # First paragraph gets timestamp and icon
            if i == 0:
                print(f"\n{DIM}[{timestamp()}]{RESET} {BOLD}{BLUE}💬{RESET} {para}")
            else:
                # Subsequent paragraphs are indented slightly
                print(f"   {para}")

        self.last_was_text = True

    def format_tool_use(self, tool_name, input_data, tool_result=None):
        """Format a tool call with its result"""
        # Build tool description
        if tool_name == 'Read':
            path = input_data.get('file_path', '')
            desc = f"read: {path}"

        elif tool_name == 'Write':
            path = input_data.get('file_path', '')
            size = len(input_data.get('content', ''))
            desc = f"write: {path} ({size} chars)"

        elif tool_name == 'Edit':
            path = input_data.get('file_path', '')
            desc = f"edit: {path}"

        elif tool_name == 'Bash':
            cmd = input_data.get('command', '')
            # Truncate long commands
            if len(cmd) > 80:
                cmd = cmd[:77] + '...'
            desc = f"bash: {cmd}"

        elif tool_name == 'Grep':
            pattern = input_data.get('pattern', '')
            path = input_data.get('path', '.')
            desc = f"grep: '{pattern}' in {path}"

        elif tool_name == 'Glob':
            pattern = input_data.get('pattern', '')
            desc = f"glob: {pattern}"

        elif tool_name == 'TodoWrite':
            todos = input_data.get('todos', [])
            desc = f"todo: update ({len(todos)} items)"

        else:
            desc = f"{tool_name.lower()}"

        # Check if result is an error
        is_error = False
        if tool_result:
            result_lower = tool_result.lower()
            is_error = 'error' in result_lower or 'failed' in result_lower or 'is_error' in result_lower

        # Format output
        if is_error:
            # Errors are prominent
            print(f"\n  {RED}✗{RESET} {desc}")
            if tool_result:
                output_lines = format_tool_output(tool_result, tool_name, is_error=True)
                if output_lines:
                    for line in output_lines:
                        print(f"    {RED}{line}{RESET}")
        else:
            # Success - show tool and smart preview
            print(f"  {DIM}•{RESET} {desc}")
            if tool_result:
                output_lines = format_tool_output(tool_result, tool_name, is_error=False)
                if output_lines:
                    for line in output_lines:
                        print(f"    {DIM}→{RESET} {line}")

        self.last_was_text = False

    def format_thinking(self, thinking_text):
        """Format thinking blocks (minimal)"""
        if thinking_text and len(thinking_text) > 100:
            # Only show if substantial thinking
            char_count = len(thinking_text)
            print(f"  {DIM}💭 thinking... ({char_count} chars){RESET}")
            self.last_was_text = False

formatter = MessageFormatter()

# Track tool uses and their results
pending_tool_uses = {}

def process_message(msg):
    """Process a single message from the stream"""
    msg_type = msg.get('type')

    # Handle nested message structure
    if 'message' in msg:
        inner_msg = msg['message']
        role = inner_msg.get('role')
        content = inner_msg.get('content', [])
    else:
        role = msg.get('role')
        content = msg.get('content', [])

    if msg_type == 'init':
        print(f"\n{BOLD}{MAGENTA}{'═' * 80}{RESET}")
        print(f"{BOLD}{MAGENTA}  🚀  Claude Session Started  {RESET}")
        print(f"{BOLD}{MAGENTA}{'═' * 80}{RESET}")
        return

    if msg_type == 'result':
        # Final result with stats
        formatter.flush_tools()
        stats = msg.get('stats', {})
        print(f"\n{DIM}{'─' * 80}{RESET}")
        print(f"{BOLD}{GREEN}  ✓  Session Complete{RESET}")
        if stats:
            input_tokens = stats.get('input_tokens', 0)
            output_tokens = stats.get('output_tokens', 0)
            cache_read = stats.get('cache_read_input_tokens', 0)
            print(f"{DIM}  Input: {input_tokens:,} tokens", end='')
            if cache_read:
                print(f" (cached: {cache_read:,})", end='')
            print(f" | Output: {output_tokens:,} tokens{RESET}")
        print(f"{DIM}{'─' * 80}{RESET}\n")
        return

    # Process content blocks
    if isinstance(content, str):
        content = [{'type': 'text', 'text': content}]

    for block in content:
        block_type = block.get('type')

        if block_type == 'text':
            text = block.get('text', '')
            if role == 'assistant' and text.strip():
                formatter.format_text_message(text)

        elif block_type == 'thinking':
            thinking = block.get('thinking', '')
            formatter.format_thinking(thinking)

        elif block_type == 'tool_use':
            # Store tool use for later pairing with result
            tool_id = block.get('id', '')
            tool_name = block.get('name', '')
            input_data = block.get('input', {})
            pending_tool_uses[tool_id] = {
                'name': tool_name,
                'input': input_data
            }

        elif block_type == 'tool_result':
            # Match with tool use
            tool_id = block.get('tool_use_id', '')
            content_data = block.get('content', '')

            # Extract text from content
            if isinstance(content_data, list):
                text_parts = []
                for item in content_data:
                    if isinstance(item, dict) and item.get('type') == 'text':
                        text_parts.append(item.get('text', ''))
                result_text = '\n'.join(text_parts)
            else:
                result_text = str(content_data)

            # Get the tool use info
            if tool_id in pending_tool_uses:
                tool_info = pending_tool_uses[tool_id]
                formatter.format_tool_use(
                    tool_info['name'],
                    tool_info['input'],
                    result_text
                )
                del pending_tool_uses[tool_id]

def main():
    """Main entry point"""
    try:
        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue

            try:
                msg = json.loads(line)
                process_message(msg)
                sys.stdout.flush()
            except json.JSONDecodeError:
                # Not valid JSON, might be regular output
                print(line)
                continue

    except KeyboardInterrupt:
        print(f"\n{YELLOW}⚠️  Interrupted{RESET}")
        sys.exit(0)
    except BrokenPipeError:
        # Handle pipe closing gracefully
        sys.exit(0)

if __name__ == '__main__':
    main()
