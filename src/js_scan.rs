pub fn mask_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut masked = bytes.to_vec();
    let mut i = 0usize;
    let mut quote: Option<u8> = None;
    let mut escaped = false;

    while i < bytes.len() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if bytes[i] == b'\\' {
                escaped = true;
            } else if bytes[i] == q {
                quote = None;
            }
            i += 1;
            continue;
        }

        match bytes[i] {
            b'\'' | b'"' | b'`' => {
                quote = Some(bytes[i]);
                i += 1;
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                masked[i] = b' ';
                masked[i + 1] = b' ';
                i += 2;
                while i < bytes.len() && !matches!(bytes[i], b'\n' | b'\r') {
                    masked[i] = b' ';
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                masked[i] = b' ';
                masked[i + 1] = b' ';
                i += 2;
                while i < bytes.len() {
                    let closes = bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/');
                    if !matches!(bytes[i], b'\n' | b'\r') {
                        masked[i] = b' ';
                    }
                    if closes {
                        masked[i + 1] = b' ';
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }

    String::from_utf8(masked).expect("masking comments preserves UTF-8")
}

pub fn mask_comments_and_strings(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut masked = bytes.to_vec();
    let mut i = 0usize;

    while i < bytes.len() {
        match bytes[i] {
            b'\'' | b'"' | b'`' => {
                let quote = bytes[i];
                masked[i] = b' ';
                i += 1;
                let mut escaped = false;
                while i < bytes.len() {
                    if !matches!(bytes[i], b'\n' | b'\r') {
                        masked[i] = b' ';
                    }
                    if escaped {
                        escaped = false;
                    } else if bytes[i] == b'\\' {
                        escaped = true;
                    } else if bytes[i] == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                masked[i] = b' ';
                masked[i + 1] = b' ';
                i += 2;
                while i < bytes.len() && !matches!(bytes[i], b'\n' | b'\r') {
                    masked[i] = b' ';
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                masked[i] = b' ';
                masked[i + 1] = b' ';
                i += 2;
                while i < bytes.len() {
                    let closes = bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/');
                    if !matches!(bytes[i], b'\n' | b'\r') {
                        masked[i] = b' ';
                    }
                    if closes {
                        masked[i + 1] = b' ';
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }

    String::from_utf8(masked).expect("masking strings preserves UTF-8")
}

pub fn find_outside_syntax(source: &str, needle: &str, offset: usize) -> Option<usize> {
    if needle.is_empty() || offset >= source.len() {
        return None;
    }

    let bytes = source.as_bytes();
    let needle = needle.as_bytes();
    let mut i = offset;
    let mut quote: Option<u8> = None;
    let mut escaped = false;

    while i < bytes.len() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if bytes[i] == b'\\' {
                escaped = true;
            } else if bytes[i] == q {
                quote = None;
            }
            i += 1;
            continue;
        }

        match bytes[i] {
            b'\'' | b'"' | b'`' => {
                quote = Some(bytes[i]);
                i += 1;
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                i += 2;
                while i < bytes.len() && !matches!(bytes[i], b'\n' | b'\r') {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i < bytes.len() {
                    if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            }
            _ => {
                if i + needle.len() <= bytes.len() && &bytes[i..i + needle.len()] == needle {
                    return Some(i);
                }
                i += 1;
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_line_and_block_comments_but_not_strings() {
        let source = r#"
const keep = "/* not a comment */";
// remove "testDir"
const value = 1; /* remove 'projects' */
"#;
        let masked = mask_comments(source);
        assert!(masked.contains(r#""/* not a comment */""#));
        assert!(!masked.contains("remove"));
        assert!(masked.contains("const value = 1;"));
    }

    #[test]
    fn finds_needles_only_outside_comments_and_strings() {
        let source = r#"
const sample = "testDir: './wrong'";
// testDir: './comment'
/* testDir: './block-comment' */
export default { testDir: './tests' };
"#;
        let index = find_outside_syntax(source, "testDir", 0).unwrap();
        assert_eq!(&source[index..index + "testDir".len()], "testDir");
        assert!(source[index..].starts_with("testDir: './tests'"));
        assert_eq!(find_outside_syntax(source, "missing", 0), None);
    }

    #[test]
    fn masks_comments_and_strings_for_key_scans() {
        let source = r#"
const sample = "projects: []";
/* projects: [] */
export default { projects: [{ name: 'chromium' }] };
"#;
        let masked = mask_comments_and_strings(source);
        assert_eq!(masked.matches("projects").count(), 1);
        assert!(masked.contains("export default"));
    }
}
