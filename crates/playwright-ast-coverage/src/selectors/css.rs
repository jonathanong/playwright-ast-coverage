use super::regex_mod::{first_capture, matcher_for_operator};
use super::types::{AttributeRegex, PlaywrightSelector, SelectorMatcher};
use super::HTML_ID_ATTRIBUTE;

pub(super) fn extract_css_attribute_selectors(
    source: &str,
    attributes: &[AttributeRegex],
    insert: &mut impl FnMut(PlaywrightSelector),
) {
    for attribute in attributes {
        for captures in attribute.regex.captures_iter(source) {
            let op = captures.get(1).expect("operator capture").as_str();
            let value = first_capture(&captures, &[2, 3]).expect("value capture");
            insert(PlaywrightSelector {
                attribute: attribute.attribute.clone(),
                selector: captures
                    .get(0)
                    .expect("selector capture")
                    .as_str()
                    .to_string(),
                matcher: matcher_for_operator(op, value),
            });
        }
    }
}

pub(super) fn extract_css_id_selectors(source: &str, insert: &mut impl FnMut(PlaywrightSelector)) {
    let mut index = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut bracket_depth = 0usize;
    let mut escaped = false;
    let mut in_comment = false;

    while index < source.len() {
        let ch = source[index..]
            .chars()
            .next()
            .expect("index is inside source");
        let ch_len = ch.len_utf8();

        if escaped {
            escaped = false;
            index += ch_len;
            continue;
        }
        if in_comment {
            if ch == '*' && source[index + ch_len..].starts_with('/') {
                in_comment = false;
                index += ch_len + 1;
            } else {
                index += ch_len;
            }
            continue;
        }
        if ch == '\\' {
            escaped = true;
            index += ch_len;
            continue;
        }
        if in_single {
            if ch == '\'' {
                in_single = false;
            }
            index += ch_len;
            continue;
        }
        if in_double {
            if ch == '"' {
                in_double = false;
            }
            index += ch_len;
            continue;
        }

        match ch {
            '\'' => in_single = true,
            '"' => in_double = true,
            '[' => bracket_depth += 1,
            ']' if bracket_depth > 0 => bracket_depth -= 1,
            '/' if source[index + ch_len..].starts_with('*') => {
                in_comment = true;
                index += 1;
            }
            '#' if bracket_depth == 0 => {
                if let Some((raw, value, end)) = css_id_selector(source, index) {
                    insert(PlaywrightSelector {
                        attribute: HTML_ID_ATTRIBUTE.to_string(),
                        selector: raw,
                        matcher: SelectorMatcher::Exact(value),
                    });
                    index = end;
                    continue;
                }
            }
            _ => {}
        }

        index += ch_len;
    }
}

fn css_id_selector(source: &str, hash_index: usize) -> Option<(String, String, usize)> {
    let mut index = hash_index + 1;
    let mut value = String::new();
    while index < source.len() {
        let ch = source[index..].chars().next()?;
        if ch == '\\' {
            let (decoded, next_index) = css_escape(source, index)?;
            value.push(decoded);
            index = next_index;
            continue;
        }
        if !is_css_identifier_char(ch) {
            break;
        }
        value.push(ch);
        index += ch.len_utf8();
    }

    if value.is_empty() || source[index..].starts_with("${") {
        return None;
    }

    Some((source[hash_index..index].to_string(), value, index))
}

pub(super) fn css_escape(source: &str, slash_index: usize) -> Option<(char, usize)> {
    let mut index = slash_index + 1;
    let first = source[index..].chars().next()?;
    if !first.is_ascii_hexdigit() {
        return Some((first, index + first.len_utf8()));
    }

    let mut hex = String::new();
    while index < source.len() && hex.len() < 6 {
        let ch = source[index..]
            .chars()
            .next()
            .expect("index is inside source");
        if !ch.is_ascii_hexdigit() {
            break;
        }
        hex.push(ch);
        index += ch.len_utf8();
    }

    index += source[index..]
        .chars()
        .next()
        .filter(|ch| ch.is_whitespace())
        .map_or(0, char::len_utf8);

    let code = u32::from_str_radix(&hex, 16).ok()?;
    char::from_u32(code).map(|ch| (ch, index))
}

fn is_css_identifier_char(ch: char) -> bool {
    ch == '-' || ch == '_' || ch.is_ascii_alphanumeric() || !ch.is_ascii()
}

#[cfg(test)]
mod tests;
