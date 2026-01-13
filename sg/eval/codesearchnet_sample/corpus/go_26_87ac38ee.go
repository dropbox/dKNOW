func Extract(src string, dest string) error {
	file, err := os.Open(src)

	if err != nil {
		return err
	}

	defer file.Close()

	return tar.Extract(bzip2.NewReader(file), dest)
}