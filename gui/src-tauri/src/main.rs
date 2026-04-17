//! World Compute Tauri GUI — desktop application entry point.
//!
//! Registers Tauri invoke commands that bridge the React frontend to the
//! worldcompute library. The GUI feature gate prevents this from affecting
//! the library build when the Tauri toolchain is not available.

mod commands;

/// Entry point for the Tauri desktop application.
///
/// When built with the `gui` feature and the Tauri frontend toolchain,
/// this launches the native window and registers all IPC commands.
/// Without the feature flag, it prints a diagnostic message.
#[cfg(feature = "gui")]
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::get_donor_status,
            commands::submit_job,
            commands::get_job_status,
            commands::get_cluster_status,
            commands::get_proposals,
            commands::cast_vote,
            commands::get_mesh_status,
            commands::pause_agent,
            commands::resume_agent,
            commands::get_settings,
            commands::update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error running worldcompute-gui");
}

#[cfg(not(feature = "gui"))]
fn main() {
    println!("worldcompute-gui: Tauri GUI scaffold");
    println!("Build with --features gui and the Tauri frontend toolchain to launch.");
    println!();
    println!("Available IPC commands:");
    println!("  get_donor_status   — donor credit balance, trust score, state");
    println!("  submit_job         — submit a job manifest, returns job_id");
    println!("  get_job_status     — query job progress and state");
    println!("  get_cluster_status — node count, coordinator info");
    println!("  get_proposals      — governance proposal list");
    println!("  cast_vote          — vote on a governance proposal");
    println!("  get_mesh_status    — mesh LLM session info");
    println!("  pause_agent        — pause the donor agent");
    println!("  resume_agent       — resume the donor agent");
    println!("  get_settings       — current workload/resource settings");
    println!("  update_settings    — update workload class or resource caps");
    println!();

    // Demonstrate that library calls compile correctly
    let status = commands::get_donor_status();
    println!("Sample get_donor_status() -> {status}");
    let cluster = commands::get_cluster_status();
    println!("Sample get_cluster_status() -> {cluster}");
}
