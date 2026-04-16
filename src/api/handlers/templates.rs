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
                    default_disk_size: config
                        .disks
                        .first()
                        .map(|d| d.size.clone())
                        .unwrap_or_default(),
                    tags: tags_from_os(&os),
                }
            })
        })
        .collect();

    Json(results)
}

#[cfg(feature = "web")]
async fn get_template(
    Path(name): Path<String>,
) -> Result<Json<TemplateResponse>, StatusCode> {
    match TEMPLATES.get(&name) {
        Some(config) => {
            let os = os_type_from_name(&name);
            Ok(Json(TemplateResponse {
                name: name.clone(),
                description: description_from_name(&name),
                os_type: os.clone(),
                default_cpus: config.cpu.cores,
                default_memory: config.memory.size.clone(),
                default_disk_size: config
                    .disks
                    .first()
                    .map(|d| d.size.clone())
                    .unwrap_or_default(),
                tags: tags_from_os(&os),
            }))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}
