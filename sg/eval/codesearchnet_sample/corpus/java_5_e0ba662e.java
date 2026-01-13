@Override
    protected boolean checkResourcePermissions(
        CmsPermissionSet required,
        boolean neededForFolder,
        CmsMessageContainer errorMessage) {

        boolean hasPermissions = false;
        try {
            CmsResource res;
            if (neededForFolder) {
                // check permissions for the folder the resource is in
                res = getCms().readResource(CmsResource.getParentFolder(getParamResource()), CmsResourceFilter.ALL);
            } else {
                res = getCms().readResource(getParamResource(), CmsResourceFilter.ALL);
            }
            hasPermissions = getCms().hasPermissions(res, required, false, CmsResourceFilter.ALL)
                && (OpenCms.getRoleManager().hasRoleForResource(
                    getCms(),
                    CmsRole.ELEMENT_AUTHOR,
                    getCms().getSitePath(res))
                    || OpenCms.getRoleManager().hasRoleForResource(
                        getCms(),
                        CmsRole.PROJECT_MANAGER,
                        getCms().getSitePath(res))
                    || OpenCms.getRoleManager().hasRoleForResource(
                        getCms(),
                        CmsRole.ACCOUNT_MANAGER,
                        getCms().getSitePath(res)));
        } catch (CmsException e) {
            // should usually never happen
            if (LOG.isInfoEnabled()) {
                LOG.info(e);
            }
        }

        if (!hasPermissions) {
            // store the error message in the users session
            getSettings().setErrorMessage(errorMessage);
        }

        return hasPermissions;
    }