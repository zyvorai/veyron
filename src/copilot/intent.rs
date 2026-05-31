// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopilotIntent {
    VmDoctor,
    SchedulingExplainer,
    UnhealthyFleet,
    YamlBuilder,
    BackupAdvisor,
    CostAdvisor,
    NetworkLens,
    GuestInspector,
    GuestFilesystem,
    StorageDoctor,
    SecuritySentinel,
    PerformanceAdvisor,
    GitopsAdvisor,
    ForecastAdvisor,
    IntegrationsAdvisor,
    ComplianceAdvisor,
    ObservabilityAdvisor,
    CiliumAdvisor,
    NodeAdvisor,
    DriftAdvisor,
    AlertAdvisor,
    SloAdvisor,
    MigrationAdvisor,
    QuotaAdvisor,
    CatalogAdvisor,
    VeleroDrAdvisor,
    ErrorExplainer,
    GeneralHelp,
}

pub fn detect_intent(query: &str) -> CopilotIntent {
    let lower = query.to_lowercase();

    if lower.contains("velero dr")
        || lower.contains("velero advisor")
        || (lower.contains("disaster recovery") && lower.contains("velero"))
        || (lower.contains("dr readiness") && !lower.contains("gitops"))
    {
        return CopilotIntent::VeleroDrAdvisor;
    }

    if lower.contains("catalog advisor")
        || lower.contains("catalog health")
        || lower.contains("catalog sync")
        || (lower.contains("catalog") && lower.contains("sync"))
        || (lower.contains("template") && lower.contains("missing"))
    {
        return CopilotIntent::CatalogAdvisor;
    }

    if lower.contains("quota advisor")
        || lower.contains("quota pressure")
        || (lower.contains("resourcequota") && lower.contains("full"))
        || (lower.contains("quota") && lower.contains("limit"))
    {
        return CopilotIntent::QuotaAdvisor;
    }

    if lower.contains("cve")
        || lower.contains("trivy")
        || lower.contains("vulnerability")
    {
        return CopilotIntent::SecuritySentinel;
    }

    if lower.contains("migration advisor")
        || lower.contains("live migration")
        || (lower.contains("migration") && (lower.contains("running") || lower.contains("in progress")))
    {
        return CopilotIntent::MigrationAdvisor;
    }

    if lower.contains("slo advisor")
        || lower.contains("availability slo")
        || (lower.contains("fleet") && lower.contains("availability"))
    {
        return CopilotIntent::SloAdvisor;
    }

    if lower.contains("alert advisor")
        || lower.contains("warning events")
        || (lower.contains("active") && lower.contains("alerts"))
    {
        return CopilotIntent::AlertAdvisor;
    }

    if lower.contains("drift advisor")
        || lower.contains("operator drift")
        || lower.contains("template drift")
        || (lower.contains("drift") && lower.contains("vmrogue"))
        || (lower.contains("which") && lower.contains("drift") && !lower.contains("gitops"))
    {
        return CopilotIntent::DriftAdvisor;
    }

    if lower.contains("node advisor")
        || lower.contains("node capacity")
        || lower.contains("node pressure")
        || (lower.contains("nodes") && lower.contains("ready"))
    {
        return CopilotIntent::NodeAdvisor;
    }

    if lower.contains("cilium advisor")
        || lower.contains("cilium posture")
        || (lower.contains("network policy") && lower.contains("posture"))
    {
        return CopilotIntent::CiliumAdvisor;
    }

    if lower.contains("observability advisor")
        || lower.contains("observability stack")
        || (lower.contains("observability") && lower.contains("installed"))
    {
        return CopilotIntent::ObservabilityAdvisor;
    }

    if lower.contains("compliance advisor")
        || lower.contains("compliance score")
        || lower.contains("compliance posture")
    {
        return CopilotIntent::ComplianceAdvisor;
    }

    if lower.contains("integrations advisor")
        || lower.contains("which integrations")
        || (lower.contains("integration") && lower.contains("configured"))
        || lower.contains("optional backends")
    {
        return CopilotIntent::IntegrationsAdvisor;
    }

    if lower.contains("forecast advisor")
        || lower.contains("capacity forecast")
        || lower.contains("capacity growth")
        || (lower.contains("forecast") && (lower.contains("capacity") || lower.contains("30-day")))
    {
        return CopilotIntent::ForecastAdvisor;
    }

    if lower.contains("gitops advisor")
        || lower.contains("gitops drift")
        || (lower.contains("gitops") && (lower.contains("drift") || lower.contains("argo") || lower.contains("flux")))
    {
        return CopilotIntent::GitopsAdvisor;
    }

    if lower.contains("performance advisor")
        || lower.contains("performance hotspot")
        || (lower.contains("high") && (lower.contains("cpu") || lower.contains("memory")))
        || (lower.contains("hot") && lower.contains("vm"))
    {
        return CopilotIntent::PerformanceAdvisor;
    }

    if lower.contains("backup") || lower.contains("velero") || lower.contains("unprotected") {
        return CopilotIntent::BackupAdvisor;
    }

    if lower.contains("cost advisor")
        || lower.contains("cost copilot")
        || lower.contains("expensive")
        || (lower.contains("cost") && lower.contains("spend"))
        || lower.contains("finops")
        || (lower.contains("how much") && lower.contains("vm"))
    {
        return CopilotIntent::CostAdvisor;
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
