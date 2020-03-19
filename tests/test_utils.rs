use dvm::compiler::mvir;

#[test]
fn test_replace_s_prefixed_string() {
    assert_eq!(
        mvir::find_and_replace_s_prefixed_strings(r#"a = s"BTCUSD";"#),
        r#"a = h"425443555344";"#
    );
    assert_eq!(
        mvir::find_and_replace_s_prefixed_strings(r#"a = s"";"#),
        r#"a = h"";"#
    );
}
