// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

package controller

import (
	"context"

	"k8s.io/apimachinery/pkg/apis/meta/v1/unstructured"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/util/workqueue"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/event"
	"sigs.k8s.io/controller-runtime/pkg/reconcile"
)

// EnqueueForOwner enqueues requests for the owner of an unstructured resource.
// This is used to watch KubeVirt VMIs and re-reconcile the owning VeyronVM.
type EnqueueForOwner struct {
	OwnerKind string
}

func (e *EnqueueForOwner) Create(ctx context.Context, evt event.CreateEvent, q workqueue.TypedRateLimitingInterface[reconcile.Request]) {
	e.enqueueOwner(evt.Object, q)
}

func (e *EnqueueForOwner) Update(ctx context.Context, evt event.UpdateEvent, q workqueue.TypedRateLimitingInterface[reconcile.Request]) {
	e.enqueueOwner(evt.ObjectNew, q)
}

func (e *EnqueueForOwner) Delete(ctx context.Context, evt event.DeleteEvent, q workqueue.TypedRateLimitingInterface[reconcile.Request]) {
	e.enqueueOwner(evt.Object, q)
}

func (e *EnqueueForOwner) Generic(ctx context.Context, evt event.GenericEvent, q workqueue.TypedRateLimitingInterface[reconcile.Request]) {
	e.enqueueOwner(evt.Object, q)
}

func (e *EnqueueForOwner) enqueueOwner(obj client.Object, q workqueue.TypedRateLimitingInterface[reconcile.Request]) {
	if obj == nil {
		return
	}

	// For unstructured objects, look at owner references
	if u, ok := obj.(*unstructured.Unstructured); ok {
		for _, ref := range u.GetOwnerReferences() {
			if ref.Kind == e.OwnerKind {
				q.Add(reconcile.Request{
					NamespacedName: types.NamespacedName{
						Name:      ref.Name,
						Namespace: u.GetNamespace(),
					},
				})
				return
			}
		}
	}

	// Also check typed objects
	for _, ref := range obj.GetOwnerReferences() {
		if ref.Kind == e.OwnerKind {
			q.Add(reconcile.Request{
				NamespacedName: types.NamespacedName{
					Name:      ref.Name,
					Namespace: obj.GetNamespace(),
				},
			})
			return
		}
	}
}

// EnqueueVmiForVeyronVM enqueues the VeyronVM with the same name/namespace as a VMI.
// VMI owner references point at the KubeVirt VirtualMachine, which shares its name with VeyronVM.
type EnqueueVmiForVeyronVM struct{}

func (e *EnqueueVmiForVeyronVM) Create(ctx context.Context, evt event.CreateEvent, q workqueue.TypedRateLimitingInterface[reconcile.Request]) {
	e.enqueue(evt.Object, q)
}

func (e *EnqueueVmiForVeyronVM) Update(ctx context.Context, evt event.UpdateEvent, q workqueue.TypedRateLimitingInterface[reconcile.Request]) {
	e.enqueue(evt.ObjectNew, q)
}

func (e *EnqueueVmiForVeyronVM) Delete(ctx context.Context, evt event.DeleteEvent, q workqueue.TypedRateLimitingInterface[reconcile.Request]) {
	e.enqueue(evt.Object, q)
}

func (e *EnqueueVmiForVeyronVM) Generic(ctx context.Context, evt event.GenericEvent, q workqueue.TypedRateLimitingInterface[reconcile.Request]) {
	e.enqueue(evt.Object, q)
}

func (e *EnqueueVmiForVeyronVM) enqueue(obj client.Object, q workqueue.TypedRateLimitingInterface[reconcile.Request]) {
	if obj == nil {
		return
	}
	q.Add(reconcile.Request{
		NamespacedName: types.NamespacedName{
			Name:      obj.GetName(),
			Namespace: obj.GetNamespace(),
		},
	})
}
