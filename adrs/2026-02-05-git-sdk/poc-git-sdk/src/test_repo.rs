use std::path::Path;

use git2::Repository;
use tempfile::TempDir;

#[derive(Debug)]
pub struct TestRepo {
    tempdir: TempDir,
}

impl TestRepo {
    pub fn new() -> Result<Self, std::io::Error> {
        let tempdir = TempDir::new()?;
        Ok(Self { tempdir })
    }

    pub fn init(&self) -> Result<Repository, git2::Error> {
        Repository::init(self.path())
    }

    pub fn path(&self) -> &Path {
        self.tempdir.path()
    }
}
