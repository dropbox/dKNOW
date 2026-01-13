def set_ftp_service(username = nil, password = nil)
      self.user_name = username
      self.password  = password
      self.service   = Credential::Service::FTP
    end