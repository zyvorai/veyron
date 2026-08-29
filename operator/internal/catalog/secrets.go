// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package catalog

import (
	"context"
	"encoding/json"
	"encoding/xml"
	"fmt"
	"strings"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/types"
	"sigs.k8s.io/controller-runtime/pkg/client"

	veyronv1alpha1 "github.com/zyvorai/veyron/operator/api/v1alpha1"
)

// DomainJoinCredentials holds AD join parameters from a Secret JSON payload.
type DomainJoinCredentials struct {
	Domain   string `json:"domain"`
	OU       string `json:"ou,omitempty"`
	Username string `json:"username"`
	Password string `json:"password"`
}

// ResolveCloudInitUserData returns inline userData or loads it from Secret refs.
func ResolveCloudInitUserData(ctx context.Context, c client.Client, ns string, spec *veyronv1alpha1.VeyronVMSpec) (string, error) {
	if spec == nil || spec.CloudInit == nil {
		return "", nil
	}
	ci := spec.CloudInit

	if ci.UserDataSecretRef != nil {
		data, err := readSecretKey(ctx, c, ns, ci.UserDataSecretRef)
		if err != nil {
			return "", fmt.Errorf("cloudInit userDataSecretRef: %w", err)
		}
		return data, nil
	}

	userData := ci.UserData

	// NOTE: domainJoinSecretRef deliberately does NOT reach cloud-init.
	//
	// It used to be rendered into a PowerShell `Add-Computer` block with the join
	// password inlined as a string literal. That put domain credentials in the
	// config-drive volume (readable inside the guest) AND in the Cloudbase-Init log,
	// which records the userdata script it executes — plaintext, on disk, forever.
	//
	// The join now goes through the Windows-supported `<Identification>` element of
	// an unattend answer file, delivered as sysprep media. Windows scrubs credentials
	// from C:\Windows\Panther\unattend.xml once it has processed them.
	// See BuildDomainJoinUnattend / the controller's sysprep Secret reconcile.

	// sysprepSecretRef is NOT cloud-init material: the converter mounts it as a
	// KubeVirt `sysprep` CD-ROM volume (see converter.buildVolumes). All we do here
	// is fail fast if the Secret is unusable, so a bad ref surfaces as a reconcile
	// error instead of a VM that silently boots without its answer file.
	if spec.Windows != nil && spec.Windows.SysprepSecretRef != nil {
		if err := validateSysprepSecret(ctx, c, ns, spec.Windows.SysprepSecretRef); err != nil {
			return "", fmt.Errorf("windows sysprepSecretRef: %w", err)
		}
	}

	return userData, nil
}

// SysprepAnswerFileKey is the key KubeVirt reads from a sysprep Secret/ConfigMap.
// KubeVirt is case-insensitive here, but it must be one of these two names.
const SysprepAnswerFileKey = "autounattend.xml"

// validateSysprepSecret checks the referenced Secret exists and actually carries an
// answer file. Without this a typo'd key means Windows Setup boots to an interactive
// prompt with no indication anything is wrong.
func validateSysprepSecret(ctx context.Context, c client.Client, defaultNS string, ref *veyronv1alpha1.SecretKeyRef) error {
	if ref == nil || ref.Name == "" {
		return fmt.Errorf("invalid secret ref: name is required")
	}
	secNS := ref.Namespace
	if secNS == "" {
		secNS = defaultNS
	}

	var sec corev1.Secret
	if err := c.Get(ctx, client.ObjectKey{Namespace: secNS, Name: ref.Name}, &sec); err != nil {
		return fmt.Errorf("get secret %s/%s: %w", secNS, ref.Name, err)
	}

	for k := range sec.Data {
		if strings.EqualFold(k, SysprepAnswerFileKey) {
			return nil
		}
	}
	return fmt.Errorf(
		"secret %s/%s has no %q key (found: %v) — KubeVirt will not mount an answer file",
		secNS, ref.Name, SysprepAnswerFileKey, secretKeys(sec),
	)
}

