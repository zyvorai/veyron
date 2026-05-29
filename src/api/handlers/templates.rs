// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

#[cfg(feature = "web")]
use axum::{Json, Router, extract::Path, http::StatusCode, routing::get};
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use crate::templates::TEMPLATES;

/// Template response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateResponse {
    pub name: String,
    pub description: String,
    pub os_type: String,
    pub default_cpus: u32,
    pub default_memory: String,
    pub default_disk_size: String,
    pub tags: Vec<String>,
}

#[cfg(feature = "web")]
pub fn router() -> Router {
    Router::new()
        .route("/templates", get(list_templates))
        .route("/templates/{name}", get(get_template))
}

#[cfg(feature = "web")]
fn os_type_from_name(name: &str) -> String {
    if name.starts_with("ubuntu") {
        "ubuntu".to_string()
    } else if name.starts_with("fedora") {
        "fedora".to_string()
    } else if name.starts_with("centos") {
        "centos".to_string()
    } else if name.starts_with("debian") {
        "debian".to_string()
    } else if name.starts_with("rhel") {
        "rhel".to_string()
    } else if name.starts_with("almalinux") {
        "almalinux".to_string()
    } else if name.starts_with("rocky") {
        "rocky".to_string()
    } else if name.starts_with("opensuse") {
        "opensuse".to_string()
    } else if name.starts_with("alpine") {
        "alpine".to_string()
    } else if name.starts_with("arch") {
        "arch".to_string()
    } else if name.starts_with("oracle") {
        "oracle".to_string()
    } else if name.starts_with("windows") {
        "windows".to_string()
    } else if name.starts_with("freebsd") {
        "freebsd".to_string()
    } else if name.starts_with("flatcar") {
        "flatcar".to_string()
    } else if name.starts_with("talos") {
        "talos".to_string()
    } else {
        "linux".to_string()
    }
}

#[cfg(feature = "web")]
fn description_from_name(name: &str) -> String {
    let os = os_type_from_name(name);
    let version_part = name.strip_prefix(&os).unwrap_or("").trim_start_matches('-');
    if version_part.is_empty() {
        format!("{} VM template", capitalize(&os))
    } else {
        format!("{} {} VM template", capitalize(&os), version_part)
    }
}

#[cfg(feature = "web")]
fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[cfg(feature = "web")]
fn tags_from_os(os: &str) -> Vec<String> {
    match os {
        "windows" => vec!["windows".to_string(), "microsoft".to_string()],
        "freebsd" => vec!["bsd".to_string(), "unix".to_string()],
        "ubuntu" | "debian" => vec!["linux".to_string(), "debian-family".to_string()],
        "fedora" | "centos" | "rhel" | "almalinux" | "rocky" | "oracle" => {
            vec!["linux".to_string(), "rhel-family".to_string()]
        }
        "opensuse" => vec!["linux".to_string(), "suse-family".to_string()],
        "alpine" => vec!["linux".to_string(), "minimal".to_string()],
        "arch" => vec!["linux".to_string(), "rolling-release".to_string()],
        "flatcar" | "talos" => vec!["linux".to_string(), "container-os".to_string()],
        _ => vec!["linux".to_string()],
    }
}

#[cfg(feature = "web")]
async fn list_templates() -> Json<Vec<TemplateResponse>> {
    let names = TEMPLATES.list();
    let results: Vec<TemplateResponse> = names
        .iter()
        .filter_map(|name| {
            TEMPLATES.get(name).map(|config| {
                let os = os_type_from_name(name);
                TemplateResponse {
                    name: name.clone(),
                    description: description_from_name(name),
                    os_type: os.clone(),
                    default_cpus: config.cpu.cores,
                    default_memory: config.memory.size.clone(),
                    default_disk_size: config.default_disk_size_label(),
                    tags: tags_from_os(&os),
                }
            })
        })
        .collect();

    Json(results)
}

#[cfg(feature = "web")]
async fn get_template(Path(name): Path<String>) -> Result<Json<TemplateResponse>, StatusCode> {
    match TEMPLATES.get(&name) {
        Some(config) => {
            let os = os_type_from_name(&name);
            Ok(Json(TemplateResponse {
                name: name.clone(),
                description: description_from_name(&name),
                os_type: os.clone(),
                default_cpus: config.cpu.cores,
                default_memory: config.memory.size.clone(),
                default_disk_size: config.default_disk_size_label(),
                tags: tags_from_os(&os),
            }))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_type_detection() {
        assert_eq!(os_type_from_name("ubuntu-24.04"), "ubuntu");
        assert_eq!(os_type_from_name("fedora-41"), "fedora");
        assert_eq!(os_type_from_name("windows-server-2022"), "windows");
        assert_eq!(os_type_from_name("freebsd-14"), "freebsd");
        assert_eq!(os_type_from_name("alpine-3.20"), "alpine");
        assert_eq!(os_type_from_name("unknown-os"), "linux");
    }

    #[test]
    fn test_description_generation() {
        let desc = description_from_name("ubuntu-24.04");
        assert!(desc.contains("Ubuntu"));
        assert!(desc.contains("24.04"));

        let desc = description_from_name("fedora");
        assert!(desc.contains("Fedora"));
    }

    #[test]
    fn test_tags_generation() {
        let tags = tags_from_os("ubuntu");
        assert!(tags.contains(&"linux".to_string()));
        assert!(tags.contains(&"debian-family".to_string()));

        let tags = tags_from_os("windows");
        assert!(tags.contains(&"windows".to_string()));

        let tags = tags_from_os("flatcar");
        assert!(tags.contains(&"container-os".to_string()));
    }

    #[test]
    fn test_capitalize() {
        assert_eq!(capitalize("ubuntu"), "Ubuntu");
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("a"), "A");
    }

    #[test]
    fn test_template_response_serialization() {
        let resp = TemplateResponse {
            name: "test-vm".to_string(),
            description: "Test VM".to_string(),
            os_type: "linux".to_string(),
            default_cpus: 2,
            default_memory: "4Gi".to_string(),
            default_disk_size: "20Gi".to_string(),
            tags: vec!["linux".to_string()],
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("test-vm"));
        assert!(json.contains("4Gi"));

        let deserialized: TemplateResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test-vm");
        assert_eq!(deserialized.default_cpus, 2);
    }

    #[tokio::test]
    async fn test_list_templates_handler() {
        let result = list_templates().await;
        let templates = result.0;
        assert!(!templates.is_empty());

        // Verify all templates have required fields
        for t in &templates {
            assert!(!t.name.is_empty());
            assert!(!t.os_type.is_empty());
            assert!(t.default_cpus > 0);
            assert!(!t.default_memory.is_empty());
            assert!(!t.tags.is_empty());
        }
    }

    #[tokio::test]
    async fn test_get_template_found() {
        let result = get_template(axum::extract::Path("ubuntu-24.04".to_string())).await;
        assert!(result.is_ok());
        let template = result.unwrap().0;
        assert_eq!(template.name, "ubuntu-24.04");
        assert_eq!(template.os_type, "ubuntu");
    }

    #[tokio::test]
    async fn test_get_template_not_found() {
        let result = get_template(axum::extract::Path("nonexistent".to_string())).await;
        assert!(result.is_err());
    }
}
