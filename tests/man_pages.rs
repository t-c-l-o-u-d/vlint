// SPDX-License-Identifier: AGPL-3.0-or-later

#[test]
fn vlint1_has_required_sections() {
    let s = std::str::from_utf8(vlint::man::VLINT_1).unwrap();
    assert!(s.contains("SYNOPSIS"), "vlint.1 missing SYNOPSIS");
    assert!(s.contains("OPTIONS"), "vlint.1 missing OPTIONS");
    assert!(s.contains("EXIT CODES"), "vlint.1 missing EXIT CODES");
    assert!(s.contains(r"\-\-tools"), "vlint.1 missing --tools");
    assert!(s.contains("verbose"), "vlint.1 missing --verbose");
}

#[test]
fn vlint5_has_required_sections() {
    let s = std::str::from_utf8(vlint::man::VLINT_5).unwrap();
    assert!(
        s.contains("FILE LOCATIONS"),
        "vlint.5 missing FILE LOCATIONS"
    );
    assert!(s.contains("SEE ALSO"), "vlint.5 missing SEE ALSO");
    assert!(s.contains("backend="), "vlint.5 missing backend=");
    assert!(s.contains("auto_update="), "vlint.5 missing auto_update=");
    assert!(
        s.contains("man_page_install="),
        "vlint.5 missing man_page_install="
    );
}
