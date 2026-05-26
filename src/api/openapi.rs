// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OpenAPI specification generator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: ApiInfo,
    pub servers: Vec<ServerInfo>,
    pub paths: HashMap<String, PathItem>,
    pub components: Components,
    pub tags: Vec<Tag>,
}

/// API information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiInfo {
    pub title: String,
    pub description: String,
    pub version: String,
    pub contact: Option<ContactInfo>,
    pub license: Option<LicenseInfo>,
}

/// Contact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    pub name: String,
    pub url: Option<String>,
    pub email: Option<String>,
}

/// License information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    pub name: String,
    pub url: Option<String>,
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub url: String,
    pub description: String,
}

/// Path item (operations on a path)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathItem {
    pub operations: HashMap<String, Operation>,
}

/// API operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub summary: String,
    pub description: Option<String>,
    pub operation_id: String,
    pub tags: Vec<String>,
    pub parameters: Vec<Parameter>,
    pub responses: HashMap<String, ResponseSpec>,
    pub security: Vec<String>,
}

/// Request parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub location: ParameterLocation,
    pub required: bool,
    pub description: String,
    pub schema_type: String,
}

/// Parameter location
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterLocation {
    Path,
    Query,
    Header,
    Cookie,
}

impl std::fmt::Display for ParameterLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParameterLocation::Path => write!(f, "path"),
            ParameterLocation::Query => write!(f, "query"),
            ParameterLocation::Header => write!(f, "header"),
            ParameterLocation::Cookie => write!(f, "cookie"),
        }
    }
}

/// Response specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseSpec {
    pub description: String,
    pub content_type: Option<String>,
    pub schema_ref: Option<String>,
}

/// Components (reusable schemas)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
    pub schemas: HashMap<String, SchemaDefinition>,
    pub security_schemes: HashMap<String, SecurityScheme>,
}

/// Schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDefinition {
    pub schema_type: String,
    pub description: String,
    pub properties: HashMap<String, PropertyDef>,
    pub required: Vec<String>,
}

/// Property definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyDef {
    pub prop_type: String,
    pub description: String,
    pub example: Option<String>,
}

/// Security scheme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScheme {
    pub scheme_type: String,
    pub scheme: Option<String>,
    pub bearer_format: Option<String>,
    pub description: String,
}

/// API tag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub description: String,
}

impl OpenApiSpec {
    pub fn new() -> Self {
        Self {
            openapi: "3.0.3".to_string(),
            info: ApiInfo {
                title: "VMRogue API".to_string(),
                description: "REST API for managing KubeVirt virtual machines".to_string(),
                version: "1.0.0".to_string(),
                contact: Some(ContactInfo {
                    name: "VMRogue".to_string(),
                    url: Some("https://github.com/ssahani/VMRogue".to_string()),
                    email: None,
                }),
                license: Some(LicenseInfo {
                    name: "MIT OR Apache-2.0".to_string(),
                    url: None,
                }),
            },
            servers: vec![ServerInfo {
                url: "http://localhost:8080".to_string(),
                description: "Local development server".to_string(),
            }],
            paths: HashMap::new(),
            components: Components {
                schemas: HashMap::new(),
                security_schemes: HashMap::new(),
            },
            tags: Vec::new(),
        }
    }

    pub fn add_server(&mut self, url: &str, description: &str) {
        self.servers.push(ServerInfo {
            url: url.to_string(),
            description: description.to_string(),
        });
    }

    pub fn add_tag(&mut self, name: &str, description: &str) {
        self.tags.push(Tag {
            name: name.to_string(),
            description: description.to_string(),
        });
    }

    pub fn add_path(&mut self, path: &str, item: PathItem) {
        self.paths.insert(path.to_string(), item);
    }

    pub fn add_schema(&mut self, name: &str, schema: SchemaDefinition) {
        self.components.schemas.insert(name.to_string(), schema);
    }

    pub fn add_security_scheme(&mut self, name: &str, scheme: SecurityScheme) {
        self.components
            .security_schemes
            .insert(name.to_string(), scheme);
    }

