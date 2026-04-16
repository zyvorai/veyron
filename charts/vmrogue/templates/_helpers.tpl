{{/*
Expand the name of the chart.
*/}}
{{- define "vmrogue.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "vmrogue.fullname" -}}
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
{{- define "vmrogue.labels" -}}
helm.sh/chart: {{ include "vmrogue.name" . }}-{{ .Chart.Version }}
{{ include "vmrogue.selectorLabels" . }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "vmrogue.selectorLabels" -}}
app.kubernetes.io/name: {{ include "vmrogue.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Service account name
*/}}
{{- define "vmrogue.serviceAccountName" -}}
{{- if .Values.serviceAccount.name }}
{{- .Values.serviceAccount.name }}
{{- else }}
{{- include "vmrogue.fullname" . }}
{{- end }}
{{- end }}

{{/*
Image tag
*/}}
{{- define "vmrogue.imageTag" -}}
{{- default .Chart.AppVersion .Values.image.tag }}
{{- end }}
