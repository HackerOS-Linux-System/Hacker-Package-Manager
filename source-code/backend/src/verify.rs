use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub fn verify(path: &str, checksum: &str) -> Result<()> {
    let computed = compute_dir_hash(Path::new(path))?;
    if computed != checksum {
        return Err(anyhow!("Checksum mismatch: computed {}, expected {}", computed, checksum));
    }
    Ok(())
}

pub fn compute_dir_hash(dir: &Path) -> Result<String> {
    let entries: Vec<_> = WalkDir::new(dir)
    .sort_by(|a, b| a.file_name().cmp(b.file_name()))
    .into_iter()
    .filter_map(|e| e.ok())
    .filter(|e| e.file_type().is_file())
    .map(|e| e.path().to_owned())
    .collect();
    let mut hasher = Sha256::new();
    for file_path in entries {
        let data = fs::read(&file_path)?;
        hasher.update(&data);
    }
    let hash = hasher.finalize();
    Ok(hex::encode(hash))
}
