use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use std::error::Error;
use std::env;
use serde::{Deserialize, Serialize};
use regex::Regex;

#[derive(Deserialize, Serialize, Clone)]
struct Config {
    source_dir: String,
    #[serde(default)]
    rules: Vec<ArchiveRule>,
}

#[derive(Deserialize, Serialize, Clone)]
struct ArchiveRule {
    folder_name: String,
    chinese_name: String,
    target_dir: String,
    pattern: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "config.toml"
    };

    println!("Using config file: {}", config_path);

    let mut config = load_config(config_path)?;
    update_rules(&mut config)?;
    save_config(&config, config_path)?;
    process_files(&config)?;
    Ok(())
}

fn load_config(path: &str) -> Result<Config, Box<dyn Error>> {
    let content = fs::read_to_string(path).unwrap_or_else(|_| {
        println!("Config file not found. Creating a new one.");
        String::from("source_dir = \"\"\n")
    });
    let mut config: Config = toml::from_str(&content)?;
    if config.source_dir.is_empty() {
        return Err("source_dir cannot be empty".into());
    }
    Ok(config)
}

fn save_config(config: &Config, path: &str) -> Result<(), Box<dyn Error>> {
    let content = toml::to_string(config)?;
    fs::write(path, content)?;
    println!("Config saved to: {}", path);
    Ok(())
}

fn update_rules(config: &mut Config) -> Result<(), Box<dyn Error>> {
    let source_dir = Path::new(&config.source_dir);
    let mut existing_rules: HashMap<String, ArchiveRule> = config.rules.iter().cloned().map(|r| (r.folder_name.clone(), r)).collect();

    let mut updated_rules = Vec::new();

    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let folder_name = entry.file_name().to_string_lossy().into_owned();
            let rule = existing_rules.remove(&folder_name).unwrap_or_else(|| ArchiveRule {
                folder_name: folder_name.clone(),
                chinese_name: String::new(),
                target_dir: String::from("/data/animenew"),
                pattern: String::from(r"S(\d+)E(\d+)"),
            });
            updated_rules.push(rule);
        }
    }

    config.rules = updated_rules;
    Ok(())
}

fn process_files(config: &Config) -> Result<(), Box<dyn Error>> {
    for rule in &config.rules {
        if rule.chinese_name.is_empty() {
            println!("Skipping folder '{}' as chinese_name is empty", rule.folder_name);
            continue;
        }

        let source_folder = Path::new(&config.source_dir).join(&rule.folder_name);
        if !source_folder.exists() {
            println!("Source folder not found: {:?}", source_folder);
            continue;
        }

        let re = Regex::new(&rule.pattern)?;
        for entry in fs::read_dir(source_folder)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                    if let Some(captures) = re.captures(file_name) {
                        let season = captures.get(1).map_or("01", |m| m.as_str());
                        let episode = captures.get(2).map_or("01", |m| m.as_str());

                        let destination = Path::new(&rule.target_dir)
                            .join(&rule.chinese_name)
                            .join(format!("Season {:02}", season.parse::<u32>()?));

                        fs::create_dir_all(&destination)?;

                        let new_file_name = format!("{} - S{}E{}{}",
                                                    rule.chinese_name,
                                                    season,
                                                    episode,
                                                    path.extension().map_or_else(|| "".to_string(), |ext| format!(".{}", ext.to_str().unwrap()))
                        );

                        let new_path = destination.join(&new_file_name);

                        // Check if the file already exists in the destination
                        if new_path.exists() {
                            println!("File already exists, skipping: {:?}", new_path);
                            continue;
                        }

                        fs::rename(&path, &new_path)?;
                        println!("Moved {:?} to {:?}", path, new_path);
                    }
                }
            }
        }
    }
    Ok(())
}