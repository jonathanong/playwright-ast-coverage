pub fn count_code_lines(source: &str) -> usize {
    let mut count = 0;
    let mut block_depth: usize = 0;
    let mut in_string = false; // persists across lines for multi-line strings
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Char literals and escape state reset at each line boundary.
        let mut in_char = false;
        let mut escape = false;
        let bytes = trimmed.as_bytes();
        let mut i = 0;
        let mut is_code = false;
        while i < bytes.len() {
            let b = bytes[i];
            if escape {
                escape = false;
                is_code = true;
                i += 1;
                continue;
            }
            if in_string {
                is_code = true;
                if b == b'\\' {
                    escape = true;
                } else if b == b'"' {
                    in_string = false;
                }
                i += 1;
                continue;
            }
            if in_char {
                is_code = true;
                if b == b'\\' {
                    escape = true;
                } else if b == b'\'' {
                    in_char = false;
                }
                i += 1;
                continue;
            }
            if block_depth > 0 {
                if i + 1 < bytes.len() && b == b'*' && bytes[i + 1] == b'/' {
                    block_depth -= 1;
                    i += 2;
                } else if i + 1 < bytes.len() && b == b'/' && bytes[i + 1] == b'*' {
                    // Rust supports nested block comments
                    block_depth += 1;
                    i += 2;
                } else {
                    i += 1;
                }
            } else if b == b'"' {
                in_string = true;
                is_code = true;
                i += 1;
            } else if b == b'\'' {
                in_char = true;
                is_code = true;
                i += 1;
            } else if i + 1 < bytes.len() && b == b'/' && bytes[i + 1] == b'*' {
                block_depth += 1;
                i += 2;
            } else if i + 1 < bytes.len() && b == b'/' && bytes[i + 1] == b'/' {
                break;
            } else {
                is_code = true;
                i += 1;
            }
        }
        if is_code {
            count += 1;
        }
    }
    count
}
