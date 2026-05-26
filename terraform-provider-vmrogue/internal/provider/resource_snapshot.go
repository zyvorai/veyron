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

type snapshotResource struct {
	client *apiClient
}

type snapshotModel struct {
	Namespace    types.String `tfsdk:"namespace"`
	VMName       types.String `tfsdk:"vm_name"`
	SnapshotName types.String `tfsdk:"snapshot_name"`
}

func NewSnapshotResource() resource.Resource {
	return &snapshotResource{}
}

func (r *snapshotResource) Metadata(_ context.Context, _ resource.MetadataRequest, resp *resource.MetadataResponse) {
	resp.TypeName = "vmrogue_snapshot"
}

func (r *snapshotResource) Schema(_ context.Context, _ resource.SchemaRequest, resp *resource.SchemaResponse) {
	resp.Schema = schema.Schema{
		Attributes: map[string]schema.Attribute{
			"namespace":     schema.StringAttribute{Required: true},
			"vm_name":         schema.StringAttribute{Required: true},
			"snapshot_name":   schema.StringAttribute{Required: true},
		},
	}
}

func (r *snapshotResource) Configure(_ context.Context, req resource.ConfigureRequest, _ *resource.ConfigureResponse) {
	if req.ProviderData != nil {
		r.client = newAPIClient(req.ProviderData.(*providerData))
	}
}

func (r *snapshotResource) Create(ctx context.Context, req resource.CreateRequest, resp *resource.CreateResponse) {
	var plan snapshotModel
	resp.Diagnostics.Append(req.Plan.Get(ctx, &plan)...)
	if resp.Diagnostics.HasError() {
		return
	}
	path := fmt.Sprintf("/api/v1/snapshots/%s/%s/create",
		plan.Namespace.ValueString(), plan.VMName.ValueString())
	body := map[string]any{"snapshot_name": plan.SnapshotName.ValueString()}
	if err := r.client.do(ctx, "POST", path, body, nil); err != nil {
		resp.Diagnostics.AddError("Create snapshot failed", err.Error())
		return
	}
	resp.Diagnostics.Append(resp.State.Set(ctx, &plan)...)
}

func (r *snapshotResource) Read(ctx context.Context, req resource.ReadRequest, resp *resource.ReadResponse) {
	resp.Diagnostics.Append(req.State.Get(ctx, &snapshotModel{})...)
}

func (r *snapshotResource) Update(_ context.Context, _ resource.UpdateRequest, _ *resource.UpdateResponse) {}

func (r *snapshotResource) Delete(ctx context.Context, req resource.DeleteRequest, resp *resource.DeleteResponse) {
	var state snapshotModel
	resp.Diagnostics.Append(req.State.Get(ctx, &state)...)
	if resp.Diagnostics.HasError() {
		return
	}
	path := fmt.Sprintf("/api/v1/snapshots/%s/%s/delete",
		state.Namespace.ValueString(), state.SnapshotName.ValueString())
	if err := r.client.do(ctx, "POST", path, nil, nil); err != nil {
		resp.Diagnostics.AddError("Delete snapshot failed", err.Error())
	}
}
