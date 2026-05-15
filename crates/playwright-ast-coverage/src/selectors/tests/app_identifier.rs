use crate::selectors::{extract_app_selectors, AppSelector};
use std::collections::BTreeMap;
use std::path::Path;

fn attrs() -> Vec<String> {
    vec!["data-testid".to_string(), "data-pw".to_string()]
}

#[test]
fn extracts_static_identifier_default_jsx_selectors() {
    let selectors = extract_app_selectors(
        Path::new("app/page.tsx"),
        r#"
        export function Link({ 'data-pw': dataPw = 'rss-feed-link' }) {
            return <a data-pw={dataPw}>RSS</a>;
        }
        export function Button({ passThrough }) {
            return (
                <>
                    <button data-pw={passThrough}>Save</button>
                    <button data-pw={1 + 1}>Count</button>
                </>
            );
        }
        export function DynamicLink({ dataPw }) {
            return <a data-pw={dataPw}>Dynamic</a>;
        }
        export const ArrowLink = ({ dataPw = 'arrow-link' }) => {
            return <a data-pw={dataPw}>Arrow</a>;
        };
        export function DirectDefault(dataPw = 'direct-link') {
            return <a data-pw={dataPw}>Direct</a>;
        }
        export function ArrayDefault([dataPw = 'array-link']) {
            return <a data-pw={dataPw}>Array</a>;
        }
        export function NonStringDefault({ value = makeId() }) {
            return <a data-pw={value}>Computed</a>;
        }
        export function NestedShadow({ dataPw = 'outer-link' }) {
            function Inner({ dataPw }) { return <a data-pw={dataPw}>Inner</a>; }
            return <Inner />;
        }
        export function Reassigned({ reassigned = 'assigned-link' }) {
            reassigned = makeId();
            return <a data-pw={reassigned}>Assigned</a>;
        }
        export function CompoundReassigned({ compound = 'compound-link' }) {
            compound += '-dynamic';
            return <a data-pw={compound}>Compound</a>;
        }
        export function DestructuredShadow({ shadowed = 'shadowed-link' }, props) {
            const { shadowed } = props;
            return <a data-pw={shadowed}>Shadowed</a>;
        }
        export function CommentAndStringText({ dataPw = 'comment-safe-link' }) {
            // dataPw = makeId();
            const message = "dataPw = makeId();";
            return <a data-pw={dataPw}>Comment safe</a>;
        }
        export function TemplateExpressionMutation({ mutated = 'template-mutation-link' }) {
            const label = `${mutated = makeId()}`;
            return <a data-pw={mutated}>Template mutation</a>;
        }
        export function EarlierHelperParam({ dataPw = 'helper-param-link' }) {
            function helper(dataPw) { return dataPw; }
            const local = (dataPw) => dataPw;
            return <a data-pw={dataPw}>{helper(local('x'))}</a>;
        }
        export function WithHelper({ dataPw = 'helper-link' }) {
            const isReady = () => dataPw === 'helper-link';
            return isReady() ? <a data-pw={dataPw}>Ready</a> : null;
        }
        export function ShortName({ id = 'short-link' }) {
            const userId = makeId();
            return <a data-pw={id}>Short</a>;
        }
        "#,
        &attrs(),
        &BTreeMap::new(),
    )
    .unwrap();

    let mut values: Vec<String> = selectors.iter().map(AppSelector::display_value).collect();
    values.sort();
    values.dedup();
    assert_eq!(
        values,
        vec![
            "array-link",
            "arrow-link",
            "comment-safe-link",
            "direct-link",
            "helper-link",
            "helper-param-link",
            "rss-feed-link",
            "short-link",
            "{1 + 1}",
            "{compound}",
            "{dataPw}",
            "{mutated}",
            "{passThrough}",
            "{reassigned}",
            "{shadowed}",
            "{value}",
        ]
    );
}
