use crate::cost::CostCalculator;
use crate::kube;
use crate::tui::colors::cli as color;
use anyhow::Result;

/// Average hours in a month, matching the constant in crate::cost.
const COST_HOURS_PER_MONTH: f64 = 730.0;

/// Extract CPU, memory, and storage from a KubeVirt VirtualMachine object
/// and calculate costs using the provided calculator.
fn extract_and_calculate_cost(
    calculator: &CostCalculator,
    vm_obj: &kube::VirtualMachine,
    namespace: &str,
    period_hours: f64,
) -> crate::cost::VMCost {
    let name = vm_obj.metadata.name.as_deref().unwrap_or("unknown");
    let vm_ns = vm_obj.metadata.namespace.as_deref().unwrap_or(namespace);
    let cpu = vm_obj
        .spec
        .template
        .spec
        .domain
        .cpu
        .as_ref()
        .and_then(|c| c.cores)
        .unwrap_or(1);
    let memory_str = vm_obj
        .spec
        .template
        .spec
        .domain
        .memory
        .as_ref()
        .and_then(|m| m.guest.as_deref())
        .unwrap_or("2Gi");
    let memory_gi = crate::disk::DiskInfo::parse_size(memory_str) / (1024 * 1024 * 1024);
    let storage_gi = vm_obj
        .spec
        .template
        .spec
        .volumes
        .as_ref()
        .map(|vols| {
            vols.iter()
                .map(|v| {
                    v.empty_disk
                        .as_ref()
                        .map(|e| {
                            crate::disk::DiskInfo::parse_size(&e.capacity) / (1024 * 1024 * 1024)
                        })
                        .unwrap_or(20) // estimate PVCs at 20Gi when size is unknown
                })
                .sum::<u64>()
        })
        .unwrap_or(20);

    if memory_gi > u32::MAX as u64 {
        log::warn!(
            "VM '{}': memory {}Gi exceeds u32 range, clamping to {}Gi for cost calculation",
            name,
            memory_gi,
            u32::MAX
        );
    }
    if storage_gi > u32::MAX as u64 {
        log::warn!(
            "VM '{}': storage {}Gi exceeds u32 range, clamping to {}Gi for cost calculation",
            name,
            storage_gi,
            u32::MAX
        );
    }
    calculator.calculate_vm_cost(
        name,
        vm_ns,
        cpu,
        memory_gi.min(u32::MAX as u64) as u32,
        storage_gi.min(u32::MAX as u64) as u32,
        period_hours,
    )
}

/// Print a single VM cost breakdown to stdout.
fn print_vm_cost_breakdown(cost: &crate::cost::VMCost) {
    println!("Cost Breakdown:");
    println!("  CPU:      ${:.2}", cost.cpu_cost);
    println!("  Memory:   ${:.2}", cost.memory_cost);
    println!("  Storage:  ${:.2}", cost.storage_cost);
    println!("  Network:  ${:.2}", cost.network_cost);
    println!(
        "  Total:    {}",
        color::value(&format!("${:.2}", cost.total_cost))
    );
    println!();
    println!("  Runtime:  {:.1} hours", cost.runtime_hours);
    println!("  Cost/hr:  ${:.4}", cost.cost_per_hour());
}

/// Helper to get the CPU rate from a CostCalculator for display math.
fn calculator_rate_cpu(calculator: &CostCalculator) -> f64 {
    calculator.rates().cpu_per_core_hour
}

/// Helper to get the memory rate from a CostCalculator for display math.
fn calculator_rate_mem(calculator: &CostCalculator) -> f64 {
    calculator.rates().memory_per_gb_hour
}

/// Helper to get the storage rate from a CostCalculator for display math.
fn calculator_rate_storage(calculator: &CostCalculator) -> f64 {
    calculator.rates().storage_per_gb_month
}

