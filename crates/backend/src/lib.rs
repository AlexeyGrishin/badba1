//! Backend library crate.
//!
//! Placeholder contents; real backend functionality will replace them.

/// Returns the name of this crate.
pub fn name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_is_backend() {
        assert_eq!(name(), "backend");
    }
}
