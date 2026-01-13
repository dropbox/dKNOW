public function offsetGet($offset)
    {
        if ($this->has($offset))
        {
            return $this->get($offset);
        }

        throw new OffsetNotExistsException('Offset: ' . $offset . ' not exists.');
    }