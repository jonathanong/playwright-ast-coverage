use crate::analysis::context::{RouteIndex, RouteTarget};
use crate::fsutil::relative_string;
use crate::matcher;
use crate::routes::Route;
use crate::url::is_dynamic_pattern_segment;
use std::path::Path;

pub(crate) fn route_index(root: &Path, routes: &[Route]) -> RouteIndex {
    let mut index = RouteIndex::default();
    for route in routes {
        let target = RouteTarget {
            route_file: relative_string(root, &route.file),
            pattern: route.pattern.clone(),
            segments: matcher::pattern_segments(&route.pattern)
                .into_iter()
                .map(str::to_string)
                .collect(),
        };
        match target.segments.first() {
            None => index.root.push(target),
            Some(first) if is_dynamic_pattern_segment(first) => index.dynamic_first.push(target),
            Some(first) => index
                .literal_first
                .entry(first.clone())
                .or_default()
                .push(target),
        }
    }
    index
}

pub(crate) fn route_specificity(segments: &[String]) -> Vec<u8> {
    let mut specificity: Vec<u8> = segments
        .iter()
        .map(|segment| {
            if segment == "**" {
                0
            } else if segment == "*" {
                1
            } else if segment.starts_with(':') {
                2
            } else {
                3
            }
        })
        .collect();
    specificity.push(4);
    specificity
}

impl RouteIndex {
    pub(crate) fn candidates<'a>(&'a self, reference_segments: &[&str]) -> Vec<&'a RouteTarget> {
        if reference_segments.is_empty() {
            return self.root.iter().chain(&self.dynamic_first).collect();
        }

        let mut candidates: Vec<&RouteTarget> = self.dynamic_first.iter().collect();
        if let Some(literal) = self.literal_first.get(reference_segments[0]) {
            candidates.extend(literal);
        }
        candidates
    }
}
