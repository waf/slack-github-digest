# Slack Github Digest

Post a digest of your GitHub followers' open-source contributions to slack! Written in Rust.

*Current Status: works, but not much testing has been done. Needs error handling and code organization!*

1. Generate a GitHub access token with read-only permissions on public repos.
1. Generate a Slack "Incoming Webhook" url.
1. Copy the example file `slack-github-config-example.toml` to `slack-github-config.toml`. Add both the GitHub Access Token and the Slack webhook URL to this file.
1. `cargo run`

The user you use for Step 1 will be queried for followers. Their contributions will be summarized in a slack message.
