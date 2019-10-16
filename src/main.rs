use graphql_client::{GraphQLQuery, Response};
use reqwest;
use failure::Error;
use serde_derive::Deserialize;
use itertools::Itertools;
use std::fs;
use std::collections::{HashMap};
use chrono::{Utc, Duration};

#[derive(Deserialize)]
struct Config {
    github: GitHubConfig,
    slack: SlackConfig,
}
#[derive(Deserialize)]
struct GitHubConfig {
    token: String,
    report_days_in_past: i64
}
#[derive(Deserialize)]
struct SlackConfig {
    hook_url: String,
    username: String,
    channel: String,
    icon: String
}


type DateTime = String;
type URI = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/queries/schema.graphql",
    query_path = "src/queries/digest.graphql",
    response_derives = "Debug"
)]
pub struct FollowerDigest;

fn main() -> Result<(), Error> {
    let config_file_contents = fs::read_to_string("slack-github-config.toml")
        .expect("Could not find config file");
    let config : Config = toml::from_str(&config_file_contents)
        .expect("Could not parse config file");

    let start_date : DateTime = (Utc::now() + Duration::days(-1 * config.github.report_days_in_past))
        .format("%Y-%m-%dT00:00:00")
        .to_string();
    let query = FollowerDigest::build_query(follower_digest::Variables {
        from: start_date
    });

    let client = reqwest::Client::new();
    let mut response = client
        .post("https://api.github.com/graphql")
        .bearer_auth(config.github.token)
        .json(&query)
        .send()?;
    
    let json: Response<follower_digest::ResponseData> = response.json()?;
    println!("{:#?}", json);

    let followers = json.data
        .expect("missing data")
        .viewer
        .followers
        .nodes
        .expect("missing followers")
        .into_iter()
        .map(|follower| format_follower_accomplishments(&follower.expect("empty follower")))
        .collect::<Vec<_>>();

    println!("{:?}", followers);

    let message = build_message(&followers);

    let mut json_message = HashMap::new();
    json_message.insert("text", message);
    json_message.insert("username", config.slack.username);
    json_message.insert("channel", config.slack.channel);
    json_message.insert("icon_emoji", config.slack.icon);
    json_message.insert("unfurl_links", false.to_string());

    client.post(&config.slack.hook_url)
        .json(&json_message)
        .send()?;
    
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

fn build_contributions(contribution: &ContributionSummary) -> String {

    let mut results = vec![];

    if contribution.new_repos.len() > 0 {
        let mut new_repo_text = String::new();
        let text = format!("created the repo{s} ", 
            s = if contribution.new_repos.len() == 1 { "" } else { "s" }
        );
        new_repo_text.push_str(&text);
        new_repo_text.push_str(&comma_separate_list(contribution.new_repos.iter().map(link_repo).collect()));
        results.push(new_repo_text);
    }

    if contribution.commits.len() > 0 {
        let mut commit_text = String::new();
        commit_text.push_str("pushed ");
        commit_text.push_str(&comma_separate_list(
            contribution.commits.iter()
                .map(|c| 
                    format!("{count} commits to {repo}", count = c.count, repo = link_repo(&c.repo))
                )
                .collect::<Vec<_>>()
            )
        );
        results.push(commit_text);
    }
    if contribution.forked_repos.len() > 0 {
        let mut fork_text = String::new();
        let text = format!("forked the repo{s} ", 
            s = if contribution.forked_repos.len() == 1 { "" } else { "s" }
        );
        fork_text.push_str(&text);
        fork_text.push_str(&comma_separate_list(contribution.forked_repos.iter().map(link_repo).collect()));
        results.push(fork_text);
    }
    if contribution.pull_requests.len() > 0 {
        let mut pr_text = String::new();
        pr_text.push_str(&"contributed to PRs in ");
        pr_text.push_str(&comma_separate_list(
            contribution.pull_requests.iter()
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
        results.push(pr_text);
    }
    if contribution.issues.len() > 0 {
        let mut issue_text = String::new();
        issue_text.push_str("helped out with issues in ");
        issue_text.push_str(&comma_separate_list(
            contribution.issues.iter()
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
        results.push(issue_text);
    }

    if results.len() == 0 {
        return "".to_owned();
    }

    format!("- {name} {contribution}.\n", name = contribution.name, contribution = results.join("; "))
}

#[derive(Debug)]
struct Repo
{
    name: String,
    url: String,    
    language: String,
    is_fork: bool,
}

#[derive(Debug)]
struct Commit
{
    count: i64,
    repo: Repo,    
}

#[derive(Debug)]
struct ContributionLink
{
    number: i64,
    url: String
}
#[derive(Debug)]
struct RepoIssues
{
    repo: Repo,    
    issues: Vec<ContributionLink>
}
#[derive(Debug)]
struct RepoPullRequests
{
    repo: Repo,    
    pull_requests: Vec<ContributionLink>
}

#[derive(Debug)]
struct ContributionSummary
{
    name: String,
    new_repos: Vec<Repo>,
    forked_repos: Vec<Repo>,
    commits: Vec<Commit>,
    issues: Vec<RepoIssues>,
    pull_requests: Vec<RepoPullRequests>
}

fn format_follower_accomplishments(follower: &follower_digest::FollowerDigestViewerFollowersNodes) -> ContributionSummary {
    /*
        Weekly Open Source Contributions!
        - Kenji-vn created the repos Foo and Bar; pushed 5 commits Foo, 2 commits to Bar; helped out with issues in Bor (#2), and worked on PRs in Foo (#5)
        - Aaron Amm created the repos Foo and Bar, pushed commits to Foo and Bar, and helped out with issues in Bor.
    */

    let repository_contributions = &follower.contributions_collection.repository_contributions.nodes;
    let commit_contributions = &follower.contributions_collection.commit_contributions_by_repository;
    let issue_contributions = &follower.contributions_collection.issue_contributions_by_repository;
    let pull_request_contributions = &follower.contributions_collection.pull_request_contributions_by_repository;
    let review_contributions = &follower.contributions_collection.pull_request_review_contributions_by_repository;
    
    let repos = if let Some(nodes) = repository_contributions {
        nodes.into_iter()
            .map(|r| map_repo(&r.as_ref().unwrap().repository.repo))
            .collect::<Vec<_>>()
    } else {
        vec![]
    };

    let (forked_repos, new_repos) = repos
        .into_iter()
        .partition(|r| r.is_fork);

    let commits = commit_contributions
        .into_iter()
        .map(|c| Commit {
            count: c.contributions.total_count,
            repo: map_repo(&c.repository.repo)
        })
        .collect::<Vec<_>>();

    let issues = issue_contributions
        .into_iter()
        .map(|i| RepoIssues {
            issues: i.contributions.nodes.as_ref().unwrap_or(&vec![]).into_iter().map(|node| ContributionLink {
                number: node.as_ref().unwrap().issue.issue.number,
                url: node.as_ref().unwrap().issue.issue.url.to_owned()
            }).collect::<Vec<_>>(),
            repo: map_repo(&i.repository.repo)
        })
        .collect::<Vec<_>>();

    let mut pull_requests = pull_request_contributions
        .into_iter()
        .map(|i| RepoPullRequests {
            pull_requests: i.contributions.nodes.as_ref().unwrap_or(&vec![]).into_iter().map(|node| ContributionLink {
                number: node.as_ref().unwrap().pull_request.pr.number,
                url: node.as_ref().unwrap().pull_request.pr.url.to_owned()
            }).collect::<Vec<_>>(),
            repo: map_repo(&i.repository.repo)
        })
        .collect::<Vec<_>>();

    let reviews = review_contributions
        .into_iter()
        .map(|i| RepoPullRequests {
            pull_requests: i.contributions.nodes.as_ref().unwrap_or(&vec![]).into_iter().map(|node| ContributionLink {
                number: node.as_ref().unwrap().pull_request.pr.number,
                url: node.as_ref().unwrap().pull_request.pr.url.to_owned()
            }).collect::<Vec<_>>(),
            repo: map_repo(&i.repository.repo)
        })
        .collect::<Vec<_>>();

    pull_requests.extend(reviews);

    ContributionSummary {
        name: follower.name.as_ref().expect("no name").to_owned(),
        new_repos: new_repos,
        forked_repos: forked_repos,
        commits: commits,
        issues: issues,
        pull_requests: pull_requests,
    }

}

fn map_repo(repo: &follower_digest::repo) -> Repo {
    Repo {
        is_fork: repo.is_fork,
        name: repo.name.to_owned(),
        url: repo.url.to_owned(),
        language: repo.primary_language.as_ref().map_or("unknown".to_owned(), |l| l.name.to_owned())
    }
}