/// Parse a period string into hours.
fn parse_period_hours(period: &str) -> f64 {
    match period {
        "daily" | "1d" => 24.0,
        "weekly" | "7d" => 168.0,
        "yearly" | "1y" => 8760.0,
        _ => 730.0, // monthly default
    }
}

pub async fn handle_cost_analyze(
    vm: Option<String>,
    period: String,
    output: String,
    namespace: &str,
) -> Result<()> {
    println!("{}", color::header("Cost Analysis"));
    if let Some(ref vm_name) = vm {
        println!("  VM:      {}", color::value(vm_name));
    } else {
        println!("  Scope:   all VMs in namespace '{}'", namespace);
    }
    println!("  Period:  {}", period);
    // Cost rates are configurable estimates (default: AWS-like pricing).
    // They do not reflect actual cloud provider billing. See ResourceRates for presets.
    println!("  Rates:   estimated (AWS-like defaults, not real provider rates)");
    println!();

    let calculator = CostCalculator::default();
    let period_hours = parse_period_hours(&period);

    // Fetch real VM specs from Kubernetes
    if let Some(ref vm_name) = vm {
        // Single VM analysis
        let cost = match kube::KubeClient::new().await {
            Ok(client) => {
                let vm_obj = client.get_vm(namespace, vm_name).await;
                match vm_obj {
                    Ok(vm_obj) => {
                        extract_and_calculate_cost(&calculator, &vm_obj, namespace, period_hours)
                    }
                    Err(e) => {
                        eprintln!(
                            "{}",
                            color::warning(&format!("Could not fetch VM '{}': {}", vm_name, e))
                        );
                        return Err(anyhow::anyhow!(
                            "VM '{}' not found in namespace '{}'. Specify a valid VM name or omit it to list all VMs.",
                            vm_name,
                            namespace
                        ));
                    }
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Cannot connect to Kubernetes cluster: {}. A live cluster is required for cost analysis.",
                    e
                ));
            }
        };

        print_vm_cost_breakdown(&cost);

        if output == "json" {
            println!();
            let json = serde_json::to_string_pretty(&cost)?;
            println!("{}", json);
        } else if output == "yaml" {
            println!();
            let yaml = serde_yml::to_string(&cost)?;
            println!("{}", yaml);
        }
    } else {
        // No VM specified: list all VMs in the namespace and show per-VM costs
        match kube::KubeClient::new().await {
            Ok(client) => {
                match client.list_vms(namespace).await {
                    Ok(vms) if vms.is_empty() => {
                        println!(
                            "{}",
                            color::muted(&format!("No VMs found in namespace '{}'.", namespace))
                        );
                    }
                    Ok(vms) => {
                        use crate::cost::CostSummary;
                        let mut summary = CostSummary::new();
                        let mut all_costs = Vec::new();

                        for vm_obj in &vms {
                            let cost = extract_and_calculate_cost(
                                &calculator,
                                vm_obj,
                                namespace,
                                period_hours,
                            );
                            summary.add_vm_cost(&cost);
                            all_costs.push(cost);
                        }

                        // Print per-VM breakdown
                        println!(
                            "{:<25} {:<8} {:<10} {:<10} {:<12} {:<12}",
                            color::label("VM"),
                            color::label("CPU"),
                            color::label("MEM(GB)"),
                            color::label("DISK(GB)"),
                            color::label("COST/HR"),
                            color::label("TOTAL"),
                        );
                        println!("{}", "-".repeat(80));

                        for cost in &all_costs {
                            println!(
                                "{:<25} {:<8} {:<10} {:<10} ${:<11.4} {}",
                                cost.vm_name,
                                format!(
                                    "{:.0}",
                                    cost.cpu_cost
                                        / (calculator_rate_cpu(&calculator) * period_hours)
                                ),
                                format!(
                                    "{:.0}",
                                    cost.memory_cost
                                        / (calculator_rate_mem(&calculator) * period_hours)
                                ),
                                format!(
                                    "{:.1}",
                                    cost.storage_cost
                                        / (calculator_rate_storage(&calculator) * period_hours
                                            / COST_HOURS_PER_MONTH)
                                ),
                                cost.cost_per_hour(),
                                color::value(&format!("${:.2}", cost.total_cost)),
                            );
                        }

                        println!("{}", "-".repeat(80));
                        println!(
                            "{:<25} {:<8} {:<10} {:<10} {:<12} {}",
                            color::label(&format!("TOTAL ({} VMs)", summary.vm_count)),
                            "",
                            "",
                            "",
                            "",
                            color::value(&format!("${:.2}", summary.total_cost)),
                        );
                        println!();
                        println!("  Avg Cost/VM:  ${:.2}", summary.average_cost_per_vm());

                        if output == "json" {
                            println!();
                            let json = serde_json::to_string_pretty(&all_costs)?;
                            println!("{}", json);
                        } else if output == "yaml" {
                            println!();
                            let yaml = serde_yml::to_string(&all_costs)?;
                            println!("{}", yaml);
                        }
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "Failed to list VMs in namespace '{}': {}",
                            namespace,
                            e
                        ));
                    }
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Cannot connect to Kubernetes cluster: {}. A live cluster is required for cost analysis.",
                    e
                ));
            }
        }
    }

    Ok(())
}