// ReadSysprepAnswerFile returns the caller's autounattend.xml from the referenced
// Secret (key match is case-insensitive, as KubeVirt's is).
func ReadSysprepAnswerFile(ctx context.Context, c client.Client, defaultNS string, ref *veyronv1alpha1.SecretKeyRef) (string, error) {
	if ref == nil || ref.Name == "" {
		return "", fmt.Errorf("invalid secret ref: name is required")
	}
	secNS := ref.Namespace
	if secNS == "" {
		secNS = defaultNS
	}
	var sec corev1.Secret
	if err := c.Get(ctx, client.ObjectKey{Namespace: secNS, Name: ref.Name}, &sec); err != nil {
		return "", fmt.Errorf("get secret %s/%s: %w", secNS, ref.Name, err)
	}
	for k, v := range sec.Data {
		if strings.EqualFold(k, SysprepAnswerFileKey) {
			return string(v), nil
		}
	}
	return "", fmt.Errorf(
		"secret %s/%s has no %q key (found: %v)",
		secNS, ref.Name, SysprepAnswerFileKey, secretKeys(sec),
	)
}

func secretKeys(sec corev1.Secret) []string {
	keys := make([]string, 0, len(sec.Data))
	for k := range sec.Data {
		keys = append(keys, k)
	}
	return keys
}

func readSecretKey(ctx context.Context, c client.Client, defaultNS string, ref *veyronv1alpha1.SecretKeyRef) (string, error) {
	if ref == nil || ref.Name == "" || ref.Key == "" {
		return "", fmt.Errorf("invalid secret ref")
	}
	secNS := ref.Namespace
	if secNS == "" {
		secNS = defaultNS
	}
	var sec corev1.Secret
	if err := c.Get(ctx, types.NamespacedName{Name: ref.Name, Namespace: secNS}, &sec); err != nil {
		return "", err
	}
	val, ok := sec.Data[ref.Key]
	if !ok {
		if s, ok := sec.StringData[ref.Key]; ok {
			return s, nil
		}
		return "", fmt.Errorf("key %q not found in secret %s/%s", ref.Key, secNS, ref.Name)
	}
	return string(val), nil
}

// ResolveDomainJoinCredentials loads and validates the AD join payload.
func ResolveDomainJoinCredentials(ctx context.Context, c client.Client, ns string, ref *veyronv1alpha1.SecretKeyRef) (*DomainJoinCredentials, error) {
	raw, err := readSecretKey(ctx, c, ns, ref)
	if err != nil {
		return nil, err
	}
	var creds DomainJoinCredentials
	if err := json.Unmarshal([]byte(raw), &creds); err != nil {
		return nil, fmt.Errorf("domain join secret JSON: %w", err)
	}
	if creds.Domain == "" || creds.Username == "" || creds.Password == "" {
		return nil, fmt.Errorf("domain join secret missing domain, username, or password")
	}
	return &creds, nil
}

// splitDomainUser splits "CORP\\joinsvc" or "joinsvc@corp.example.com" into the
// account name and its domain. Unattend wants them in separate elements; passing
// "CORP\joinsvc" as the Username makes the join fail with a bare "access denied".
func splitDomainUser(username, joinDomain string) (user, domain string) {
	if i := strings.LastIndex(username, "\\"); i >= 0 {
		return username[i+1:], username[:i]
	}
	if i := strings.LastIndex(username, "@"); i >= 0 {
		return username[:i], username[i+1:]
	}
	return username, joinDomain
}

