protected function getCleanContent()
    {
        $content = var_export($this->content, true);

        # Replace the array openers with square brackets
        $content = preg_replace('#^(\s*)array\s*\(#i', '\\1[', $content);
        $content = preg_replace('#=>(\s*)array\s*\(#is', '=> [', $content);

        # Replace the array closers with square brackets
        $content = preg_replace('#^(\s*)\),#im', '\\1],', $content);
        $content = substr($content, 0, -1) . ']';

        // Remove integer indexes for unassociative array lists
        $content = preg_replace('#(\s*)\d+\s*=>\s*(.*)#i', '\\1\\2', $content);

        $content = $this->replaceDoubleSpaceIndentWithCustomSpaceIndent($content);

        return $content;
    }