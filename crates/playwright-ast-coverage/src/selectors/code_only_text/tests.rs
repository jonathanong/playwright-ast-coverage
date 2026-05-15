use super::*;
use crate::selectors::shadowing::has_identifier_reassignment;

#[test]
fn code_only_text_masks_comments_and_string_literals() {
    let masked = code_only_text(
        "const id = 'data\\'Pw';\n// dataPw = line\n/* dataPw = block\n*/ const text = \"data\\\"Pw\"; const tpl = `data ${dataPw = makeId({ nested: true })} \\`Pw`; dataPw += '-x';",
    );

    assert!(masked.contains("const id ="));
    assert!(masked.contains("const text ="));
    assert!(masked.contains("const tpl ="));
    assert!(masked.contains("dataPw = makeId({ nested: true })"));
    assert!(masked.contains("dataPw +="));
    assert!(!has_identifier_reassignment(
        "'dataPw = string';\n\"dataPw += string\";\n`dataPw++`;\n/* dataPw ??= block */",
        "dataPw"
    ));
}
