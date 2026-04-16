mod commands;

fn main() {
    println!("worldcompute-gui: Tauri scaffold ready");
    println!("Available commands:");
    println!("  get_donor_status    -> {:}", commands::get_donor_status());
    println!("  get_job_status      -> {:}", commands::get_job_status());
    println!("  get_cluster_status  -> {:}", commands::get_cluster_status());
    println!("  get_mesh_status     -> {:}", commands::get_mesh_status());
    println!("  submit_job          -> {:}", commands::submit_job());
    println!("  pause_agent         -> {:}", commands::pause_agent());
    println!("  resume_agent        -> {:}", commands::resume_agent());
}
