use auditable_info::{audit_info_from_json, Error};
use std::path::PathBuf;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn rejects_out_of_bounds_dependency_index() {
    let error = audit_info_from_json(&fixture("out_of_bounds_dependency.json")).unwrap_err();
    assert!(
        matches!(
            error,
            Error::AuditDataValidation(
                auditable_serde::ValidationError::DependencyIndexOutOfBounds { .. }
            )
        ),
        "got {:?}",
        error
    );
    assert!(error.to_string().contains("invalid dependency tree"));
}

#[test]
fn rejects_duplicate_dependency_edge() {
    let error = audit_info_from_json(&fixture("duplicate_dependency_edge.json")).unwrap_err();
    assert!(
        matches!(
            error,
            Error::AuditDataValidation(
                auditable_serde::ValidationError::DuplicateDependency { .. }
            )
        ),
        "got {:?}",
        error
    );
}

#[test]
fn rejects_data_without_a_root_package() {
    let error = audit_info_from_json(&fixture("no_root_package.json")).unwrap_err();
    assert!(
        matches!(
            error,
            Error::AuditDataValidation(auditable_serde::ValidationError::NoRoot)
        ),
        "got {:?}",
        error
    );
}

#[test]
fn rejects_data_with_multiple_root_packages() {
    let error = audit_info_from_json(&fixture("multiple_root_packages.json")).unwrap_err();
    assert!(
        matches!(
            error,
            Error::AuditDataValidation(auditable_serde::ValidationError::MultipleRoots)
        ),
        "got {:?}",
        error
    );
}

#[test]
fn reports_syntax_errors_separately_from_validation_errors() {
    let error = audit_info_from_json("{not valid json").unwrap_err();
    assert!(matches!(error, Error::Json(_)), "got {:?}", error);
}

#[test]
fn accepts_legacy_and_current_formats() {
    let legacy = audit_info_from_json(&fixture("valid_format_0.json")).unwrap();
    assert_eq!(legacy.format, 0);
    assert_eq!(legacy.packages.len(), 2);

    let current = audit_info_from_json(&fixture("valid_format_8.json")).unwrap();
    assert_eq!(current.format, 8);
    assert_eq!(current.packages.len(), 2);
}
