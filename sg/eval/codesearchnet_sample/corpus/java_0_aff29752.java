@Override
    public Map<String, LogRepositoryBrowser> getSubProcesses() {
        HashMap<String, LogRepositoryBrowser> result = new HashMap<String, LogRepositoryBrowser>();
        for (File file : listFiles(subprocFilter)) {
            String id = getSubProcessId(file);
            String[] newIds = Arrays.copyOf(ids, ids.length + 1);
            newIds[ids.length] = id;
            result.put(id, new LogRepositoryBrowserImpl(file, newIds));
        }
        return result;
    }