func (s *IdentityService) GetGithubConnectors(withSecrets bool) ([]services.GithubConnector, error) {
	startKey := backend.Key(webPrefix, connectorsPrefix, githubPrefix, connectorsPrefix)
	result, err := s.GetRange(context.TODO(), startKey, backend.RangeEnd(startKey), backend.NoLimit)
	if err != nil {
		return nil, trace.Wrap(err)
	}
	connectors := make([]services.GithubConnector, len(result.Items))
	for i, item := range result.Items {
		connector, err := services.GetGithubConnectorMarshaler().Unmarshal(item.Value)
		if err != nil {
			return nil, trace.Wrap(err)
		}
		if !withSecrets {
			connector.SetClientSecret("")
		}
		connectors[i] = connector
	}
	return connectors, nil
}