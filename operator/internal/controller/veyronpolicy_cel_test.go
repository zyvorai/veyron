// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

package controller

import (
	"testing"

	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/util/intstr"

	veyronv1alpha1 "github.com/zyvorai/veyron/operator/api/v1alpha1"
)

func TestEvaluateCELExtendedVariables(t *testing.T) {
	vm := &veyronv1alpha1.VeyronVM{
		ObjectMeta: metav1.ObjectMeta{Name: "win", Namespace: "prod", Labels: map[string]string{"env": "prod"}},
	}
	vm.Spec.CPU.Cores = 8
	vm.Spec.Memory.Size = "16Gi"
	vm.Spec.GPUs = []veyronv1alpha1.GPUSpec{{}}
	facts := &VMFacts{
		RDPExposed:          map[string]bool{"win": true},
		InternetEgress:      map[string]bool{},
		HasSnapshotSchedule: map[string]bool{"win": true},
	}
	cases := map[string]bool{
		"cpu_cores > 4":                                true,
		"memory_gib >= 16.0":                           true,
		"gpu_count == 1":                               true,
		"!rdp_exposed":                                 false,
		"!internet_egress":                             true,
		"has_snapshot_schedule":                        true,
		"labels['env'] == 'prod'":                      true,
		"!('team' in labels) || labels['team'] != ''": true,
	}
	for expr, want := range cases {
		got, err := evaluateCEL(expr, vm, facts)
		if err != nil {
			t.Fatalf("%s: %v", expr, err)
		}
		if got != want {
			t.Errorf("%s = %v, want %v", expr, got, want)
		}
	}
	if got, err := evaluateCEL("rdp_exposed", vm, nil); err != nil || got {
		t.Errorf("nil facts should read as false, got %v %v", got, err)
	}
}

func TestRDPTarget(t *testing.T) {
	svc := &corev1.Service{Spec: corev1.ServiceSpec{
		Selector: map[string]string{"kubevirt.io/vm": "win"},
		Ports:    []corev1.ServicePort{{Port: 33890, TargetPort: intstr.FromInt(3389)}},
	}}
	if rdpTarget(svc) != "win" {
		t.Fatal("expected win")
	}
	svc.Spec.Ports[0].TargetPort = intstr.FromInt(22)
	if rdpTarget(svc) != "" {
		t.Fatal("ssh service is not RDP")
	}
}
