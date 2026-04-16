{{- define "vmrogue-operator.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "vmrogue-operator.fullname" -}}
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

{{- define "vmrogue-operator.labels" -}}
helm.sh/chart: {{ include "vmrogue-operator.name" . }}-{{ .Chart.Version }}
{{ include "vmrogue-operator.selectorLabels" . }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{- define "vmrogue-operator.selectorLabels" -}}
app.kubernetes.io/name: {{ include "vmrogue-operator.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{- define "vmrogue-operator.serviceAccountName" -}}
{{- if .Values.serviceAccount.name }}
{{- .Values.serviceAccount.name }}
{{- else }}
{{- include "vmrogue-operator.fullname" . }}
{{- end }}
{{- end }}

{{- define "vmrogue-operator.imageTag" -}}
{{- default .Chart.AppVersion .Values.image.tag }}
{{- end }}
