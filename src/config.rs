use std::fs::File;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_yaml::{self};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub cache_file: Option<String>,
    // we track here which config file (if any) the config came from, but we don't need to write
    // that to the file
    #[serde(skip_serializing)]
    pub config_file: Option<String>,
}

const CONFIG_LOCATIONS: &[&str] = &["~/.config/ghn/config.yaml"];
const DEFAULT_CACHE_FILE: &str = "cache.yaml";

impl Default for Config {
    fn default() -> Self {
        let sanitized_cfg = sanitize_tilde_to_home(CONFIG_LOCATIONS[0]);
        let parent = Path::new(&sanitized_cfg).parent().unwrap();
        Config {
            cache_file: Some(String::from(
                Path::new(parent).join(DEFAULT_CACHE_FILE).to_str().unwrap(),
            )),
            config_file: Some(sanitized_cfg),
        }
    }
}
impl Config {
    pub fn new(filepath: Option<String>) -> Result<Self, String> {
        // lets try to load a config first
        let mut cfg = match filepath {
            Some(f) => {
                let sanitized_cfg_file = sanitize_tilde_to_home(f.as_str());
                let mut cfg = try_yaml_read(&sanitized_cfg_file)
                    .map_err(|e| format!("unable to parse config file '{f}': {e}"))?;
                cfg.config_file = Some(f);
                cfg
            }
            None => {
                // No config file was provided so lets check default locations
                for cfg_path in CONFIG_LOCATIONS {
                    let sanitized_cfg_path = sanitize_tilde_to_home(cfg_path);
                    match try_yaml_read(&sanitized_cfg_path) {
                        Ok(mut cfg) => {
                            cfg.config_file = Some(sanitized_cfg_path);
                        }
                        Err(_) => {
                            // we don't really care if default locations don't have a config file.
                            // Maybe at some point this can be made smarter to determine an existing
                            // but unreadable file throwing an error
                            Config::default();
                        }
                    }
                }
                Config::default()
            }
        };
        // from here on out the cache_file is set
        let cfg_file = cfg.config_file.as_ref().unwrap();
        let cache_file = cfg.cache_file.unwrap_or(String::from(DEFAULT_CACHE_FILE));
        let absolute_cache_file = cache_file_absolute_path(&cache_file, &cfg_file)
            .map_err(|_| format!("Unable to get absolute cache file path for '{cache_file}'",))?;
        cfg.cache_file = Some(absolute_cache_file);

        Ok(cfg)
    }

    pub fn to_yaml(&self) -> Result<String, String> {
        match serde_yaml::to_string(&Config::default()) {
            Ok(serialized) => Ok(serialized),
            Err(e) => Err(e.to_string()),
        }
    }
}

fn cache_file_absolute_path(cache_file: &String, cfg_file: &String) -> Result<String, String> {
    // nothing to do if already absolute path
    if cache_file.starts_with("/") {
        return Ok(format!("{cache_file}"));
    }
    let parent = Path::new(&cfg_file).parent().ok_or_else(|| {
        format!("Provided config file path '{cfg_file}' has no parent directory.",)
    })?;
    let absolute_cache_file = Path::new(parent)
        .join(cache_file)
        .into_os_string()
        .into_string()
        .map_err(|_| format!("Unable to get absolute cache file path for '{cache_file}'",))?;

    Ok(String::from(absolute_cache_file))
}

fn sanitize_tilde_to_home(path: &str) -> String {
    let home = std::env::var("HOME").unwrap();
    if path.starts_with("~/") {
        // we should be fine to unwrap() here since we already checked for the prefix
        format!("{}/{}", home, path.strip_prefix("~/").unwrap())
    } else {
        String::from(path)
    }
}

fn try_yaml_read(fpath: &String) -> Result<Config, String> {
    let open_file =
        File::open(fpath.clone()).map_err(|e| format!("unable to read file '{}': {}", fpath, e))?;
    let cfg: Config = serde_yaml::from_reader(open_file)
        .map_err(|e| format!("unable to parse config file '{}': {}", fpath, e))?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_simple_config() {
        let cfg = Config::new(Some(String::from("fixtures/config/simple_config.yaml"))).unwrap();
        assert_eq!(cfg.cache_file.unwrap(), "fixtures/config/cache.yaml");
    }
    #[test]
    fn test_sanitize_tilde() {
        unsafe {
            std::env::set_var("HOME", "/home/test");
        }
        assert_eq!(sanitize_tilde_to_home("./bla"), "./bla");
        assert_eq!(sanitize_tilde_to_home("~/bla"), "/home/test/bla");
    }
}
