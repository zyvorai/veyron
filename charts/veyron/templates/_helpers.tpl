{{/*
Expand the name of the chart.
*/}}
{{- define "veyron.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "veyron.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "veyron.labels" -}}
helm.sh/chart: {{ include "veyron.name" . }}-{{ .Chart.Version }}
{{ include "veyron.selectorLabels" . }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "veyron.selectorLabels" -}}
app.kubernetes.io/name: veyron
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/component: api
{{- end }}

{{/*
ClusterRole name (defaults to veyron; not the Deployment name veyron-api).
*/}}
{{- define "veyron.clusterRoleName" -}}
{{- default "veyron" .Values.rbac.clusterRoleName }}
{{- end }}

{{/*
Service account name
*/}}
{{- define "veyron.serviceAccountName" -}}
{{- if .Values.serviceAccount.name }}
{{- .Values.serviceAccount.name }}
{{- else }}
{{- include "veyron.fullname" . }}
{{- end }}
{{- end }}

{{/*
API key Secret name (veyron-api-key, not veyron-api-api-key).
*/}}
{{- define "veyron.apiKeySecretName" -}}
{{- default "veyron-api-key" .Values.auth.secretName }}
{{- end }}

{{/*
Image tag
*/}}
{{- define "veyron.imageTag" -}}
{{- default .Chart.AppVersion .Values.image.tag }}
{{- end }}
