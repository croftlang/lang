/// Converts `InvoiceRepository` to `invoice_repository`.
pub fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = name.chars().collect();

    for (index, &c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            let previous_is_lower = index > 0 && chars[index - 1].is_lowercase();
            let next_is_lower = chars.get(index + 1).is_some_and(|n| n.is_lowercase());
            let previous_is_upper = index > 0 && chars[index - 1].is_uppercase();

            if index > 0 && (previous_is_lower || (previous_is_upper && next_is_lower)) {
                result.push('_');
            }

            result.extend(c.to_lowercase());
        } else {
            result.push(c);
        }
    }

    result
}

pub fn is_pascal_case(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_uppercase()) && !name.contains('_')
}

pub fn is_snake_case(name: &str) -> bool {
    name.chars()
        .all(|c| c.is_lowercase() || c.is_ascii_digit() || c == '_')
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_lowercase() || c == '_')
}

pub fn is_upper_snake_case(name: &str) -> bool {
    name.chars()
        .all(|c| c.is_uppercase() || c.is_ascii_digit() || c == '_')
        && name.chars().next().is_some_and(|c| c.is_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_to_snake_case() {
        assert_eq!(to_snake_case("InvoiceRepository"), "invoice_repository");
        assert_eq!(to_snake_case("HTTPClient"), "http_client");
        assert_eq!(to_snake_case("NonEmpty"), "non_empty");
    }

    #[test]
    fn recognises_casing() {
        assert!(is_pascal_case("Invoice"));
        assert!(!is_pascal_case("invoice"));
        assert!(is_snake_case("issue_day"));
        assert!(!is_snake_case("issueDay"));
        assert!(is_upper_snake_case("MAX_LINES"));
    }
}
