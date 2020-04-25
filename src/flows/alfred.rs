use crate::display;
use crate::parser;
use crate::structures::cheat::Suggestion;
use crate::structures::{error::command::BashSpawnError, option::Config};
use anyhow::Context;
use anyhow::Error;
use std::env;
use std::process::{Command, Stdio};

pub fn main(config: Config) -> Result<(), Error> {
    let mut child = Command::new("cat")
        .stdin(Stdio::piped())
        .spawn()
        .context("Unable to create child")?;
    let stdin = child.stdin.as_mut().context("Unable to get stdin")?;

    display::alfred::print_items_start(None);

    parser::read_all(&config, stdin).context("Failed to parse variables intended for finder")?;

    // make sure everything was printed to stdout before attempting to close the items vector
    let _ = child.wait_with_output().context("Failed to wait for fzf")?;

    display::alfred::print_items_end();
    Ok(())
}

fn prompt_with_suggestions(
    _variable_name: &str,
    _config: &Config,
    suggestion: &Suggestion,
) -> Result<String, Error> {
    let (suggestion_command, _suggestion_opts) = suggestion;

    let child = Command::new("bash")
        .stdout(Stdio::piped())
        .arg("-c")
        .arg(&suggestion_command)
        .spawn()
        .map_err(|e| BashSpawnError::new(suggestion_command, e))?;

    let suggestions = String::from_utf8(
        child
            .wait_with_output()
            .context("Failed to wait and collect output from bash")?
            .stdout,
    )
    .context("Suggestions are invalid utf8")?;

    Ok(suggestions)
}

pub fn suggestions(config: Config) -> Result<(), Error> {
    let mut child = Command::new("cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .context("Unable to create child")?;
    let stdin = child.stdin.as_mut().context("Unable to get stdin")?;

    let variables = parser::read_all(&config, stdin)
        .context("Failed to parse variables intended for finder")?;

    let tags = env::var("tags").context(r#"The env var "tags" isn't set"#)?;
    let snippet = env::var("snippet").context(r#"The env var "snippet" isn't set"#)?;

    let varname = display::VAR_REGEX.captures_iter(&snippet).next();

    if let Some(varname) = varname {
        let varname = &varname[0];
        let varname = &varname[1..varname.len() - 1];

        display::alfred::print_items_start(Some(varname));

        let lines = variables
            .get(&tags, &varname)
            .ok_or_else(|| anyhow!("No suggestions"))
            .and_then(|suggestion| {
                Ok(prompt_with_suggestions(&varname, &config, suggestion).unwrap())
            })?;

        let mut writer = display::alfred::new_writer();

        for line in lines.split('\n') {
            writer.write_suggestion(&snippet, &varname, &line);
        }
    } else {
        display::alfred::print_items_start(None);
    }

    display::alfred::print_items_end();

    Ok(())
}

pub fn transform() -> Result<(), Error> {
    let snippet = env::var("snippet").context(r#"The env var "snippet" isn't set"#)?;
    let varname = env::var("varname").context(r#"The env var "varname" isn't set"#)?;
    let value = env::var(&varname).context(format!(r#"The env var "{}" isn't set"#, &varname))?;

    let bracketed_varname = format!("<{}>", varname);
    let interpolated_snippet = snippet.replace(&bracketed_varname, &value);
    println!("{}", interpolated_snippet);

    Ok(())
}