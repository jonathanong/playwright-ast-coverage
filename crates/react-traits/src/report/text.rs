use crate::report::types::{ComponentFacts, Violation};

pub(crate) fn print_results(results: &[ComponentFacts], depth: usize) {
    for facts in results {
        print_component(facts, depth, 0);
        println!();
    }
}

fn print_component(facts: &ComponentFacts, max_depth: usize, current_depth: usize) {
    let indent = "  ".repeat(current_depth);
    println!("{}{}#{}", indent, facts.file, facts.name);
    let i2 = "  ".repeat(current_depth + 1);
    println!("{}hasState: {}", i2, facts.has_state);
    println!("{}hasProps: {}", i2, facts.has_props);
    println!("{}passesProps: {}", i2, facts.passes_props);
    println!("{}usesMemo: {}", i2, facts.uses_memo);
    println!("{}usesContextProvider: {}", i2, facts.uses_context_provider);
    println!("{}usesSuspense: {}", i2, facts.uses_suspense);
    println!("{}hasFetch: {}", i2, !facts.fetches.is_empty());
    println!("{}environment: {}", i2, facts.environment);
    if !facts.children.is_empty() {
        println!("{}children:", i2);
        for child in &facts.children {
            println!("{}  {}#{}", i2, child.file, child.name);
        }
    }
    if !facts.dependencies.is_empty() {
        println!("{}dependencies:", i2);
        for dep in &facts.dependencies {
            println!("{}  {}", i2, dep);
        }
    }
    if let Some(agg) = &facts.inherited_from_children {
        println!("{}inheritedFromChildren:", i2);
        let i3 = "  ".repeat(current_depth + 2);
        println!("{}hasFetch: {}", i3, agg.has_fetch);
    }
    if current_depth < max_depth {
        // In a full recursive implementation, child ComponentFacts would be resolved
        // and printed here. For now children are listed as refs only.
    }
}

#[cfg(test)]
mod tests;

pub(crate) fn print_violations(violations: &[Violation]) {
    for v in violations {
        if let Some(detail) = &v.detail {
            println!("{}#{}: {} {}", v.file, v.component, v.rule, detail);
        } else {
            println!("{}#{}: {}", v.file, v.component, v.rule);
        }
    }
}
