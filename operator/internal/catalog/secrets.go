// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package catalog

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/types"
	"sigs.k8s.io/controller-runtime/pkg/client"

	veyronv1alpha1 "github.com/ssahani/Veyron/operator/api/v1alpha1"
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

	if spec.Windows != nil && spec.Windows.DomainJoinSecretRef != nil {
		joinScript, err := renderDomainJoinUserData(ctx, c, ns, spec.Windows.DomainJoinSecretRef)
		if err != nil {
			return "", err
		}
		if userData != "" {
			userData = userData + "\n" + joinScript
		} else {
			userData = joinScript
		}
	}

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

func renderDomainJoinUserData(ctx context.Context, c client.Client, ns string, ref *veyronv1alpha1.SecretKeyRef) (string, error) {
	raw, err := readSecretKey(ctx, c, ns, ref)
	if err != nil {
		return "", err
	}
	var creds DomainJoinCredentials
	if err := json.Unmarshal([]byte(raw), &creds); err != nil {
		return "", fmt.Errorf("domain join secret JSON: %w", err)
	}
	if creds.Domain == "" || creds.Username == "" || creds.Password == "" {
		return "", fmt.Errorf("domain join secret missing domain, username, or password")
	}
	domain := strings.ReplaceAll(creds.Domain, "'", "''")
	user := strings.ReplaceAll(creds.Username, "'", "''")
	pass := strings.ReplaceAll(creds.Password, "'", "''")
	ou := strings.ReplaceAll(creds.OU, "'", "''")

	script := "#ps1_sysnative\n"
	if ou != "" {
		script += fmt.Sprintf("$ou = '%s'\n", ou)
	}
	script += fmt.Sprintf("$domain = '%s'\n$user = '%s'\n$pass = '%s' | ConvertTo-SecureString -AsPlainText -Force\n", domain, user, pass)
	script += "$cred = New-Object System.Management.Automation.PSCredential($user, $pass)\n"
	if ou != "" {
		script += "Add-Computer -DomainName $domain -Credential $cred -OUPath $ou -Force -Restart\n"
	} else {
		script += "Add-Computer -DomainName $domain -Credential $cred -Force -Restart\n"
	}
	return script, nil
}
