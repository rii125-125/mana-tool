use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct ManaboxConfig {
    pub file: Vec<String>,
    pub must: Vec<String>,
    pub select: Vec<String>,
}

#[derive(Debug)]
pub struct FileSnapshot {
    pub files: HashMap<String, String>,
}

impl ManaboxConfig {
    const CONFIG_NAME: &'static str = "manabox.yml"; // 名前を変更

    pub fn load() -> Result<Self> {
        let content = fs::read_to_string(Self::CONFIG_NAME)
            .context(format!("Failed to read config file: {}", Self::CONFIG_NAME))?;
        let config: ManaboxConfig = serde_yaml::from_str(&content)
            .context("Failed to parse YAML in manabox.yml")?;
        Ok(config)
    }

    pub fn save_default() -> Result<()> {
        let default_content = r#"file: ["target/", "node_modules/"]
must: ["Cargo.toml", "Cargo.lock"]
select: ["README.md"]
"#;
        fs::write(Self::CONFIG_NAME, default_content)
            .context("Failed to create manabox.yml")
    }
}

pub fn init_mana(_name: &Option<String>) -> Result<()> {
    // 1. ディレクトリ作成
    if !Path::new(".mana").exists() {
        fs::create_dir_all(".mana/objects")?;
        fs::create_dir_all(".mana/storage/main")?;
        fs::write(".mana/now", "main")?;
        println!("✨ Created .mana directory.");
    }

    // 2. manabox.yml 作成
    if !Path::new(ManaboxConfig::CONFIG_NAME).exists() {
        ManaboxConfig::save_default()?;
        println!("📄 Created {}", ManaboxConfig::CONFIG_NAME);
    } else {
        println!("✋ {} already exists.", ManaboxConfig::CONFIG_NAME);
    }
    Ok(())
}

pub fn calculate_hash(path: &Path) -> Result<String> {
    let mut content = fs::read(path)
        .context(format!("Failed to read file: {:?}", path))?;

    // 末尾の空白・改行をトリミング（バイナリのまま処理）
    while let Some(&last) = content.last() {
        if last == b'\n' || last == b'\r' || last == b' ' {
            content.pop();
        } else {
            break;
        }
    }

    if content.is_empty() {
        // 空ファイルの場合は特定のハッシュではなくエラーにする選択もアリだが、
        // ここでは空文字のハッシュを許容せず、明示的に空であることを返すかエラーにする
        anyhow::bail!("File content is empty (or only whitespace): {:?}", path);
    }

    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(hex::encode(hasher.finalize()))
}

pub fn scan_workspace(config: &ManaboxConfig) -> Result<FileSnapshot> {
    let mut files = HashMap::new();
    for file_name in &config.must {
        let path = Path::new(file_name);
        if path.exists() {
            let hash = calculate_hash(path)?;
            files.insert(file_name.clone(), hash);
        }
    }
    Ok(FileSnapshot { files })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_scan_workspace_basic() -> Result<()> {
        // 1. 一時ディレクトリを作成
        let temp = tempfile::tempdir()?;
        let temp_path = temp.path();

        // 2. 一時ディレクトリの中にテストファイルを作る (絶対パスで管理)
        let file_name = "hello.txt";
        let file_full_path = temp_path.join(file_name);
        fs::write(&file_full_path, b"hello mana")?;

        // 3. 設定ファイル。ここには「ファイル名」だけを入れる
        let config = ManaboxConfig {
            file: vec![],
            must: vec![file_name.to_string()],
            select: vec![],
        };

        // 4. スキャン実行。
        // ※scan_workspaceがカレントディレクトリを見る仕様なら、
        // 関数内で結合するか、一時的にディレクトリを移動する
        let snapshot = {
            let previous_dir = std::env::current_dir()?;
            std::env::set_current_dir(temp_path)?;
            let res = scan_workspace(&config);
            std::env::set_current_dir(previous_dir)?; // 元に戻す
            res?
        };

        // 5. 検証
        // ここで .get().expect() を使うことで、1548... が捏造される隙を与えない
        let actual_hash = snapshot.files.get(file_name)
            .context(format!("File [{}] not found in snapshot. Keys: {:?}", file_name, snapshot.files.keys()))?;

        let expected_hash = "274a7732296c09819970921a8d0034606f2e8f19293114d2e057388716399676";
        assert_eq!(actual_hash, expected_hash);

        Ok(())
    }
}