mod clipboard;
mod utils;

use crate::clipboard::set_clipboard;
use anyhow::Result;
use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input};
use handlebars::{template::Template, Handlebars};
use std::{
    collections::HashMap,
    env::current_dir,
    fs::{self, read_to_string},
    path::PathBuf,
};
use utils::expand_home_dir;

/// Reilly's custom templating/snippet tool
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(short, long, value_parser, default_value_t = false)]
    verbose: bool,

    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate file(s) from a template (default subcommand)
    Template,
    /// Copy a snippet to the clipboard
    Snippet,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(command) = &cli.command {
        match command {
            Commands::Template => template_command(&cli)?,
            Commands::Snippet => snippet_command(&cli)?,
        }
    } else {
        template_command(&cli)?;
    }

    Ok(())
}

fn template_command(cli: &Cli) -> Result<()> {
    let mut variables = default_variables()?;

    let templates = get_templates(template_dir())?;

    let template_names: Vec<String> = templates
        .iter()
        .map(|t| {
            let file_name = t.path().file_name().unwrap().to_string_lossy();
            match t {
                TemplateItem::File { .. } => file_name.to_string(),
                TemplateItem::FileCollection { files, .. } => {
                    format!("{} ({} files)", file_name, files.len())
                }
            }
        })
        .collect();

    let selection_index = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick a template")
        .default(0)
        .items(&template_names)
        .interact()
        .expect("Failed to get user input");

    let selected_template = &templates[selection_index];

    match selected_template {
        TemplateItem::File { path } => {
            write_item_to_disk_interactive(path, &mut variables, cli.verbose)?;
        }
        TemplateItem::FileCollection { files, .. } => {
            for file in files {
                write_item_to_disk_interactive(file, &mut variables, cli.verbose)?;
            }
        }
    }

    Ok(())
}

fn snippet_command(cli: &Cli) -> Result<()> {
    let snippets = get_snippets(snippet_dir())?;
    if cli.verbose {
        dbg!(&snippets);
    }

    let snippet_names: Vec<String> = snippets.iter().map(|s| s.name.clone()).collect();

    let selection_index = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick a snippet")
        .default(0)
        .items(&snippet_names)
        .interact()
        .expect("Failed to get user input");

    let selected_snippet = &snippets[selection_index];
    set_clipboard(&selected_snippet.contents)?;
    println!("Copied snippet '{}' to clipboard", selected_snippet.name);

    Ok(())
}

fn get_snippets(snippet_dir: PathBuf) -> Result<Vec<Snippet>> {
    let files: Vec<_> = fs::read_dir(snippet_dir)?.filter_map(|t| t.ok()).collect();

    let mut snippets: Vec<Snippet> = Vec::new();

    for file in files {
        let path = file.path();
        let file_name = path.file_name().unwrap().to_string_lossy();
        let contents = read_to_string(&path)?;
        snippets.push(Snippet {
            name: file_name.into_owned(),
            contents: contents,
        });
    }

    Ok(snippets)
}

fn write_item_to_disk_interactive(
    template_item_path: &PathBuf,
    variables: &mut HashMap<String, String>,
    verbose: bool,
) -> Result<(), anyhow::Error> {
    let template_string = fs::read_to_string(template_item_path)?;
    dbg!(&template_string);

    let file_header = get_header(&template_string);

    if verbose {
        dbg!(&file_header);
    }

    let template_string = without_header(&template_string);
    let original_file_name = template_item_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let output_dir: PathBuf = match file_header.get("output_dir") {
        Some(dir) => {
            let ret = expand_home_dir(dir.into());
            println!("asdfas");
            if verbose {
                dbg!(&ret);
            }

            ret
        }
        None => current_dir()?,
    };

    let suggested_file_name = file_header.get("filename").unwrap_or(&original_file_name);

    // Start getting user input
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
    let output_path = output_dir.join(new_file_name);
    fs::write(&output_path, rendered_template)?;
    println!("Wrote {output_path:?} to disk");
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

fn template_dir() -> PathBuf {
    let home_dir = home::home_dir().expect("could not get home dir");
    home_dir.join("dotfiles/templates")
}

fn snippet_dir() -> PathBuf {
    let home_dir = home::home_dir().expect("could not get home dir");
    home_dir.join("dotfiles/snippets")
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
            result.push('\n');
        }
    }

    result
}

fn get_header(file_contents: &str) -> HashMap<String, String> {
    let raw = get_raw_header(file_contents);

    if let Some(raw) = raw {
        if let Ok(parsed) = parse_header(&raw) {
            return parsed;
        } else {
            println!("Failed to parse header");
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
fn test_parse_arbitrary_headers() {
    let header = get_header(
        "
    # templater.foo = 1
    # templater.bar = baz
    foo bar",
    );

    assert_eq!(header.get("foo").unwrap(), "1");
    assert_eq!(header.get("bar").unwrap(), "baz");
}

#[test]
fn test_parse_headers() {
    let header = get_header(
        "
        # templater.output_dir = ~/foo/bar
        # templater.filename = foo
    foo",
    );

    dbg!(&header);

    assert_eq!(header.len(), 2);

    assert_eq!(header.get("output_dir").unwrap(), "~/foo/bar");
    assert_eq!(header.get("filename").unwrap(), "foo");
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

#[test]
fn test_get_files() -> Result<()> {
    let template_dir = "test-templates";

    let templates = get_templates(template_dir)?;
    assert_eq!(templates.len(), 2);

    assert_eq!(
        templates,
        vec![
            TemplateItem::FileCollection {
                directory_path: "test-templates/bar".into(),
                files: vec![
                    "test-templates/bar/baz.c".into(),
                    "test-templates/bar/makefile".into(),
                ]
            },
            TemplateItem::File {
                path: "test-templates/foo.txt".into()
            },
        ]
    );

    Ok(())
}

fn get_templates<P>(template_dir: P) -> Result<Vec<TemplateItem>>
where
    P: AsRef<std::path::Path>,
{
    let files: Vec<_> = fs::read_dir(template_dir)?.filter_map(|t| t.ok()).collect();
    let mut templates: Vec<_> = files
        .iter()
        .map(|f| -> TemplateItem {
            if let Ok(file_type) = f.file_type() {
                if file_type.is_dir() {
                    let mut files: Vec<PathBuf> = fs::read_dir(f.path())
                        .expect("could not read dir")
                        .filter_map(|t| t.ok())
                        .map(|f| f.path())
                        .collect();
                    files.sort();
                    return TemplateItem::FileCollection {
                        directory_path: f.path(),
                        files,
                    };
                }
            }

            TemplateItem::File { path: f.path() }
        })
        .collect();

    templates.sort_by(|a, b| a.path().cmp(b.path()));

    Ok(templates)
}

#[derive(Debug)]
enum TemplateItem {
    File {
        path: PathBuf,
    },
    FileCollection {
        directory_path: PathBuf,
        files: Vec<PathBuf>,
    },
}

#[derive(Debug)]
struct Snippet {
    name: String,
    contents: String,
}

impl PartialEq for TemplateItem {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::File { path: l_path }, Self::File { path: r_path }) => l_path == r_path,
            (
                Self::FileCollection {
                    directory_path: l_directory_path,
                    files: l_files,
                },
                Self::FileCollection {
                    directory_path: r_directory_path,
                    files: r_files,
                },
            ) => l_directory_path == r_directory_path && l_files == r_files,
            _ => false,
        }
    }
}

impl TemplateItem {
    fn path(&self) -> &PathBuf {
        match self {
            TemplateItem::File { path } => path,
            TemplateItem::FileCollection { directory_path, .. } => directory_path,
        }
    }
}
