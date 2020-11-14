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

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = read_config_file("slack-github-config.toml")?;
    let contributions = github::query_contributions(&config.github).await?;
    slack::send_contribution_message(&config.slack, &contributions).await?;

    Ok(())
}

fn read_config_file(filename: &str) -> Result<Config, Error> {
    let config_file_contents = fs::read_to_string(filename)?;
    let config : Config = toml::from_str(&config_file_contents)?;
    Ok(config)
}