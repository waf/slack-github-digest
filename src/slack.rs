use failure::Error;
use std::collections::{HashMap};
use itertools::Itertools;
use serde_derive::Deserialize;
use reqwest;
use crate::github::*;

#[derive(Deserialize)]
pub struct SlackConfig {
    pub hook_url: String,
    pub username: String,
    pub channel: String,
    pub icon: String
}

pub async fn send_contribution_message(config: &SlackConfig, contributions: &Vec<ContributionSummary>) -> Result<(), Error> {

    let message = build_message(contributions);

    let mut json_message : HashMap<&str, &str> = HashMap::new();
    json_message.insert("text", &message);
    json_message.insert("username", &config.username);
    json_message.insert("channel", &config.channel);
    json_message.insert("icon_emoji", &config.icon);
    json_message.insert("unfurl_links", "false");

    print!("{}", message);

    let client = reqwest::Client::new();
    client.post(&config.hook_url)
        .json(&json_message)
        .send()
        .await?;

    Ok(())
}

fn build_message(contributions: &Vec<ContributionSummary>) -> String {
    /*
        Weekly Open Source Contributions!
        - Joe created the repos Foo, Boom and Bar; pushed 5 commits Foo and 2 commits to Bar; helped out with issues in Bor (#2), and worked on PRs in Foo (#5, #7)
        - Bob created the repos Foo and Bar, pushed commits to Foo and Bar, and helped out with issues in Bor.
    */

    format!("Recent Open Source Contributions!\n{}",
        contributions
            .into_iter()
            .map(|c| build_contributions(&c))
            .join("")
    )
}

fn build_contributions(contribution: &ContributionSummary) -> String {

    let mut results = vec![];

    if contribution.new_repos.len() > 0 {
        results.push(build_repo_contributions(&contribution.new_repos));
    }
    if contribution.commits.len() > 0 {
        results.push(build_commit_contributions(&contribution.commits));
    }
    if contribution.pull_requests.len() > 0 {
        results.push(build_pull_request_contributions(&contribution.pull_requests));
    }
    if contribution.issues.len() > 0 {
        results.push(build_issue_contributions(&contribution.issues));
    }

    if results.len() == 0 {
        return "".to_owned();
    }

    format!("- {name} {contribution}.\n", name = contribution.name, contribution = results.join("; "))
}

fn build_repo_contributions(new_repos: &Vec<Repo>) -> String {
    let mut new_repo_text = String::new();
    let text = format!("created the repo{s} ", 
        s = if new_repos.len() == 1 { "" } else { "s" }
    );
    new_repo_text.push_str(&text);
    new_repo_text.push_str(&comma_separate_list(new_repos.iter().map(link_repo).collect()));
    new_repo_text
}

fn build_commit_contributions(commits: &Vec<Commit>) -> String {
    let mut commit_text = String::new();
    commit_text.push_str("pushed commits to ");
    commit_text.push_str(&comma_separate_list(
        commits.iter()
            .map(|c| link_repo(&c.repo))
            .collect::<Vec<_>>()
        )
    );
    commit_text
}

fn build_pull_request_contributions(pull_requests: &Vec<RepoPullRequests>) -> String {
    let mut pr_text = String::new();
    pr_text.push_str(&"contributed to PRs in ");
    pr_text.push_str(&comma_separate_list(
        pull_requests.iter()
            .map(|c|
                format!("{repo} ({prs})",
                    repo = link_repo(&c.repo),
                    prs = comma_separate_list(
                        c.pull_requests.iter()
                        .map(|pr| link(&pr.url, &format!("#{}", pr.number)))
                        .collect()
                    )
                )
            )
            .collect()
    ));
    pr_text
}

fn build_issue_contributions(issues: &Vec<RepoIssues>) -> String {
    let mut issue_text = String::new();
    issue_text.push_str("helped out with issues in ");
    issue_text.push_str(&comma_separate_list(
        issues.iter()
            .map(|c|
                format!("{repo} ({issues})",
                    repo = link_repo(&c.repo),
                    issues = comma_separate_list(
                        c.issues.iter()
                        .map(|issue| link(&issue.url, &format!("#{}", issue.number)))
                        .collect()
                    )
                )
            )
            .collect()
    ));
    issue_text
}

fn comma_separate_list(words: Vec<String>) -> String {
    let mut separated = String::new();
    if words.len() == 1 {
        separated.push_str(&words[0]);
    } else if let Some((last, rest)) = words.split_last() {
        separated.push_str(&rest.iter().join(", "));
        separated.push_str(" and ");
        separated.push_str(last);
    } 
    separated
}

fn link(url: &str, text: &str) -> String{
    format!("<{url}|{text}>", url = url, text = text)
}

fn link_repo(repo: &Repo) -> String {
    link(&repo.url, &repo.name)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comma_separate_list() {
        assert_eq!(comma_separate_list(vec!["foo".to_owned()]), "foo");
        assert_eq!(comma_separate_list(vec!["foo".to_owned(), "bar".to_owned()]), "foo and bar");
        assert_eq!(comma_separate_list(vec!["foo".to_owned(), "bar".to_owned(), "baz".to_owned()]), "foo, bar and baz");
    }
}