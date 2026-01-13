protected function createSiteInfo( $data )
    {
        if ( $this->isMultisite() )
        {
            return new SiteInfo( $data );
        }
        else
        {
            $db         = $this->getDbAdapter();
            $driver     = $db->getDriver();
            $schema     = $db->getCurrentSchema();
            $config     = $driver->getConnection()->getConnectionParameters();
            $domain     = empty( $config['defaultDomain'] ) ? php_uname( 'n' ) : $config['defaultDomain'];
            $subdomain  = empty( $data['subdomain'] ) ? '' : $data['subdomain'];
            $fulldomain = ( $subdomain ? $subdomain . '.' : '' ) . $domain;

            return new SiteInfo( array(
                'schema'        => $schema,
                'domain'        => $domain,
                'subdomain'     => $subdomain,
                'subdomainId'   => empty( $data['id'] ) ? null : $data['id'],
                'fulldomain'    => $fulldomain,
            ) );
        }
    }