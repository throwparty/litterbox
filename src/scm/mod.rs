use std::path::{Path, PathBuf};
use std::sync::Mutex;

use git2::{BranchType, ObjectType, Repository};

use crate::domain::{slugify, SandboxError, ScmError};

pub trait Scm {
    fn create_branch(&self, slug: &str) -> Result<String, SandboxError>;
    fn delete_branch(&self, slug: &str) -> Result<(), SandboxError>;
    fn make_archive(&self, reference: &str) -> Result<Vec<u8>, SandboxError>;
    fn list_sandboxes(&self) -> Result<Vec<String>, SandboxError>;
    fn repo_prefix(&self) -> Result<String, SandboxError>;
}

pub struct GitScm {
    repo: Repository,
}

impl GitScm {
    pub fn open(path: &Path) -> Result<Self, SandboxError> {
        Repository::open(path)
            .map(|repo| Self { repo })
            .map_err(|source| SandboxError::Scm(ScmError::Open { source }))
    }

    fn branch_name(slug: &str) -> String {
        format!("litterbox/{}", slug)
    }

    fn repo_root(&self) -> PathBuf {
        self.repo
            .workdir()
            .map(PathBuf::from)
            .unwrap_or_else(|| self.repo.path().to_path_buf())
    }

    fn repo_prefix(&self) -> String {
        repo_prefix_from_path(&self.repo_root())
    }

    fn head_commit(&self) -> Result<git2::Commit<'_>, SandboxError> {
        let head = self
            .repo
            .head()
            .map_err(|source| SandboxError::Scm(ScmError::BranchCreate { source }))?;
        head.peel_to_commit()
            .map_err(|source| SandboxError::Scm(ScmError::BranchCreate { source }))
    }

    fn tree_from_reference(&self, reference: &str) -> Result<git2::Tree<'_>, SandboxError> {
        let obj = self
            .repo
            .revparse_single(reference)
            .map_err(|source| SandboxError::Scm(ScmError::Archive { source }))?;
        obj.peel_to_tree()
            .map_err(|source| SandboxError::Scm(ScmError::Archive { source }))
    }

    fn append_tree(
        repo: &Repository,
        builder: &mut tar::Builder<Vec<u8>>,
        tree: &git2::Tree<'_>,
        base: &Path,
    ) -> Result<(), SandboxError> {
        for entry in tree.iter() {
            Self::append_entry(repo, builder, base, &entry)?;
        }

        Ok(())
    }

    fn append_entry(
        repo: &Repository,
        builder: &mut tar::Builder<Vec<u8>>,
        base: &Path,
        entry: &git2::TreeEntry<'_>,
    ) -> Result<(), SandboxError> {
        let name = entry
            .name()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid path"))?;
        let path = base.join(PathBuf::from(name));

        match entry.kind() {
            Some(ObjectType::Tree) => {
                let subtree = entry
                    .to_object(repo)
                    .map_err(|source| SandboxError::Scm(ScmError::Archive { source }))?
                    .peel_to_tree()
                    .map_err(|source| SandboxError::Scm(ScmError::Archive { source }))?;
                Self::append_tree(repo, builder, &subtree, &path)
            }
            Some(ObjectType::Blob) => Self::append_blob(repo, builder, &path, entry),
            _ => Ok(()),
        }
    }

    fn append_blob(
        repo: &Repository,
        builder: &mut tar::Builder<Vec<u8>>,
        path: &Path,
        entry: &git2::TreeEntry<'_>,
    ) -> Result<(), SandboxError> {
        let blob = entry
            .to_object(repo)
            .map_err(|source| SandboxError::Scm(ScmError::Archive { source }))?
            .peel_to_blob()
            .map_err(|source| SandboxError::Scm(ScmError::Archive { source }))?;

        let mut header = tar::Header::new_gnu();
        let mode = match entry.filemode() {
            0 => 0o644,
            value => value as u32,
        };
        let size = u64::try_from(blob.size()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "blob too large")
        })?;
        header.set_size(size);
        header.set_mode(mode);
        header.set_cksum();

        builder.append_data(&mut header, path, blob.content())?;
        Ok(())
    }
}

pub struct ThreadSafeScm {
    inner: Mutex<GitScm>,
}

impl ThreadSafeScm {
    pub fn open(path: &Path) -> Result<Self, SandboxError> {
        GitScm::open(path).map(|scm| Self {
            inner: Mutex::new(scm),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, GitScm>, SandboxError> {
        self.inner.lock().map_err(|_| {
            SandboxError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "SCM lock poisoned",
            ))
        })
    }
}

impl Scm for GitScm {
    fn create_branch(&self, slug: &str) -> Result<String, SandboxError> {
        let branch_name = Self::branch_name(slug);
        if self
            .repo
            .find_branch(&branch_name, BranchType::Local)
            .is_ok()
        {
            return Err(SandboxError::SandboxExists {
                name: slug.to_string(),
            });
        }

        let target = self.head_commit()?;
        self.repo
            .branch(&branch_name, &target, false)
            .map_err(|source| SandboxError::Scm(ScmError::BranchCreate { source }))?;

        Ok(branch_name)
    }

    fn delete_branch(&self, slug: &str) -> Result<(), SandboxError> {
        let branch_name = Self::branch_name(slug);
        let mut branch = self
            .repo
            .find_branch(&branch_name, BranchType::Local)
            .map_err(|_| SandboxError::SandboxNotFound {
                name: slug.to_string(),
            })?;

        branch
            .delete()
            .map_err(|source| SandboxError::Scm(ScmError::BranchDelete { source }))
    }

