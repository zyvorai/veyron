package provider

import (
	"context"

	"github.com/hashicorp/terraform-plugin-framework/datasource"
	"github.com/hashicorp/terraform-plugin-framework/datasource/schema"
	"github.com/hashicorp/terraform-plugin-framework/types"
)

type templatesDataSource struct {
	client *apiClient
}

type templatesModel struct {
	Names types.List `tfsdk:"names"`
}

func NewTemplatesDataSource() datasource.DataSource {
	return &templatesDataSource{}
}

func (d *templatesDataSource) Metadata(_ context.Context, _ datasource.MetadataRequest, resp *datasource.MetadataResponse) {
	resp.TypeName = "vmrogue_templates"
}

func (d *templatesDataSource) Schema(_ context.Context, _ datasource.SchemaRequest, resp *datasource.SchemaResponse) {
	resp.Schema = schema.Schema{
		Attributes: map[string]schema.Attribute{
			"names": schema.ListAttribute{ElementType: types.StringType, Computed: true},
		},
	}
}

func (d *templatesDataSource) Configure(_ context.Context, req datasource.ConfigureRequest, _ *datasource.ConfigureResponse) {
	if req.ProviderData != nil {
		d.client = newAPIClient(req.ProviderData.(*providerData))
	}
}

func (d *templatesDataSource) Read(ctx context.Context, req datasource.ReadRequest, resp *datasource.ReadResponse) {
	var state templatesModel
	resp.Diagnostics.Append(req.Config.Get(ctx, &state)...)
	var rows []struct {
		Name string `json:"name"`
	}
	if err := d.client.do(ctx, "GET", "/api/v1/templates", nil, &rows); err != nil {
		resp.Diagnostics.AddError("Read templates failed", err.Error())
		return
	}
	names := make([]types.String, 0, len(rows))
	for _, row := range rows {
		names = append(names, types.StringValue(row.Name))
	}
	list, diags := types.ListValueFrom(ctx, types.StringType, names)
	resp.Diagnostics.Append(diags...)
	state.Names = list
	resp.Diagnostics.Append(resp.State.Set(ctx, &state)...)
}

type storageClassesDataSource struct{ client *apiClient }

func NewStorageClassesDataSource() datasource.DataSource { return &storageClassesDataSource{} }

func (d *storageClassesDataSource) Metadata(_ context.Context, _ datasource.MetadataRequest, resp *datasource.MetadataResponse) {
	resp.TypeName = "vmrogue_storage_classes"
}

func (d *storageClassesDataSource) Schema(_ context.Context, _ datasource.SchemaRequest, resp *datasource.SchemaResponse) {
	resp.Schema = schema.Schema{
		Attributes: map[string]schema.Attribute{
			"names": schema.ListAttribute{ElementType: types.StringType, Computed: true},
		},
	}
}

func (d *storageClassesDataSource) Configure(_ context.Context, req datasource.ConfigureRequest, _ *datasource.ConfigureResponse) {
	if req.ProviderData != nil {
		d.client = newAPIClient(req.ProviderData.(*providerData))
	}
}

func (d *storageClassesDataSource) Read(ctx context.Context, req datasource.ReadRequest, resp *datasource.ReadResponse) {
	var state templatesModel
	var rows []struct {
		Name string `json:"name"`
	}
	if err := d.client.do(ctx, "GET", "/api/v1/storage/classes", nil, &rows); err != nil {
		resp.Diagnostics.AddError("Read storage classes failed", err.Error())
		return
	}
	names := make([]types.String, 0, len(rows))
	for _, row := range rows {
		names = append(names, types.StringValue(row.Name))
	}
	list, diags := types.ListValueFrom(ctx, types.StringType, names)
	resp.Diagnostics.Append(diags...)
	state.Names = list
	resp.Diagnostics.Append(resp.State.Set(ctx, &state)...)
}

type nodesDataSource struct{ client *apiClient }

func NewNodesDataSource() datasource.DataSource { return &nodesDataSource{} }

func (d *nodesDataSource) Metadata(_ context.Context, _ datasource.MetadataRequest, resp *datasource.MetadataResponse) {
	resp.TypeName = "vmrogue_nodes"
}

func (d *nodesDataSource) Schema(_ context.Context, _ datasource.SchemaRequest, resp *datasource.SchemaResponse) {
	resp.Schema = schema.Schema{
		Attributes: map[string]schema.Attribute{
			"names": schema.ListAttribute{ElementType: types.StringType, Computed: true},
		},
	}
}

func (d *nodesDataSource) Configure(_ context.Context, req datasource.ConfigureRequest, _ *datasource.ConfigureResponse) {
	if req.ProviderData != nil {
		d.client = newAPIClient(req.ProviderData.(*providerData))
	}
}

func (d *nodesDataSource) Read(ctx context.Context, req datasource.ReadRequest, resp *datasource.ReadResponse) {
	var state templatesModel
	var rows []struct {
		Name string `json:"name"`
	}
	if err := d.client.do(ctx, "GET", "/api/v1/nodes", nil, &rows); err != nil {
		resp.Diagnostics.AddError("Read nodes failed", err.Error())
		return
	}
	names := make([]types.String, 0, len(rows))
	for _, row := range rows {
		names = append(names, types.StringValue(row.Name))
	}
	list, diags := types.ListValueFrom(ctx, types.StringType, names)
	resp.Diagnostics.Append(diags...)
	state.Names = list
	resp.Diagnostics.Append(resp.State.Set(ctx, &state)...)
}
