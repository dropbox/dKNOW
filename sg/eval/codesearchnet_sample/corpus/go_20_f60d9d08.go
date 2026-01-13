func NewClient(dialTimeout time.Duration, allowV2 bool) Client {
	return &client{
		dialTimeout: dialTimeout,
		connections: make(map[string]*connection),
		allowV2:     allowV2,
	}
}