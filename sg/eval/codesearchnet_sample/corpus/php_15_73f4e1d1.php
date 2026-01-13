private function formatResponse()
    {
        $return = array();

        // Method
        if (isset($this->state['method'])) {
            $return['method'] = $this->state['method'];
        }

        // Request
        if (!empty($this->state['request'])) {
            $return['request'] = $this->state['request'];
        }

        // Response
        if (!empty($this->state['response'])) {
            $return['response'] = $this->state['response'];
        }

        return $return;
    }