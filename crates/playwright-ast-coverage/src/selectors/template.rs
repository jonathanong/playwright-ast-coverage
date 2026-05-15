use super::types::TemplatePattern;

impl TemplatePattern {
    pub fn new(raw: &str) -> Option<Self> {
        let parts = template_parts(raw);
        if parts.iter().all(|part| part.is_empty()) {
            return None;
        }
        Some(Self {
            raw: raw.to_string(),
            parts,
            starts_static: !raw.starts_with("${"),
            ends_static: !raw.ends_with('}'),
        })
    }

    pub(super) fn matches_exact(&self, value: &str) -> bool {
        let non_empty: Vec<&str> = self
            .parts
            .iter()
            .filter(|part| !part.is_empty())
            .map(String::as_str)
            .collect();
        if non_empty.is_empty() {
            return false;
        }
        if self.starts_static && !value.starts_with(non_empty[0]) {
            return false;
        }
        if self.ends_static && !value.ends_with(non_empty[non_empty.len() - 1]) {
            return false;
        }

        let mut offset = 0;
        for part in non_empty {
            let Some(index) = value[offset..].find(part) else {
                return false;
            };
            offset += index + part.len();
        }
        true
    }

    pub(super) fn sample(&self) -> String {
        let mut sample = String::new();
        for (index, part) in self.parts.iter().enumerate() {
            if index > 0 {
                sample.push('x');
            }
            sample.push_str(part);
        }
        sample
    }

    pub(super) fn first_static(&self) -> Option<&str> {
        self.parts
            .iter()
            .find(|part| !part.is_empty())
            .map(String::as_str)
    }

    pub(super) fn last_static(&self) -> Option<&str> {
        self.parts
            .iter()
            .rev()
            .find(|part| !part.is_empty())
            .map(String::as_str)
    }

    pub(super) fn contains_static(&self, needle: &str) -> bool {
        self.parts
            .iter()
            .any(|part| !part.is_empty() && (part.contains(needle) || needle.contains(part)))
    }
}

pub(super) fn template_parts(source: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find("${") {
        parts.push(rest[..start].to_string());
        let expression = &rest[start + 2..];
        let Some(end) = expression.find('}') else {
            return vec![source.to_string()];
        };
        rest = &expression[end + 1..];
    }
    parts.push(rest.to_string());
    parts
}

#[cfg(test)]
mod tests;
