func (b *BigIP) ServerSSLProfiles() (*ServerSSLProfiles, error) {
	var serverSSLProfiles ServerSSLProfiles
	err, _ := b.getForEntity(&serverSSLProfiles, uriLtm, uriProfile, uriServerSSL)
	if err != nil {
		return nil, err
	}

	return &serverSSLProfiles, nil
}