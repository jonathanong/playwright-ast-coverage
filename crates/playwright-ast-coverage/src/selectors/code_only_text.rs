pub(super) fn code_only_text(source: &str) -> String {
    let mut chars = source.chars().peekable();
    let mut output = String::with_capacity(source.len());
    let mut in_single = false;
    let mut in_double = false;
    let mut in_template = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut escaped = false;
    let mut template_expression_depth = 0usize;

    while let Some(ch) = chars.next() {
        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
                output.push(ch);
            } else {
                output.push(' ');
            }
            continue;
        }
        if in_block_comment {
            if ch == '*' && chars.peek().is_some_and(|next| *next == '/') {
                output.push(' ');
                output.push(' ');
                chars.next();
                in_block_comment = false;
            } else {
                output.push(if ch == '\n' { '\n' } else { ' ' });
            }
            continue;
        }
        if escaped {
            escaped = false;
            output.push(' ');
            continue;
        }
        if (in_single || in_double || in_template) && ch == '\\' {
            escaped = true;
            output.push(' ');
            continue;
        }
        if template_expression_depth > 0 {
            if ch == '{' {
                template_expression_depth += 1;
            } else if ch == '}' {
                template_expression_depth -= 1;
                if template_expression_depth == 0 {
                    in_template = true;
                    output.push(' ');
                    continue;
                }
            }
            output.push(ch);
            continue;
        }
        if in_single {
            in_single = ch != '\'';
            output.push(' ');
            continue;
        }
        if in_double {
            in_double = ch != '"';
            output.push(' ');
            continue;
        }
        if in_template {
            if ch == '$' && chars.peek().is_some_and(|next| *next == '{') {
                output.push(' ');
                output.push(' ');
                chars.next();
                in_template = false;
                template_expression_depth = 1;
            } else {
                in_template = ch != '`';
                output.push(' ');
            }
            continue;
        }

        if ch == '/' && chars.peek().is_some_and(|next| *next == '/') {
            output.push(' ');
            output.push(' ');
            chars.next();
            in_line_comment = true;
        } else if ch == '/' && chars.peek().is_some_and(|next| *next == '*') {
            output.push(' ');
            output.push(' ');
            chars.next();
            in_block_comment = true;
        } else if ch == '\'' {
            output.push(' ');
            in_single = true;
        } else if ch == '"' {
            output.push(' ');
            in_double = true;
        } else if ch == '`' {
            output.push(' ');
            in_template = true;
        } else {
            output.push(ch);
        }
    }

    output
}

#[cfg(test)]
mod tests;
