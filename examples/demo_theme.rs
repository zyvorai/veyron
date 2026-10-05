// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

// Demo program to showcase Veyron CLI theme colors
// Run with: cargo run --example demo_theme

use veyron::tui::colors::cli;
use veyron::tui::colors::{resource_bar, vm_status_symbol};

fn main() {
    println!("\n{}", cli::header("🎨 Veyron CLI Theme Demonstration"));
    println!("{}\n", cli::muted(&"=".repeat(60)));

    // Section 1: Headers and Text
    println!("{}", cli::header("1. Text Styles"));
    println!("   Header:    {}", cli::header("This is a header"));
    println!("   Label:     {}", cli::label("This is a label"));
    println!("   Value:     {}", cli::value("This is a value"));
    println!("   Muted:     {}", cli::muted("This is muted text"));
    println!("   Command:   {}", cli::command("veyron list"));
    println!("   Path:      {}", cli::path("/path/to/file.yaml"));
    println!();

    // Section 2: Messages
    println!("{}", cli::header("2. Status Messages"));
    println!("   {}", cli::success("Operation completed successfully"));
    println!("   {}", cli::error("Operation failed with error"));
    println!("   {}", cli::warning("Warning: resource usage high"));
    println!("   {}", cli::info("Connecting to Kubernetes cluster..."));
    println!();

    // Section 3: VM Status
    println!("{}", cli::header("3. VM Status Indicators"));
    let statuses = vec![
        "running",
        "pending",
        "stopped",
        "failed",
        "migrating",
        "paused",
    ];
    for status in statuses {
        println!(
            "   {} {:<12} {}",
            vm_status_symbol(status),
            status.to_uppercase(),
            cli::vm_status(status)
        );
    }
    println!();

    // Section 4: Namespaces
    println!("{}", cli::header("4. Namespace Colors"));
    println!("   Default namespace:  {}", cli::namespace("default"));
    println!("   System namespace:   {}", cli::namespace("kube-system"));
    println!("   User namespace:     {}", cli::namespace("production"));
    println!();

    // Section 5: Resources
    println!("{}", cli::header("5. Resource Colors"));
    println!("   CPU:     {}", cli::resource("4 cores", "cpu"));
    println!("   Memory:  {}", cli::resource("16Gi", "memory"));
    println!("   Disk:    {}", cli::resource("100Gi", "disk"));
    println!("   Network: {}", cli::resource("eth0", "network"));
    println!();

    // Section 6: Resource Bars
    println!("{}", cli::header("6. Resource Usage Bars"));
    let usages = vec![
        (50.0, "Healthy - below 70%"),
        (75.0, "Warning - 70-90%"),
        (95.0, "Critical - above 90%"),
    ];
    for (usage, desc) in usages {
        let bar = resource_bar(usage, 30);
        println!("   [{}] {:>5.1}%  {}", bar, usage, cli::muted(desc));
    }
    println!();

    // Section 7: Sample VM List
    println!("{}", cli::header("7. Sample VM List (Themed Table)"));
    println!();
    println!(
        "   {:<25} {:<20} {:<20}",
        cli::header("NAME"),
        cli::header("NAMESPACE"),
        cli::header("STATUS")
    );
    println!("   {}", cli::muted(&"-".repeat(65)));

    let vms = vec![
        ("web-server-1", "default", "running"),
        ("db-primary", "production", "running"),
        ("cache-redis", "production", "pending"),
        ("test-vm", "default", "stopped"),
        ("backup-server", "kube-system", "failed"),
        ("migrate-vm", "production", "migrating"),
    ];

    for (name, namespace, status) in vms {
        println!(
            "   {:<25} {:<30} {} {}",
            cli::vm_name(name),
            cli::namespace(namespace),
            vm_status_symbol(status),
            cli::vm_status(status)
        );
    }
    println!();

    // Section 8: Wizard-style output
    println!("{}", cli::header("8. Wizard-Style Configuration Display"));
    println!();
    println!(
        "   {}",
        cli::info("Creating VM with the following configuration:")
    );
    println!(
        "     {:<12} {}",
        cli::label("Name:"),
        cli::value("production-db")
    );
    println!(
        "     {:<12} {}",
        cli::label("Template:"),
        cli::value("ubuntu")
    );
    println!(
        "     {:<12} {}",
        cli::label("CPU:"),
        cli::resource("8 cores", "cpu")
    );
    println!(
        "     {:<12} {}",
        cli::label("Memory:"),
        cli::resource("32Gi", "memory")
    );
    println!(
        "     {:<12} {}",
        cli::label("Disk:"),
        cli::resource("500Gi", "disk")
    );
    println!();
    println!("   {}", cli::success("VM created successfully"));
    println!();

    // Footer
    println!("{}", cli::muted(&"=".repeat(60)));
    println!(
        "{}",
        cli::info("Theme colors based on GuestKit design patterns")
    );
    println!(
        "{}\n",
        cli::muted("Run 'cargo run --example demo_theme' to see this demo")
    );
}
