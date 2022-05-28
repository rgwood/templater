use std::{collections::HashMap, path::PathBuf, fs};

use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input};
use handlebars::{template::Template, Handlebars};

fn main() -> Result<()> {
    let mut variables = HashMap::new();
    variables.insert(
        "name".to_string(),
        "Reilly".to_string(),
    );

    let templates = all_templates()?;


    let template_names: Vec<&str> = templates.iter()
        .map(|t| t.file_name().unwrap().to_str().unwrap())
        .collect();

    let selection_index = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick a template")
        .default(0)
        .items(&template_names)
        .interact()
        .expect("Failed to get user input");

    let selection = &templates[selection_index];

    let original_file_name = selection.file_name().unwrap();

    let file_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What name should the new file be?")
        // .default(selection)
        .with_initial_text(original_file_name.to_string_lossy())
        .interact_text()
        .unwrap();

    eprintln!("File name: {file_name}");



    let template_string = "Hello {{name}}";


    let template = Template::compile(template_string).unwrap();
    // TODO: look at elements in template
    println!("{}", Handlebars::new().render_template(template_string, &variables)?);







    for element in template.elements {
        match element {
            handlebars::template::TemplateElement::Expression(e) => {
                let name = &e.name.as_name();
                dbg!(name);
            }
            _ => {}
        }
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
        .map(|t| t.path()).collect();
    Ok(templates)
}
