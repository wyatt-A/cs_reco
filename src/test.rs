use crate::resource::*;
use crate::volume_index::VolumeIndex;
use std::fs::create_dir_all;
use std::path::Path;
use crate::bart_wrapper::BartPicsSettings;
use crate::volume_manager::{VolumeManager,launch_volume_manager};
use crate::slurm::BatchScript;
use std::process::Command;

pub fn main_test_local(){
    let mut bart_settings = BartPicsSettings::quick();
    bart_settings.set_bart_binary("/Users/wyatt/build/bart-0.7.00/bart");
    let bart_settings_file = "/Users/wyatt/local_recon/reco_settings";
    let scanner = Host::new("mrs","stejskal");
    let workdir = "/Users/wyatt/local_recon";

    let cwd = Path::new(workdir);
    if !cwd.exists(){ create_dir_all(cwd).expect("unable to create specified working directory")}

    let ptab = "/Users/Wyatt/cs_recon/test_data/petableCS_stream/stream_CS256_8x_pa18_pb54";

    let vpath = "/d/smis/N20220816_00/_01_46_3b0/volume_index.txt";

    let mrd_vol_offset = 0;

    bart_settings.to_file(bart_settings_file);

    let r = sync_raw_from_remote_host(workdir,vpath,&scanner);

    r.item.iter().for_each(|item| {
        let mrd = item.local_path();
        let volworkdir = item.local_dir();
        if  !VolumeManager::is_done(&volworkdir){
            launch_volume_manager(
            &volworkdir,
            &mrd,
            ptab,
            mrd_vol_offset,
            bart_settings_file
            );
        }else{
            println!("volume manager is finished with work");
        }
    });

}


pub fn main_test_cluster(){
    let mut bart_settings = BartPicsSettings::quick();
    bart_settings.set_bart_binary("/cm/shared/apps/bart/usr/bin/bart");
    let bart_settings_file = "/home/wa41/cs_recon_test/reco_settings";
    let scanner = Host::new("mrs","stejskal");
    let workdir = "/home/wa41/cs_recon_test/local_recon";
    
    let cwd = Path::new(workdir);
    if !cwd.exists(){ create_dir_all(cwd).expect("unable to create specified working directory")}
    
    let ptab = "/home/wa41/cs_recon_test/stream_CS256_8x_pa18_pb54";
    
    let vpath = "/d/smis/N20220816_00/_01_46_3b0/volume_index.txt";

    let mrd_vol_offset = 0;
    
    bart_settings.to_file(bart_settings_file);

    let r = sync_raw_from_remote_host(workdir,vpath,&scanner);
    
    r.item.iter().for_each(|item| {
        let mrd = item.local_path();
        let volworkdir = item.local_dir();
        let vp = Path::new(&volworkdir);
        if !VolumeManager::is_done(&volworkdir){
            let mut cmd = Command::new("/home/wa41/cs_recon_test/cs_reco");
            cmd.arg("volume-manager");
            cmd.arg(&volworkdir);
            cmd.arg(&mrd);
            cmd.arg(ptab);
            cmd.arg(mrd_vol_offset.to_string());
            cmd.arg(bart_settings_file);
            // let r = cmd.spawn().expect("failed to launch command");
            // let o = r.wait_with_output().expect("failed to wait");
            // println!("{:?}",o.stdout);
            let cmd = format!("{:?}",cmd);
            let mut j = BatchScript::new(&format!("slurm_job"));
            j.options.output = vp.join("slurm-log.out").into_os_string().into_string().unwrap();
            j.commands.push("hostname".to_string());
            j.commands.push(cmd);
            j.submit(&volworkdir);
        }
    }
    );
    
    }