public RewriteResult match(String path) {

        for (CmsRewriteAlias alias : m_aliases) {
            try {
                Pattern pattern = Pattern.compile(alias.getPatternString());
                Matcher matcher = pattern.matcher(path);
                if (matcher.matches()) {
                    String newPath = matcher.replaceFirst(alias.getReplacementString());
                    return new RewriteResult(newPath, alias);
                }
            } catch (PatternSyntaxException e) {
                LOG.warn(e.getLocalizedMessage(), e);
            } catch (IndexOutOfBoundsException e) {
                LOG.warn(e.getLocalizedMessage(), e);
            }
        }
        return null;
    }