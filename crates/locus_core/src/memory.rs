use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Fact,
    Action,
    Decision,
    Observation,
    Feedback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextScope {
    Terminal,
    Editor,
    UserIntent,
    Errors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEvent {
    pub event_kind: EventKind,
    pub context_scope: ContextScope,
    pub source: String,
    pub payload: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_to: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extends: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reinforces: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contradicts: Option<Vec<String>>,
}

impl MemoryEvent {
    pub fn new(
        event_kind: EventKind,
        context_scope: ContextScope,
        source: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            event_kind,
            context_scope,
            source: source.into(),
            payload,
            related_to: None,
            extends: None,
            reinforces: None,
            contradicts: None,
        }
    }

    pub fn with_related_to(mut self, related_to: Vec<String>) -> Self {
        self.related_to = Some(related_to);
        self
    }

    pub fn with_extends(mut self, extends: Vec<String>) -> Self {
        self.extends = Some(extends);
        self
    }

    pub fn with_reinforces(mut self, reinforces: Vec<String>) -> Self {
        self.reinforces = Some(reinforces);
        self
    }

    pub fn with_contradicts(mut self, contradicts: Vec<String>) -> Self {
        self.contradicts = Some(contradicts);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_kind_serialization() {
        let kind = EventKind::Fact;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"fact\"");

        let decoded: EventKind = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, EventKind::Fact);
    }

    #[test]
    fn test_all_event_kinds() {
        let kinds = vec![
            EventKind::Fact,
            EventKind::Action,
            EventKind::Decision,
            EventKind::Observation,
            EventKind::Feedback,
        ];
        for kind in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            let decoded: EventKind = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, kind);
        }
    }

    #[test]
    fn test_context_scope_serialization() {
        let scope = ContextScope::Terminal;
        let json = serde_json::to_string(&scope).unwrap();
        assert_eq!(json, "\"terminal\"");
    }

    #[test]
    fn test_all_context_scopes() {
        let scopes = vec![
            ContextScope::Terminal,
            ContextScope::Editor,
            ContextScope::UserIntent,
            ContextScope::Errors,
        ];
        for scope in scopes {
            let json = serde_json::to_string(&scope).unwrap();
            let decoded: ContextScope = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, scope);
        }
    }

    #[test]
    fn test_memory_event_new() {
        let event = MemoryEvent::new(
            EventKind::Decision,
            ContextScope::Terminal,
            "test_source",
            serde_json::json!({"key": "value"}),
        );
        assert_eq!(event.event_kind, EventKind::Decision);
        assert_eq!(event.context_scope, ContextScope::Terminal);
        assert_eq!(event.source, "test_source");
        assert_eq!(event.related_to, None);
    }

    #[test]
    fn test_memory_event_with_related_to() {
        let event = MemoryEvent::new(
            EventKind::Fact,
            ContextScope::Editor,
            "source",
            serde_json::json!({}),
        )
        .with_related_to(vec!["ctx1".to_string(), "ctx2".to_string()]);

        assert_eq!(event.related_to, Some(vec!["ctx1".to_string(), "ctx2".to_string()]));
    }

    #[test]
    fn test_memory_event_serialization() {
        let event = MemoryEvent::new(
            EventKind::Action,
            ContextScope::Terminal,
            "runtime",
            serde_json::json!({"tool": "bash", "command": "ls"}),
        );

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event_kind\":\"action\""));
        assert!(json.contains("\"context_scope\":\"terminal\""));

        let decoded: MemoryEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.event_kind, EventKind::Action);
    }

    #[test]
    fn test_memory_event_skip_none_fields() {
        let event = MemoryEvent::new(
            EventKind::Fact,
            ContextScope::Terminal,
            "source",
            serde_json::json!({}),
        );

        let json = serde_json::to_string(&event).unwrap();
        assert!(!json.contains("related_to"));
        assert!(!json.contains("extends"));
    }
}
