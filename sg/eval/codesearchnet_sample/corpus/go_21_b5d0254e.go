func (t *Template) GetAWSEventsRuleWithName(name string) (*resources.AWSEventsRule, error) {
	if untyped, ok := t.Resources[name]; ok {
		switch resource := untyped.(type) {
		case *resources.AWSEventsRule:
			return resource, nil
		}
	}
	return nil, fmt.Errorf("resource %q of type AWSEventsRule not found", name)
}