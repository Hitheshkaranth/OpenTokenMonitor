use crate::usage_scanners::read_claude_oauth_credentials;

pub fn read_access_token() -> Option<String> {
    let creds = read_claude_oauth_credentials();
    if creds.access_token.trim().is_empty() {
        None
    } else {
        Some(creds.access_token)
    }
}
