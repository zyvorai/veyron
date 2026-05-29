// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopilotIntent {
    VmDoctor,
    SchedulingExplainer,
    UnhealthyFleet,
    YamlBuilder,
    MigrationAdvisor,
    ErrorExplainer,
    GeneralHelp,
}

pub fn detect_intent(query: &str) -> CopilotIntent {
    let lower = query.to_lowercase();

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

    if lower.contains("migrate") || lower.contains("vmware") || lower.contains("vmdk") {
        return CopilotIntent::MigrationAdvisor;
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
