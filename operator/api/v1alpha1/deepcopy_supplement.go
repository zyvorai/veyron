// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Supplemental deepcopy for catalog CRDs and extended VM spec fields.

package v1alpha1

import (
	runtime "k8s.io/apimachinery/pkg/runtime"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

func (in *SecretKeyRef) DeepCopyInto(out *SecretKeyRef) {
	*out = *in
}

func (in *SecretKeyRef) DeepCopy() *SecretKeyRef {
	if in == nil {
		return nil
	}
	out := new(SecretKeyRef)
	in.DeepCopyInto(out)
	return out
}

func (in *WindowsSpec) DeepCopyInto(out *WindowsSpec) {
	*out = *in
	if in.SysprepSecretRef != nil {
		in, out := &in.SysprepSecretRef, &out.SysprepSecretRef
		*out = (*in).DeepCopy()
	}
	if in.DomainJoinSecretRef != nil {
		in, out := &in.DomainJoinSecretRef, &out.DomainJoinSecretRef
		*out = (*in).DeepCopy()
	}
}

func (in *WindowsSpec) DeepCopy() *WindowsSpec {
	if in == nil {
		return nil
	}
	out := new(WindowsSpec)
	in.DeepCopyInto(out)
	return out
}

func (in *GPUSpec) DeepCopyInto(out *GPUSpec) {
	*out = *in
	if in.VirtualGPUOptions != nil {
		in, out := &in.VirtualGPUOptions, &out.VirtualGPUOptions
		*out = (*in).DeepCopy()
	}
}

func (in *GPUSpec) DeepCopy() *GPUSpec {
	if in == nil {
		return nil
	}
	out := new(GPUSpec)
	in.DeepCopyInto(out)
	return out
}

func (in *VGPUOptionsSpec) DeepCopyInto(out *VGPUOptionsSpec) {
	*out = *in
	if in.Display != nil {
		in, out := &in.Display, &out.Display
		*out = new(bool)
		**out = **in
	}
	if in.RAMFB != nil {
		in, out := &in.RAMFB, &out.RAMFB
		*out = new(bool)
		**out = **in
	}
}

func (in *VGPUOptionsSpec) DeepCopy() *VGPUOptionsSpec {
	if in == nil {
		return nil
	}
	out := new(VGPUOptionsSpec)
	in.DeepCopyInto(out)
	return out
}

func (in *HostDeviceSpec) DeepCopyInto(out *HostDeviceSpec) {
	*out = *in
}

func (in *HostDeviceSpec) DeepCopy() *HostDeviceSpec {
	if in == nil {
		return nil
	}
	out := new(HostDeviceSpec)
	in.DeepCopyInto(out)
	return out
}

func (in *VMTemplate) DeepCopyInto(out *VMTemplate) {
	*out = *in
	out.TypeMeta = in.TypeMeta
	in.ObjectMeta.DeepCopyInto(&out.ObjectMeta)
	in.Spec.DeepCopyInto(&out.Spec)
	in.Status.DeepCopyInto(&out.Status)
}

func (in *VMTemplate) DeepCopy() *VMTemplate {
	if in == nil {
		return nil
	}
	out := new(VMTemplate)
	in.DeepCopyInto(out)
	return out
}

func (in *VMTemplate) DeepCopyObject() runtime.Object {
	if c := in.DeepCopy(); c != nil {
		return c
	}
	return nil
}

func (in *VMTemplateList) DeepCopyInto(out *VMTemplateList) {
	*out = *in
	out.TypeMeta = in.TypeMeta
	in.ListMeta.DeepCopyInto(&out.ListMeta)
	if in.Items != nil {
		in, out := &in.Items, &out.Items
		*out = make([]VMTemplate, len(*in))
		for i := range *in {
			(*in)[i].DeepCopyInto(&(*out)[i])
		}
	}
}

func (in *VMTemplateList) DeepCopy() *VMTemplateList {
	if in == nil {
		return nil
	}
	out := new(VMTemplateList)
	in.DeepCopyInto(out)
	return out
}

func (in *VMTemplateList) DeepCopyObject() runtime.Object {
	if c := in.DeepCopy(); c != nil {
		return c
	}
	return nil
}

func (in *VMTemplateSpec) DeepCopyInto(out *VMTemplateSpec) {
	*out = *in
	if in.Tags != nil {
		in, out := &in.Tags, &out.Tags
		*out = make([]string, len(*in))
		copy(*out, *in)
	}
	if in.RecommendedProfiles != nil {
		in, out := &in.RecommendedProfiles, &out.RecommendedProfiles
		*out = make([]string, len(*in))
		copy(*out, *in)
	}
	in.Default.DeepCopyInto(&out.Default)
}

func (in *VMTemplateSpec) DeepCopy() *VMTemplateSpec {
	if in == nil {
		return nil
	}
	out := new(VMTemplateSpec)
	in.DeepCopyInto(out)
	return out
}

func (in *VMTemplateStatus) DeepCopyInto(out *VMTemplateStatus) {
	*out = *in
	if in.Conditions != nil {
		in, out := &in.Conditions, &out.Conditions
		*out = make([]metav1.Condition, len(*in))
		for i := range *in {
			(*in)[i].DeepCopyInto(&(*out)[i])
		}
	}
}

func (in *VMTemplateStatus) DeepCopy() *VMTemplateStatus {
	if in == nil {
		return nil
	}
	out := new(VMTemplateStatus)
	in.DeepCopyInto(out)
	return out
}

func (in *VMProfile) DeepCopyInto(out *VMProfile) {
	*out = *in
	out.TypeMeta = in.TypeMeta
	in.ObjectMeta.DeepCopyInto(&out.ObjectMeta)
	in.Spec.DeepCopyInto(&out.Spec)
	in.Status.DeepCopyInto(&out.Status)
}

func (in *VMProfile) DeepCopy() *VMProfile {
	if in == nil {
		return nil
	}
	out := new(VMProfile)
	in.DeepCopyInto(out)
	return out
}

func (in *VMProfile) DeepCopyObject() runtime.Object {
	if c := in.DeepCopy(); c != nil {
		return c
	}
	return nil
}

func (in *VMProfileList) DeepCopyInto(out *VMProfileList) {
	*out = *in
	out.TypeMeta = in.TypeMeta
	in.ListMeta.DeepCopyInto(&out.ListMeta)
	if in.Items != nil {
		in, out := &in.Items, &out.Items
		*out = make([]VMProfile, len(*in))
		for i := range *in {
			(*in)[i].DeepCopyInto(&(*out)[i])
		}
	}
}

func (in *VMProfileList) DeepCopy() *VMProfileList {
	if in == nil {
		return nil
	}
	out := new(VMProfileList)
	in.DeepCopyInto(out)
	return out
}

func (in *VMProfileList) DeepCopyObject() runtime.Object {
	if c := in.DeepCopy(); c != nil {
		return c
	}
	return nil
}

func (in *VMProfileSpec) DeepCopyInto(out *VMProfileSpec) {
	*out = *in
	if in.UseCases != nil {
		in, out := &in.UseCases, &out.UseCases
		*out = make([]string, len(*in))
		copy(*out, *in)
	}
	if in.RecommendedTemplates != nil {
		in, out := &in.RecommendedTemplates, &out.RecommendedTemplates
		*out = make([]string, len(*in))
		copy(*out, *in)
	}
}

func (in *VMProfileSpec) DeepCopy() *VMProfileSpec {
	if in == nil {
		return nil
	}
	out := new(VMProfileSpec)
	in.DeepCopyInto(out)
	return out
}

func (in *VMProfileStatus) DeepCopyInto(out *VMProfileStatus) {
	*out = *in
	if in.Conditions != nil {
		in, out := &in.Conditions, &out.Conditions
		*out = make([]metav1.Condition, len(*in))
		for i := range *in {
			(*in)[i].DeepCopyInto(&(*out)[i])
		}
	}
}

func (in *VMProfileStatus) DeepCopy() *VMProfileStatus {
	if in == nil {
		return nil
	}
	out := new(VMProfileStatus)
	in.DeepCopyInto(out)
	return out
}

// DeepCopyVeyronVMSpecExtended copies Windows and cloud-init secret refs (post-codegen fields).
func DeepCopyVeyronVMSpecExtended(in *VeyronVMSpec) VeyronVMSpec {
	out := *in
	if in.CloudInit != nil {
		ci := *in.CloudInit
		if in.CloudInit.UserDataSecretRef != nil {
			ci.UserDataSecretRef = in.CloudInit.UserDataSecretRef.DeepCopy()
		}
		out.CloudInit = &ci
	}
	if in.Windows != nil {
		out.Windows = in.Windows.DeepCopy()
	}
	return out
}
