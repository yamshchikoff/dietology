use std::path::PathBuf;

pub struct DataLoader {
    base_path: PathBuf,
}

impl DataLoader {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub fn for_development() -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        Self::new(manifest_dir.join("..").join("data"))
    }

    pub fn read_bytes(&self, relative_path: &str) -> Result<Vec<u8>, std::io::Error> {
        let full_path = self.base_path.join(relative_path);
        std::fs::read(&full_path)
    }

    pub fn read_json<T: serde::de::DeserializeOwned>(
        &self,
        relative_path: &str,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let bytes = self.read_bytes(relative_path)?;
        let value = serde_json::from_slice(&bytes)?;
        Ok(value)
    }
}
