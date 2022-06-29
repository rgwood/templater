#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::Result;
use handlebars::{template, Handlebars};

fn main() -> Result<()> {
    render_template();

    // let templates = all_templates()?;

    // for template in templates {
    //     // god, dealing with OsStrings is annoying
    //     // println!("{}", template.to_str().unwrap());
    //     println!("{}", template.file_name().unwrap().to_str().unwrap());
    // }

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

fn render_template() {
    let template_string = r#"{{#if windows}}on windows{{else}}not windows{{/if}} \{{ foo }} "#;

    let mut variables = HashMap::<&str, &str>::new();

    #[cfg(target_os = "windows")]
    variables.insert("windows", "true");
    #[cfg(target_os = "linux")]
    variables.insert("linux", "true");
    #[cfg(target_os = "macos")]
    variables.insert("macos", "true");

    let rendered_template = Handlebars::new()
        .render_template(template_string, &variables)
        .unwrap();
    println!("{rendered_template}");
}
