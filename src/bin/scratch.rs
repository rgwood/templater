use std::{
    fs,
    path::PathBuf,
};

use anyhow::Result;

fn main() -> Result<()> {
    let templates = all_templates()?;

    for template in templates {
        // god, dealing with OsStrings is annoying
        // println!("{}", template.to_str().unwrap());
        println!("{}", template.file_name().unwrap().to_str().unwrap());
    }

    Ok(())
}

fn all_templates() -> Result<Vec<PathBuf>> {
    let home_dir = home::home_dir().expect("could not get home dir");
    let template_dir = home_dir.join("dotfiles/templates");

    let templates: Vec<PathBuf> = fs::read_dir(template_dir)?
        .filter_map(|t| t.ok())
        .filter(|t| {
            if let Ok(file_type) = t.file_type() {
                return file_type.is_file();
            }
            false
        })
        .map(|t| t.path())
        .collect();
    Ok(templates)
}
