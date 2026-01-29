use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    /// Regex for validating code fields (organization code, etc.)
    /// Must be lowercase alphanumeric with hyphens
    /// - Valid: "pemkot-surabaya", "org123", "my-org-name"
    /// - Invalid: "-org", "org-", "org--name", "Org", "org_name"
    pub static ref CODE_REGEX: Regex = Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").unwrap();

    /// Regex for validating username fields
    /// Must start with letter or underscore and contain only alphanumeric characters and underscores
    /// - Valid: "john_doe", "user123", "_admin", "JohnDoe"
    /// - Invalid: "123user", "-user", "user-name", "user name"
    pub static ref USERNAME_REGEX: Regex = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_regex_valid() {
        assert!(CODE_REGEX.is_match("pemkot-surabaya"));
        assert!(CODE_REGEX.is_match("org123"));
        assert!(CODE_REGEX.is_match("my-org-name"));
        assert!(CODE_REGEX.is_match("a"));
        assert!(CODE_REGEX.is_match("abc123"));
        assert!(CODE_REGEX.is_match("a-b-c"));
    }

    #[test]
    fn test_code_regex_invalid() {
        assert!(!CODE_REGEX.is_match("-org")); // starts with hyphen
        assert!(!CODE_REGEX.is_match("org-")); // ends with hyphen
        assert!(!CODE_REGEX.is_match("org--name")); // double hyphen
        assert!(!CODE_REGEX.is_match("Org")); // uppercase
        assert!(!CODE_REGEX.is_match("org_name")); // underscore
        assert!(!CODE_REGEX.is_match("")); // empty
        assert!(!CODE_REGEX.is_match("org name")); // space
    }
}
