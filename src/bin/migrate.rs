use std::fs;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let old_path = Path::new("shared-memory.json");
    let new_dir = Path::new("storage/default");
    let new_path = new_dir.join("shared.json");

    if !old_path.exists() {
        println!("Old memory file not found. Skipping migration.");
        return Ok(());
    }

    fs::create_dir_all(new_dir)?;
    fs::copy(old_path, &new_path)?;
    
    println!("Successfully migrated {} to {:?}", old_path.display(), new_path);
    Ok(())
}
