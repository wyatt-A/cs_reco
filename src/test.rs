use crate::{resource::*, utils};
use crate::volume_index::VolumeIndex;
use std::collections::{HashMap, HashSet};
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use crate::bart_wrapper::BartPicsSettings;
use crate::volume_manager::{VolumeManager,launch_volume_manager,launch_volume_manager_job};
use crate::slurm::{self,BatchScript, JobState};
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

    let (vidx,vols) = sync_raw_from_remote_host(workdir,vpath,&scanner);

    vols.item.iter().for_each(|item| {
        let mrd = item.local_path();
        let volworkdir = item.local_dir();
        let p = Path::new(&volworkdir);
        println!("{:?}",p.file_name());
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
    let bart_settings_file = "/privateShares/wa41/cs_recon_test/reco_settings";
    let scanner = Host::new("mrs","stejskal");
    let workdir = "/privateShares/wa41/cs_recon_test/local_recon";
    
    let cwd = Path::new(workdir);
    if !cwd.exists(){ create_dir_all(cwd).expect("unable to create specified working directory")}
    
    let volman_jobs_file = cwd.join("volman_jobs");

    let ptab = "/home/wa41/cs_recon_test/stream_CS256_8x_pa18_pb54";
    
    let vpath = "/d/smis/recon_test_data/_01_46_3b0/volume_index.txt";

    let mrd_vol_offset = 0;
    
    bart_settings.to_file(bart_settings_file);

    let raw_base_path = Path::new(vpath).parent().unwrap();
    let local_raw_path = Path::new(workdir).join("raw");
    if !local_raw_path.exists(){ create_dir_all(&local_raw_path).expect("issue creating directory"); }
    
    let local_vpath = VolumeIndex::fetch_from(vpath,&scanner,workdir);
    let ready_mrds = VolumeIndex::read_ready(&local_vpath);
    let all_mrds = VolumeIndex::read_all(&local_vpath);

    let mut r = ResourceList::open(local_raw_path.to_str().unwrap());
    r.set_host(&scanner);
    ready_mrds.iter().for_each(|(mrd,_)| {
        let srcpath = Path::new(raw_base_path).join(mrd);
        r.try_add(Resource::new(srcpath.to_str().unwrap(),""));
    });
    r.start_transfer();

    //let (vidx,vols) = sync_raw_from_remote_host(workdir,vpath,&scanner);

    let mut vol_man_jobs = match volman_jobs_file.exists(){
        true => {
            let s = utils::read_to_string(volman_jobs_file.to_str().unwrap(),"toml");
            toml::from_str(&s).expect("cannot deserialize hash")
        },
        false => HashMap::<PathBuf,Option<u32>>::new()
    };

    let mut job_states = HashMap::<PathBuf,slurm::JobState>::new();

    // start volume managers that havn't already started
    all_mrds.iter().for_each(|(index,mrd)| {
        let voldir = local_raw_path.join(index);
        if !voldir.exists(){create_dir_all(&voldir).expect("issue creating directory");}
        if !VolumeManager::exists(voldir.to_str().unwrap()) && mrd.is_some(){
            let job_id = launch_volume_manager_job(voldir.to_str().unwrap(),&mrd.clone().unwrap(),&ptab,mrd_vol_offset,&bart_settings_file);
            vol_man_jobs.insert(voldir.clone(),Some(job_id));
        }else {
            vol_man_jobs.insert(voldir.clone(),None);
        }
    });

    // check job states
    vol_man_jobs.iter().for_each(|(vol,job)|{
        if job.is_some(){
            let jstate = slurm::get_job_state(job.unwrap());
            job_states.insert(vol.clone(),jstate);
        }
    });

    // restart volume managers that have not finished but are not actively running
    job_states.iter().for_each(|(vol,state)|{
        if *state == JobState::Completed && !VolumeManager::is_done(vol.to_str().unwrap()){
            // get mrd for this vol manager
            let mrd = all_mrds.get(vol.clone().to_str().unwrap()).unwrap().clone().unwrap();
            // relaunch the vol manager and get a new job id
            let job_id = launch_volume_manager_job(vol.to_str().unwrap(),&mrd,&ptab,mrd_vol_offset,&bart_settings_file);
            // update list of job ids
            vol_man_jobs.insert(vol.clone(),Some(job_id));
        }
    });

    let vol_man_jobs_str = toml::to_string(&vol_man_jobs).expect("cannot serialize hash");
    
    utils::write_to_file(volman_jobs_file.to_str().unwrap(),"toml",&vol_man_jobs_str);
    }