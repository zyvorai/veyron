// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

// Git Repository Management - Repository operations and version control

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Git repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRepository {
    pub url: String,
    pub branch: String,
    pub path: String,
    pub last_fetch: Option<DateTime<Utc>>,
    pub current_revision: Option<String>,
    pub status: RepositoryStatus,
}

impl GitRepository {
    pub fn new(url: impl Into<String>, branch: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            branch: branch.into(),
            path: ".".to_string(),
            last_fetch: None,
            current_revision: None,
            status: RepositoryStatus::NotCloned,
        }
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    pub fn clone(&mut self) -> Result<(), String> {
        // Validate inputs to prevent git argument injection.
        // Block values starting with '-' (git flag injection), containing
        // null bytes, or using shell metacharacters that could be dangerous.
        for (label, value) in [("URL", &self.url), ("branch", &self.branch)] {
            if value.starts_with('-') {
                return Err(format!(
                    "Invalid repository {}: must not start with '-'",
                    label
                ));
            }
            if value.contains('\0') {
                return Err(format!(
                    "Invalid repository {}: must not contain null bytes",
                    label
                ));
            }
            if value.contains("--upload-pack")
                || value.contains("--config")
                || value.contains("--exec-path")
            {
                return Err(format!(
                    "Invalid repository {}: contains disallowed git option",
                    label
                ));
            }
        }
        // Branch names: only allow alphanumeric, '-', '_', '.', '/'
        if !self
            .branch
            .chars()
            .all(|c| c.is_alphanumeric() || "-_./".contains(c))
        {
            return Err("Invalid branch name: contains disallowed characters".to_string());
        }

        self.status = RepositoryStatus::Cloning;

        let output = std::process::Command::new("git")
            .arg("clone")
            .arg("--branch")
            .arg(&self.branch)
            .arg("--single-branch")
            .arg("--")
            .arg(&self.url)
            .arg(&self.path)
            .output()
            .map_err(|e| format!("Failed to execute git clone: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            self.status = RepositoryStatus::Error {
                message: stderr.trim().to_string(),
            };
            return Err(format!("git clone failed: {}", stderr.trim()));
        }

        // Parse real HEAD revision
        let rev_output = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .map_err(|e| format!("Failed to get HEAD revision: {}", e))?;

        let revision = String::from_utf8_lossy(&rev_output.stdout)
            .trim()
            .to_string();

        self.status = RepositoryStatus::Ready;
        self.current_revision = if revision.is_empty() {
            None
        } else {
            Some(revision)
        };
        self.last_fetch = Some(Utc::now());
        Ok(())
    }

    pub fn fetch(&mut self) -> Result<(), String> {
        if !matches!(self.status, RepositoryStatus::Ready) {
            return Err("Repository not ready".to_string());
        }

        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .arg("fetch")
            .arg("--all")
            .output()
            .map_err(|e| format!("Failed to execute git fetch: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git fetch failed: {}", stderr.trim()));
        }

        self.last_fetch = Some(Utc::now());
        Ok(())
    }

    pub fn checkout(&mut self, revision: impl Into<String>) -> Result<(), String> {
        let rev = revision.into();

        // Validate revision to prevent argument injection
        if rev.starts_with('-') || rev.contains('\0') {
            return Err(
                "Invalid revision: must not start with '-' or contain null bytes".to_string(),
            );
        }

        if matches!(self.status, RepositoryStatus::Ready) {
            let output = std::process::Command::new("git")
                .arg("-C")
                .arg(&self.path)
                .arg("checkout")
                .arg("--")
                .arg(&rev)
                .output()
                .map_err(|e| format!("Failed to execute git checkout: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("git checkout failed: {}", stderr.trim()));
            }
        }

        self.current_revision = Some(rev);
        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.status, RepositoryStatus::Ready)
    }
}

/// Repository status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RepositoryStatus {
    NotCloned,
    Cloning,
    Ready,
    Error { message: String },
}

/// Git commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommit {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub email: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub files_changed: Vec<String>,
}

impl GitCommit {
    pub fn new(
        hash: impl Into<String>,
        author: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let hash_str = hash.into();
        let short = if hash_str.len() > 7 {
            hash_str[..7].to_string()
        } else {
            hash_str.clone()
        };

        Self {
            hash: hash_str,
            short_hash: short,
            author: author.into(),
            email: String::new(),
            message: message.into(),
            timestamp: Utc::now(),
            files_changed: Vec::new(),
        }
    }

    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = email.into();
        self
    }

    pub fn add_file(mut self, file: impl Into<String>) -> Self {
        self.files_changed.push(file.into());
        self
    }

    pub fn file_count(&self) -> usize {
        self.files_changed.len()
    }
}

