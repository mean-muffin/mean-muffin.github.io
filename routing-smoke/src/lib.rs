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

pub fn classify(_method: Method, path: &str) -> Result<Surface, ClassifyError> {
    if !path.starts_with('/')
        || path.contains('?')
        || path.contains('#')
        || (path.len() > 1 && path.contains("//"))
        || path.split('/').any(|segment| segment == "." || segment == "..")
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
    Missing,
    SurfaceMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    bindings: Vec<Binding>,
}

impl Plan {
    #[must_use]
    pub fn new(mut bindings: Vec<Binding>) -> Self {
        bindings.sort_by(|left, right| left.route_key.cmp(&right.route_key));
        assert!(
            bindings
                .windows(2)
                .all(|pair| pair[0].route_key != pair[1].route_key),
            "duplicate route key"
        );
        Self { bindings }
    }

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
        assert_eq!(classify(Method::Get, "/static/logo.svg"), Ok(Surface::Static));
        assert_eq!(classify(Method::Head, "/_/docs/rpc"), Ok(Surface::Docs));
        assert_eq!(classify(Method::Get, "/users/42"), Ok(Surface::Page));
        assert_eq!(
            classify(Method::Get, "/_/unknown"),
            Err(ClassifyError::ReservedInternal)
        );
    }

    #[test]
    fn static_or_docs_miss_cannot_become_a_page() {
        assert_eq!(classify(Method::Get, "/static/missing"), Ok(Surface::Static));
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
        ]);

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
    fn query_text_is_not_part_of_normalized_routing_input() {
        assert_eq!(
            classify(Method::Get, "/account?force=lambda"),
            Err(ClassifyError::InvalidPath)
        );
    }
}
