use std::{collections::HashMap, env::current_dir, fs, path::PathBuf};

use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input};
use handlebars::{template::Template, Handlebars};

fn main() -> Result<()> {
    let mut variables = default_variables()?;

    let templates = all_templates()?;

    let template_names: Vec<&str> = templates
        .iter()
        .map(|t| t.file_name().unwrap().to_str().unwrap())
        .collect();

    let selection_index = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick a template")
        .default(0)
        .items(&template_names)
        .interact()
        .expect("Failed to get user input");

    let selected_template = &templates[selection_index];
    let template_string = fs::read_to_string(selected_template)?;

    let file_header = get_header(&template_string);
    let template_string = without_header(&template_string);

    let original_file_name = selected_template
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let suggested_file_name = file_header.get("filename").unwrap_or(&original_file_name);

    let new_file_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What name should the new file be?")
        .with_initial_text(suggested_file_name)
        .interact_text()
        .expect("failed to get file name");

    let template = Template::compile(&template_string).unwrap();

    for element in template.elements {
        if let handlebars::template::TemplateElement::Expression(e) = element {
            let name = e.name.as_name().expect("could not get name");
            if !variables.contains_key(name) {
                let msg = format!(
                    "Variable '{name}' found in template but not set. What should it be set to?"
                );
                let value: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt(msg)
                    .allow_empty(true)
                    .interact_text()?;

                variables.insert(name.to_string(), value);
            }
        }
    }

    let rendered_template = Handlebars::new().render_template(&template_string, &variables)?;
    println!("{rendered_template}");

    let output_path = current_dir()?.join(new_file_name);
    fs::write(output_path, rendered_template)?;

    Ok(())
}

fn default_variables() -> Result<HashMap<String, String>> {
    let mut variables = HashMap::<String, String>::new();
    let current_dir = current_dir()?;
    let current_dir_string = current_dir.to_string_lossy().to_string();
    variables.insert("pwd".to_string(), current_dir_string.clone());
    variables.insert("current_dir_path".to_string(), current_dir_string);
    variables.insert(
        "current_dir_name".to_string(),
        current_dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
    );

    #[cfg(target_os = "windows")]
    variables.insert("windows".to_string(), "true".to_string());
    #[cfg(target_os = "linux")]
    variables.insert("linux".to_string(), "true".to_string());
    #[cfg(target_os = "macos")]
    variables.insert("macos".to_string(), "true".to_string());

    Ok(variables)
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

fn without_header(file_contents: &str) -> String {
    let mut result = String::new();

    let mut past_header = false;

    for line in file_contents.lines() {
        let trimmed = line.trim();

        let is_header_line = trimmed.is_empty() || trimmed.contains("templater.");
        if !past_header && !is_header_line {
            past_header = true;
        }

        if past_header {
            result.push_str(line);
            result.push_str("\n");
        }
    }

    result
}

fn get_header(file_contents: &str) -> HashMap<String, String> {
    let raw = get_raw_header(file_contents);

    if let Some(raw) = raw {
        if let Ok(parsed) = parse_header(&raw) {
            return parsed;
        }
    }

    HashMap::new()
}

fn get_raw_header(file_contents: &str) -> Option<Vec<String>> {
    let mut ret = Vec::<String>::new();

    for line in file_contents.lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        if line.starts_with("# templater") || line.starts_with("#templater") {
            ret.push(line.to_string());
        }
    }

    if ret.is_empty() {
        None
    } else {
        Some(ret)
    }
}

fn parse_header(header: &Vec<String>) -> Result<HashMap<String, String>> {
    let mut variables = HashMap::<String, String>::new();

    for line in header {
        let line = line.trim();

        // ex: # templater.filename = index.ts
        let re = regex::Regex::new(r"# ?templater\.(\w*) ?= ?(.*)").unwrap();

        // usually there will only be 1
        for cap in re.captures_iter(line) {
            variables.insert(cap[1].to_string(), cap[2].to_string());
        }
    }
    Ok(variables)
}

#[test]
fn test_parse_filename() {
    let header = get_header(
        "
    # templater.filename = index.ts
    foo bar",
    );

    assert_eq!(header.get("filename").unwrap(), "index.ts");
}

#[test]
fn test_without_header() {
    let without_header = without_header(
        "
    # templater.filename = index.ts
    foo bar",
    );

    assert_eq!(without_header.trim(), "foo bar");
}
