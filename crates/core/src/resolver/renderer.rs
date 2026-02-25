// Template renderer - renders path/table/catalog templates
// Implements token substitution for template rendering

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateContext {
    Generic,
    Path,
    Endpoint,
}

/// Regex pattern for template placeholders: {token}
static TOKEN_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{([^{}]*)\}").expect("invalid token regex"));

/// Supported token format.
static TOKEN_NAME_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").expect("invalid token-name regex"));

/// Parse template and extract all token names
pub fn parse_template(template: &str) -> Vec<String> {
    TOKEN_PATTERN
        .captures_iter(template)
        .map(|cap| cap[1].to_string())
        .collect()
}

/// Render template with context substitution
/// Returns error if any token cannot be resolved
pub fn render_template(
    template: &str,
    context: &HashMap<String, String>,
) -> Result<String, String> {
    render_template_with_context(template, context, TemplateContext::Generic)
}

/// Render template for a specific output context.
pub fn render_template_with_context(
    template: &str,
    context: &HashMap<String, String>,
    template_context: TemplateContext,
) -> Result<String, String> {
    let mut result = template.to_string();
    let tokens = parse_template(template);

    for token in tokens {
        if !TOKEN_NAME_PATTERN.is_match(&token) {
            return Err(format!(
                "unknown token '{{{}}}' in template '{}'",
                token, template
            ));
        }
        match context.get(&token) {
            Some(value) => {
                validate_token_value(&token, value, template_context)?;
                let rendered_value = match template_context {
                    TemplateContext::Generic => value.to_string(),
                    TemplateContext::Path | TemplateContext::Endpoint => percent_encode(value),
                };
                let token_placeholder = format!("{{{}}}", token);
                result = result.replace(&token_placeholder, &rendered_value);
            }
            None => {
                return Err(format!(
                    "unknown token '{{{}}}' in template '{}'",
                    token, template
                ));
            }
        }
    }

    Ok(result)
}

fn validate_token_value(
    token: &str,
    value: &str,
    template_context: TemplateContext,
) -> Result<(), String> {
    if value.contains("..") {
        return Err(format!(
            "unsafe value for token '{}': parent traversal is not allowed",
            token
        ));
    }

    if template_context == TemplateContext::Generic && (value.contains('/') || value.contains('\\'))
    {
        return Err(format!(
            "unsafe value for token '{}': path separators are not allowed",
            token
        ));
    }

    if value.chars().any(char::is_control) {
        return Err(format!(
            "unsafe value for token '{}': control characters are not allowed",
            token
        ));
    }

    Ok(())
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        let is_unreserved =
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~');
        if is_unreserved {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{:02X}", byte));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_template_extracts_tokens() {
        let tokens = parse_template("/data/{period_id}/{table_name}.parquet");
        assert_eq!(tokens, vec!["period_id", "table_name"]);
    }

    #[test]
    fn test_parse_template_no_tokens() {
        let tokens = parse_template("/data/static/path.parquet");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_render_template_success() {
        let mut context = HashMap::new();
        context.insert("period_id".to_string(), "2024-01".to_string());
        context.insert("table_name".to_string(), "sales".to_string());

        let result = render_template("/data/{period_id}/{table_name}.parquet", &context);
        assert_eq!(result.unwrap(), "/data/2024-01/sales.parquet");
    }

    #[test]
    fn test_render_template_unknown_token() {
        let context = HashMap::new();
        let result = render_template("/data/{unknown_token}/file.parquet", &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown_token"));
    }

    #[test]
    fn test_render_template_rejects_invalid_placeholder_token() {
        let context = HashMap::new();
        let result = render_template("/data/{unknown-token}/file.parquet", &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown-token"));
    }

    #[test]
    fn test_render_template_rejects_parent_traversal_in_value() {
        let mut context = HashMap::new();
        context.insert("period_id".to_string(), "../secret".to_string());
        let result = render_template("/data/{period_id}/file.parquet", &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsafe value"));
    }

    #[test]
    fn test_render_template_rejects_separator_in_value() {
        let mut context = HashMap::new();
        context.insert("dataset_id".to_string(), "team/a".to_string());
        let result = render_template("/data/{dataset_id}/file.parquet", &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("path separators"));
    }

    #[test]
    fn test_render_template_with_context_percent_encodes_endpoint_values() {
        let mut context = HashMap::new();
        context.insert("table_name".to_string(), "sales report?#%".to_string());
        let result = render_template_with_context(
            "https://example.com/{table_name}",
            &context,
            TemplateContext::Endpoint,
        )
        .unwrap();
        assert_eq!(
            result,
            "https://example.com/sales%20report%3F%23%25".to_string()
        );
    }

    #[test]
    fn test_render_template_with_context_allows_separator_for_encoding() {
        let mut context = HashMap::new();
        context.insert("dataset_id".to_string(), "team/a".to_string());
        let result = render_template_with_context(
            "/data/{dataset_id}/file.parquet",
            &context,
            TemplateContext::Path,
        )
        .unwrap();
        assert_eq!(result, "/data/team%2Fa/file.parquet".to_string());
    }
}