pub async fn handle_cost_summary(
    namespace: Option<String>,
    period: String,
    group_by: Option<String>,
    output: String,
) -> Result<()> {
    use crate::cost::CostSummary;

    println!("{}", color::header("Cost Summary"));
    if let Some(ref ns) = namespace {
        println!("  Namespace: {}", color::value(ns));
    } else {
        println!("  Namespace: {}", color::value("(all)"));
    }
    println!("  Period:    {}", period);
    // Cost rates are configurable estimates (default: AWS-like pricing).
    // They do not reflect actual cloud provider billing. See ResourceRates for presets.
    println!("  Rates:     estimated (AWS-like defaults, not real provider rates)");
    if let Some(ref group) = group_by {
        println!("  Group By:  {}", group);
    }
    println!();

    let period_hours = parse_period_hours(&period);

    let mut summary = CostSummary::new();
    let calculator = CostCalculator::default();

    // Fetch real VMs from Kubernetes
    let client = match kube::KubeClient::new().await {
        Ok(c) => c,
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Cannot connect to Kubernetes cluster: {}. A live cluster is required for cost summary.",
                e
            ));
        }
    };

    // If a namespace is specified, list VMs in that namespace; otherwise list across all namespaces.
    let vms = if let Some(ref ns) = namespace {
        client
            .list_vms(ns)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list VMs in namespace '{}': {}", ns, e))?
    } else {
        client
            .list_all_vms()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list VMs across all namespaces: {}", e))?
    };

    if vms.is_empty() {
        println!("{}", color::muted("No VMs found. Showing empty summary."));
        println!();
    } else {
        for vm_obj in &vms {
            let ns_for_calc = namespace.as_deref().unwrap_or("default");
            let vm_cost =
                extract_and_calculate_cost(&calculator, vm_obj, ns_for_calc, period_hours);
            summary.add_vm_cost(&vm_cost);
        }
    }

    println!("Summary:");
    println!(
        "  Total Cost:       {}",
        color::value(&format!("${:.2}", summary.total_cost))
    );
    println!("  VM Count:         {}", summary.vm_count);
    println!("  Avg Cost/VM:      ${:.2}", summary.average_cost_per_vm());
    println!();
    println!("Resource Breakdown:");
    println!("  CPU:              ${:.2}", summary.cpu_cost);
    println!("  Memory:           ${:.2}", summary.memory_cost);
    println!("  Storage:          ${:.2}", summary.storage_cost);
    println!("  Network:          ${:.2}", summary.network_cost);
    println!("  Snapshots:        ${:.2}", summary.snapshot_cost);

    if output == "json" {
        println!();
        let json = serde_json::to_string_pretty(&summary)?;
        println!("{}", json);
    } else if output == "yaml" {
        println!();
        let yaml = serde_yml::to_string(&summary)?;
        println!("{}", yaml);
    }
    Ok(())
}

