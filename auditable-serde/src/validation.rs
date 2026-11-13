use crate::{is_default, Package, VersionInfo};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, convert::TryFrom, fmt::Display};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub(crate) struct RawVersionInfo {
    pub packages: Vec<Package>,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_default")]
    pub format: u32,
}

/// An error returned when the dependency tree embedded in the input is structurally invalid.
///
/// The audit data is embedded in a binary and can therefore be tampered with,
/// so it must not be trusted until it passes validation.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// No package has `root` set to `true`.
    NoRoot,
    /// More than one package has `root` set to `true`.
    MultipleRoots,
    /// A package lists a dependency index that is outside of the package array.
    DependencyIndexOutOfBounds {
        /// Index of the package that contains the invalid reference.
        package: usize,
        /// The out-of-bounds dependency index.
        dependency: usize,
    },
    /// The same dependency index is listed more than once for a package.
    DuplicateDependency {
        /// Index of the package that contains the duplicate edge.
        package: usize,
        /// The dependency index that is listed more than once.
        dependency: usize,
    },
    /// The dependency graph contains a cycle.
    CyclicDependency,
}

impl std::error::Error for ValidationError {}

impl Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::NoRoot => {
                write!(f, "No root package specified in the input JSON")
            }
            ValidationError::MultipleRoots => {
                write!(f, "Multiple root packages specified in the input JSON")
            }
            ValidationError::DependencyIndexOutOfBounds {
                package,
                dependency,
            } => write!(
                f,
                "Package at index {package} references a dependency at index {dependency}, which is out of bounds"
            ),
            ValidationError::DuplicateDependency {
                package,
                dependency,
            } => write!(
                f,
                "Package at index {package} lists the dependency at index {dependency} more than once"
            ),
            ValidationError::CyclicDependency => {
                write!(f, "The input JSON specifies a cyclic dependency graph")
            }
        }
    }
}

impl TryFrom<RawVersionInfo> for VersionInfo {
    type Error = ValidationError;

    fn try_from(v: RawVersionInfo) -> Result<Self, Self::Error> {
        validate(&v)?;
        Ok(VersionInfo {
            packages: v.packages,
            format: v.format,
        })
    }
}

fn validate(v: &RawVersionInfo) -> Result<(), ValidationError> {
    validate_dependency_indices(v)?;
    validate_root_packages(v)?;
    validate_acyclic(v)?;
    Ok(())
}

fn validate_dependency_indices(v: &RawVersionInfo) -> Result<(), ValidationError> {
    let package_count = v.packages.len();
    for (package_index, package) in v.packages.iter().enumerate() {
        let mut seen_dependencies = HashSet::with_capacity(package.dependencies.len());
        for dependency in &package.dependencies {
            if *dependency >= package_count {
                return Err(ValidationError::DependencyIndexOutOfBounds {
                    package: package_index,
                    dependency: *dependency,
                });
            }
            if !seen_dependencies.insert(*dependency) {
                return Err(ValidationError::DuplicateDependency {
                    package: package_index,
                    dependency: *dependency,
                });
            }
        }
    }
    Ok(())
}

fn validate_root_packages(v: &RawVersionInfo) -> Result<(), ValidationError> {
    let mut root_count = 0;
    for package in &v.packages {
        if package.root {
            root_count += 1;
        }
    }
    match root_count {
        0 => Err(ValidationError::NoRoot),
        1 => Ok(()),
        _ => Err(ValidationError::MultipleRoots),
    }
}

fn validate_acyclic(v: &RawVersionInfo) -> Result<(), ValidationError> {
    // I've reviewed the `topological_sort` crate and it appears to be high-quality,
    // so I'm not concerned about having it exposed to untrusted input.
    // It's better than my hand-rolled version would be.

    // populate the topological sorting map
    let mut ts = topological_sort::TopologicalSort::<usize>::new();
    for (index, package) in v.packages.iter().enumerate() {
        for dep in &package.dependencies {
            ts.add_dependency(*dep, index);
        }
    }
    // drain all elements that are not part of a cycle
    while ts.pop().is_some() {}
    // if the set isn't empty, the graph has cycles
    if ts.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::CyclicDependency)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use crate::*;

    fn dummy_package(pkg_counter: u32, root: bool, deps: Vec<usize>) -> Package {
        Package {
            name: format!("test_{pkg_counter}"),
            version: semver::Version::from_str("0.0.0").unwrap(),
            source: Source::Local,
            kind: DependencyKind::Build,
            dependencies: deps,
            root: root,
        }
    }

    // these tests are very basic because `topological_sort` crate is already tested extensively

    #[test]
    fn cyclic_dependencies() {
        let pkg0 = dummy_package(0, true, vec![1]);
        let pkg1 = dummy_package(1, false, vec![0]);
        let raw = RawVersionInfo {
            packages: vec![pkg0, pkg1],
            format: 0,
        };
        assert!(matches!(
            VersionInfo::try_from(raw),
            Err(ValidationError::CyclicDependency)
        ));
    }

    #[test]
    fn no_cyclic_dependencies() {
        let pkg0 = dummy_package(0, true, vec![1]);
        let pkg1 = dummy_package(1, false, vec![]);
        let raw = RawVersionInfo {
            packages: vec![pkg0, pkg1],
            format: 0,
        };
        assert!(VersionInfo::try_from(raw).is_ok());
    }

    #[test]
    fn no_root_package() {
        let pkg0 = dummy_package(0, false, vec![]);
        let raw = RawVersionInfo {
            packages: vec![pkg0],
            format: 0,
        };
        assert!(matches!(
            VersionInfo::try_from(raw),
            Err(ValidationError::NoRoot)
        ));
    }

    #[test]
    fn multiple_root_packages() {
        let pkg0 = dummy_package(0, true, vec![]);
        let pkg1 = dummy_package(1, true, vec![]);
        let raw = RawVersionInfo {
            packages: vec![pkg0, pkg1],
            format: 0,
        };
        assert!(matches!(
            VersionInfo::try_from(raw),
            Err(ValidationError::MultipleRoots)
        ));
    }

    #[test]
    fn out_of_bounds_dependency_index() {
        let pkg0 = dummy_package(0, true, vec![1]);
        let raw = RawVersionInfo {
            packages: vec![pkg0],
            format: 0,
        };
        assert!(matches!(
            VersionInfo::try_from(raw),
            Err(ValidationError::DependencyIndexOutOfBounds {
                package: 0,
                dependency: 1
            })
        ));
    }

    #[test]
    fn duplicate_dependency_edge() {
        let pkg0 = dummy_package(0, true, vec![1, 1]);
        let pkg1 = dummy_package(1, false, vec![]);
        let raw = RawVersionInfo {
            packages: vec![pkg0, pkg1],
            format: 0,
        };
        assert!(matches!(
            VersionInfo::try_from(raw),
            Err(ValidationError::DuplicateDependency {
                package: 0,
                dependency: 1
            })
        ));
    }
}
