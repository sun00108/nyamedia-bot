use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::thread;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ConfigEntry {
    source: String,
    destination: String,
    regex: String,
}

fn read_config<P: AsRef<Path>>(path: P) -> io::Result<Vec<ConfigEntry>> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: Vec<ConfigEntry> = serde_json::from_str(&contents)?;
    Ok(config)
}

fn write_config_pending<P: AsRef<Path>>(path: P, config: &Vec<ConfigEntry>) -> io::Result<()> {
    let mut file = File::create(path)?;
    let contents = serde_json::to_string_pretty(config)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

fn monitor_folder<P: AsRef<Path>>(folder_path: P, config_path: P, pending_config_path: P) -> io::Result<()> {
    loop {
        let config = read_config(&config_path)?;
        let mut pending_config = read_config(&pending_config_path).unwrap_or_else(|_| Vec::new());
        let mut existing_sources: Vec<String> = pending_config.iter().map(|e| e.source.clone()).collect();

        for entry in fs::read_dir(&folder_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Some(config_entry) = config.iter().find(|e| e.source == folder_name) {
                        if !existing_sources.contains(&config_entry.source) {
                            let pending_entry = ConfigEntry {
                                source: config_entry.source.clone(),
                                destination: "".to_string(),
                                regex: config_entry.regex.clone(),
                            };
                            pending_config.push(pending_entry);
                            existing_sources.push(config_entry.source.clone());
                        }
                    }
                }
            }
        }

        write_config_pending(&pending_config_path, &pending_config)?;

        thread::sleep(Duration::from_secs(10)); // 每10秒检查一次
    }
}

fn main() -> io::Result<()> {
    let folder_path = std::env::args().nth(1).expect("No folder path provided");
    let config_path = Path::new(&folder_path).join("config.json").to_string_lossy().into_owned();
    let pending_config_path = Path::new(&folder_path).join("configpending.json").to_string_lossy().into_owned();

    monitor_folder(folder_path, config_path, pending_config_path)
}
