use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use std::error::Error;
use std::env;
use serde::{Deserialize, Serialize};
use regex::Regex;
use reqwest;
use serde_json::Value;

#[derive(Deserialize, Serialize, Clone)]
struct Config {
    source_dir: String,
    tmdb_api_key: String,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "config.toml"
    };

    println!("Using config file: {}", config_path);

    let mut config = load_config(config_path)?;
    update_rules(&mut config).await?;
    save_config(&config, config_path)?;
    process_files(&config)?;
    Ok(())
}

fn load_config(path: &str) -> Result<Config, Box<dyn Error>> {
    let content = fs::read_to_string(path).unwrap_or_else(|_| {
        println!("Config file not found. Creating a new one.");
        String::from("source_dir = \"\"\ntmdb_api_key = \"\"\n")
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

async fn update_rules(config: &mut Config) -> Result<(), Box<dyn Error>> {
    let source_dir = Path::new(&config.source_dir);
    let mut existing_rules: HashMap<String, ArchiveRule> = config.rules.iter().cloned().map(|r| (r.folder_name.clone(), r)).collect();

    let mut updated_rules = Vec::new();

    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let folder_name = entry.file_name().to_string_lossy().into_owned();

            // 检查规则是否已存在，直接复用
            if let Some(existing_rule) = existing_rules.remove(&folder_name) {
                updated_rules.push(existing_rule);
                continue;
            }

            // 创建新规则
            let chinese_name = extract_name_and_query_tmdb(&folder_name, &config.tmdb_api_key).await.unwrap_or_else(|_| String::new());

            updated_rules.push(ArchiveRule {
                folder_name: folder_name.clone(),
                chinese_name,
                target_dir: String::from("/data/animenew"), // 新规则默认值
                pattern: String::from(r"S(\d+)E(\d+)"),
            });
        }
    }

    // 保留所有未匹配的现有规则
    updated_rules.extend(existing_rules.into_values());

    config.rules = updated_rules;
    Ok(())
}

async fn extract_name_and_query_tmdb(folder_name: &str, api_key: &str) -> Result<String, Box<dyn Error>> {
    // Extract the name part before 'S0x'
    let re = Regex::new(r"^(.*?)S\d+")?;
    if let Some(captures) = re.captures(folder_name) {
        let raw_name = captures.get(1).map_or("", |m| m.as_str()).replace('.', " ");

        println!("Extracted name: {}", raw_name);
        // Query TMDB API
        if let Ok(chinese_name) = query_tmdb(&raw_name, api_key).await {
            return Ok(chinese_name);
        }
    }
    Ok(String::new())
}

async fn query_tmdb(raw_name: &str, api_key: &str) -> Result<String, Box<dyn Error>> {
    println!("Querying TMDB for: {}", raw_name);
    let url = format!(
        "https://api.themoviedb.org/3/search/tv?language=zh-CN&api_key={}&query={}",
        api_key,
        raw_name
    );

    let response = reqwest::get(&url).await?.text().await?;
    println!("TMDB response: {}", response);
    let json: Value = serde_json::from_str(&response)?;

    if let Some(results) = json["results"].as_array() {
        if let Some(first_result) = results.first() {
            if let Some(chinese_name) = first_result["name"].as_str() {
                return Ok(chinese_name.to_string());
            }
        }
    }

    Err("TMDB query failed".into())
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