use std::{
    fs,
    path::PathBuf,
};

use nanoserde::DeJson;

const DB_NAME: &str = "tascli.db";
const DEFAULT_DATA_DIR: &[&str] = &[".local", "share", "tascli"];
const CONFIG_PATH: &[&str] = &[".config", "tascli", "config.json"];

#[derive(Default, DeJson)]
pub struct Config {
    /// Only supports full path.
    #[nserde(default)]
    pub data_dir: String,
}

pub fn get_data_path() -> Result<PathBuf, String> {
    let home_dir = home::home_dir().ok_or_else(|| String::from("cannot find home directory"))?;
    let data_dir = match get_config_data_dir(home_dir.clone()) {
        Some(dir_path) => str_to_pathbuf(dir_path)?,
        None => DEFAULT_DATA_DIR.iter().fold(home_dir, |p, d| p.join(d)),
    };
    fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data directory: {}", e))?;
    Ok(data_dir.join(DB_NAME))
}

// Quick passthrough for reading config file
// If config file do not exist, return quickly
fn get_config_data_dir(home_dir: PathBuf) -> Option<String> {
    let config_path = CONFIG_PATH.iter().fold(home_dir, |p, d| p.join(d));
    if !config_path.exists() {
        return None;
    }
    let config_content = match fs::read_to_string(&config_path) {
        Ok(content) => content,
        Err(_) => return None,
    };
    let config: Config = match DeJson::deserialize_json(&config_content) {
        Ok(config) => config,
        Err(_) => return None,
    };
    if config.data_dir.is_empty() {
        None
    } else {
        Some(config.data_dir)
    }
}

fn str_to_pathbuf(dir_path: String) -> Result<PathBuf, String> {
    if dir_path.starts_with("~") {
        // We have already executed home_dir previously
        let mut path_buf = home::home_dir().unwrap();
        if dir_path.len() > 2 && dir_path.starts_with("~/") {
            path_buf.push(&dir_path[2..]);
        }
        Ok(path_buf)
    } else if dir_path.starts_with("/") {
        Ok(PathBuf::from(dir_path))
    } else {
        Err(format!("data directory must be absolute or home relative, and start with '~' or '/', it cannot be {}", dir_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_to_pathbuf_with_tilde() {
        // Test with just "~"
        let result = str_to_pathbuf("~".to_string()).unwrap();
        let expected = home::home_dir().unwrap();
        assert_eq!(result, expected);

        let result = str_to_pathbuf("~/".to_string()).unwrap();
        let expected = home::home_dir().unwrap();
        assert_eq!(result, expected);

        let result = str_to_pathbuf("~/some/path".to_string()).unwrap();
        let expected = home::home_dir().unwrap().join("some").join("path");
        assert_eq!(result, expected);

        let result = str_to_pathbuf("some/relative/path".to_string());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("must be absolute or home relative"));
    }
}