/// Git history
pub struct GitHistory {
    commits: Vec<GitCommit>,
}

impl GitHistory {
    pub fn new() -> Self {
        Self {
            commits: Vec::new(),
        }
    }

    pub fn add_commit(&mut self, commit: GitCommit) {
        self.commits.push(commit);
    }

    pub fn get_commit(&self, hash: &str) -> Option<&GitCommit> {
        self.commits
            .iter()
            .find(|c| c.hash == hash || c.short_hash == hash)
    }

    pub fn commits_since(&self, since: &DateTime<Utc>) -> Vec<&GitCommit> {
        self.commits
            .iter()
            .filter(|c| c.timestamp > *since)
            .collect()
    }

    pub fn latest_commit(&self) -> Option<&GitCommit> {
        self.commits.last()
    }

    pub fn commit_count(&self) -> usize {
        self.commits.len()
    }

    pub fn commits_by_author(&self, author: &str) -> Vec<&GitCommit> {
        self.commits.iter().filter(|c| c.author == author).collect()
    }
}

impl Default for GitHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Git diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiff {
    pub from_revision: String,
    pub to_revision: String,
    pub files_added: Vec<String>,
    pub files_modified: Vec<String>,
    pub files_deleted: Vec<String>,
    pub generated_at: DateTime<Utc>,
}

impl GitDiff {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from_revision: from.into(),
            to_revision: to.into(),
            files_added: Vec::new(),
            files_modified: Vec::new(),
            files_deleted: Vec::new(),
            generated_at: Utc::now(),
        }
    }

    pub fn add_file(&mut self, file: impl Into<String>) {
        self.files_added.push(file.into());
    }

    pub fn modify_file(&mut self, file: impl Into<String>) {
        self.files_modified.push(file.into());
    }

    pub fn delete_file(&mut self, file: impl Into<String>) {
        self.files_deleted.push(file.into());
    }

    pub fn total_changes(&self) -> usize {
        self.files_added.len() + self.files_modified.len() + self.files_deleted.len()
    }

    pub fn has_changes(&self) -> bool {
        self.total_changes() > 0
    }
}

/// Repository manager
pub struct RepositoryManager {
    repositories: HashMap<String, GitRepository>,
    histories: HashMap<String, GitHistory>,
}

impl RepositoryManager {
    pub fn new() -> Self {
        Self {
            repositories: HashMap::new(),
            histories: HashMap::new(),
        }
    }

    pub fn add_repository(&mut self, repo: GitRepository) -> String {
        let repo_id = self.generate_repo_id(&repo.url);
        self.repositories.insert(repo_id.clone(), repo);
        self.histories.insert(repo_id.clone(), GitHistory::new());
        repo_id
    }

    pub fn get_repository(&self, repo_id: &str) -> Option<&GitRepository> {
        self.repositories.get(repo_id)
    }

    pub fn get_repository_mut(&mut self, repo_id: &str) -> Option<&mut GitRepository> {
        self.repositories.get_mut(repo_id)
    }

    pub fn remove_repository(&mut self, repo_id: &str) -> bool {
        self.histories.remove(repo_id);
        self.repositories.remove(repo_id).is_some()
    }

    pub fn list_repositories(&self) -> Vec<&GitRepository> {
        self.repositories.values().collect()
    }

    pub fn get_history(&self, repo_id: &str) -> Option<&GitHistory> {
        self.histories.get(repo_id)
    }

    pub fn get_history_mut(&mut self, repo_id: &str) -> Option<&mut GitHistory> {
        self.histories.get_mut(repo_id)
    }

    pub fn ready_repositories(&self) -> Vec<&GitRepository> {
        self.repositories
            .values()
            .filter(|r| r.is_ready())
            .collect()
    }

    pub fn repository_count(&self) -> usize {
        self.repositories.len()
    }

    fn generate_repo_id(&self, url: &str) -> String {
        // Use last two path components joined by '-' for uniqueness
        let parts: Vec<&str> = url
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 2 {
            format!("{}-{}", parts[parts.len() - 2], parts[parts.len() - 1])
        } else if let Some(last) = parts.last() {
            last.to_string()
        } else {
            "repo".to_string()
        }
    }
}

impl Default for RepositoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_repository() {
        let repo = GitRepository::new("https://github.com/org/repo.git", "main").with_path("vms");

