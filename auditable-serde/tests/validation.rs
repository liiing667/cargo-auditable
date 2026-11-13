use auditable_serde::{ParseError, ValidationError, VersionInfo};
use std::path::PathBuf;
use std::str::FromStr;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn rejects_out_of_bounds_dependency_index() {
    let error = VersionInfo::parse(&fixture("out_of_bounds_dependency.json")).unwrap_err();
    match error {
        ParseError::Validation(ValidationError::DependencyIndexOutOfBounds {
            package,
            dependency,
        }) => {
            assert_eq!(package, 0);
            assert_eq!(dependency, 5);
        }
        other => panic!("expected an out-of-bounds index error, got {:?}", other),
    }
}

#[test]
fn rejects_duplicate_dependency_edge() {
    let error = VersionInfo::parse(&fixture("duplicate_dependency_edge.json")).unwrap_err();
    match error {
        ParseError::Validation(ValidationError::DuplicateDependency {
            package,
            dependency,
        }) => {
            assert_eq!(package, 0);
            assert_eq!(dependency, 1);
        }
        other => panic!("expected a duplicate edge error, got {:?}", other),
    }
}

#[test]
fn rejects_data_without_a_root_package() {
    let error = VersionInfo::parse(&fixture("no_root_package.json")).unwrap_err();
    match error {
        ParseError::Validation(ValidationError::NoRoot) => {}
        other => panic!("expected a no-root error, got {:?}", other),
    }
}

#[test]
fn rejects_data_with_multiple_root_packages() {
    let error = VersionInfo::parse(&fixture("multiple_root_packages.json")).unwrap_err();
    match error {
        ParseError::Validation(ValidationError::MultipleRoots) => {}
        other => panic!("expected a multiple-roots error, got {:?}", other),
    }
}

#[test]
fn rejects_syntactically_invalid_json_as_a_json_error() {
    let error = VersionInfo::parse("{not valid json").unwrap_err();
    assert!(matches!(error, ParseError::Json(_)));
}

#[test]
fn accepts_legacy_format_without_a_format_field() {
    let info = VersionInfo::parse(&fixture("valid_format_0.json")).unwrap();
    assert_eq!(info.format, 0);
    assert_eq!(info.packages.len(), 2);
    assert!(info.packages[0].root);
}

#[test]
fn accepts_current_format_8() {
    let info = VersionInfo::parse(&fixture("valid_format_8.json")).unwrap();
    assert_eq!(info.format, 8);
    assert_eq!(info.packages.len(), 2);
}

#[test]
fn from_str_rejects_malformed_input_too() {
    // The `FromStr` API cannot expose the error kind, but must still fail closed.
    assert!(VersionInfo::from_str(&fixture("out_of_bounds_dependency.json")).is_err());
    assert!(VersionInfo::from_str(&fixture("duplicate_dependency_edge.json")).is_err());
    assert!(VersionInfo::from_str(&fixture("no_root_package.json")).is_err());
}
