use crate::error::GenericError;

pub fn remove_newline(string: &str) -> &str {
    let mut end = string.len();
    for (i, ch) in string.chars().rev().enumerate() {
        if i > 2 {
            break;
        }
        if ch == '\r' || ch == '\n' {
            end -= 1;
        }
    }
    &string[..end]
}

pub fn find_char(
    string: impl AsRef<str>,
    to_find: char,
    from: usize,
    to: usize,
) -> Result<usize, GenericError> {
    let string = string.as_ref();
    for (i, ch) in string.chars().enumerate() {
        if i < from {
            continue;
        }
        if i > to {
            break;
        }

        if ch == to_find {
            return Ok(i);
        }
    }
    Err(format!("Could not find {} in {}", to_find, string).into())
}

pub fn rfind_char(
    string: impl AsRef<str>,
    to_find: char,
    from: usize,
    to: usize,
) -> Result<usize, GenericError> {
    let string = string.as_ref();
    for (i, ch) in string.chars().rev().enumerate() {
        if i < from {
            continue;
        }
        if i > to {
            break;
        }

        if ch == to_find {
            return Ok(string.len() - i);
        }
    }
    Err(format!("Could not find {} in {}", to_find, string).into())
}
