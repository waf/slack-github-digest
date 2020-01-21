use failure::Error;
use chrono::{Utc, Duration};
use serde_derive::Deserialize;
use graphql_client::{GraphQLQuery, Response};
use reqwest;

pub fn query_contributions(config: &GitHubConfig) -> Result<Vec<ContributionSummary>, Error> {

    let start_date : DateTime = (Utc::now() + Duration::days(-1 * config.report_days_in_past))
        .format("%Y-%m-%dT00:00:00")
        .to_string();

    let query = FollowerDigest::build_query(follower_digest::Variables {
        from: start_date
    });

    let client = reqwest::Client::new();
    let mut response = client
        .post("https://api.github.com/graphql")
        .bearer_auth(&config.token)
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

    Ok(followers)
}

fn format_follower_accomplishments(follower: &follower_digest::FollowerDigestViewerFollowersNodes) -> ContributionSummary {
    /*
        Weekly Open Source Contributions!
        - Kenji-vn created the repos Foo and Bar; pushed 5 commits Foo, 2 commits to Bar; helped out with issues in Bor (#2), and worked on PRs in Foo (#5)
        - Aaron Amm created the repos Foo and Bar, pushed commits to Foo and Bar, and helped out with issues in Bor.
    */

    let repository_contributions = &follower.contributions_collection.repository_contributions.nodes;
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

    let commits = map_commits(&follower.contributions_collection.commit_contributions_by_repository);
    let issues = map_issues(&follower.contributions_collection.issue_contributions_by_repository);

    let mut pull_requests = map_pull_requests(&follower.contributions_collection.pull_request_contributions_by_repository);
    let reviews = map_pull_request_reviews(&follower.contributions_collection.pull_request_review_contributions_by_repository);
    pull_requests.extend(reviews);

    let name = if let Some(full_name) = &follower.name {
        full_name
    } else {
        &follower.login
    };

    ContributionSummary {
        name: name.to_owned(),
        new_repos: new_repos,
        forked_repos: forked_repos,
        commits: commits,
        issues: issues,
        pull_requests: pull_requests,
    }
}

fn map_commits(commits: &std::vec::Vec<follower_digest::FollowerDigestViewerFollowersNodesContributionsCollectionCommitContributionsByRepository>) -> Vec<Commit> {
    commits
        .into_iter()
        .map(|c| Commit {
            count: c.contributions.total_count,
            repo: map_repo(&c.repository.repo)
        })
        .collect::<Vec<_>>()
}

fn map_issues(issues: &std::vec::Vec<follower_digest::FollowerDigestViewerFollowersNodesContributionsCollectionIssueContributionsByRepository>) -> Vec<RepoIssues> {
    issues
        .into_iter()
        .map(|i| RepoIssues {
            issues: i.contributions.nodes.as_ref().unwrap_or(&vec![]).into_iter().map(|node| ContributionLink {
                number: node.as_ref().unwrap().issue.issue.number,
                url: node.as_ref().unwrap().issue.issue.url.to_owned()
            }).collect::<Vec<_>>(),
            repo: map_repo(&i.repository.repo)
        })
        .collect::<Vec<_>>()
}

fn map_pull_requests(pull_requests: &std::vec::Vec<follower_digest::FollowerDigestViewerFollowersNodesContributionsCollectionPullRequestContributionsByRepository>) -> Vec<RepoPullRequests> {
    pull_requests
        .into_iter()
        .map(|i| RepoPullRequests {
            pull_requests: i.contributions.nodes.as_ref().unwrap_or(&vec![]).into_iter().map(|node| ContributionLink {
                number: node.as_ref().unwrap().pull_request.pr.number,
                url: node.as_ref().unwrap().pull_request.pr.url.to_owned()
            }).collect::<Vec<_>>(),
            repo: map_repo(&i.repository.repo)
        })
        .collect::<Vec<_>>()
}

fn map_pull_request_reviews(reviews: &std::vec::Vec<follower_digest::FollowerDigestViewerFollowersNodesContributionsCollectionPullRequestReviewContributionsByRepository>) -> Vec<RepoPullRequests> {
    reviews
        .into_iter()
        .map(|i| RepoPullRequests {
            pull_requests: i.contributions.nodes.as_ref().unwrap_or(&vec![]).into_iter().map(|node| ContributionLink {
                number: node.as_ref().unwrap().pull_request.pr.number,
                url: node.as_ref().unwrap().pull_request.pr.url.to_owned()
            }).collect::<Vec<_>>(),
            repo: map_repo(&i.repository.repo)
        })
        .collect::<Vec<_>>()
}

fn map_repo(repo: &follower_digest::repo) -> Repo {
    Repo {
        is_fork: repo.is_fork,
        name: repo.name.to_owned(),
        url: repo.url.to_owned(),
        language: repo.primary_language.as_ref().map_or("unknown".to_owned(), |l| l.name.to_owned())
    }
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


#[derive(Deserialize)]
pub struct GitHubConfig {
    pub token: String,
    pub report_days_in_past: i64
}

#[derive(Debug)]
pub struct ContributionSummary
{
    pub name: String,
    pub new_repos: Vec<Repo>,
    pub forked_repos: Vec<Repo>,
    pub commits: Vec<Commit>,
    pub issues: Vec<RepoIssues>,
    pub pull_requests: Vec<RepoPullRequests>
}

#[derive(Debug)]
pub struct Repo
{
    pub name: String,
    pub url: String,    
    pub language: String,
    pub is_fork: bool,
}

#[derive(Debug)]
pub struct Commit
{
    pub count: i64,
    pub repo: Repo,    
}

#[derive(Debug)]
pub struct RepoIssues
{
    pub repo: Repo,    
    pub issues: Vec<ContributionLink>
}

#[derive(Debug)]
pub struct RepoPullRequests
{
    pub repo: Repo,    
    pub pull_requests: Vec<ContributionLink>
}

#[derive(Debug)]
pub struct ContributionLink
{
    pub number: i64,
    pub url: String
}
