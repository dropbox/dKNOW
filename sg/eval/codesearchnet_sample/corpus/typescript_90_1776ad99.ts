/**
 * Tool result content block (in user messages)
 * Source: lines 179508-179509
 */
export interface ToolResultBlock {
  type: 'tool_result';
  tool_use_id: string;
  content: string | ContentBlock[];
  is_error?: boolean;
}