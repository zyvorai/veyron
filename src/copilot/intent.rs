// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopilotIntent {
    VmDoctor,
    SchedulingExplainer,
    UnhealthyFleet,
    YamlBuilder,
    BackupAdvisor,
    NetworkLens,
    GuestInspector,
    GuestFilesystem,
    StorageDoctor,
    SecuritySentinel,
    ErrorExplainer,
    GeneralHelp,
}

pub fn detect_intent(query: &str) -> CopilotIntent {
    let lower = query.to_lowercase();

    if lower.contains("backup") || lower.contains("velero") || lower.contains("unprotected") {
        return CopilotIntent::BackupAdvisor;
    }

    if lower.contains("storage doctor")
        || (lower.contains("pvc") && (lower.contains("full") || lower.contains("bloat")))
        || (lower.contains("disk") && lower.contains("full"))
        || lower.contains("snapshot sprawl")
    {
        return CopilotIntent::StorageDoctor;
    }

    if lower.contains("security sentinel")
        || (lower.contains("security") && (lower.contains("rdp") || lower.contains("exposed")))
        || lower.contains("security review")
    {
        return CopilotIntent::SecuritySentinel;
    }

    if lower.contains("guest filesystem")
        || lower.contains("guest fs")
        || (lower.contains("filesystem") && lower.contains("guest"))
        || (lower.contains("in-guest") && (lower.contains("disk") || lower.contains("storage")))
        || lower.contains("df -")
        || (lower.contains("df") && lower.contains("guest"))
    {
        return CopilotIntent::GuestFilesystem;
    }

    if lower.contains("guest agent")
        || lower.contains("guest inspector")
        || lower.contains("guest os")
        || (lower.contains("guest") && lower.contains("inspect"))
    {
        return CopilotIntent::GuestInspector;
    }

    if lower.contains("network")
        || lower.contains("connectivity")
        || lower.contains("multus")
        || lower.contains("firewall")
        || lower.contains("nodeport")
    {
        return CopilotIntent::NetworkLens;
    }

    if lower.contains("insufficient memory")
        || lower.contains("nodes are available")
        || lower.contains("persistentvolumeclaim")
        || lower.contains("forbidden")
        || (lower.contains("error") && lower.len() > 80)
    {
        return CopilotIntent::ErrorExplainer;
    }

    if lower.contains("unhealthy") || lower.contains("degraded") || lower.contains("failed vm") {
        return CopilotIntent::UnhealthyFleet;
    }

    if lower.contains("scheduling")
        || lower.contains("pending")
        || lower.contains("unschedulable")
        || lower.contains("stuck in scheduling")
    {
        return CopilotIntent::SchedulingExplainer;
    }

    if lower.contains("create")
        || lower.contains("generate yaml")
        || lower.contains("forge")
        || (lower.contains("windows") && (lower.contains("cpu") || lower.contains("ram")))
        || lower.contains("postgresql")
    {
        return CopilotIntent::YamlBuilder;
    }

    if lower.contains("why")
        || lower.contains("not starting")
        || lower.contains("won't start")
        || lower.contains("doctor")
        || lower.contains("health")
        || lower.contains("diagnose")
    {
        return CopilotIntent::VmDoctor;
    }

    if lower.contains("storage") && lower.contains("consum") {
        return CopilotIntent::VmDoctor;
    }

    CopilotIntent::GeneralHelp
}
