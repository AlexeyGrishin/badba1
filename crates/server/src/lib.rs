//! Web server for badba1: a REST API over the databases in a folder (image files
//! in any storage format the `backend` library supports), and a
//! small web page for viewing and editing their records.

mod api;

use std::path::Path;

pub use api::{app, valid_id};

/// Checks that `dir` exists and is a folder, for use as the database folder.
pub fn check_db_dir(dir: &Path) -> Result<(), String> {
    if dir.is_dir() {
        Ok(())
    } else {
        Err(format!("database folder {} does not exist", dir.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_folder_is_named() {
        let err = check_db_dir(Path::new("no-such-dir")).unwrap_err();
        assert!(err.contains("no-such-dir"), "{err}");
    }

    #[test]
    fn existing_folder_is_accepted() {
        assert!(check_db_dir(&std::env::temp_dir()).is_ok());
    }

    #[test]
    fn id_rules() {
        assert!(valid_id("example1") && valid_id("a-b_C9"));
        assert!(!valid_id("") && !valid_id("../x") && !valid_id("a.b") && !valid_id("a b"));
    }
}
