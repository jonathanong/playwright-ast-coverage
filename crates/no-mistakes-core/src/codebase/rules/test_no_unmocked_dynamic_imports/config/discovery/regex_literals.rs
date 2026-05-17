use regex::Regex;

pub(super) fn extract_test_regex_literals(source: &str) -> Vec<String> {
    let re = Regex::new(r#"\btestRegex\s*:\s*"#).expect("testRegex start regex compiles");
    let mut regexes = Vec::new();
    for mat in re.find_iter(source) {
        let mut idx = mat.end();
        skip_space(source, &mut idx);
        if source[idx..].starts_with('/') {
            if let Some((pattern, _)) = parse_regex_literal(source, idx) {
                regexes.push(pattern);
            }
        } else if source[idx..].starts_with('[') {
            extract_array_literals(source, idx + '['.len_utf8(), &mut regexes);
        }
    }
    regexes
}

fn extract_array_literals(source: &str, mut idx: usize, regexes: &mut Vec<String>) {
    while idx < source.len() {
        skip_space(source, &mut idx);
        if source[idx..].starts_with(']') {
            break;
        }
        if source[idx..].starts_with('/') {
            if let Some((pattern, end)) = parse_regex_literal(source, idx) {
                regexes.push(pattern);
                idx = end;
                continue;
            }
        }
        idx += next_char_len(source, idx).unwrap_or(1);
    }
}

fn parse_regex_literal(source: &str, start: usize) -> Option<(String, usize)> {
    let mut pattern = String::new();
    let mut escaped = false;
    let mut in_class = false;
    for (offset, ch) in source[start + 1..].char_indices() {
        let idx = start + 1 + offset;
        if escaped {
            pattern.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            pattern.push(ch);
            escaped = true;
            continue;
        }
        if ch == '[' {
            in_class = true;
        } else if ch == ']' {
            in_class = false;
        } else if ch == '/' && !in_class {
            return Some((pattern, regex_literal_end(source, idx)));
        }
        pattern.push(ch);
    }
    None
}

fn regex_literal_end(source: &str, slash: usize) -> usize {
    let mut end = slash + '/'.len_utf8();
    while end < source.len() {
        let flag = source[end..]
            .chars()
            .next()
            .expect("end is within a valid UTF-8 string");
        if !flag.is_ascii_alphabetic() {
            break;
        }
        end += flag.len_utf8();
    }
    end
}

fn skip_space(source: &str, idx: &mut usize) {
    while *idx < source.len() {
        let ch = source[*idx..]
            .chars()
            .next()
            .expect("idx is within a valid UTF-8 string");
        if !ch.is_whitespace() {
            break;
        }
        *idx += ch.len_utf8();
    }
}

fn next_char_len(source: &str, idx: usize) -> Option<usize> {
    source[idx..].chars().next().map(char::len_utf8)
}
