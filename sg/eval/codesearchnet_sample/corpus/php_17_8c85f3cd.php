public function ends_with ($needle)
	{
		ensure('Argument', $needle, 'is_a_string', __CLASS__, __METHOD__);

		if ($needle instanceof self) {
			$needle = $needle->to_s();
		}

		$len = strlen($needle);

		return 0 === strcmp($this->slice(-$len, $len)->to_s(), $needle);
	}