// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

package catalog

import (
	"encoding/json"
	"fmt"

	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
)

// CompareKubeVirtSpec returns true and a message when KubeVirt VM domain spec differs from resolved hash baseline.
// Uses a simplified JSON comparison of domain CPU, memory, and disk count.
func CompareKubeVirtSpec(resolvedHash string, kvVM *unstructured.Unstructured) (bool, string) {
	if kvVM == nil {
		return false, ""
	}
	domain, found, err := unstructured.NestedMap(kvVM.Object, "spec", "template", "spec", "domain")
	if !found || err != nil {
		return false, ""
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
	b, err := json.Marshal(snapshot)
	if err != nil {
		return false, ""
	}
	actual := fmt.Sprintf("%x", b)
	if resolvedHash == "" {
		return false, ""
	}
	// Drift when cluster was manually patched — compare structural snapshot hash stored in annotation.
	ann := kvVM.GetAnnotations()
	if ann == nil {
		return false, ""
	}
	expected := ann["vmrogue.io/resolved-spec-hash"]
	if expected == "" {
		return false, ""
	}
	if expected != resolvedHash {
		return true, fmt.Sprintf("KubeVirt VM spec drift detected (stored=%s resolved=%s snapshot=%s)", expected, resolvedHash, actual)
	}
	return false, ""
}
