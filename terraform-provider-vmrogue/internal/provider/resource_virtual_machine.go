// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package provider

import (
	"context"
	"fmt"

	"github.com/hashicorp/terraform-plugin-framework/resource"
	"github.com/hashicorp/terraform-plugin-framework/resource/schema"
	"github.com/hashicorp/terraform-plugin-framework/types"
)

type virtualMachineResource struct {
	client *apiClient
}

type vmModel struct {
	Name      types.String `tfsdk:"name"`
	Namespace types.String `tfsdk:"namespace"`
	Template  types.String `tfsdk:"template"`
	CPUs      types.Int64  `tfsdk:"cpus"`
	Memory    types.String `tfsdk:"memory"`
	Start     types.Bool   `tfsdk:"start"`
}

func NewVirtualMachineResource() resource.Resource {
	return &virtualMachineResource{}
}

func (r *virtualMachineResource) Metadata(_ context.Context, _ resource.MetadataRequest, resp *resource.MetadataResponse) {
	resp.TypeName = "vmrogue_virtual_machine"
}

func (r *virtualMachineResource) Schema(_ context.Context, _ resource.SchemaRequest, resp *resource.SchemaResponse) {
	resp.Schema = schema.Schema{
		Description: "KubeVirt VirtualMachine managed via Veyron API.",
		Attributes: map[string]schema.Attribute{
			"name":      schema.StringAttribute{Required: true},
			"namespace": schema.StringAttribute{Required: true},
			"template":  schema.StringAttribute{Required: true},
			"cpus":      schema.Int64Attribute{Optional: true},
			"memory":    schema.StringAttribute{Optional: true},
			"start":     schema.BoolAttribute{Optional: true},
		},
	}
}

func (r *virtualMachineResource) Configure(_ context.Context, req resource.ConfigureRequest, _ *resource.ConfigureResponse) {
	if req.ProviderData == nil {
		return
	}
	r.client = newAPIClient(req.ProviderData.(*providerData))
}

func (r *virtualMachineResource) Create(ctx context.Context, req resource.CreateRequest, resp *resource.CreateResponse) {
	var plan vmModel
	resp.Diagnostics.Append(req.Plan.Get(ctx, &plan)...)
	if resp.Diagnostics.HasError() {
		return
	}
	body := map[string]any{
		"name":      plan.Name.ValueString(),
		"namespace": plan.Namespace.ValueString(),
		"template":  plan.Template.ValueString(),
		"start":     plan.Start.ValueBool(),
	}
	if !plan.CPUs.IsNull() {
		body["cpus"] = plan.CPUs.ValueInt64()
	}
	if !plan.Memory.IsNull() {
		body["memory"] = plan.Memory.ValueString()
	}
	if err := r.client.do(ctx, "POST", "/api/v1/vms", body, nil); err != nil {
		resp.Diagnostics.AddError("Create VM failed", err.Error())
		return
	}
	resp.Diagnostics.Append(resp.State.Set(ctx, &plan)...)
}

func (r *virtualMachineResource) Read(ctx context.Context, req resource.ReadRequest, resp *resource.ReadResponse) {
	var state vmModel
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	path := fmt.Sprintf("/api/v1/vms/%s/%s", state.Namespace.ValueString(), state.Name.ValueString())
	var vm map[string]any
	if err := r.client.do(ctx, "GET", path, nil, &vm); err != nil {
		resp.State.RemoveResource(ctx)
		return
	}
	resp.Diagnostics.Append(resp.State.Set(ctx, &state)...)
}

func (r *virtualMachineResource) Update(ctx context.Context, req resource.UpdateRequest, resp *resource.UpdateResponse) {
	var plan vmModel
	resp.Diagnostics.Append(req.Plan.Get(ctx, &plan)...)
	if resp.Diagnostics.HasError() {
		return
	}
	path := fmt.Sprintf("/api/v1/vms/%s/%s", plan.Namespace.ValueString(), plan.Name.ValueString())
	if err := r.client.do(ctx, "PUT", path, map[string]any{
		"cpus":   plan.CPUs.ValueInt64(),
		"memory": plan.Memory.ValueString(),
	}, nil); err != nil {
		resp.Diagnostics.AddError("Update VM failed", err.Error())
		return
	}
	resp.Diagnostics.Append(resp.State.Set(ctx, &plan)...)
}

func (r *virtualMachineResource) Delete(ctx context.Context, req resource.DeleteRequest, resp *resource.DeleteResponse) {
	var state vmModel
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	path := fmt.Sprintf("/api/v1/vms/%s/%s", state.Namespace.ValueString(), state.Name.ValueString())
	if err := r.client.do(ctx, "DELETE", path, nil, nil); err != nil {
		resp.Diagnostics.AddError("Delete VM failed", err.Error())
	}
}
