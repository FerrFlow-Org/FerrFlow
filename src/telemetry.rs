use serde::Serialize;

const DEFAULT_API_URL: &str = "https://api.ferrflow.com";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum EventType {
    Check,
    Release,
    VersionBump,
    Init,
    Error,
}

#[derive(Serialize)]
struct EventPayload {
    event_type: EventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    package_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    package_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
}

fn is_enabled() -> bool {
    check_enabled(std::env::var("FERRFLOW_TELEMETRY").ok().as_deref())
}

fn check_enabled(val: Option<&str>) -> bool {
    match val {
        Some(v) => !matches!(v.to_lowercase().as_str(), "false" | "0" | "off" | "no"),
        None => true,
    }
}

fn api_url() -> String {
    std::env::var("FERRFLOW_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_string())
}

pub fn send_event(
    event_type: EventType,
    package_name: Option<&str>,
    package_version: Option<&str>,
    metadata: Option<serde_json::Value>,
) {
    if !is_enabled() {
        return;
    }

    let payload = EventPayload {
        event_type,
        package_name: package_name.map(String::from),
        package_version: package_version.map(String::from),
        metadata,
    };

    let url = format!("{}/events", api_url());

    std::thread::spawn(move || {
        let agent = ureq::Agent::new_with_defaults();
        let _ = agent
            .post(&url)
            .header("Content-Type", "application/json")
            .send_json(&payload);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&EventType::Check).unwrap(),
            "\"check\""
        );
        assert_eq!(
            serde_json::to_string(&EventType::Release).unwrap(),
            "\"release\""
        );
        assert_eq!(
            serde_json::to_string(&EventType::VersionBump).unwrap(),
            "\"version_bump\""
        );
        assert_eq!(serde_json::to_string(&EventType::Init).unwrap(), "\"init\"");
        assert_eq!(
            serde_json::to_string(&EventType::Error).unwrap(),
            "\"error\""
        );
    }

    #[test]
    fn payload_skips_none_fields() {
        let payload = EventPayload {
            event_type: EventType::Check,
            package_name: None,
            package_version: None,
            metadata: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert!(!json.as_object().unwrap().contains_key("package_name"));
        assert!(!json.as_object().unwrap().contains_key("package_version"));
        assert!(!json.as_object().unwrap().contains_key("metadata"));
    }

    #[test]
    fn payload_includes_present_fields() {
        let payload = EventPayload {
            event_type: EventType::Release,
            package_name: Some("my-pkg".into()),
            package_version: Some("1.0.0".into()),
            metadata: None,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["event_type"], "release");
        assert_eq!(json["package_name"], "my-pkg");
        assert_eq!(json["package_version"], "1.0.0");
    }

    #[test]
    fn check_enabled_disabled_values() {
        for val in ["false", "0", "off", "no", "FALSE", "Off", "NO"] {
            assert!(!check_enabled(Some(val)), "should be disabled for {val}");
        }
    }

    #[test]
    fn check_enabled_default() {
        assert!(check_enabled(None));
    }

    #[test]
    fn check_enabled_true_values() {
        for val in ["true", "1", "yes", "anything"] {
            assert!(check_enabled(Some(val)), "should be enabled for {val}");
        }
    }
}
