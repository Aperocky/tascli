use std::{
    fs,
    path::PathBuf,
};

use crate::{
    actions::display,
    args::parser::OpsBackupCommand,
    config::{
        get_data_path,
        str_to_pathbuf,
    },
};

const BACKUP_FILENAME: &str = "tascli_bak.db";

pub fn handle_backupcmd(cmd: &OpsBackupCommand) -> Result<(), String> {
    backup_path(cmd.path.clone())
}

pub fn backup_path(path: Option<String>) -> Result<(), String> {
    let source_path = get_data_path()?;
    if !source_path.exists() {
        return Err("Source database does not exist".to_string());
    }

    let dest_path = resolve_dest_path(&source_path, path)?;
    fs::copy(&source_path, &dest_path).map_err(|e| format!("Failed to backup database: {}", e))?;

    display::print_bold(&format!("Backed up to: {}", dest_path.display()));
    Ok(())
}

fn resolve_dest_path(source_path: &PathBuf, path: Option<String>) -> Result<PathBuf, String> {
    match path {
        None => {
            let parent = source_path
                .parent()
                .ok_or("Cannot determine source directory")?;
            Ok(parent.join(BACKUP_FILENAME))
        }
        Some(p) => {
            let dest = str_to_pathbuf(p)?;
            if dest.is_dir() {
                Ok(dest.join(BACKUP_FILENAME))
            } else {
                // Ensure parent directory exists
                if let Some(parent) = dest.parent() {
                    if !parent.as_os_str().is_empty() && !parent.exists() {
                        return Err(format!("Directory does not exist: {}", parent.display()));
                    }
                }
                Ok(dest)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn test_resolve_dest_path() {
        let temp_dir = tempdir().unwrap();
        let base = temp_dir.path().to_str().unwrap();
        let source_path = PathBuf::from(format!("{}/tascli.db", base));
        File::create(&source_path).unwrap();

        // No path provided - use same directory
        let dest = resolve_dest_path(&source_path, None).unwrap();
        assert_eq!(dest, PathBuf::from(format!("{}/{}", base, BACKUP_FILENAME)));

        // Directory path provided
        let sub_dir = format!("{}/backups", base);
        fs::create_dir(&sub_dir).unwrap();
        let dest = resolve_dest_path(&source_path, Some(sub_dir.clone())).unwrap();
        assert_eq!(dest, PathBuf::from(format!("{}/{}", sub_dir, BACKUP_FILENAME)));

        // File path provided
        let custom_file = format!("{}/my_backup.db", base);
        let dest = resolve_dest_path(&source_path, Some(custom_file.clone())).unwrap();
        assert_eq!(dest, PathBuf::from(&custom_file));

        // Non-existent parent directory
        let bad_path = format!("{}/nonexistent/backup.db", base);
        let result = resolve_dest_path(&source_path, Some(bad_path));
        assert!(result.is_err());
    }
}
