use std::path::PathBuf;
use tempfile::TempDir;
use std::fs;
use std::net::TcpListener;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::{Command, Child};

pub struct TestEnv {
    #[allow(dead_code)]
    pub temp_dir: TempDir,
    pub storage_path: PathBuf,
    pub docs_path: PathBuf,
}

pub struct TestServer {
    child: Child,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

impl TestEnv {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join("storage");
        let docs_path = temp_dir.path().join("docs");
        
        fs::create_dir_all(&storage_path).unwrap();
        fs::create_dir_all(storage_path.join("dummy_project")).unwrap();
        fs::create_dir_all(&docs_path).unwrap();
        
        fs::write(docs_path.join("effective_work.md"), "test policy").unwrap();
        fs::write(docs_path.join("AGENT_GUIDELINES.md"), "test guidelines").unwrap();
        
        Self {
            temp_dir,
            storage_path,
            docs_path,
        }
    }

    pub fn get_free_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    }

    pub async fn start_server(&self, port: u16, api_key: Option<&str>) -> TestServer {
        self.start_server_with_args(port, api_key, vec![]).await
    }

    pub async fn start_server_with_args(&self, port: u16, api_key: Option<&str>, extra_args: Vec<&str>) -> TestServer {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_mcp-memory-server-rust"));
        
        cmd.arg("--mode").arg("hybrid")
           .arg("--port").arg(port.to_string())
           .arg("--root").arg(self.storage_path.to_str().unwrap())
           .arg("--docs-dir").arg(self.docs_path.to_str().unwrap());

        cmd.env_remove("MCP_API_KEY");

        if let Some(key) = api_key {
            cmd.arg("--api-key").arg(key);
        }

        for arg in extra_args {
            cmd.arg(arg);
        }

        cmd.stdout(Stdio::null())
           .stderr(Stdio::null());

        let mut child = cmd.spawn().expect("Failed to start server");
        
        tokio::time::sleep(Duration::from_millis(600)).await;
        
        if let Ok(Some(status)) = child.try_wait() {
            panic!("Server died immediately with status: {}. Check if all required args are provided.", status);
        }

        TestServer { child }
    }
}