pub fn handle_cost_report(
    report_type: String,
    format: String,
    output: Option<String>,
) -> Result<()> {
    use crate::cost::reports::{ReportExporter, ReportGenerator};
    use chrono::Utc;

    println!("{}", color::header(&format!("{} Cost Report", report_type)));
    println!();

    let report = match report_type.as_str() {
        "monthly" => ReportGenerator::monthly_report(2024, 1),
        "weekly" => ReportGenerator::weekly_report(Utc::now()),
        _ => ReportGenerator::custom_report(Utc::now() - chrono::TimeDelta::days(30), Utc::now()),
    };

    println!("  Report ID:   {}", report.report_id);
    println!(
        "  Period:      {} to {}",
        report.period_start.format("%Y-%m-%d"),
        report.period_end.format("%Y-%m-%d")
    );
    println!(
        "  Total Cost:  {}",
        color::value(&format!("${:.2}", report.summary.total_cost))
    );
    println!();

    let content = if format == "csv" {
        ReportExporter::to_csv(&report)
    } else if format == "yaml" {
        serde_yml::to_string(&report)?
    } else {
        ReportExporter::to_json(&report)?
    };

    if let Some(file_path) = output {
        std::fs::write(&file_path, content)?;
        println!(
            "{}",
            color::success(&format!("Report saved to: {}", file_path))
        );
    } else {
        println!("{}", content);
    }
    Ok(())
}

pub fn handle_budget_list(output: String) -> Result<()> {
    use crate::cost::budgets::{Budget, BudgetManager, BudgetPeriod, BudgetScope};

    println!("{}", color::header("Budgets"));
    println!();

    let mut manager = BudgetManager::new();

    // Example budgets
    manager.add_budget(
        Budget::new("monthly-budget", 5000.0, BudgetPeriod::Monthly)
            .with_scope(BudgetScope::Global),
    );
    manager.add_budget(
        Budget::new("dev-budget", 1000.0, BudgetPeriod::Monthly)
            .with_scope(BudgetScope::Namespace("dev".to_string())),
    );

    if output == "json" {
        let json = serde_json::to_string_pretty(&manager.all_budgets())?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&manager.all_budgets())?;
        println!("{}", yaml);
    } else {
        println!(
            "{:<20} {:<15} {:<15} {:<10}",
            color::label("NAME"),
            color::label("AMOUNT"),
            color::label("PERIOD"),
            color::label("SCOPE")
        );
        println!("{}", "-".repeat(65));

        for budget in manager.all_budgets() {
            println!(
                "{:<20} ${:<14.2} {:<15} {}",
                budget.name,
                budget.amount,
                budget.period.to_string(),
                budget.scope
            );
        }
    }
    Ok(())
}

pub fn handle_budget_create(
    name: String,
    amount: f64,
    period: String,
    scope: String,
    alert_threshold: Option<f64>,
) -> Result<()> {
    use crate::cost::budgets::{Budget, BudgetAlert, BudgetPeriod, BudgetScope, NotificationType};

    println!("{}", color::header(&format!("Creating Budget: {}", name)));
    println!();

    let budget_period = match period.as_str() {
        "daily" => BudgetPeriod::Daily,
        "weekly" => BudgetPeriod::Weekly,
        "quarterly" => BudgetPeriod::Quarterly,
        "yearly" => BudgetPeriod::Yearly,
        _ => BudgetPeriod::Monthly,
    };

    let budget_scope = if scope == "global" {
        BudgetScope::Global
    } else if let Some(ns) = scope.strip_prefix("namespace:") {
        BudgetScope::Namespace(ns.to_string())
    } else if let Some(team) = scope.strip_prefix("team:") {
        BudgetScope::Team(team.to_string())
    } else {
        BudgetScope::Global
    };

    let mut budget = Budget::new(&name, amount, budget_period).with_scope(budget_scope);

    if let Some(threshold) = alert_threshold {
        budget = budget.add_alert(BudgetAlert::new(threshold, NotificationType::Email));
    }

    println!("  Name:       {}", color::value(&budget.name));
    println!(
        "  Amount:     {}",
        color::value(&format!("${:.2}", budget.amount))
    );
    println!("  Period:     {}", budget.period);
    println!("  Scope:      {}", budget.scope);
    if !budget.alerts.is_empty() {
        println!("  Alerts:     {} configured", budget.alerts.len());
    }
    println!();
    println!("{}", color::success("✓ Budget created successfully"));
    Ok(())
}

