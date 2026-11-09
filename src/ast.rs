use anyhow::{bail, Context, Result};
use kdl::{KdlDocument, KdlEntry, KdlNode, KdlValue};

/// Navigates a KdlDocument using a dot-separated path (e.g. "layout.gaps" or "session.socket-wait-timeout")
/// and returns the value as a String if found.
pub fn get_value(doc: &KdlDocument, path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return None;
    }

    let mut current_doc = doc;
    for (i, &part) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;

        if is_last {
            // Check if node exists and has an argument value
            if let Some(val) = current_doc.get_arg(part) {
                return Some(kdl_value_to_string(val));
            }
            // Or node exists and has a child or prop with this name
            if let Some(node) = current_doc.get(part) {
                // If the node has entries, take the first entry's value
                if let Some(entry) = node.entries().first() {
                    return Some(kdl_value_to_string(entry.value()));
                }
            }
            return None;
        } else {
            // Intermediate node: navigate into its children
            let node = current_doc.get(part)?;
            current_doc = node.children()?;
        }
    }

    None
}

/// Sets or creates a property at the specified dot-separated path.
/// Automatically infers bool, integer, float, or string.
pub fn set_value(doc: &mut KdlDocument, path: &str, raw_value: &str) -> Result<()> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        bail!("Path cannot be empty");
    }

    let val = parse_raw_value(raw_value);

    let mut current_doc = doc;
    for (i, &part) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;

        if is_last {
            // Find existing node or create one
            if let Some(existing_node) = current_doc.get_mut(part) {
                // Update first argument or replace entry
                if let Some(entry) = existing_node.entries_mut().first_mut() {
                    *entry = KdlEntry::new(val);
                } else {
                    existing_node.push(KdlEntry::new(val));
                }
            } else {
                // Create new node with argument
                let mut node = KdlNode::new(part);
                node.push(KdlEntry::new(val));
                current_doc.nodes_mut().push(node);
            }
            return Ok(());
        } else {
            // Ensure intermediate node exists
            if current_doc.get(part).is_none() {
                let mut new_node = KdlNode::new(part);
                new_node.ensure_children();
                current_doc.nodes_mut().push(new_node);
            }

            let node = current_doc
                .get_mut(part)
                .with_context(|| format!("Failed to access node {}", part))?;
            current_doc = node.ensure_children();
        }
    }

    Ok(())
}

/// Recursively traverses a KdlDocument and returns all leaf properties as (dot_path, value) pairs.
pub fn list_values(doc: &KdlDocument) -> Vec<(String, String)> {
    let mut results = Vec::new();
    traverse_nodes(doc.nodes(), "", &mut results);
    results
}

fn traverse_nodes(nodes: &[KdlNode], prefix: &str, results: &mut Vec<(String, String)>) {
    for node in nodes {
        let name = node.name().value();
        let current_path = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}.{}", prefix, name)
        };

        // If the node has entries (values), record them
        if let Some(first) = node.entries().first() {
            results.push((current_path.clone(), kdl_value_to_string(first.value())));
        }

        // If the node has children, recurse
        if let Some(children) = node.children() {
            traverse_nodes(children.nodes(), &current_path, results);
        }
    }
}

/// Converts a KdlValue to its clean string representation
pub fn kdl_value_to_string(val: &KdlValue) -> String {
    match val {
        KdlValue::RawString(s) | KdlValue::String(s) => s.clone(),
        KdlValue::Base10(i) | KdlValue::Base2(i) | KdlValue::Base8(i) | KdlValue::Base16(i) => {
            i.to_string()
        }
        KdlValue::Base10Float(f) => f.to_string(),
        KdlValue::Bool(b) => b.to_string(),
        KdlValue::Null => "null".to_string(),
    }
}

/// Parses a string into the most appropriate KdlValue
pub fn parse_raw_value(s: &str) -> KdlValue {
    if s == "true" {
        return KdlValue::Bool(true);
    }
    if s == "false" {
        return KdlValue::Bool(false);
    }
    if s == "null" {
        return KdlValue::Null;
    }
    if let Ok(i) = s.parse::<i64>() {
        return KdlValue::Base10(i);
    }
    if let Ok(f) = s.parse::<f64>() {
        return KdlValue::Base10Float(f);
    }
    // Default to string
    KdlValue::String(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_value_flat_and_nested() {
        let input = r#"
// Top level comment
socket-wait-timeout 10
autostart true

layout {
    gaps 16
    center-focused-column "never"
}
"#;
        let doc: KdlDocument = input.parse().unwrap();

        assert_eq!(get_value(&doc, "socket-wait-timeout"), Some("10".to_string()));
        assert_eq!(get_value(&doc, "autostart"), Some("true".to_string()));
        assert_eq!(get_value(&doc, "layout.gaps"), Some("16".to_string()));
        assert_eq!(
            get_value(&doc, "layout.center-focused-column"),
            Some("never".to_string())
        );
        assert_eq!(get_value(&doc, "nonexistent"), None);
        assert_eq!(get_value(&doc, "layout.nonexistent"), None);
    }

    #[test]
    fn test_set_value_existing() {
        let input = r#"
layout {
    gaps 16
}
"#;
        let mut doc: KdlDocument = input.parse().unwrap();
        set_value(&mut doc, "layout.gaps", "24").unwrap();

        assert_eq!(get_value(&doc, "layout.gaps"), Some("24".to_string()));
        let rendered = doc.to_string();
        assert!(rendered.contains("24"));
    }

    #[test]
    fn test_set_value_new_nested() {
        let input = r#"
layout {
    gaps 16
}
"#;
        let mut doc: KdlDocument = input.parse().unwrap();
        set_value(&mut doc, "appearance.theme", "dark").unwrap();

        assert_eq!(get_value(&doc, "appearance.theme"), Some("dark".to_string()));
        let rendered = doc.to_string();
        assert!(rendered.contains("appearance"));
        assert!(rendered.contains("dark"));
    }

    #[test]
    fn test_list_values() {
        let input = r#"
socket-wait-timeout 10

layout {
    gaps 16
    border "solid"
}
"#;
        let doc: KdlDocument = input.parse().unwrap();
        let list = list_values(&doc);

        assert!(list.iter().any(|(k, v)| k == "socket-wait-timeout" && v == "10"));
        assert!(list.iter().any(|(k, v)| k == "layout.gaps" && v == "16"));
        assert!(list.iter().any(|(k, v)| k == "layout.border" && v == "solid"));
    }
}
