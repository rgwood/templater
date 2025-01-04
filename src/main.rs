// mod clipboard;
mod utils;

use anyhow::Result;
use clap::{Parser, Subcommand};
use clipboard_anywhere::{get_clipboard, set_clipboard};
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input};
use handlebars::{template::Template, Handlebars};
use std::{
    collections::HashMap,
    env::current_dir,
    fs::{self, read_to_string},
    path::{Path, PathBuf},
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
    /// Generate file(s) from a template (default)
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
    let mut variables = default_variables()?;
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

    let snippet_template = Template::compile(&selected_snippet.contents).unwrap();
    for element in snippet_template.elements {
        if let handlebars::template::TemplateElement::Expression(e) = element {
            let name = e.name.as_name().expect("could not get name");

            if name == "clipboard_contents" && !variables.contains_key(name) {
                let clipboard_contents = match get_clipboard() {
                    Ok(s) => s,
                    Err(_) => Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("Could not get clipboard contents. Enter manually:")
                        .allow_empty(true)
                        .interact_text()?,
                };
                variables.insert(name.to_string(), clipboard_contents);
            }

            if !variables.contains_key(name) {
                let msg = format!(
                    "Variable '{name}' found in snippet but not set. What should it be set to?"
                );
                let value: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt(msg)
                    .allow_empty(true)
                    .interact_text()?;

                variables.insert(name.to_string(), value);
            }
        }
    }
    let rendered_template =
        Handlebars::new().render_template(&selected_snippet.contents, &variables)?;

    // remove trailing newline so we can paste shell 1-liners without executing
    let mut trimmed_contents = rendered_template;
    if trimmed_contents.ends_with('\n') {
        trimmed_contents.pop();
        if trimmed_contents.ends_with('\r') {
            trimmed_contents.pop();
        }
    }

    set_clipboard(&trimmed_contents)?;
    println!("Copied snippet '{}' to clipboard:", selected_snippet.name);
    println!("{}", trimmed_contents);

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
            contents,
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
    let file_header = get_header(&template_string);

    if verbose {
        dbg!(&file_header);
    }

    let template_string = without_header(&template_string);

    let output_dir: PathBuf = match file_header.get("output_dir") {
        Some(dir) => {
            let ret = expand_home_dir(dir);
            if verbose {
                dbg!(&ret);
            }
            ret
        }
        None => current_dir()?,
    };

    let file_name = file_header.get("filename").cloned().unwrap_or_else(|| {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Provide a file name")
            .interact_text()
            .expect("failed to get file name")
    });

    let file_already_exists = Path::exists(&output_dir.join(&file_name));

    if file_already_exists {
        let overwrite = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("File '{file_name}' already exists. Overwrite?"))
            .default(false)
            .wait_for_newline(true)
            .interact()?;

        if !overwrite {
            return Ok(());
        }
    }

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
    let output_path = output_dir.join(file_name);
    fs::write(&output_path, rendered_template)?;
    println!("Wrote '{}' to disk", output_path.to_string_lossy());

    // Check if the file should be set as executable
    if let Some(set_executable) = file_header.get("set_executable") {
        if set_executable.to_lowercase() == "true" {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&output_path)?.permissions();
                perms.set_mode(0o755); // rwxr-xr-x
                fs::set_permissions(&output_path, perms)?;
                println!("Set '{}' as executable", output_path.to_string_lossy());
            }
            #[cfg(not(unix))]
            {
                println!(
                    "Warning: Setting file as executable is only supported on Unix-like systems"
                );
            }
        }
    }
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
