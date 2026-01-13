fn path_equals(dent: &DirEntry, handle: &Handle) -> Result<bool, Error> {
    #[cfg(unix)]
    fn never_equal(dent: &DirEntry, handle: &Handle) -> bool {
        dent.ino() != Some(handle.ino())
    }

    #[cfg(not(unix))]
    fn never_equal(_: &DirEntry, _: &Handle) -> bool {
        false
    }

    // If we know for sure that these two things aren't equal, then avoid
    // the costly extra stat call to determine equality.
    if dent.is_stdin() || never_equal(dent, handle) {
        return Ok(false);
    }
    Handle::from_path(dent.path())
        .map(|h| &h == handle)
        .map_err(|err| Error::Io(err).with_path(dent.path()))
}