protected function validateParams(array $params)
    {
        foreach ($params as $param) {
            if (!$this->hasRequestParameter($param)) {
                throw new common_exception_MissingParameter($param .' is missing from the request.', $this->getRequestURI());
            }

            if (empty($this->getRequestParameter($param))) {
                throw new common_exception_ValidationFailed($param, $param .' cannot be empty');
            }
        }
    }