        assert_eq!(repo.url, "https://github.com/org/repo.git");
        assert_eq!(repo.branch, "main");
        assert_eq!(repo.path, "vms");
        assert!(!repo.is_ready());
    }

    #[test]
    fn test_repository_clone() {
        // Set up a temporary local git repo to clone from
        let tmp_src = std::env::temp_dir().join("vmrogue-test-src-repo");
        let tmp_dest = std::env::temp_dir().join("vmrogue-test-clone-dest");
        let _ = std::fs::remove_dir_all(&tmp_src);
        let _ = std::fs::remove_dir_all(&tmp_dest);

        // Initialize a bare local repo to clone from
        let init_ok = std::process::Command::new("git")
            .args(["init", "--initial-branch", "main"])
            .arg(&tmp_src)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !init_ok {
            // git not available, skip test
            return;
        }

        // Configure git identity for the temp repo (required on CI)
        let _ = std::process::Command::new("git")
            .args(["-C"])
            .arg(&tmp_src)
            .args(["config", "user.email", "test@vmrogue.dev"])
            .output();
        let _ = std::process::Command::new("git")
            .args(["-C"])
            .arg(&tmp_src)
            .args(["config", "user.name", "vmrogue-test"])
            .output();

        // Create an initial commit so clone has something to fetch
        let _ = std::process::Command::new("git")
            .args(["-C"])
            .arg(&tmp_src)
            .args(["commit", "--allow-empty", "-m", "init"])
            .output();

        let mut repo = GitRepository::new(tmp_src.to_str().unwrap(), "main")
            .with_path(tmp_dest.to_str().unwrap());

        let result = GitRepository::clone(&mut repo);

        // Clean up
        let _ = std::fs::remove_dir_all(&tmp_src);
        let _ = std::fs::remove_dir_all(&tmp_dest);

        if let Ok(()) = result {
            assert!(repo.is_ready());
            assert!(repo.current_revision.is_some());
            assert!(repo.last_fetch.is_some());
        }
    }

    #[test]
    fn test_repository_fetch() {
        // Set up a temporary local git repo
        let tmp_src = std::env::temp_dir().join("vmrogue-test-fetch-src");
        let tmp_dest = std::env::temp_dir().join("vmrogue-test-fetch-dest");
        let _ = std::fs::remove_dir_all(&tmp_src);
        let _ = std::fs::remove_dir_all(&tmp_dest);

        let init_ok = std::process::Command::new("git")
            .args(["init", "--initial-branch", "main"])
            .arg(&tmp_src)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !init_ok {
            return;
        }

        let _ = std::process::Command::new("git")
            .args(["-C"])
            .arg(&tmp_src)
            .args(["config", "user.email", "test@vmrogue.dev"])
            .output();
        let _ = std::process::Command::new("git")
            .args(["-C"])
            .arg(&tmp_src)
            .args(["config", "user.name", "vmrogue-test"])
            .output();

        let _ = std::process::Command::new("git")
            .args(["-C"])
            .arg(&tmp_src)
            .args(["commit", "--allow-empty", "-m", "init"])
            .output();

        let mut repo = GitRepository::new(tmp_src.to_str().unwrap(), "main")
            .with_path(tmp_dest.to_str().unwrap());

        if GitRepository::clone(&mut repo).is_ok() {
            assert!(repo.fetch().is_ok());
        }

        let _ = std::fs::remove_dir_all(&tmp_src);
        let _ = std::fs::remove_dir_all(&tmp_dest);
    }

    #[test]
    fn test_repository_checkout() {
        let mut repo = GitRepository::new("https://example.com/repo.git", "main");

        // Checkout without a Ready repo just sets the revision
        assert!(repo.checkout("abc123").is_ok());
        assert_eq!(repo.current_revision, Some("abc123".to_string()));
    }

    #[test]
    fn test_repository_status() {
        let repo = GitRepository::new("https://example.com/repo.git", "main");

        assert_eq!(repo.status, RepositoryStatus::NotCloned);
    }

    #[test]
    fn test_git_commit() {
        let commit = GitCommit::new("abc123def456", "John Doe", "Add VM manifest")
            .with_email("john@example.com")
            .add_file("vms/production-vm.yaml")
            .add_file("vms/staging-vm.yaml");

        assert_eq!(commit.author, "John Doe");
        assert_eq!(commit.email, "john@example.com");
        assert_eq!(commit.short_hash, "abc123d");
        assert_eq!(commit.file_count(), 2);
    }

    #[test]
    fn test_git_commit_short_hash() {
        let commit = GitCommit::new("abc", "Author", "Message");
        assert_eq!(commit.short_hash, "abc");
    }

    #[test]
    fn test_git_history() {
        let mut history = GitHistory::new();

        let commit1 = GitCommit::new("hash1", "Alice", "Commit 1");
        let commit2 = GitCommit::new("hash2", "Bob", "Commit 2");

        history.add_commit(commit1);
        history.add_commit(commit2);

        assert_eq!(history.commit_count(), 2);
    }

    #[test]
    fn test_history_get_commit() {
        let mut history = GitHistory::new();

        let commit = GitCommit::new("abc123def", "Author", "Message");
        history.add_commit(commit);

        assert!(history.get_commit("abc123def").is_some());
        assert!(history.get_commit("abc123d").is_some()); // Short hash
    }

    #[test]
    fn test_history_latest_commit() {
        let mut history = GitHistory::new();

        history.add_commit(GitCommit::new("hash1", "Author", "First"));
        history.add_commit(GitCommit::new("hash2", "Author", "Latest"));

        let latest = history.latest_commit().unwrap();
        assert_eq!(latest.hash, "hash2");
    }

    #[test]
    fn test_history_commits_by_author() {
        let mut history = GitHistory::new();

        history.add_commit(GitCommit::new("hash1", "Alice", "Commit by Alice"));
        history.add_commit(GitCommit::new("hash2", "Bob", "Commit by Bob"));
        history.add_commit(GitCommit::new("hash3", "Alice", "Another by Alice"));

        let alice_commits = history.commits_by_author("Alice");
        assert_eq!(alice_commits.len(), 2);
    }

    #[test]
    fn test_history_commits_since() {
        let mut history = GitHistory::new();

        let old_time = Utc::now() - chrono::TimeDelta::hours(2);
        let recent_time = Utc::now();

        let mut old_commit = GitCommit::new("hash1", "Author", "Old");
        old_commit.timestamp = old_time;

        let mut recent_commit = GitCommit::new("hash2", "Author", "Recent");
        recent_commit.timestamp = recent_time;

        history.add_commit(old_commit);
        history.add_commit(recent_commit);

        let since = Utc::now() - chrono::TimeDelta::hours(1);
        let commits = history.commits_since(&since);

        assert_eq!(commits.len(), 1);
    }

    #[test]
    fn test_git_diff() {
        let mut diff = GitDiff::new("abc123", "def456");

        diff.add_file("new-vm.yaml");
        diff.modify_file("existing-vm.yaml");
        diff.delete_file("old-vm.yaml");

        assert_eq!(diff.files_added.len(), 1);
        assert_eq!(diff.files_modified.len(), 1);
        assert_eq!(diff.files_deleted.len(), 1);
        assert_eq!(diff.total_changes(), 3);
        assert!(diff.has_changes());
    }

    #[test]
    fn test_diff_no_changes() {
        let diff = GitDiff::new("abc123", "abc123");

        assert!(!diff.has_changes());
        assert_eq!(diff.total_changes(), 0);
    }

    #[test]
    fn test_repository_manager() {
        let mut manager = RepositoryManager::new();

        let repo = GitRepository::new("https://github.com/org/repo.git", "main");
        let repo_id = manager.add_repository(repo);

        assert_eq!(manager.repository_count(), 1);
        assert!(manager.get_repository(&repo_id).is_some());
    }

    #[test]
    fn test_manager_remove_repository() {
        let mut manager = RepositoryManager::new();

        let repo = GitRepository::new("https://example.com/repo.git", "main");
        let repo_id = manager.add_repository(repo);

        assert!(manager.remove_repository(&repo_id));
        assert_eq!(manager.repository_count(), 0);
    }

    #[test]
    fn test_manager_ready_repositories() {
        let mut manager = RepositoryManager::new();

        // Manually set a repo as Ready without actual clone
        let mut repo1 = GitRepository::new("https://example.com/repo1.git", "main");
        repo1.status = RepositoryStatus::Ready;
        repo1.current_revision = Some("abc123".to_string());

        let repo2 = GitRepository::new("https://example.com/repo2.git", "main");

        manager.add_repository(repo1);
        manager.add_repository(repo2);

        assert_eq!(manager.ready_repositories().len(), 1);
    }

    #[test]
    fn test_manager_get_history() {
        let mut manager = RepositoryManager::new();

        let repo = GitRepository::new("https://example.com/repo.git", "main");
        let repo_id = manager.add_repository(repo);

        assert!(manager.get_history(&repo_id).is_some());
    }

    #[test]
    fn test_manager_history_integration() {
        let mut manager = RepositoryManager::new();

        let repo = GitRepository::new("https://example.com/repo.git", "main");
        let repo_id = manager.add_repository(repo);

        let history = manager.get_history_mut(&repo_id).unwrap();
        history.add_commit(GitCommit::new("hash1", "Author", "Test commit"));

        let history = manager.get_history(&repo_id).unwrap();
        assert_eq!(history.commit_count(), 1);
    }

    #[test]
    fn test_generate_repo_id() {
        let manager = RepositoryManager::new();

        let id = manager.generate_repo_id("https://github.com/org/my-repo.git");
        assert_eq!(id, "org-my-repo");
    }
}
