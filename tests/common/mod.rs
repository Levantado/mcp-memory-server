use std::path::PathBuf;
use tempfile::TempDir;
use std::fs;

pub struct TestEnv {
    #[allow(dead_code)]
    pub temp_dir: TempDir,
    pub storage_path: PathBuf,
    pub docs_path: PathBuf,
}

impl TestEnv {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("storage");
        let docs_path = temp_dir.path().join("docs");
        
        fs::create_dir_all(&storage_path).unwrap();
        fs::create_dir_all(&docs_path).unwrap();
        
        // Create dummy guidelines
        fs::write(docs_path.join("effective_work.md"), "test policy").unwrap();
        fs::write(docs_path.join("AGENT_GUIDELINES.md"), "test guidelines").unwrap();
        
        Self {
            temp_dir,
            storage_path,
            docs_path,
        }
    }
}