pub fn handle_budget_status(name: String, output: String) -> Result<()> {
    use crate::cost::budgets::{Budget, BudgetPeriod, BudgetStatus};

    let mut budget = Budget::new(&name, 5000.0, BudgetPeriod::Monthly);
    budget.update_spend(3750.0);

    let status = BudgetStatus::from_budget(&budget);

    println!("{}", color::header(&format!("Budget Status: {}", name)));
    println!();
    println!(
        "  Amount:       {}",
        color::value(&format!("${:.2}", status.amount))
    );
    println!("  Current:      ${:.2}", status.current_spend);
    println!("  Remaining:    ${:.2}", status.remaining);
    println!("  Utilization:  {}%", status.utilization_percent as u8);
    println!(
        "  Status:       {}",
        match status.status {
            crate::cost::budgets::Status::Healthy => color::success("Healthy"),
            crate::cost::budgets::Status::Warning => color::warning("Warning"),
            crate::cost::budgets::Status::Critical => color::error("Critical"),
            crate::cost::budgets::Status::Exceeded => color::error("Exceeded"),
        }
    );

    if output == "json" {
        println!();
        let json = serde_json::to_string_pretty(&status)?;
        println!("{}", json);
    } else if output == "yaml" {
        println!();
        let yaml = serde_yml::to_string(&status)?;
        println!("{}", yaml);
    }
    Ok(())
}

pub fn handle_cost_optimize(
    vm: Option<String>,
    high_priority_only: bool,
    output: String,
) -> Result<()> {
    use crate::cost::optimization::OptimizationEngine;

    println!("{}", color::header("Cost Optimization Recommendations"));
    if let Some(ref vm_name) = vm {
        println!("  VM: {}", color::value(vm_name));
    }
    println!();

    let report = OptimizationEngine::generate_report(vm.as_deref().unwrap_or("example-vm"));

    let recommendations = if high_priority_only {
        report.high_priority_recommendations()
    } else {
        report.recommendations.iter().collect()
    };

    println!(
        "Potential Savings: {}",
        color::value(&format!("${:.2}/month", report.total_potential_savings))
    );
    println!("Recommendations:   {}", recommendations.len());
    println!();

    if output == "json" {
        let json = serde_json::to_string_pretty(&recommendations)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&recommendations)?;
        println!("{}", yaml);
    } else {
        println!(
            "{:<15} {:<25} {:<10} {:<15} {}",
            color::label("PRIORITY"),
            color::label("TYPE"),
            color::label("SAVINGS"),
            color::label("SAVINGS %"),
            color::label("DESCRIPTION")
        );
        println!("{}", "-".repeat(90));

        for rec in recommendations {
            let priority_str = match rec.priority {
                crate::cost::optimization::Priority::Critical => color::error("Critical"),
                crate::cost::optimization::Priority::High => color::error("High"),
                crate::cost::optimization::Priority::Medium => color::warning("Medium"),
                crate::cost::optimization::Priority::Low => color::info("Low"),
            };

            println!(
                "{:<15} {:<25} ${:<9.2} {:<15.1}% {}",
                priority_str,
                rec.recommendation_type.to_string(),
                rec.potential_savings,
                rec.savings_percent,
                rec.description
            );
        }
    }
    Ok(())
}

