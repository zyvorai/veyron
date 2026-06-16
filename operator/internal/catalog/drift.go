// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package catalog

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"

	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
)

func domainSnapshotFromKV(kvVM *unstructured.Unstructured) map[string]interface{} {
	if kvVM == nil {
		return nil
	}
	domain, found, err := unstructured.NestedMap(kvVM.Object, "spec", "template", "spec", "domain")
	if !found || err != nil {
		return nil
	}
	snapshot := map[string]interface{}{
		"cpu":    domain["cpu"],
		"memory": domain["memory"],
	}
	devices, _, _ := unstructured.NestedMap(domain, "devices")
	if devices != nil {
		snapshot["disks"] = devices["disks"]
		snapshot["interfaces"] = devices["interfaces"]
	}
	return snapshot
}

// KubeVirtDomainHash returns a stable SHA-256 hash of the KubeVirt VM domain snapshot.
func KubeVirtDomainHash(kvVM *unstructured.Unstructured) (string, error) {
	snapshot := domainSnapshotFromKV(kvVM)
	if snapshot == nil {
		return "", fmt.Errorf("no domain spec on KubeVirt VM")
	}
	b, err := json.Marshal(snapshot)
	if err != nil {
		return "", err
	}
	sum := sha256.Sum256(b)
	return hex.EncodeToString(sum[:]), nil
}

// CompareKubeVirtSpec returns true when the live KubeVirt domain differs from the last applied hash.
func CompareKubeVirtSpec(_ string, kvVM *unstructured.Unstructured) (bool, string) {
	if kvVM == nil {
		return false, ""
	}
	ann := kvVM.GetAnnotations()
	if ann == nil {
		return false, ""
	}
	expected := ann["veyron.io/kubevirt-domain-hash"]
	if expected == "" {
		return false, ""
	}
	actual, err := KubeVirtDomainHash(kvVM)
	if err != nil {
		return false, ""
	}
	if actual != expected {
		return true, fmt.Sprintf("KubeVirt VM spec drift detected (expected=%s actual=%s)", expected, actual)
	}
	return false, ""
}
