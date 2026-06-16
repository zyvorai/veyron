# Terraform Provider Veyron (v0.1)

Terraform provider for the [Veyron](https://github.com/zyvor/Veyron) REST API — KubeVirt VM lifecycle, snapshots, and tenant namespace quotas.

## Resources (v0.1)

| Resource | API |
|----------|-----|
| `veyron_virtual_machine` | `POST/PUT/DELETE /api/v1/vms` |
| `veyron_snapshot` | `POST /api/v1/snapshots/:ns/:vm/create` |
| `veyron_namespace_quota` | `POST /api/v1/tenants` (bootstrap namespace + ResourceQuota) |

## Data sources

| Data source | API |
|-------------|-----|
| `veyron_templates` | `GET /api/v1/templates` |
| `veyron_storage_classes` | `GET /api/v1/storage/classes` |
| `veyron_nodes` | `GET /api/v1/nodes` |

## Configure

```hcl
terraform {
  required_providers {
    veyron = {
      source = "zyvor/veyron"
    }
  }
}

provider "veyron" {
  endpoint = "https://veyron.example.com"
  api_key  = var.veyron_api_key
  # or bearer_token for OIDC service accounts
}
```

## Example

```hcl
resource "veyron_namespace_quota" "team_a" {
  id           = "team-a"
  display_name = "Team A"
  owner_email  = "team-a@example.com"
  cpu_quota    = "40"
  memory_quota = "128Gi"
  max_vms      = 100
}

resource "veyron_virtual_machine" "web" {
  name       = "web-01"
  namespace  = veyron_namespace_quota.team_a.namespace
  template   = "ubuntu-22.04"
  cpus       = 2
  memory     = "4Gi"
  start      = true
}

resource "veyron_snapshot" "web_baseline" {
  namespace     = veyron_virtual_machine.web.namespace
  vm_name       = veyron_virtual_machine.web.name
  snapshot_name = "baseline"
}
```

## Build

```bash
cd terraform-provider-veyron
go build -o terraform-provider-veyron
```

OpenAPI contract: `GET /api/openapi.json` on the Veyron API server.
