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

	vmroguev1alpha1 "github.com/ssahani/vmrogue/operator/api/v1alpha1"
)

// DomainJoinCredentials holds AD join parameters from a Secret JSON payload.
type DomainJoinCredentials struct {
	Domain   string `json:"domain"`
	OU       string `json:"ou,omitempty"`
	Username string `json:"username"`
	Password string `json:"password"`
}

// ResolveCloudInitUserData returns inline userData or loads it from Secret refs.
func ResolveCloudInitUserData(ctx context.Context, c client.Client, ns string, spec *vmroguev1alpha1.VMRogueVMSpec) (string, error) {
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

	if spec.Windows != nil && spec.Windows.SysprepSecretRef != nil {
		_, err := readSecretKey(ctx, c, ns, spec.Windows.SysprepSecretRef)
		if err != nil {
			return "", fmt.Errorf("windows sysprepSecretRef: %w", err)
		}
		// Sysprep unattend is consumed by Cloudbase-Init via metadata; annotate via comment in userData.
		if userData != "" {
			userData += "\n# sysprep unattend loaded from secret " + spec.Windows.SysprepSecretRef.Name
		}
	}

	return userData, nil
}

func readSecretKey(ctx context.Context, c client.Client, defaultNS string, ref *vmroguev1alpha1.SecretKeyRef) (string, error) {
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

func renderDomainJoinUserData(ctx context.Context, c client.Client, ns string, ref *vmroguev1alpha1.SecretKeyRef) (string, error) {
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