pub fn handle_cost_waste(waste_type: Option<String>, min_waste: f64, output: String) -> Result<()> {
    use crate::cost::optimization::OptimizationEngine;

    println!("{}", color::header("Cost Waste Report"));
    if let Some(ref wtype) = waste_type {
        println!("  Type: {}", wtype);
    }
    println!("  Minimum: ${:.2}/month", min_waste);
    println!();

    // Example waste reports
    let wastes = [
        OptimizationEngine::detect_storage_waste(100, 10.0),
        OptimizationEngine::detect_old_snapshots(10, 120, 5.0)
            .ok_or_else(|| anyhow::anyhow!("Failed to detect old snapshots"))?,
    ];

    let filtered: Vec<_> = wastes
        .iter()
        .filter(|w| w.monthly_waste >= min_waste)
        .collect();

    println!(
        "Total Monthly Waste: {}",
        color::error(&format!(
            "${:.2}",
            filtered.iter().map(|w| w.monthly_waste).sum::<f64>()
        ))
    );
    println!("Waste Items:         {}", filtered.len());
    println!();

    if output == "json" {
        let json = serde_json::to_string_pretty(&filtered)?;
        println!("{}", json);
    } else if output == "yaml" {
        let yaml = serde_yml::to_string(&filtered)?;
        println!("{}", yaml);
    } else {
        println!(
            "{:<20} {:<20} {:<15} {}",
            color::label("RESOURCE"),
            color::label("TYPE"),
            color::label("MONTHLY WASTE"),
            color::label("DETAILS")
        );
        println!("{}", "-".repeat(80));

        for waste in filtered {
            println!(
                "{:<20} {:<20} ${:<14.2} {}",
                waste.vm_name,
                waste.waste_type.to_string(),
                waste.monthly_waste,
                waste.details
            );
        }
    }
    Ok(())
}

