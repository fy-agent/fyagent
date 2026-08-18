use super::{SecretRef, SecretRefDisplay};

/// Public projection primitives. These never carry material and never insert
/// a `[REDACTED]` value placeholder.
pub(crate) fn secret_ref_display(secret_ref: &SecretRef) -> SecretRefDisplay {
    SecretRefDisplay::derive_from(secret_ref)
}

pub(crate) fn redact_owner_id_for_display(owner_id: &str) -> Option<String> {
    if owner_id.is_empty() {
        return None;
    }
    let tail = owner_id
        .chars()
        .rev()
        .take(2)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    Some(format!("…{tail}"))
}
