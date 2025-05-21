use super::binary::{Node, Value};

const INDENT_XML: bool = false;
const MAX_BYTES_TO_PRINT_AS_HEX: usize = 128;

impl Node {
    pub fn to_xml(&self) -> String {
        self.to_xml_string_internal(0)
    }

    fn to_xml_string_internal(&self, level: usize) -> String {
        let indent = if INDENT_XML { "  ".repeat(level) } else { String::new() };
        let attr_str = self.attribute_string();
        let content = self.content_string(level + 1);

        if content.is_empty() {
            format!("{indent}<{}{} />", self.tag, attr_str)
        } else {
            let newline = if INDENT_XML { "\n" } else { "" };
            let mut s = format!("{indent}<{}{}>{newline}", self.tag, attr_str);
            for line in &content {
                s.push_str(line);
                s.push_str(newline);
            }
            s.push_str(&format!("{indent}</{}>", self.tag));
            s
        }
    }

    fn attribute_string(&self) -> String {
        if self.attributes.is_empty() {
            return String::new();
        }

        let mut entries: Vec<_> = self.attributes.iter().map(|(k, v)| {
            (k.clone(), Self::value_as_string(v))
        }).collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0)); // deterministic

        let joined = entries.into_iter()
            .map(|(k, v)| format!(r#"{}="{}""#, k, v))
            .collect::<Vec<_>>()
            .join(" ");

        format!(" {}", joined)
    }

    fn content_string(&self, level: usize) -> Vec<String> {
        match &self.content {
            Some(Value::List(nodes)) => nodes.iter().flat_map(|n| n.to_xml_string_internal(level).split('\n').map(|s| s.to_string()).collect::<Vec<_>>()).collect(),
            Some(Value::Node(n)) => n.to_xml_string_internal(level).split('\n').map(|s| s.to_string()).collect(),
            Some(Value::Bytes(bytes)) => Self::format_bytes(bytes, level),
            Some(Value::Str(s)) => Self::split_and_indent(s, level),
            Some(Value::Null) | None => vec![],
            Some(_) => vec![Self::indent_line("[unsupported]".into(), level)],
        }
    }

    fn value_as_string(v: &Value) -> String {
        match v {
            Value::Str(s) => s.clone(),
            Value::Jid(jid) => {
                let user = jid.user.as_deref().unwrap_or("");
                let server = jid.server.as_deref().unwrap_or("");
                format!("{}@{}", user, server)
            }
            Value::Bytes(b) => {
                if let Ok(s) = std::str::from_utf8(b) {
                    if s.chars().all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace()) {
                        return s.to_string();
                    }
                }
                hex::encode(b)
            }
            _ => "".to_string(),
        }
    }

    fn format_bytes(b: &[u8], level: usize) -> Vec<String> {
        if let Ok(s) = std::str::from_utf8(b) {
            if s.chars().all(|c| c.is_ascii_graphic() || c.is_ascii_whitespace()) {
                return Self::split_and_indent(s, level);
            }
        }

        if b.len() > MAX_BYTES_TO_PRINT_AS_HEX {
            vec![Self::indent_line(format!("<!-- {} bytes -->", b.len()), level)]
        } else if INDENT_XML {
            hex::encode(b)
                .as_bytes()
                .chunks(80)
                .map(|chunk| Self::indent_line(String::from_utf8_lossy(chunk).into(), level))
                .collect()
        } else {
            vec![Self::indent_line(hex::encode(b), level)]
        }
    }

    fn split_and_indent(s: &str, level: usize) -> Vec<String> {
        if INDENT_XML {
            s.lines().map(|l| Self::indent_line(l.to_string(), level)).collect()
        } else {
            vec![s.replace('\n', "\\n")]
        }
    }

    fn indent_line(s: String, level: usize) -> String {
        if INDENT_XML {
            format!("{}{}", "  ".repeat(level), s)
        } else {
            s
        }
    }
}