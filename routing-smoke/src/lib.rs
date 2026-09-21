#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Head,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Page,
    Static,
    Docs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassifyError {
    InvalidPath,
    ReservedInternal,
}

/// Classify a normalized GET/HEAD path before any resource lookup.
///
/// # Errors
///
/// Returns `ClassifyError::InvalidPath` for non-normalized path input and
/// `ClassifyError::ReservedInternal` for an unadmitted `/_/**` namespace.
pub fn classify(_method: Method, path: &str) -> Result<Surface, ClassifyError> {
    if !path.starts_with('/')
        || path.contains('?')
        || path.contains('#')
        || (path.len() > 1 && path.contains("//"))
        || path
            .split('/')
            .any(|segment| segment == "." || segment == "..")
    {
        return Err(ClassifyError::InvalidPath);
    }
    if has_boundary(path, "/static") {
        return Ok(Surface::Static);
    }
    if has_boundary(path, "/_/docs") {
        return Ok(Surface::Docs);
    }
    if has_boundary(path, "/_") {
        return Err(ClassifyError::ReservedInternal);
    }
    Ok(Surface::Page)
}

fn has_boundary(path: &str, prefix: &str) -> bool {
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|remainder| remainder.starts_with('/'))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Standalone { origin_id: String },
    Lambda { unit_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub route_key: String,
    pub surface: Surface,
    pub target: Target,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    Duplicate,
    Missing,
    SurfaceMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    bindings: Vec<Binding>,
}

impl Plan {
    /// Build an immutable route-key-to-execution-target plan.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError::Duplicate` when two bindings claim the same route
    /// key. Registration order never resolves an execution-placement conflict.
    pub fn new(mut bindings: Vec<Binding>) -> Result<Self, ResolveError> {
        bindings.sort_by(|left, right| left.route_key.cmp(&right.route_key));
        if bindings
            .windows(2)
            .any(|pair| pair[0].route_key == pair[1].route_key)
        {
            return Err(ResolveError::Duplicate);
        }
        Ok(Self { bindings })
    }

    /// Resolve a previously matched route key to its deployment target.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError::Missing` for an unknown route key and
    /// `ResolveError::SurfaceMismatch` if the caller presents a route key under
    /// the wrong previously-classified web surface.
    pub fn resolve(&self, surface: Surface, route_key: &str) -> Result<&Target, ResolveError> {
        let index = self
            .bindings
            .binary_search_by(|binding| binding.route_key.as_str().cmp(route_key))
            .map_err(|_| ResolveError::Missing)?;
        let binding = &self.bindings[index];
        if binding.surface != surface {
            return Err(ResolveError::SurfaceMismatch);
        }
        Ok(&binding.target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surfaces_are_disjoint_before_resource_lookup() {
        assert_eq!(
            classify(Method::Get, "/static/logo.svg"),
            Ok(Surface::Static)
        );
        assert_eq!(classify(Method::Head, "/_/docs/rpc"), Ok(Surface::Docs));
        assert_eq!(classify(Method::Get, "/users/42"), Ok(Surface::Page));
        assert_eq!(
            classify(Method::Get, "/_/unknown"),
            Err(ClassifyError::ReservedInternal)
        );
    }

    #[test]
    fn static_or_docs_miss_cannot_become_a_page() {
        assert_eq!(
            classify(Method::Get, "/static/missing"),
            Ok(Surface::Static)
        );
        assert_eq!(classify(Method::Get, "/_/docs/missing"), Ok(Surface::Docs));
    }

    #[test]
    fn deployment_plan_not_request_metadata_selects_origin() {
        let plan = Plan::new(vec![
            Binding {
                route_key: "page:/".to_owned(),
                surface: Surface::Page,
                target: Target::Standalone {
                    origin_id: "web-origin".to_owned(),
                },
            },
            Binding {
                route_key: "page:/account".to_owned(),
                surface: Surface::Page,
                target: Target::Lambda {
                    unit_id: "lambda-account".to_owned(),
                },
            },
            Binding {
                route_key: "surface:static".to_owned(),
                surface: Surface::Static,
                target: Target::Lambda {
                    unit_id: "lambda-static".to_owned(),
                },
            },
            Binding {
                route_key: "surface:docs".to_owned(),
                surface: Surface::Docs,
                target: Target::Standalone {
                    origin_id: "web-origin".to_owned(),
                },
            },
        ])
        .unwrap();

        assert!(matches!(
            plan.resolve(Surface::Page, "page:/").unwrap(),
            Target::Standalone { .. }
        ));
        assert!(matches!(
            plan.resolve(Surface::Page, "page:/account").unwrap(),
            Target::Lambda { .. }
        ));
        assert!(matches!(
            plan.resolve(Surface::Static, "surface:static").unwrap(),
            Target::Lambda { .. }
        ));
    }

    #[test]
    fn duplicate_route_key_fails_closed() {
        assert_eq!(
            Plan::new(vec![
                Binding {
                    route_key: "page:/".to_owned(),
                    surface: Surface::Page,
                    target: Target::Standalone {
                        origin_id: "a".to_owned(),
                    },
                },
                Binding {
                    route_key: "page:/".to_owned(),
                    surface: Surface::Page,
                    target: Target::Lambda {
                        unit_id: "b".to_owned(),
                    },
                },
            ]),
            Err(ResolveError::Duplicate)
        );
    }

    #[test]
    fn query_text_is_not_part_of_normalized_routing_input() {
        assert_eq!(
            classify(Method::Get, "/account?force=lambda"),
            Err(ClassifyError::InvalidPath)
        );
    }
}
