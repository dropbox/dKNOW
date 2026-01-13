function(attrs) {
    if (attrs.authentication !== 'KERBEROS') {
      if (attrs.kerberos_service_name) {
        throw new TypeError(format(
          'The kerberos_service_name field does not apply when '
          + 'using %s for authentication.', attrs.authentication));
      }
      if (attrs.kerberos_principal) {
        throw new TypeError(format(
          'The kerberos_principal field does not apply when '
          + 'using %s for authentication.', attrs.authentication));
      }
      if (attrs.kerberos_password) {
        throw new TypeError(format(
          'The kerberos_password field does not apply when '
          + 'using %s for authentication.', attrs.authentication));
      }
    }

    if (attrs.authentication === 'KERBEROS') {
      if (!attrs.kerberos_principal) {
        throw new TypeError(format(
          'The kerberos_principal field is required when '
          + 'using KERBEROS for authentication.'));
      }
    }
  }