    pub fn path_count(&self) -> usize {
        self.paths.len()
    }

    pub fn schema_count(&self) -> usize {
        self.components.schemas.len()
    }

    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }

    pub fn operation_count(&self) -> usize {
        self.paths.values().map(|p| p.operations.len()).sum()
    }
}

impl Default for OpenApiSpec {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate the default VMRogue OpenAPI spec
pub fn generate_default_spec() -> OpenApiSpec {
    let mut spec = OpenApiSpec::new();

    // Add tags
    spec.add_tag("vms", "Virtual Machine operations");
    spec.add_tag("templates", "VM template management");
    spec.add_tag("profiles", "Resource profile management");
    spec.add_tag("blueprints", "Multi-VM blueprint management");
    spec.add_tag("snapshots", "VM snapshot operations");
    spec.add_tag("events", "Activity event tracking");
    spec.add_tag("health", "Health check endpoints");
    spec.add_tag("rdp", "Windows RDP remote desktop sessions");

    // Add security scheme
    spec.add_security_scheme(
        "bearerAuth",
        SecurityScheme {
            scheme_type: "http".to_string(),
            scheme: Some("bearer".to_string()),
            bearer_format: Some("JWT".to_string()),
            description: "Bearer token authentication".to_string(),
        },
    );

    spec.add_security_scheme(
        "apiKey",
        SecurityScheme {
            scheme_type: "apiKey".to_string(),
            scheme: None,
            bearer_format: None,
            description: "API key authentication via X-API-Key header".to_string(),
        },
    );

    // Add VM schema
    let mut vm_props = HashMap::new();
    vm_props.insert(
        "name".to_string(),
        PropertyDef {
            prop_type: "string".to_string(),
            description: "VM name".to_string(),
            example: Some("my-vm".to_string()),
        },
    );
    vm_props.insert(
        "namespace".to_string(),
        PropertyDef {
            prop_type: "string".to_string(),
            description: "Kubernetes namespace".to_string(),
            example: Some("default".to_string()),
        },
    );
    vm_props.insert(
        "cpu_cores".to_string(),
        PropertyDef {
            prop_type: "integer".to_string(),
            description: "Number of CPU cores".to_string(),
            example: Some("4".to_string()),
        },
    );
    vm_props.insert(
        "memory".to_string(),
        PropertyDef {
            prop_type: "string".to_string(),
            description: "Memory size".to_string(),
            example: Some("8Gi".to_string()),
        },
    );
    vm_props.insert(
        "status".to_string(),
        PropertyDef {
            prop_type: "string".to_string(),
            description: "VM status".to_string(),
            example: Some("Running".to_string()),
        },
    );

    spec.add_schema(
        "VirtualMachine",
        SchemaDefinition {
            schema_type: "object".to_string(),
            description: "A KubeVirt virtual machine".to_string(),
            properties: vm_props,
            required: vec!["name".to_string(), "namespace".to_string()],
        },
    );

    // Add VM list path
    let mut list_ops = HashMap::new();
    list_ops.insert(
        "get".to_string(),
        Operation {
            summary: "List virtual machines".to_string(),
            description: Some(
                "Returns a list of all virtual machines in the namespace".to_string(),
            ),
            operation_id: "listVMs".to_string(),
            tags: vec!["vms".to_string()],
            parameters: vec![Parameter {
                name: "namespace".to_string(),
                location: ParameterLocation::Query,
                required: false,
                description: "Filter by namespace".to_string(),
                schema_type: "string".to_string(),
            }],
            responses: {
                let mut r = HashMap::new();
                r.insert(
                    "200".to_string(),
                    ResponseSpec {
                        description: "List of VMs".to_string(),
                        content_type: Some("application/json".to_string()),
                        schema_ref: Some("#/components/schemas/VirtualMachine".to_string()),
                    },
                );
                r
            },
            security: Vec::new(),
        },
    );

    spec.add_path(
        "/api/v1/vms",
        PathItem {
            operations: list_ops,
        },
    );

    // Add health path
    let mut health_ops = HashMap::new();
    health_ops.insert(
        "get".to_string(),
        Operation {
            summary: "Health check".to_string(),
            description: Some("Returns the health status of the API".to_string()),
            operation_id: "healthCheck".to_string(),
            tags: vec!["health".to_string()],
            parameters: Vec::new(),
            responses: {
                let mut r = HashMap::new();
                r.insert(
                    "200".to_string(),
                    ResponseSpec {
                        description: "API is healthy".to_string(),
                        content_type: Some("application/json".to_string()),
                        schema_ref: None,
                    },
                );
                r
            },
            security: Vec::new(),
        },
    );

    spec.add_path(
        "/api/v1/health",
        PathItem {
            operations: health_ops,
        },
    );

    // Add ActivityEvent schema
    let mut event_props = HashMap::new();
    event_props.insert(
        "event_id".to_string(),
        PropertyDef {
            prop_type: "string".to_string(),
            description: "Unique event identifier".to_string(),
            example: Some("evt-20260401-120000-000000".to_string()),
        },
    );
    event_props.insert(
        "event_type".to_string(),
        PropertyDef {
            prop_type: "string".to_string(),
            description: "Event type".to_string(),
            example: Some("vm.started".to_string()),
        },
    );
    event_props.insert(
        "source".to_string(),
        PropertyDef {
            prop_type: "string".to_string(),
            description: "Source VM or resource".to_string(),
            example: Some("web-server-01".to_string()),
        },
    );
    event_props.insert(
        "timestamp".to_string(),
        PropertyDef {
            prop_type: "string".to_string(),
            description: "ISO 8601 timestamp".to_string(),
            example: Some("2026-04-01T12:00:00Z".to_string()),
        },
    );

    spec.add_schema(
        "ActivityEvent",
        SchemaDefinition {
            schema_type: "object".to_string(),
            description: "An activity event from VM operations".to_string(),
            properties: event_props,
            required: vec![
                "event_id".to_string(),
                "event_type".to_string(),
                "source".to_string(),
                "timestamp".to_string(),
            ],
        },
    );

    // Add events list path
    let mut events_ops = HashMap::new();
    events_ops.insert(
        "get".to_string(),
        Operation {
            summary: "List activity events".to_string(),
            description: Some(
                "Returns a list of all activity events, optionally filtered by VM".to_string(),
            ),
            operation_id: "listEvents".to_string(),
            tags: vec!["events".to_string()],
            parameters: vec![
                Parameter {
                    name: "vm".to_string(),
                    location: ParameterLocation::Query,
                    required: false,
                    description: "Filter events by VM name".to_string(),
                    schema_type: "string".to_string(),
                },
                Parameter {
                    name: "limit".to_string(),
                    location: ParameterLocation::Query,
                    required: false,
                    description: "Maximum number of events to return".to_string(),
                    schema_type: "integer".to_string(),
                },
            ],
            responses: {
                let mut r = HashMap::new();
                r.insert(
                    "200".to_string(),
                    ResponseSpec {
                        description: "List of activity events".to_string(),
                        content_type: Some("application/json".to_string()),
                        schema_ref: Some("#/components/schemas/ActivityEvent".to_string()),
                    },
                );
                r
            },
            security: Vec::new(),
        },
    );

    spec.add_path(
        "/api/v1/events",
        PathItem {
            operations: events_ops,
        },
    );

    // Add recent events path
    let mut recent_ops = HashMap::new();
    recent_ops.insert(
        "get".to_string(),
        Operation {
            summary: "List recent activity events".to_string(),
            description: Some("Returns the most recent activity events".to_string()),
            operation_id: "listRecentEvents".to_string(),
            tags: vec!["events".to_string()],
            parameters: vec![Parameter {
                name: "limit".to_string(),
                location: ParameterLocation::Query,
                required: false,
                description: "Maximum number of events to return".to_string(),
                schema_type: "integer".to_string(),
            }],
            responses: {
                let mut r = HashMap::new();
                r.insert(
                    "200".to_string(),
                    ResponseSpec {
                        description: "List of recent events".to_string(),
                        content_type: Some("application/json".to_string()),
                        schema_ref: Some("#/components/schemas/ActivityEvent".to_string()),
                    },
                );
                r
            },
            security: Vec::new(),
        },
    );

    spec.add_path(
        "/api/v1/events/recent",
        PathItem {
            operations: recent_ops,
        },
    );

    // Add VM-specific events path
    let mut vm_events_ops = HashMap::new();
    vm_events_ops.insert(
        "get".to_string(),
        Operation {
            summary: "List events for a specific VM".to_string(),
            description: Some("Returns activity events filtered by VM name".to_string()),
            operation_id: "listVMEvents".to_string(),
            tags: vec!["events".to_string()],
            parameters: vec![Parameter {
                name: "name".to_string(),
                location: ParameterLocation::Path,
                required: true,
                description: "VM name".to_string(),
                schema_type: "string".to_string(),
            }],
            responses: {
                let mut r = HashMap::new();
                r.insert(
                    "200".to_string(),
                    ResponseSpec {
                        description: "List of events for the VM".to_string(),
                        content_type: Some("application/json".to_string()),
                        schema_ref: Some("#/components/schemas/ActivityEvent".to_string()),
                    },
                );
                r
            },
            security: Vec::new(),
        },
    );

    spec.add_path(
        "/api/v1/events/vm/{name}",
        PathItem {
            operations: vm_events_ops,
        },
    );

    // Add RDP session schema
    let mut rdp_session_props = HashMap::new();
    rdp_session_props.insert(
        "session_id".to_string(),
        PropertyDef {
            prop_type: "string".to_string(),
            description: "Unique RDP session identifier".to_string(),
            example: Some("rdp-1712419200000000".to_string()),
        },
    );
    rdp_session_props.insert(
        "vm_name".to_string(),
        PropertyDef {
            prop_type: "string".to_string(),
            description: "Target VM name".to_string(),
            example: Some("windows-server-01".to_string()),
        },
    );
    rdp_session_props.insert(
        "state".to_string(),
        PropertyDef {
            prop_type: "string".to_string(),
            description: "Session state".to_string(),
            example: Some("Connected".to_string()),
        },
    );
    rdp_session_props.insert(
        "websocket_url".to_string(),
        PropertyDef {
            prop_type: "string".to_string(),
            description: "WebSocket endpoint for this session".to_string(),
            example: Some("/api/v1/ws/rdp/rdp-1712419200000000".to_string()),
        },
    );
    rdp_session_props.insert(
        "resolution".to_string(),
        PropertyDef {
            prop_type: "string".to_string(),
            description: "Display resolution".to_string(),
            example: Some("1920x1080".to_string()),
        },
    );

    spec.add_schema(
        "RdpSession",
        SchemaDefinition {
            schema_type: "object".to_string(),
            description: "An RDP remote desktop session to a Windows VM".to_string(),
            properties: rdp_session_props,
            required: vec![
                "session_id".to_string(),
                "vm_name".to_string(),
                "state".to_string(),
            ],
        },
    );

    // Add RDP session list path
    let mut rdp_list_ops = HashMap::new();
    rdp_list_ops.insert(
        "get".to_string(),
        Operation {
            summary: "List active RDP sessions".to_string(),
            description: Some("Returns all active Windows RDP sessions".to_string()),
            operation_id: "listRdpSessions".to_string(),
            tags: vec!["rdp".to_string()],
            parameters: Vec::new(),
            responses: {
                let mut r = HashMap::new();
                r.insert(
                    "200".to_string(),
                    ResponseSpec {
                        description: "List of RDP sessions".to_string(),
                        content_type: Some("application/json".to_string()),
                        schema_ref: Some("#/components/schemas/RdpSession".to_string()),
                    },
                );
                r
            },
            security: Vec::new(),
        },
    );
    rdp_list_ops.insert(
        "post".to_string(),
        Operation {
            summary: "Create a new RDP session".to_string(),
            description: Some(
                "Creates a new RDP session to a Windows VM and returns a WebSocket URL for connection"
                    .to_string(),
            ),
            operation_id: "createRdpSession".to_string(),
            tags: vec!["rdp".to_string()],
            parameters: Vec::new(),
            responses: {
                let mut r = HashMap::new();
                r.insert(
                    "200".to_string(),
                    ResponseSpec {
                        description: "Created RDP session with WebSocket URL".to_string(),
                        content_type: Some("application/json".to_string()),
                        schema_ref: Some("#/components/schemas/RdpSession".to_string()),
                    },
                );
                r
            },
            security: vec!["bearerAuth".to_string()],
        },
    );

    spec.add_path(
        "/api/v1/rdp/sessions",
        PathItem {
            operations: rdp_list_ops,
        },
    );

    // Add RDP session detail path
    let mut rdp_detail_ops = HashMap::new();
    rdp_detail_ops.insert(
        "get".to_string(),
        Operation {
            summary: "Get RDP session details".to_string(),
            description: Some("Returns details for a specific RDP session".to_string()),
            operation_id: "getRdpSession".to_string(),
            tags: vec!["rdp".to_string()],
            parameters: vec![Parameter {
                name: "id".to_string(),
                location: ParameterLocation::Path,
                required: true,
                description: "RDP session ID".to_string(),
                schema_type: "string".to_string(),
            }],
            responses: {
                let mut r = HashMap::new();
                r.insert(
                    "200".to_string(),
                    ResponseSpec {
                        description: "RDP session details".to_string(),
                        content_type: Some("application/json".to_string()),
                        schema_ref: Some("#/components/schemas/RdpSession".to_string()),
                    },
                );
                r
            },
            security: Vec::new(),
        },
    );
    rdp_detail_ops.insert(
        "delete".to_string(),
        Operation {
            summary: "Delete an RDP session".to_string(),
            description: Some("Terminates and removes an RDP session".to_string()),
            operation_id: "deleteRdpSession".to_string(),
            tags: vec!["rdp".to_string()],
            parameters: vec![Parameter {
                name: "id".to_string(),
                location: ParameterLocation::Path,
                required: true,
                description: "RDP session ID".to_string(),
                schema_type: "string".to_string(),
            }],
            responses: {
                let mut r = HashMap::new();
                r.insert(
                    "200".to_string(),
                    ResponseSpec {
                        description: "Session deleted".to_string(),
                        content_type: Some("application/json".to_string()),
                        schema_ref: None,
                    },
                );
                r
            },
            security: vec!["bearerAuth".to_string()],
        },
    );

    spec.add_path(
        "/api/v1/rdp/sessions/{id}",
        PathItem {
            operations: rdp_detail_ops,
        },
    );

    // Add RDP VM discovery path
    let mut rdp_vms_ops = HashMap::new();
    rdp_vms_ops.insert(
        "get".to_string(),
        Operation {
            summary: "List RDP-capable VMs".to_string(),
            description: Some("Returns VMs that support Windows RDP connections".to_string()),
            operation_id: "listRdpCapableVms".to_string(),
            tags: vec!["rdp".to_string()],
            parameters: Vec::new(),
            responses: {
                let mut r = HashMap::new();
                r.insert(
                    "200".to_string(),
                    ResponseSpec {
                        description: "List of RDP-capable VMs".to_string(),
                        content_type: Some("application/json".to_string()),
                        schema_ref: None,
                    },
                );
                r
            },
            security: Vec::new(),
        },
    );

    spec.add_path(
        "/api/v1/rdp/vms",
        PathItem {
            operations: rdp_vms_ops,
        },
    );

    let ns_name_params = || {
        vec![
            Parameter {
                name: "ns".to_string(),
                location: ParameterLocation::Path,
                required: true,
                description: "Namespace".to_string(),
                schema_type: "string".to_string(),
            },
            Parameter {
                name: "name".to_string(),
                location: ParameterLocation::Path,
                required: true,
                description: "VirtualMachine name".to_string(),
                schema_type: "string".to_string(),
            },
        ]
    };

    let mut rdp_expose_get = HashMap::new();
    rdp_expose_get.insert(
        "get".to_string(),
        Operation {
            summary: "Get RDP NodePort exposure status".to_string(),
            description: Some(
                "Returns guest IP, Windows detection, NodePort Service status, and suggested ports."
                    .to_string(),
            ),
            operation_id: "getVmRdpExpose".to_string(),
            tags: vec!["vms".to_string(), "rdp".to_string()],
            parameters: ns_name_params(),
            responses: {
                let mut r = HashMap::new();
                r.insert(
                    "200".to_string(),
                    ResponseSpec {
                        description: "RDP access status".to_string(),
                        content_type: Some("application/json".to_string()),
                        schema_ref: None,
                    },
                );
                r
            },
            security: vec!["apiKey".to_string()],
        },
    );
    let mut rdp_expose_put = rdp_expose_get.clone();
    rdp_expose_put.insert(
        "put".to_string(),
        Operation {
            summary: "Create or remove RDP NodePort exposure".to_string(),
            description: Some(
                "Body: { \"enabled\": true, \"service_type\": \"NodePort\", \"node_port\": 30101 }"
                    .to_string(),
            ),
            operation_id: "putVmRdpExpose".to_string(),
            tags: vec!["vms".to_string(), "rdp".to_string()],
            parameters: ns_name_params(),
            responses: {
                let mut r = HashMap::new();
                r.insert(
                    "200".to_string(),
                    ResponseSpec {
                        description: "Updated RDP access status".to_string(),
                        content_type: Some("application/json".to_string()),
                        schema_ref: None,
                    },
                );
                r
            },
            security: vec!["apiKey".to_string()],
        },
    );
    let mut rdp_expose_delete = HashMap::new();
    rdp_expose_delete.insert(
        "delete".to_string(),
        Operation {
            summary: "Delete RDP NodePort Service".to_string(),
            description: None,
            operation_id: "deleteVmRdpExpose".to_string(),
            tags: vec!["vms".to_string(), "rdp".to_string()],
            parameters: ns_name_params(),
            responses: {
                let mut r = HashMap::new();
                r.insert(
                    "200".to_string(),
                    ResponseSpec {
                        description: "RDP Service removed".to_string(),
                        content_type: Some("application/json".to_string()),
                        schema_ref: None,
                    },
                );
                r
            },
            security: vec!["apiKey".to_string()],
        },
    );
    let mut rdp_expose_ops = rdp_expose_get;
    rdp_expose_ops.extend(rdp_expose_put);
    rdp_expose_ops.extend(rdp_expose_delete);

    spec.add_path(
        "/api/v1/vms/{ns}/{name}/rdp-expose",
        PathItem {
            operations: rdp_expose_ops,
        },
    );

    let guest_rdp_post = |op_id: &str, summary: &str| Operation {
        summary: summary.to_string(),
        description: Some(
            "Runs PowerShell in the guest via QEMU guest-agent (guest-exec). VM must be Running with AgentConnected."
                .to_string(),
        ),
        operation_id: op_id.to_string(),
        tags: vec!["vms".to_string(), "rdp".to_string()],
        parameters: ns_name_params(),
        responses: {
            let mut r = HashMap::new();
            r.insert(
                "200".to_string(),
                ResponseSpec {
                    description: "Guest exec result".to_string(),
                    content_type: Some("application/json".to_string()),
                    schema_ref: None,
                },
            );
            r
        },
        security: vec!["apiKey".to_string()],
    };

    let mut enable_rdp_ops = HashMap::new();
    enable_rdp_ops.insert(
        "post".to_string(),
        guest_rdp_post(
            "guestAgentEnableRdp",
            "Enable Windows Remote Desktop in the guest",
        ),
    );
    spec.add_path(
        "/api/v1/vms/{ns}/{name}/guest-agent/enable-rdp",
        PathItem {
            operations: enable_rdp_ops,
        },
    );

    let mut disable_rdp_ops = HashMap::new();
    disable_rdp_ops.insert(
        "post".to_string(),
        guest_rdp_post(
            "guestAgentDisableRdp",
            "Disable Windows Remote Desktop in the guest",
        ),
    );
    spec.add_path(
        "/api/v1/vms/{ns}/{name}/guest-agent/disable-rdp",
        PathItem {
            operations: disable_rdp_ops,
        },
    );

    spec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_new() {
        let spec = OpenApiSpec::new();
        assert_eq!(spec.openapi, "3.0.3");
        assert_eq!(spec.info.title, "VMRogue API");
        assert!(!spec.servers.is_empty());
    }

    #[test]
    fn test_openapi_spec_add_server() {
        let mut spec = OpenApiSpec::new();
        spec.add_server("https://api.example.com", "Production");
        assert_eq!(spec.servers.len(), 2);
    }

    #[test]
    fn test_openapi_spec_add_tag() {
        let mut spec = OpenApiSpec::new();
        spec.add_tag("vms", "VM operations");
        assert_eq!(spec.tag_count(), 1);
    }

    #[test]
    fn test_openapi_spec_add_path() {
        let mut spec = OpenApiSpec::new();
        spec.add_path(
            "/api/v1/test",
            PathItem {
                operations: HashMap::new(),
            },
        );
        assert_eq!(spec.path_count(), 1);
    }

    #[test]
    fn test_openapi_spec_add_schema() {
        let mut spec = OpenApiSpec::new();
        spec.add_schema(
            "TestSchema",
            SchemaDefinition {
                schema_type: "object".to_string(),
                description: "Test".to_string(),
                properties: HashMap::new(),
                required: Vec::new(),
            },
        );
        assert_eq!(spec.schema_count(), 1);
    }

    #[test]
    fn test_openapi_spec_operation_count() {
        let mut spec = OpenApiSpec::new();

        let mut ops = HashMap::new();
        ops.insert(
            "get".to_string(),
            Operation {
                summary: "Get".to_string(),
                description: None,
                operation_id: "get".to_string(),
                tags: Vec::new(),
                parameters: Vec::new(),
                responses: HashMap::new(),
                security: Vec::new(),
            },
        );
        ops.insert(
            "post".to_string(),
            Operation {
                summary: "Post".to_string(),
                description: None,
                operation_id: "post".to_string(),
                tags: Vec::new(),
                parameters: Vec::new(),
                responses: HashMap::new(),
                security: Vec::new(),
            },
        );

        spec.add_path("/test", PathItem { operations: ops });
        assert_eq!(spec.operation_count(), 2);
    }

    #[test]
    fn test_generate_default_spec() {
        let spec = generate_default_spec();

        assert!(spec.tag_count() >= 5);
        assert!(spec.path_count() >= 2);
        assert!(spec.schema_count() >= 1);
        assert!(!spec.components.security_schemes.is_empty());
    }

    #[test]
    fn test_default_spec_has_vm_schema() {
        let spec = generate_default_spec();
        assert!(spec.components.schemas.contains_key("VirtualMachine"));
    }

    #[test]
    fn test_default_spec_has_security_schemes() {
        let spec = generate_default_spec();
        assert!(spec.components.security_schemes.contains_key("bearerAuth"));
        assert!(spec.components.security_schemes.contains_key("apiKey"));
    }

    #[test]
    fn test_parameter_location_display() {
        assert_eq!(ParameterLocation::Path.to_string(), "path");
        assert_eq!(ParameterLocation::Query.to_string(), "query");
        assert_eq!(ParameterLocation::Header.to_string(), "header");
        assert_eq!(ParameterLocation::Cookie.to_string(), "cookie");
    }

    #[test]
    fn test_parameter_location_equality() {
        assert_eq!(ParameterLocation::Path, ParameterLocation::Path);
        assert_ne!(ParameterLocation::Path, ParameterLocation::Query);
    }
}
