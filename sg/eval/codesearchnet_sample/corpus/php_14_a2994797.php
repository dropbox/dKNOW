protected function findSourceCodeReferences($lines, $test)
    {
        preg_match_all(
            config('tddd.root.regex_file_matcher'),
            strip_tags($this->brToCR($lines)),
            $matches,
            PREG_SET_ORDER
        );

        return array_filter($matches);
    }