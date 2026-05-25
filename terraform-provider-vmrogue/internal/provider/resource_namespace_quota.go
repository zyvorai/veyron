package provider

import (
	"context"

	"github.com/hashicorp/terraform-plugin-framework/resource"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema"
	"github.com/hashicorp/terraform-plugin-framework/types"
)

type namespaceQuotaResource struct {
	client *apiClient
}

type namespaceQuotaModel struct {
	ID           types.String `tfsdk:"id"`
	DisplayName  types.String `tfsdk:"display_name"`
	OwnerEmail   types.String `tfsdk:"owner_email"`
	CPUQuota     types.String `tfsdk:"cpu_quota"`
	MemoryQuota  types.String `tfsdk:"memory_quota"`
	MaxVMs       types.Int64  `tfsdk:"max_vms"`
	Namespace    types.String `tfsdk:"namespace"`
}

func NewNamespaceQuotaResource() resource.Resource {
	return &namespaceQuotaResource{}
}

func (r *namespaceQuotaResource) Metadata(_ context.Context, _ resource.MetadataRequest, resp *resource.MetadataResponse) {
	resp.TypeName = "vmrogue_namespace_quota"
}

func (r *namespaceQuotaResource) Schema(_ context.Context, _ resource.SchemaRequest, resp *resource.SchemaResponse) {
	resp.Schema = schema.Schema{
		Description: "Tenant workspace with namespace bootstrap and ResourceQuota via VMRogue /tenants API.",
		Attributes: map[string]schema.Attribute{
			"id":            schema.StringAttribute{Required: true},
			"display_name":  schema.StringAttribute{Required: true},
			"owner_email":   schema.StringAttribute{Required: true},
			"cpu_quota":     schema.StringAttribute{Optional: true},
			"memory_quota":  schema.StringAttribute{Optional: true},
			"max_vms":       schema.Int64Attribute{Optional: true},
			"namespace":     schema.StringAttribute{Computed: true},
		},
	}
}

func (r *namespaceQuotaResource) Configure(_ context.Context, req resource.ConfigureRequest, _ *resource.ConfigureResponse) {
	if req.ProviderData != nil {
		r.client = newAPIClient(req.ProviderData.(*providerData))
	}
}

func (r *namespaceQuotaResource) Create(ctx context.Context, req resource.CreateRequest, resp *resource.CreateResponse) {
	var plan namespaceQuotaModel
	resp.Diagnostics.Append(req.Plan.Get(ctx, &plan)...)
	if resp.Diagnostics.HasError() {
		return
	}
	body := map[string]any{
		"id":                   plan.ID.ValueString(),
		"display_name":         plan.DisplayName.ValueString(),
		"owner_email":          plan.OwnerEmail.ValueString(),
		"bootstrap_namespace":  true,
	}
	if !plan.CPUQuota.IsNull() {
		body["cpu_quota"] = plan.CPUQuota.ValueString()
	}
	if !plan.MemoryQuota.IsNull() {
		body["memory_quota"] = plan.MemoryQuota.ValueString()
	}
	if !plan.MaxVMs.IsNull() {
		body["max_vms"] = plan.MaxVMs.ValueInt64()
	}
	var tenant struct {
		ID          string   `json:"id"`
		Namespaces  []string `json:"namespaces"`
	}
	if err := r.client.do(ctx, "POST", "/api/v1/tenants", body, &tenant); err != nil {
		resp.Diagnostics.AddError("Create tenant failed", err.Error())
		return
	}
	if len(tenant.Namespaces) > 0 {
		plan.Namespace = types.StringValue(tenant.Namespaces[0])
	} else {
		plan.Namespace = types.StringValue("tenant-" + plan.ID.ValueString())
	}
	resp.Diagnostics.Append(resp.State.Set(ctx, &plan)...)
}

func (r *namespaceQuotaResource) Read(ctx context.Context, req resource.ReadRequest, resp *resource.ReadResponse) {
	resp.Diagnostics.Append(req.State.Get(ctx, &namespaceQuotaModel{})...)
}

func (r *namespaceQuotaResource) Update(_ context.Context, _ resource.UpdateRequest, _ *resource.UpdateResponse) {}

func (r *namespaceQuotaResource) Delete(ctx context.Context, req resource.DeleteRequest, resp *resource.DeleteResponse) {
	var state namespaceQuotaModel
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	if err := r.client.do(ctx, "DELETE", "/api/v1/tenants/"+state.ID.ValueString(), nil, nil); err != nil {
		resp.Diagnostics.AddError("Delete tenant failed", err.Error())
	}
}
