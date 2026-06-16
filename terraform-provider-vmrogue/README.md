# Terraform Provider Veyron (v0.1)

Terraform provider for the [VMRogue](https://github.com/zyvor/VMRogue) REST API — KubeVirt VM lifecycle, snapshots, and tenant namespace quotas.

## Resources (v0.1)

| Resource | API |
|----------|-----|
| `vmrogue_virtual_machine` | `POST/PUT/DELETE /api/v1/vms` |
| `vmrogue_snapshot` | `POST /api/v1/snapshots/:ns/:vm/create` |
| `vmrogue_namespace_quota` | `POST /api/v1/tenants` (bootstrap namespace + ResourceQuota) |

## Data sources

| Data source | API |
|-------------|-----|
| `vmrogue_templates` | `GET /api/v1/templates` |
| `vmrogue_storage_classes` | `GET /api/v1/storage/classes` |
| `vmrogue_nodes` | `GET /api/v1/nodes` |

## Configure

```hcl
terraform {
  required_providers {
    vmrogue = {
      source = "zyvor/vmrogue"
    }
  }
}

provider "vmrogue" {
  endpoint = "https://vmrogue.example.com"
  api_key  = var.vmrogue_api_key
  # or bearer_token for OIDC service accounts
}
```

## Example

```hcl
resource "vmrogue_namespace_quota" "team_a" {
  id           = "team-a"
  display_name = "Team A"
  owner_email  = "team-a@example.com"
  cpu_quota    = "40"
  memory_quota = "128Gi"
  max_vms      = 100
}

resource "vmrogue_virtual_machine" "web" {
  name       = "web-01"
  namespace  = vmrogue_namespace_quota.team_a.namespace
  template   = "ubuntu-22.04"
  cpus       = 2
  memory     = "4Gi"
  start      = true
}

resource "vmrogue_snapshot" "web_baseline" {
  namespace     = vmrogue_virtual_machine.web.namespace
  vm_name       = vmrogue_virtual_machine.web.name
  snapshot_name = "baseline"
}
```

## Build

```bash
cd terraform-provider-vmrogue
go build -o terraform-provider-vmrogue
```

OpenAPI contract: `GET /api/openapi.json` on the Veyron API server.
