func (t *TimeRange) HasFrom() bool {
	if t != nil && t.From != nil {
		return true
	}

	return false
}