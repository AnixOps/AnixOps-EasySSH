#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! easyssh-core = { path = "../../core" }
//! tokio = { version = "1", features = ["full"] }
//! ```

use easyssh_core::workflow_engine::*;
use easyssh_core::workflow_variables::*;
use easyssh_core::macro_recorder::*;
use easyssh_core::script_library::*;
use easyssh_core::workflow_scheduler::*;

#[tokio::main]
async fn main() {
    println!("=== EasySSH Workflow System Demo ===\n");

    // 1. Create a workflow
    println!("1. Creating a deployment workflow...");
    let mut workflow = Workflow::new("Server Deployment")
        .with_description("Deploy application to production servers")
        .with_category("deployment");

    // Add steps
    let check_step = WorkflowStep::new(StepType::Command, "Health Check")
        .with_config(StepConfig::Command {
            command: "df -h / && free -h".to_string(),
            target: "remote".to_string(),
            working_dir: None,
            env_vars: std::collections::HashMap::new(),
            capture_output: true,
            fail_on_error: true,
        })
        .with_position(100.0, 100.0);
    let check_id = workflow.add_step(check_step);

    let upload_step = WorkflowStep::new(StepType::Transfer, "Upload Package")
        .with_config(StepConfig::Transfer {
            direction: "upload".to_string(),
            local_path: "{{deployment.package_path}}".to_string(),
            remote_path: "/tmp/deploy.tar.gz".to_string(),
            permissions: Some("644".to_string()),
            create_dirs: true,
        })
        .with_position(300.0, 100.0);
    let upload_id = workflow.add_step(upload_step);

    let install_step = WorkflowStep::new(StepType::Command, "Install")
        .with_config(StepConfig::Command {
            command: "tar -xzf /tmp/deploy.tar.gz && ./install.sh".to_string(),
            target: "remote".to_string(),
            working_dir: Some("/opt/app".to_string()),
            env_vars: std::collections::HashMap::new(),
            capture_output: true,
            fail_on_error: true,
        })
        .with_position(500.0, 100.0);
    let install_id = workflow.add_step(install_step);

    // Connect steps
    workflow.connect(&check_id, &upload_id).unwrap();
    workflow.connect(&upload_id, &install_id).unwrap();

    println!("   Created workflow with {} steps", workflow.steps.len());
    println!("   Workflow ID: {}", workflow.id);

    // 2. Test variable resolution
    println!("\n2. Testing variable resolution...");
    let server = ServerContext {
        id: "srv-001".to_string(),
        name: "Production Server 1".to_string(),
        host: "192.168.1.100".to_string(),
        port: 22,
        username: "admin".to_string(),
        password: Some("secret123".to_string()),
        key_path: None,
        group: Some("production".to_string()),
        tags: vec!["web".to_string(), "critical".to_string()],
    };

    let resolver = VariableResolver::new()
        .with_server(server.clone())
        .with_system_variables();

    let template = "Connecting to {{server.host}}:{{server.port}} as {{server.username}} at {{system.timestamp}}";
    let resolved = resolver.resolve(template);
    println!("   Template: {}", template);
    println!("   Resolved: {}", resolved);

    // 3. Create a macro
    println!("\n3. Creating a macro...");
    let mut macro_recorder = MacroRecorder::new();
    macro_recorder.start_recording("Server Setup", Some(MacroServerContext {
        server_id: server.id.clone(),
        server_name: server.name.clone(),
        host: server.host.clone(),
        username: server.username.clone(),
        initial_dir: "/home/admin".to_string(),
        env_vars: std::collections::HashMap::new(),
    }));

    macro_recorder.record_ssh_command("sudo apt update", None);
    macro_recorder.record_ssh_command("sudo apt upgrade -y", None);
    macro_recorder.record_wait(5);
    macro_recorder.record_file_upload("./config/app.conf", "/etc/myapp/config.conf");

    let recorded_macro = macro_recorder.stop_recording();
    if let Some(m) = recorded_macro {
        println!("   Recorded macro with {} actions", m.actions.len());
        println!("   Macro name: {}", m.name);

        // Convert to workflow
        let converted_workflow = m.to_workflow();
        println!("   Converted to workflow with {} steps", converted_workflow.steps.len());
    }

    // 4. Create scheduled task
    println!("\n4. Creating scheduled task...");
    let task = ScheduledTask::new(
        "Daily Backup",
        &workflow.id,
        "0 2 * * *", // Daily at 2 AM
    );

    match task {
        Ok(t) => {
            println!("   Task: {}", t.name);
            println!("   Schedule: {} ({})", t.cron_expression, t.schedule_description);
            println!("   Next run: {:?}", t.next_run);
        }
        Err(e) => println!("   Error: {}", e),
    }

    // 5. Script library
    println!("\n5. Script library operations...");
    let temp_dir = std::env::temp_dir().join("easyssh_scripts_test");
    let mut library = ScriptLibrary::new(temp_dir);

    // Add workflow to library
    let workflow_id = library.add_workflow(workflow.clone(), None);
    println!("   Added workflow to library: {}", workflow_id);

    // Add macro to library
    let macro_data = Macro::new("Quick Deploy");
    let macro_id = library.add_macro(macro_data, None);
    println!("   Added macro to library: {}", macro_id);

    // Search
    let results = library.search(ScriptSearchOptions {
        query: Some("deploy".to_string()),
        ..Default::default()
    });
    println!("   Search 'deploy' found {} results", results.len());

    // 6. Workflow templates
    println!("\n6. Built-in workflow templates:");
    let deployment = WorkflowTemplates::deployment_workflow();
    println!("   - Deployment: {} steps", deployment.steps.len());

    let backup = WorkflowTemplates::backup_workflow();
    println!("   - Backup: {} steps", backup.steps.len());

    let update = WorkflowTemplates::system_update_workflow();
    println!("   - System Update: {} steps", update.steps.len());

    // 7. Cron presets
    println!("\n7. Cron schedule presets:");
    println!("   - Every minute: {}", CronPresets::every_minute());
    println!("   - Hourly: {}", CronPresets::hourly());
    println!("   - Daily: {}", CronPresets::daily());
    println!("   - Weekly: {}", CronPresets::weekly());
    println!("   - Monthly: {}", CronPresets::monthly());

    // 8. Validate workflow
    println!("\n8. Validating workflow...");
    match workflow.validate() {
        Ok(_) => println!("   Workflow is valid!"),
        Err(errors) => {
            println!("   Validation errors:");
            for error in errors {
                println!("     - {}", error);
            }
        }
    }

    println!("\n=== Demo Complete ===");
    println!("\nWorkflow System Features:");
    println!("- Visual workflow editor with drag-and-drop");
    println!("- 4 core step types: Command, Transfer, Condition, Wait");
    println!("- Variable system with templates ({{server.ip}}, {{server.username}})");
    println!("- Macro recorder for automatic script generation");
    println!("- Script library with search and categories");
    println!("- Cron-based scheduler with presets");
    println!("- Batch execution on multiple servers");
    println!("- Result aggregation and reporting");
}
