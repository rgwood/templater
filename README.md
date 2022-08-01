## Templater

A simple CLI tool for templates and code snippets. Useful when you'd rather not write out your usual makefile from scratch for the millionth time.

### Usage

Install `templater` somewhere on your path. Make sure you have some files in `~/dotfiles/templates`.

Navigate to a directory where you want to create a file from a template, then run `templater`; it will walk you through any necessary decisions.

### Templates

Templater looks for text files in `~/dotfiles/templates`. Files can use Handlebars syntax `{{ foo }}` for variables. The following variables are set by default:

- `pwd` or `current_dir_path`: working directory (full path)
- `current_dir_name`: working directory name
- `linux`/`windows`/`macos`: set to `true` depending on the current OS

If a variable in a template file is not already set, the user will be prompted to fill in a value.

Files can be given key-value headers by adding lines like this to the top:

```
# templater.filename = foo
```

### Snippets

`templater snippet` looks in `~/dotfiles/templates`. Pick a file and copy its contents to the CLI.

Snippets can optionally use a `clipboard_contents` variable:
```
wget {{clipboard_contents}}
```

### To Do

 - [ ] Support overwriting files (after prompting the user)
 - [ ] Fuzzy-find template+snippet contents, not just name
 - [ ] Display template+snippet contents in a preview pane (may require replacing dialoguer)
 - [ ] Add `clipboard_url` snippet variable that warns if clipboard contents are not a URL
