pub mod github;
pub mod slack;

use failure::Error;
use serde_derive::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct Config {
    github: github::GitHubConfig,
    slack: slack::SlackConfig,
}


fn main() -> Result<(), Error> {
    let config_file_contents = fs::read_to_string("slack-github-config.toml")
        .expect("Could not find config file");
    let config : Config = toml::from_str(&config_file_contents)
        .expect("Could not parse config file");

    let contributions = github::query_contributions(&config.github)?;

    slack::send_contribution_message(&config.slack, &contributions);
    
    Ok(())
}