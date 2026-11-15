//! `Co-authored-by` trailers.
//!
//! Git stores co-authors as trailers in the commit message, so the only thing
//! Knitt has to get right is appending one to the right paragraph without
//! mangling the subject or an existing trailer block.

/// Trailer keys Git understands and that routinely end a commit message. They
/// let a one-line paragraph count as a trailer block; any other single line is
/// treated as text so a subject such as `fix: a bug` is never split.
const KNOWN_TRAILERS: &[&str] = &[
    "co-authored-by",
    "signed-off-by",
    "reviewed-by",
    "acked-by",
    "tested-by",
    "reported-by",
    "suggested-by",
    "fixes",
    "closes",
    "refs",
];

fn trailer_key(line: &str) -> Option<String> {
    let (key, value) = line.trim_end().split_once(": ")?;
    if value.trim().is_empty() {
        return None;
    }
    if key.is_empty()
        || !key
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
        || !key.chars().any(|character| character.is_ascii_alphabetic())
    {
        return None;
    }
    Some(key.to_ascii_lowercase())
}

/// Whether the last paragraph (after the final blank line) is a trailer block.
fn ends_with_trailer_block(lines: &[&str]) -> bool {
    let mut count = 0;
    let mut known = true;
    for line in lines.iter().rev() {
        if line.trim().is_empty() {
            break;
        }
        let Some(key) = trailer_key(line) else {
            return false;
        };
        known &= KNOWN_TRAILERS.contains(&key.as_str());
        count += 1;
    }
    count >= 2 || (count == 1 && known)
}

fn valid_email(email: &str) -> bool {
    if email.contains(char::is_whitespace) || email.contains('<') || email.contains('>') {
        return false;
    }
    let mut parts = email.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

/// Returns `message` with a `Co-authored-by` trailer appended.
///
/// The trailer joins an existing trailer block when the message ends with one,
/// and a new paragraph is started otherwise. Duplicate co-authors are refused
/// so the same person is not credited twice.
pub fn add_coauthor(message: &str, name: &str, email: &str) -> Result<String, String> {
    let name = name.trim();
    let email = email.trim();
    if name.is_empty() {
        return Err("Enter the co-author's name.".to_string());
    }
    if name.contains('\n')
        || name.contains('\r')
        || email.contains('\n')
        || email.contains('\r')
    {
        return Err("The co-author details cannot contain line breaks.".to_string());
    }
    if !valid_email(email) {
        return Err("Enter a valid email address for the co-author.".to_string());
    }

    let lowered = email.to_ascii_lowercase();
    let address = format!("<{lowered}>");
    let trimmed = message.trim_end();
    for line in trimmed.lines() {
        if let Some((key, value)) = line.trim().split_once(':')
            && key.trim().eq_ignore_ascii_case("co-authored-by")
            && value.trim().to_ascii_lowercase().ends_with(&address)
        {
            return Err("That co-author is already on this commit.".to_string());
        }
    }

    let trailer = format!("Co-authored-by: {name} <{email}>");
    if trimmed.is_empty() {
        return Ok(trailer);
    }

    let lines: Vec<&str> = trimmed.lines().collect();
    let separator = if ends_with_trailer_block(&lines) {
        "\n"
    } else {
        "\n\n"
    };
    Ok(format!("{trimmed}{separator}{trailer}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_a_trailer_paragraph_to_a_subject() {
        assert_eq!(
            add_coauthor("Fix the login bug", "Ada Lovelace", "ada@example.com").unwrap(),
            "Fix the login bug\n\nCo-authored-by: Ada Lovelace <ada@example.com>"
        );
    }

    #[test]
    fn empty_message_becomes_the_trailer() {
        assert_eq!(
            add_coauthor("  ", "Ada", "ada@example.com").unwrap(),
            "Co-authored-by: Ada <ada@example.com>"
        );
    }

    #[test]
    fn joins_an_existing_trailer_block() {
        assert_eq!(
            add_coauthor(
                "Subject\n\nBody text\n\nSigned-off-by: A <a@example.com>",
                "Ada",
                "ada@example.com"
            )
            .unwrap(),
            "Subject\n\nBody text\n\nSigned-off-by: A <a@example.com>\nCo-authored-by: Ada <ada@example.com>"
        );
    }

    #[test]
    fn adds_after_an_existing_coauthor() {
        let message = "Subject\n\nCo-authored-by: A <a@example.com>\nCo-authored-by: B <b@example.com>";
        assert_eq!(
            add_coauthor(message, "Ada", "ada@example.com").unwrap(),
            format!("{message}\nCo-authored-by: Ada <ada@example.com>")
        );
    }

    #[test]
    fn conventional_subjects_are_not_treated_as_trailers() {
        assert_eq!(
            add_coauthor("fix: a bug", "Ada", "ada@example.com").unwrap(),
            "fix: a bug\n\nCo-authored-by: Ada <ada@example.com>"
        );
    }

    #[test]
    fn multi_line_trailer_blocks_do_not_need_known_keys() {
        assert_eq!(
            add_coauthor(
                "Subject\n\nPartners: Acme\nReviewed: yes",
                "Ada",
                "ada@example.com"
            )
            .unwrap(),
            "Subject\n\nPartners: Acme\nReviewed: yes\nCo-authored-by: Ada <ada@example.com>"
        );
    }

    #[test]
    fn refuses_bad_details() {
        assert_eq!(
            add_coauthor("Subject", "  ", "ada@example.com").unwrap_err(),
            "Enter the co-author's name."
        );
        for email in ["", "ada", "ada@", "@example.com", "ada@example", "a b@example.com", "a<b@c.de"] {
            assert_eq!(
                add_coauthor("Subject", "Ada", email).unwrap_err(),
                "Enter a valid email address for the co-author.",
                "{email} should be refused"
            );
        }
        assert_eq!(
            add_coauthor("Subject", "Ada\nInjected", "ada@example.com").unwrap_err(),
            "The co-author details cannot contain line breaks."
        );
    }

    #[test]
    fn refuses_a_duplicate_email_regardless_of_case() {
        let message = "Subject\n\nCo-authored-by: Ada <Ada@Example.com>";
        assert_eq!(
            add_coauthor(message, "Ada again", "ada@example.com").unwrap_err(),
            "That co-author is already on this commit."
        );
    }

    #[test]
    fn keeps_the_message_body_intact() {
        let message = "Subject\n\nFirst paragraph.\n\nSecond paragraph.";
        let result = add_coauthor(message, "Ada", "ada@example.com").unwrap();
        assert!(result.starts_with(message));
        assert!(result.ends_with("Second paragraph.\n\nCo-authored-by: Ada <ada@example.com>"));
    }
}
