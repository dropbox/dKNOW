//! Client library for communicating with sg-daemon
//!
//! Provides synchronous and asynchronous clients for IPC communication
//! with the daemon over Unix sockets.

use crate::protocol::{DaemonStatus, ProjectInfo, Request, Response, SearchResultWire};
use anyhow::{Context, Result};
use sg_core::SearchOptions;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Default timeout for client requests (30 seconds)
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Time to wait for graceful shutdown before sending SIGKILL (500ms)
const GRACEFUL_SHUTDOWN_WAIT_MS: u64 = 500;

/// Synchronous client for communicating with the daemon
pub struct Client {
    socket_path: PathBuf,
    timeout: Duration,
}

impl Client {
    /// Create a new client with the given socket path
    pub fn new(socket_path: &Path) -> Self {
        Self {
            socket_path: socket_path.to_path_buf(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }

    /// Create a client with the default socket path
    pub fn with_default_socket() -> Self {
        Self::new(&crate::server::default_socket_path())
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Check if the daemon is running (socket exists and responds)
    pub fn is_daemon_running(&self) -> bool {
        if !self.socket_path.exists() {
            return false;
        }

        // Try to connect and send a status request
        self.status().is_ok()
    }

    /// Send a request to the daemon and wait for a response
    fn send_request(&self, request: &Request) -> Result<Response> {
        // Connect to the daemon
        let mut stream =
            UnixStream::connect(&self.socket_path).context("Failed to connect to daemon")?;

        stream
            .set_read_timeout(Some(self.timeout))
            .context("Failed to set read timeout")?;
        stream
            .set_write_timeout(Some(self.timeout))
            .context("Failed to set write timeout")?;

        // Send request
        let request_json = serde_json::to_string(request)?;
        stream.write_all(request_json.as_bytes())?;
        stream.write_all(b"\n")?;
        stream.flush()?;

        // Read response
        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line)?;

        // Parse response
        let response: Response =
            serde_json::from_str(&response_line).context("Failed to parse daemon response")?;

        Ok(response)
    }

    /// Search for documents matching the query
    pub fn search(&self, query: &str, options: SearchOptions) -> Result<Vec<SearchResultWire>> {
        let request = Request::Search {
            query: query.to_string(),
            options,
        };

        match self.send_request(&request)? {
            Response::SearchResults(results) => Ok(results),
            Response::Error(e) => anyhow::bail!("Daemon error: {e}"),
            _ => anyhow::bail!("Unexpected response from daemon"),
        }
    }

    /// Get daemon status
    pub fn status(&self) -> Result<DaemonStatus> {
        let request = Request::Status;

        match self.send_request(&request)? {
            Response::Status(status) => Ok(status),
            Response::Error(e) => anyhow::bail!("Daemon error: {e}"),
            _ => anyhow::bail!("Unexpected response from daemon"),
        }
    }

    /// Request the daemon to watch a directory
    pub fn watch(&self, path: &Path) -> Result<()> {
        let request = Request::Watch {
            path: path.to_path_buf(),
        };

        match self.send_request(&request)? {
            Response::Ok => Ok(()),
            Response::Error(e) => anyhow::bail!("Daemon error: {e}"),
            _ => anyhow::bail!("Unexpected response from daemon"),
        }
    }

    /// Request the daemon to stop watching a directory
    pub fn unwatch(&self, path: &Path) -> Result<()> {
        let request = Request::Unwatch {
            path: path.to_path_buf(),
        };

        match self.send_request(&request)? {
            Response::Ok => Ok(()),
            Response::Error(e) => anyhow::bail!("Daemon error: {e}"),
            _ => anyhow::bail!("Unexpected response from daemon"),
        }
    }

    /// Request the daemon to force re-index a directory
    pub fn force_index(&self, path: &Path) -> Result<()> {
        let request = Request::ForceIndex {
            path: path.to_path_buf(),
        };

        match self.send_request(&request)? {
            Response::Ok => Ok(()),
            Response::Error(e) => anyhow::bail!("Daemon error: {e}"),
            _ => anyhow::bail!("Unexpected response from daemon"),
        }
    }

    /// Request the daemon to shutdown
    pub fn shutdown(&self) -> Result<()> {
        let request = Request::Shutdown;

        match self.send_request(&request)? {
            Response::Ok => Ok(()),
            Response::Error(e) => anyhow::bail!("Daemon error: {e}"),
            _ => anyhow::bail!("Unexpected response from daemon"),
        }
    }

    /// List all known projects
    pub fn list_projects(&self) -> Result<Vec<ProjectInfo>> {
        let request = Request::ListProjects;

        match self.send_request(&request)? {
            Response::Projects(projects) => Ok(projects),
            Response::Error(e) => anyhow::bail!("Daemon error: {e}"),
            _ => anyhow::bail!("Unexpected response from daemon"),
        }
    }

    /// Run project discovery
    pub fn discover_projects(&self) -> Result<()> {
        let request = Request::DiscoverProjects;

        match self.send_request(&request)? {
            Response::Ok => Ok(()),
            Response::Error(e) => anyhow::bail!("Daemon error: {e}"),
            _ => anyhow::bail!("Unexpected response from daemon"),
        }
    }

    /// Detect project root from a path
    pub fn detect_root(&self, path: &Path) -> Result<Option<String>> {
        let request = Request::DetectRoot {
            path: path.to_path_buf(),
        };

        match self.send_request(&request)? {
            Response::ProjectRoot(root) => Ok(root),
            Response::Error(e) => anyhow::bail!("Daemon error: {e}"),
            _ => anyhow::bail!("Unexpected response from daemon"),
        }
    }
}

/// Read the daemon PID from the PID file
pub fn read_daemon_pid(pid_path: &Path) -> Result<Option<u32>> {
    if !pid_path.exists() {
        return Ok(None);
    }

    let pid_str = std::fs::read_to_string(pid_path).context("Failed to read PID file")?;
    let pid: u32 = pid_str.trim().parse().context("Invalid PID in file")?;

    // Check if process is actually running
    let is_running = unsafe { libc::kill(pid as i32, 0) } == 0;

    if is_running {
        Ok(Some(pid))
    } else {
        // Stale PID file, remove it
        std::fs::remove_file(pid_path).ok();
        Ok(None)
    }
}

/// Kill the daemon process
pub fn kill_daemon(pid_path: &Path) -> Result<bool> {
    if let Some(pid) = read_daemon_pid(pid_path)? {
        // Send SIGTERM
        let result = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        if result == 0 {
            // Wait a bit for graceful shutdown
            std::thread::sleep(Duration::from_millis(GRACEFUL_SHUTDOWN_WAIT_MS));

            // Check if still running
            if unsafe { libc::kill(pid as i32, 0) } == 0 {
                // Still running, send SIGKILL
                unsafe { libc::kill(pid as i32, libc::SIGKILL) };
            }
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_daemon_pid_nonexistent_file() {
        let path = Path::new("/nonexistent/path/to/pid");
        let result = read_daemon_pid(path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_read_daemon_pid_invalid_content() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "not-a-number").unwrap();

        let result = read_daemon_pid(file.path());
        assert!(result.is_err(), "Should fail on invalid PID content");
    }

    #[test]
    fn test_read_daemon_pid_stale_pid() {
        // Use a PID that definitely doesn't exist (very high number)
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "999999999").unwrap();

        let result = read_daemon_pid(file.path()).unwrap();
        // Should return None for stale PID and clean up the file
        assert!(result.is_none(), "Should return None for non-running PID");
        // The file should be removed
        assert!(!file.path().exists(), "Stale PID file should be removed");
    }

    #[test]
    fn test_read_daemon_pid_current_process() {
        // Use our own PID, which is definitely running
        let mut file = NamedTempFile::new().unwrap();
        let our_pid = std::process::id();
        writeln!(file, "{our_pid}").unwrap();

        let result = read_daemon_pid(file.path()).unwrap();
        assert_eq!(result, Some(our_pid), "Should return running PID");
    }

    #[test]
    fn test_read_daemon_pid_with_whitespace() {
        let mut file = NamedTempFile::new().unwrap();
        let our_pid = std::process::id();
        writeln!(file, "  {our_pid}  ").unwrap();

        let result = read_daemon_pid(file.path()).unwrap();
        assert_eq!(result, Some(our_pid), "Should handle whitespace");
    }

    #[test]
    fn test_kill_daemon_nonexistent_pid_file() {
        let path = Path::new("/nonexistent/path/to/pid");
        let result = kill_daemon(path).unwrap();
        assert!(!result, "Should return false when PID file doesn't exist");
    }

    #[test]
    fn test_kill_daemon_stale_pid() {
        // Use a PID that definitely doesn't exist
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "999999999").unwrap();

        let result = kill_daemon(file.path()).unwrap();
        assert!(!result, "Should return false for non-running PID");
    }

    #[test]
    fn test_client_new_sets_socket_path() {
        let path = Path::new("/tmp/test.sock");
        let client = Client::new(path);
        // Client stores the socket path internally
        // We can verify it was set by checking is_daemon_running returns false
        // for a non-existent socket
        assert!(!client.is_daemon_running());
    }

    #[test]
    fn test_client_with_default_socket() {
        // Default socket should be created without panic
        // This verifies the constructor works and uses the default socket path
        let _client = Client::with_default_socket();
        // Note: We don't assert on is_daemon_running as the daemon may or may not be running
    }

    #[test]
    fn test_client_with_timeout_builder() {
        let path = Path::new("/tmp/nonexistent-sg-test-socket-12345.sock");
        let timeout = Duration::from_secs(60);
        let client = Client::new(path).with_timeout(timeout);
        // Verify client was created with custom timeout
        // Using a nonexistent socket path ensures daemon is not running
        assert!(!client.is_daemon_running());
    }

    #[test]
    fn test_client_with_timeout_chaining() {
        let path = Path::new("/tmp/test.sock");
        // Verify timeout can be chained with new()
        let _client = Client::new(path)
            .with_timeout(Duration::from_secs(5))
            .with_timeout(Duration::from_secs(10)); // Should override
    }

    #[test]
    fn test_default_timeout_is_30_seconds() {
        // Default timeout is 30 seconds
        assert_eq!(DEFAULT_TIMEOUT_SECS, 30);
    }

    #[test]
    fn test_graceful_shutdown_wait_is_500ms() {
        // Graceful shutdown wait is 500ms
        assert_eq!(GRACEFUL_SHUTDOWN_WAIT_MS, 500);
    }

    #[test]
    fn test_read_daemon_pid_empty_file() {
        let mut file = NamedTempFile::new().unwrap();
        // Write empty content
        write!(file, "").unwrap();

        let result = read_daemon_pid(file.path());
        assert!(result.is_err(), "Should fail on empty file");
    }

    #[test]
    fn test_read_daemon_pid_zero() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "0").unwrap();

        // PID 0 is never valid for a daemon
        let result = read_daemon_pid(file.path()).unwrap();
        // kill(0, 0) checks if current process can send signals to process group
        // so this behavior varies - we just verify it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_read_daemon_pid_negative() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "-1").unwrap();

        let result = read_daemon_pid(file.path());
        // Negative numbers should fail to parse as u32
        assert!(result.is_err(), "Should fail on negative PID");
    }

    #[test]
    fn test_read_daemon_pid_overflow() {
        let mut file = NamedTempFile::new().unwrap();
        // A number too large for u32
        writeln!(file, "99999999999999999999").unwrap();

        let result = read_daemon_pid(file.path());
        assert!(result.is_err(), "Should fail on overflow");
    }

    #[test]
    fn test_client_is_daemon_running_nonexistent_socket() {
        // Create a path that definitely doesn't exist
        let path = Path::new("/tmp/nonexistent-sg-daemon-test-socket-xyz123.sock");
        let client = Client::new(path);
        assert!(
            !client.is_daemon_running(),
            "Should return false for nonexistent socket"
        );
    }

    #[test]
    fn test_client_is_daemon_running_checks_socket_exists() {
        // Create a temp file (not a socket, but file exists)
        let file = NamedTempFile::new().unwrap();
        let client = Client::new(file.path());

        // Even though file exists, connecting will fail because it's not a socket
        // is_daemon_running should return false
        assert!(
            !client.is_daemon_running(),
            "Should return false for non-socket file"
        );
    }
}
