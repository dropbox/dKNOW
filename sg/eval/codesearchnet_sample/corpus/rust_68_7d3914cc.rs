pub fn contains_key(&self, name: &str) -> bool {
        if let Some(key) = name.strip_prefix("output.") {
            self.output.read(key)
        } else if let Some(key) = name.strip_prefix("preprocessor.") {
            self.preprocessor.read(key)
        } else {
            panic!("invalid key `{name}`");
        }
        .is_some()
    }