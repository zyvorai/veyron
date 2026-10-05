// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

package catalog

import (
	"testing"

	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
)

func kvVMWithDomain(annotationHash string, cores int64) *unstructured.Unstructured {
	obj := &unstructured.Unstructured{Object: map[string]interface{}{
		"spec": map[string]interface{}{
			"template": map[string]interface{}{
				"spec": map[string]interface{}{
					"domain": map[string]interface{}{
						"cpu": map[string]interface{}{"cores": cores},
					},
				},
			},
		},
	}}
	if annotationHash != "" {
		obj.SetAnnotations(map[string]string{"veyron.io/kubevirt-domain-hash": annotationHash})
	}
	return obj
}

func TestCompareKubeVirtSpecNoDriftWhenHashesMatch(t *testing.T) {
	kv := kvVMWithDomain("", 2)
	hash, err := KubeVirtDomainHash(kv)
	if err != nil {
		t.Fatal(err)
	}
	kv.SetAnnotations(map[string]string{"veyron.io/kubevirt-domain-hash": hash})

	drift, msg := CompareKubeVirtSpec("", kv)
	if drift {
		t.Fatalf("expected no drift when the domain matches its own annotation, got: %s", msg)
	}
}

// A live domain that no longer matches the hash recorded on its last reconcile
// (e.g. someone kubectl-patched cpu.cores directly) must be detected — this
// is what actually makes status.driftDetected / GET .../drift meaningful.
func TestCompareKubeVirtSpecDetectsExternalTampering(t *testing.T) {
	kv := kvVMWithDomain("", 2)
	hash, err := KubeVirtDomainHash(kv)
	if err != nil {
		t.Fatal(err)
	}
	kv.SetAnnotations(map[string]string{"veyron.io/kubevirt-domain-hash": hash})

	// Someone changes cores directly on the live object without going
	// through the VeyronVM spec — the annotation is now stale.
	_ = unstructuredSetCores(kv, 4)

	drift, msg := CompareKubeVirtSpec("", kv)
	if !drift {
		t.Fatal("expected drift when the live domain hash no longer matches the recorded annotation")
	}
	if msg == "" {
		t.Fatal("expected a non-empty drift message")
	}
}

func TestCompareKubeVirtSpecNoAnnotationMeansNoBaseline(t *testing.T) {
	// A VM that has never been annotated (e.g. pre-dates this feature) has
	// nothing to compare against — must not false-positive.
	kv := kvVMWithDomain("", 2)
	drift, _ := CompareKubeVirtSpec("", kv)
	if drift {
		t.Fatal("expected no drift when there is no prior annotation to compare against")
	}
}

func unstructuredSetCores(kv *unstructured.Unstructured, cores int64) error {
	return unstructured.SetNestedField(kv.Object, cores, "spec", "template", "spec", "domain", "cpu", "cores")
}