    fn make_archive(&self, reference: &str) -> Result<Vec<u8>, SandboxError> {
        let tree = self.tree_from_reference(reference)?;
        let mut builder = tar::Builder::new(Vec::new());
        Self::append_tree(&self.repo, &mut builder, &tree, Path::new(""))?;
        builder.finish()?;
        Ok(builder.into_inner()?)
    }

    fn list_sandboxes(&self) -> Result<Vec<String>, SandboxError> {
        let branches = self
            .repo
            .branches(Some(BranchType::Local))
            .map_err(|source| SandboxError::Scm(ScmError::BranchList { source }))?;
        let mut slugs = Vec::new();

        for branch in branches {
            let (branch, _) =
                branch.map_err(|source| SandboxError::Scm(ScmError::BranchList { source }))?;
            let name = branch
                .name()
                .map_err(|source| SandboxError::Scm(ScmError::BranchList { source }))?;
            if let Some(name) = name {
                if let Some(slug) = name.strip_prefix("litterbox/") {
                    slugs.push(slug.to_string());
                }
            }
        }

        slugs.sort();
        slugs.dedup();
        Ok(slugs)
    }

    fn repo_prefix(&self) -> Result<String, SandboxError> {
        Ok(self.repo_prefix())
    }
}

impl Scm for ThreadSafeScm {
    fn create_branch(&self, slug: &str) -> Result<String, SandboxError> {
        self.lock()?.create_branch(slug)
    }

    fn delete_branch(&self, slug: &str) -> Result<(), SandboxError> {
        self.lock()?.delete_branch(slug)
    }

    fn make_archive(&self, reference: &str) -> Result<Vec<u8>, SandboxError> {
        self.lock()?.make_archive(reference)
    }

    fn list_sandboxes(&self) -> Result<Vec<String>, SandboxError> {
        self.lock()?.list_sandboxes()
    }

    fn repo_prefix(&self) -> Result<String, SandboxError> {
        Ok(self.lock()?.repo_prefix())
    }
}

fn repo_prefix_from_path(path: &Path) -> String {
    let base = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo");
    let slug = slugify(base);
    if slug.is_empty() { "repo".to_string() } else { slug }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;
    use std::io::Cursor;

    use git2::{IndexAddOption, Signature};
    use tempfile::TempDir;

    fn init_repo() -> (TempDir, Repository) {
        let tempdir = TempDir::new().expect("tempdir");
        let repo = Repository::init(tempdir.path()).expect("repo init");

        let file_path = tempdir.path().join("README.md");
        fs::write(&file_path, "hello").expect("write file");

        let gitignore_path = tempdir.path().join(".gitignore");
        fs::write(&gitignore_path, "ignored.txt\n").expect("write gitignore");

        let mut index = repo.index().expect("index");
        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .expect("add all");
        index.write().expect("index write");
        let tree_id = index.write_tree().expect("write tree");

        let signature = Signature::now("Litterbox", "noreply@example.com")
            .expect("signature");
        {
            let tree = repo.find_tree(tree_id).expect("find tree");
            repo.commit(Some("HEAD"), &signature, &signature, "init", &tree, &[])
                .expect("commit");
        }

        (tempdir, repo)
    }

    #[test]
    fn create_branch_creates_litterbox_branch() {
        let (_tempdir, repo) = init_repo();
        let scm = GitScm { repo };

        let branch_name = scm.create_branch("my-feature").expect("create branch");
        assert_eq!(branch_name, "litterbox/my-feature");

        let branch = scm
            .repo
            .find_branch(&branch_name, BranchType::Local)
            .expect("branch exists");
        let branch_commit = branch
            .get()
            .peel_to_commit()
            .expect("branch commit");
        let head_commit = scm
            .repo
            .head()
            .expect("head")
            .peel_to_commit()
            .expect("head commit");
        assert_eq!(branch_commit.id(), head_commit.id());
    }

    #[test]
    fn create_branch_rejects_duplicates() {
        let (_tempdir, repo) = init_repo();
        let scm = GitScm { repo };

        scm.create_branch("my-feature").expect("create branch");
        let err = scm
            .create_branch("my-feature")
            .expect_err("duplicate branch");
        assert_eq!(err.to_string(), "Sandbox 'my-feature' already exists.");
    }

    #[test]
    fn delete_branch_removes_branch() {
        let (_tempdir, repo) = init_repo();
        let scm = GitScm { repo };

        let branch_name = scm.create_branch("cleanup").expect("create branch");
        scm.delete_branch("cleanup").expect("delete branch");

        assert!(scm
            .repo
            .find_branch(&branch_name, BranchType::Local)
            .is_err());
    }

    #[test]
    fn delete_branch_missing_returns_not_found() {
        let (_tempdir, repo) = init_repo();
        let scm = GitScm { repo };

        let err = scm
            .delete_branch("missing")
            .expect_err("missing branch");
        assert_eq!(err.to_string(), "Sandbox 'missing' not found.");
    }

    #[test]
    fn archive_contains_tracked_files_only() {
        let (tempdir, repo) = init_repo();
        let scm = GitScm { repo };

        let ignored_path = tempdir.path().join("ignored.txt");
        fs::write(&ignored_path, "ignored").expect("write ignored");
        let untracked_path = tempdir.path().join("notes.txt");
        fs::write(&untracked_path, "notes").expect("write untracked");

        let archive = scm.make_archive("HEAD").expect("archive");
        let mut entries = Vec::new();
        let mut reader = tar::Archive::new(Cursor::new(archive));
        for entry in reader.entries().expect("entries") {
            let entry = entry.expect("entry");
            let path = entry.path().expect("path");
            entries.push(path.to_string_lossy().to_string());
        }

        entries.sort();
        assert_eq!(entries, vec![".gitignore", "README.md"]);
    }
}
