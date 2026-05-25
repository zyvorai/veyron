package provider

import (
	"context"
	"os"

	"github.com/hashicorp/terraform-plugin-framework/datasource"
	"github.com/hashicorp/terraform-plugin-framework/provider"
	"github.com/hashicorp/terraform-plugin-framework/provider/schema"
	"github.com/hashicorp/terraform-plugin-framework/resource"
	"github.com/hashicorp/terraform-plugin-framework/types"
)

type vmrogueProvider struct {
	version string
}

type providerModel struct {
	Endpoint     types.String `tfsdk:"endpoint"`
	APIKey       types.String `tfsdk:"api_key"`
	BearerToken  types.String `tfsdk:"bearer_token"`
	InsecureTLS  types.Bool   `tfsdk:"insecure_tls"`
}

type providerData struct {
	endpoint    string
	apiKey      string
	bearerToken string
	insecureTLS bool
}

func New(version string) func() provider.Provider {
	return func() provider.Provider {
		return &vmrogueProvider{version: version}
	}
}

func (p *vmrogueProvider) Metadata(_ context.Context, _ provider.MetadataRequest, resp *provider.MetadataResponse) {
	resp.TypeName = "vmrogue"
	resp.Version = p.version
}

func (p *vmrogueProvider) Schema(_ context.Context, _ provider.SchemaRequest, resp *provider.SchemaResponse) {
	resp.Schema = schema.Schema{
		Description: "Interact with the VMRogue KubeVirt control plane API.",
		Attributes: map[string]schema.Attribute{
			"endpoint": schema.StringAttribute{
				Required:    true,
				Description: "VMRogue API base URL, e.g. https://cluster:5151",
			},
			"api_key": schema.StringAttribute{
				Optional:    true,
				Sensitive:   true,
				Description: "VMROGUE API key (X-API-Key header).",
			},
			"bearer_token": schema.StringAttribute{
				Optional:    true,
				Sensitive:   true,
				Description: "OIDC or JWT bearer token for automation.",
			},
			"insecure_tls": schema.BoolAttribute{
				Optional:    true,
				Description: "Skip TLS verification (dev clusters with self-signed certs).",
			},
		},
	}
}

func (p *vmrogueProvider) Configure(ctx context.Context, req provider.ConfigureRequest, resp *provider.ConfigureResponse) {
	var config providerModel
	resp.Diagnostics.Append(req.Config.Get(ctx, &config)...)
	if resp.Diagnostics.HasError() {
		return
	}

	endpoint := config.Endpoint.ValueString()
	if endpoint == "" {
		if env := os.Getenv("VMROGUE_ENDPOINT"); env != "" {
			endpoint = env
		}
	}
	if endpoint == "" {
		resp.Diagnostics.AddError("Missing endpoint", "Set provider endpoint or VMROGUE_ENDPOINT")
		return
	}

	apiKey := config.APIKey.ValueString()
	if apiKey == "" {
		apiKey = os.Getenv("VMROGUE_API_KEY")
	}
	bearer := config.BearerToken.ValueString()
	if bearer == "" {
		bearer = os.Getenv("VMROGUE_BEARER_TOKEN")
	}
	if apiKey == "" && bearer == "" {
		resp.Diagnostics.AddError("Missing credentials", "Set api_key or bearer_token")
		return
	}

	client := &providerData{
		endpoint:    endpoint,
		apiKey:      apiKey,
		bearerToken: bearer,
		insecureTLS: config.InsecureTLS.ValueBool(),
	}
	resp.DataSourceData = client
	resp.ResourceData = client
}

func (p *vmrogueProvider) Resources(_ context.Context) []func() resource.Resource {
	return []func() resource.Resource{
		NewVirtualMachineResource,
		NewSnapshotResource,
		NewNamespaceQuotaResource,
	}
}

func (p *vmrogueProvider) DataSources(_ context.Context) []func() datasource.DataSource {
	return []func() datasource.DataSource{
		NewTemplatesDataSource,
		NewStorageClassesDataSource,
		NewNodesDataSource,
	}
}
