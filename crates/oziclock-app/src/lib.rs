//! Application-layer composition points for OziClock use cases.

/// Returns the product name owned by the domain layer.
pub fn application_name() -> &'static str {
    oziclock_domain::PRODUCT_NAME
}
