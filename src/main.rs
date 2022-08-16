use cs_reco::volume_manager::launch_volume_manager;
use std::env;
fn main(){
    let args: Vec<String> = env::args().collect();
    let workdir = &args[1];
    let mrd = &args[2];
    let phase_table = &args[3];
    let vol_offset:usize = args[4].parse().unwrap();
    let bart_settings_file = &args[5];
    launch_volume_manager(workdir,mrd,phase_table,vol_offset,bart_settings_file);
}