// BuildDomainJoinUnattend renders a minimal unattend answer file whose only job is
// the offline/online domain join, via the Windows-supported UnattendedJoin component.
//
// The password still has to reach Windows somehow — there is no way around that — but
// this is the mechanism Microsoft designed for it: the credential lives in sysprep
// media rather than in cloud-init userdata and the Cloudbase-Init log, and Windows
// replaces it with *SENSITIVE*DATA*DELETED* in the cached copy under
// C:\Windows\Panther once the specialize pass completes.
//
// Use a least-privilege delegated join account. It should be able to join computers
// to the target OU and nothing else.
func BuildDomainJoinUnattend(creds *DomainJoinCredentials) string {
	user, credDomain := splitDomainUser(creds.Username, creds.Domain)

	var b strings.Builder
	b.WriteString(`<?xml version="1.0" encoding="utf-8"?>` + "\n")
	b.WriteString(`<unattend xmlns="urn:schemas-microsoft-com:unattend">` + "\n")
	b.WriteString(`  <settings pass="specialize">` + "\n")
	b.WriteString(`    <component name="Microsoft-Windows-UnattendedJoin" processorArchitecture="amd64" ` +
		`publicKeyToken="31bf3856ad364e35" language="neutral" versionScope="nonSxS" ` +
		`xmlns:wcm="http://schemas.microsoft.com/WMIConfig/2002/State">` + "\n")
	b.WriteString(`      <Identification>` + "\n")
	b.WriteString(`        <JoinDomain>` + xmlEscape(creds.Domain) + `</JoinDomain>` + "\n")
	if creds.OU != "" {
		b.WriteString(`        <MachineObjectOU>` + xmlEscape(creds.OU) + `</MachineObjectOU>` + "\n")
	}
	b.WriteString(`        <Credentials>` + "\n")
	b.WriteString(`          <Domain>` + xmlEscape(credDomain) + `</Domain>` + "\n")
	b.WriteString(`          <Username>` + xmlEscape(user) + `</Username>` + "\n")
	b.WriteString(`          <Password>` + xmlEscape(creds.Password) + `</Password>` + "\n")
	b.WriteString(`        </Credentials>` + "\n")
	b.WriteString(`      </Identification>` + "\n")
	b.WriteString(`    </component>` + "\n")
	b.WriteString(`  </settings>` + "\n")
	b.WriteString(`</unattend>` + "\n")
	return b.String()
}

// xmlEscape escapes the five XML predefined entities. A password containing `&` or
// `<` would otherwise produce an answer file Windows silently refuses to parse.
func xmlEscape(s string) string {
	var b strings.Builder
	if err := xml.EscapeText(&b, []byte(s)); err != nil {
		// EscapeText only fails if the writer fails; strings.Builder cannot.
		return s
	}
	return b.String()
}

// unattendedJoinComponent is the specialize-pass component, without the surrounding
// <settings> element — used when merging into a caller-supplied answer file.
func unattendedJoinComponent(creds *DomainJoinCredentials) string {
	full := BuildDomainJoinUnattend(creds)
	start := strings.Index(full, "    <component")
	end := strings.Index(full, "  </settings>")
	if start < 0 || end < 0 || end < start {
		return ""
	}
	return full[start:end]
}

// MergeDomainJoinIntoUnattend injects the UnattendedJoin component into a
// caller-supplied answer file, so a VM can carry both its own sysprep config and a
// Veyron-managed domain join.
//
// Returns an error rather than guessing when the document is not a recognizable
// unattend file: silently producing an answer file Windows won't parse means Setup
// stops at an interactive prompt on a headless VM, which is worse than failing here.
func MergeDomainJoinIntoUnattend(userXML string, creds *DomainJoinCredentials) (string, error) {
	if strings.Contains(userXML, "Microsoft-Windows-UnattendedJoin") {
		return "", fmt.Errorf(
			"answer file already contains a Microsoft-Windows-UnattendedJoin component; " +
				"remove it or drop windows.domainJoinSecretRef — Veyron will not merge two domain joins")
	}

	component := unattendedJoinComponent(creds)
	if component == "" {
		return "", fmt.Errorf("internal: could not render UnattendedJoin component")
	}

	// Preferred: reuse the existing specialize pass.
	if i := indexSpecializeOpen(userXML); i >= 0 {
		return userXML[:i] + component + userXML[i:], nil
	}

	// Otherwise add a specialize pass of our own, just before </unattend>.
	closeTag := strings.LastIndex(userXML, "</unattend>")
	if closeTag < 0 {
		return "", fmt.Errorf(
			"sysprep answer file has no </unattend> element — cannot inject the domain join; " +
				"put <Identification> in the answer file yourself and drop windows.domainJoinSecretRef")
	}
	block := "  <settings pass=\"specialize\">\n" + component + "  </settings>\n"
	return userXML[:closeTag] + block + userXML[closeTag:], nil
}

// indexSpecializeOpen returns the offset just after the opening
// <settings pass="specialize"> tag, or -1. Tolerates single quotes and spacing.
func indexSpecializeOpen(s string) int {
	for _, open := range []string{
		`<settings pass="specialize">`,
		`<settings pass='specialize'>`,
	} {
		if i := strings.Index(s, open); i >= 0 {
			return i + len(open) + 1 // +1 to land after the newline
		}
	}
	return -1
}