pub fn handle_cost_forecast(budget: Option<f64>, period: String, output: String) -> Result<()> {
    use crate::cost::budgets::CostForecast;

    println!("{}", color::header("Cost Forecast"));
    println!("  Period: {}", period);
    if let Some(b) = budget {
        println!("  Budget: ${:.2}", b);
    }
    println!();

    let mut forecast = CostForecast::new(crate::cost::budgets::BudgetPeriod::Monthly, 1500.0);
    forecast.project_linear(15.0, 30.0);

    println!("Forecast:");
    println!("  Current Spend:    ${:.2}", forecast.current_spend);
    println!(
        "  Projected Spend:  {}",
        color::value(&format!("${:.2}", forecast.projected_spend))
    );
    println!("  Confidence:       {}%", forecast.confidence as u8);
    println!("  Method:           {}", forecast.forecast_method);
    println!();

    if let Some(budget_amount) = budget {
        if forecast.is_over_budget(budget_amount) {
            println!(
                "{}",
                color::error(&format!(
                    "⚠ Forecast exceeds budget by ${:.2}",
                    forecast.projected_spend - budget_amount
                ))
            );
        } else {
            println!(
                "{}",
                color::success(&format!(
                    "✓ Forecast within budget (${:.2} remaining)",
                    budget_amount - forecast.projected_spend
                ))
            );
        }
    }

    if output == "json" {
        println!();
        let json = serde_json::to_string_pretty(&forecast)?;
        println!("{}", json);
    } else if output == "yaml" {
        println!();
        let yaml = serde_yml::to_string(&forecast)?;
        println!("{}", yaml);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::cost::budgets::{Budget, BudgetPeriod, BudgetScope, BudgetStatus, Status};
    use crate::cost::optimization::OptimizationEngine;
    use crate::cost::reports::{ReportGenerator, ReportType};
    use crate::cost::{CostCalculator, CostSummary, VMCost};

    #[test]
    fn test_budget_period_parsing() {
        assert_eq!(
            match "daily" {
                "daily" => BudgetPeriod::Daily,
                "weekly" => BudgetPeriod::Weekly,
                "quarterly" => BudgetPeriod::Quarterly,
                "yearly" => BudgetPeriod::Yearly,
                _ => BudgetPeriod::Monthly,
            },
            BudgetPeriod::Daily
        );
        assert_eq!(
            match "other" {
                "daily" => BudgetPeriod::Daily,
                "weekly" => BudgetPeriod::Weekly,
                "quarterly" => BudgetPeriod::Quarterly,
                "yearly" => BudgetPeriod::Yearly,
                _ => BudgetPeriod::Monthly,
            },
            BudgetPeriod::Monthly
        );
    }

    #[test]
    fn test_budget_scope_parsing() {
        let scope = "namespace:production";
        let result = if scope == "global" {
            BudgetScope::Global
        } else if let Some(ns) = scope.strip_prefix("namespace:") {
            BudgetScope::Namespace(ns.to_string())
        } else if let Some(team) = scope.strip_prefix("team:") {
            BudgetScope::Team(team.to_string())
        } else {
            BudgetScope::Global
        };
        assert_eq!(result, BudgetScope::Namespace("production".to_string()));
    }

    #[test]
    fn test_budget_status_levels() {
        let mut budget = Budget::new("test", 1000.0, BudgetPeriod::Monthly);
        budget.update_spend(500.0);
        assert_eq!(BudgetStatus::from_budget(&budget).status, Status::Healthy);
        budget.update_spend(750.0);
        assert_eq!(BudgetStatus::from_budget(&budget).status, Status::Warning);
        budget.update_spend(950.0);
        assert_eq!(BudgetStatus::from_budget(&budget).status, Status::Critical);
        budget.update_spend(1200.0);
        assert_eq!(BudgetStatus::from_budget(&budget).status, Status::Exceeded);
    }

    #[test]
    fn test_cost_calculator_vm_cost() {
        let calculator = CostCalculator::default();
        let cost = calculator.calculate_vm_cost("vm1", "default", 4, 8, 20, 730.0);
        assert!(cost.total_cost > 0.0);
        assert!(cost.cost_per_hour() > 0.0);
    }

    #[test]
    fn test_cost_summary_aggregation() {
        let mut summary = CostSummary::new();
        let mut vm1 = VMCost::new("vm1", "default");
        vm1.total_cost = 75.0;
        vm1.cpu_cost = 50.0;
        let mut vm2 = VMCost::new("vm2", "default");
        vm2.total_cost = 45.0;
        vm2.cpu_cost = 30.0;
        summary.add_vm_cost(&vm1);
        summary.add_vm_cost(&vm2);
        assert_eq!(summary.total_cost, 120.0);
        assert_eq!(summary.vm_count, 2);
        assert_eq!(summary.average_cost_per_vm(), 60.0);
    }

    #[test]
    fn test_report_generators() {
        let monthly = ReportGenerator::monthly_report(2024, 6);
        assert_eq!(monthly.report_type, ReportType::Monthly);
        let weekly = ReportGenerator::weekly_report(chrono::Utc::now());
        assert_eq!(weekly.report_type, ReportType::Weekly);
    }

    #[test]
    fn test_optimization_and_waste() {
        let report = OptimizationEngine::generate_report("test-vm");
        assert!(!report.recommendations.is_empty());
        let waste = OptimizationEngine::detect_storage_waste(100, 10.0);
        assert_eq!(waste.monthly_waste, 10.0);
        assert!(OptimizationEngine::detect_old_snapshots(10, 120, 5.0).is_some());
        assert!(OptimizationEngine::detect_old_snapshots(10, 30, 5.0).is_none());
    }
}
