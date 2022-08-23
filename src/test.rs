use crate::{resource::*, utils};
use crate::volume_index::VolumeIndex;
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::fs::{File,create_dir_all};
use std::io::{Read,Write};
use std::path::{Path, PathBuf};
use whoami;
use crate::bart_wrapper::BartPicsSettings;
use crate::volume_manager::{VmState,VolumeManager,launch_volume_manager,launch_volume_manager_job,re_launch_volume_manager_job};
use crate::slurm::{self,BatchScript, JobState};
use std::process::Command;

/*
    headfile=mrs_meta_data(mrd);
    headfile.dti_vols = n_volumes;
    headfile.U_code = project_code;
    headfile.U_civmid = civm_userid;
    headfile.U_specid = specimen_id;
    headfile.scanner_vendor = scanner_vendor;
    headfile.U_runno = strcat(run_number,'_',mnum);
    headfile.dim_X = vol_size(1);
    headfile.dim_Y = vol_size(2);
    headfile.dim_Z = vol_size(3);
    headfile.civm_image_code = 't9';
    headfile.civm_image_source_tag = 'imx';
    headfile.engine_work_directory = pwd;
*/

#[derive(Serialize,Deserialize)]
pub struct Recon{
    run_number:String,
    volume_data:PathBuf,
    engine_work_dir:PathBuf,
    recon_person:String,
    scanner:Scanner,
    project:ProjectSettings,
    specimen_id:String
}

#[derive(Serialize,Deserialize)]
pub struct Scanner{
    label:String,
    identity:Host,
    vendor:String,
    vol_meta_suffix:String,
    image_code:String,
    image_source_tag:String,
}

#[derive(Serialize,Deserialize)]
pub struct ProjectSettings{
    label:String,
    project_code:String,
    recon_settings:BartPicsSettings,
}

// Recon::new("grumpy","test_runno","/some/vol_index.txt","5xfad")

impl Recon{
    pub fn new(scanner:&str,runno:&str,vol_data:&str,project:&str,specimen_id:&str) -> Recon{

        let engine_work_dir = match std::env::var("BIGGUS_DISKUS"){
            Ok(dir) => Path::new(&dir).to_owned(),
            Err(_) => {
                println!("BIGGUS_DISKUS not set. Using home directory instead");
                Path::new(&std::env::var("HOME").expect("HOME not set. Are you on a Windows?")).to_owned()
            },
        };
        return Recon{
            run_number:runno.to_string(),
            volume_data:Path::new(vol_data).to_owned(),
            engine_work_dir:engine_work_dir,
            recon_person:whoami::username(),
            scanner:Scanner::open(scanner),
            project:ProjectSettings::open(project),
            specimen_id:specimen_id.to_string()
        }
    }
}

impl ProjectSettings{
    pub fn open(label:&str) -> ProjectSettings{
        let str = utils::read_to_string(label,"toml");
        return toml::from_str(&str).expect("Cannot deserialize file. Is it the correct format?");
    }
    pub fn new_template(label:&str){
        let project_settings = ProjectSettings{
            label:label.to_string(),
            project_code:"22.project.01".to_string(),
            recon_settings:BartPicsSettings::default(),
        };
        utils::write_to_file(label,"toml",&toml::to_string(&project_settings).expect("cannot serialize struct"));
    }
}

impl Scanner {
    pub fn open(label:&str) -> Scanner {
        let str = utils::read_to_string(label,"toml");
        return toml::from_str(&str).expect("Cannot deserialize file. Is it the correct format?");
    }
    pub fn new_template(label:&str){
        let scanner = Scanner{
            label:label.to_string(),
            identity:Host::new("user","hostname"),
            vendor:"mrsolutions".to_string(),
            vol_meta_suffix:"_meta.txt".to_string(),
            image_code:"t9".to_string(),
            image_source_tag:"imx".to_string(),
        };
        utils::write_to_file(label,"toml",&toml::to_string(&scanner).expect("cannot serialize struct"));
    }
}


pub fn main_test_cluster(){

    let mut bart_settings = BartPicsSettings::quick();
    bart_settings.set_bart_binary("bart");
    let bart_settings_file = "/privateShares/wa41/cs_recon_test/reco_settings";
    let scanner = Host::new("mrs","stejskal");
    let runno = "testrunno";
    let big_disk = "/privateShares/wa41/cs_recon_test";
    let ptab = "/home/wa41/cs_recon_test/stream_CS256_8x_pa18_pb54";
    let vpath = "/d/smis/recon_test_data/_01_46_3b0/volume_index.txt";
    let mrd_meta_suffix = "_meta.txt";

    // let recon_meta = ReconMeta{
    // }
    
    let base_dir = Path::new(big_disk);
    let cwd = base_dir.join(format!("{}.work",runno));
    if !cwd.exists(){ create_dir_all(&cwd).expect("unable to create specified working directory")}
    
    let volman_jobs_file = cwd.join("volume-manager-jobs").with_extension("toml");

    let mrd_vol_offset = 0;
    
    bart_settings.to_file(bart_settings_file);

    let raw_base_path = Path::new(vpath).parent().unwrap();
    let local_raw_path = Path::new(&cwd).join("raw");
    if !local_raw_path.exists(){create_dir_all(&local_raw_path).expect("issue creating directory");}
    
    let local_vpath = VolumeIndex::fetch_from(vpath,&scanner,cwd.to_str().unwrap());
    let ready_mrds = VolumeIndex::read_ready(&local_vpath);
    let all_mrds = VolumeIndex::read_all(&local_vpath);

    let mut r = ResourceList::open(local_raw_path.to_str().unwrap());
    r.set_host(&scanner);
    ready_mrds.iter().for_each(|(mrd,_)| {
        let mrdname = Path::new(mrd).file_stem().unwrap().to_str().unwrap();
        let mrd_srcpath = Path::new(raw_base_path).join(mrd);
        let meta_srcpath = Path::new(raw_base_path).join(format!("{}{}",mrdname,mrd_meta_suffix));
        r.try_add(Resource::new(mrd_srcpath.to_str().unwrap(),""));
        r.try_add(Resource::new(meta_srcpath.to_str().unwrap(),""));
    });
    r.start_transfer();

    /*
        This builds a hashmap of volume managers and their slurm
        job ids that will updated and saved every time this runs
    */
    let mut vol_man_jobs:HashMap<PathBuf,u32>;
    println!("looking for {:?} ...",volman_jobs_file);
    if volman_jobs_file.exists(){
        println!("loading jobs ...");
        let s = utils::read_to_string(volman_jobs_file.to_str().unwrap(),"toml");
        vol_man_jobs = toml::from_str(&s).expect("cannot deserialize hash");
    }
    else{
        println!("creating new jobs file ...");
        vol_man_jobs = HashMap::<PathBuf,u32>::new();
    }
    println!("{:?}",vol_man_jobs);

    /*
        We are assuming a one-to-one mapping a mrd file to a volume manager
        in "volume_index" mode. If a mrd file is available, and a volume manager
        hasn't already been launched, a new volume manager will be instantiated
    */
    all_mrds.iter().for_each(|(index,mrd)| {
        let voldir = cwd.join(index);
        if !voldir.exists(){create_dir_all(&voldir).expect("issue creating directory");}
        if !VolumeManager::exists(voldir.to_str().unwrap()) && mrd.is_some(){
            println!("vol man doesn't exist and mrd is available... submitting new job");
            let mrd_path = local_raw_path.join(mrd.clone().unwrap());
            let job_id = launch_volume_manager_job(voldir.to_str().unwrap(),runno,mrd_path.to_str().unwrap(),&ptab,mrd_vol_offset,&bart_settings_file);
            vol_man_jobs.insert(voldir.clone(),job_id);
        }
    });

    /*
        For every volume manager that has been launched, we find the job state
        from slurm. Note that this is not the volume managers state. This just tells
        us the state of the slurm job (pending,running,completed,failed ... ect)
    */
    let mut job_states = HashMap::<PathBuf,slurm::JobState>::new();
    vol_man_jobs.iter().for_each(|(vol,job)|{
        let jstate = slurm::get_job_state(*job,60);
        job_states.insert(vol.clone(),jstate.clone());
    });

    /*
        If for some reason a volume manager cannot advance state (commonly becasue it is waiting for
        image scaling information from volume 00), it will return and the slurm state will say "completed."
        In this case, we need to check for inactivity of volume managers that still have work to do. If this is
        the case, we need to restart them, returning a new slurm job id to track
    */
    job_states.iter().for_each(|(vol,state)|{
        if *state == JobState::Completed && !VolumeManager::is_done(vol.to_str().unwrap()){
            //println!("restarting {:?}",vol);
            let workdir = vol.to_str().unwrap();
            let job_id = re_launch_volume_manager_job(workdir);
            vol_man_jobs.insert(vol.clone(),job_id);
        }
    });

    /*
        Here we need to build up some info about the overall progress of the system for reporting and as a
        stop condition for rescheduling.
    */

    //let mc = all_mrds.clone();
    let mut m:Vec<&String> = all_mrds.keys().collect();
    m.sort();
    //println!("sorted idx: {:?}",m);

    let mut state_str = String::new();
    let mut n_completed:usize = 0;
    let states:Vec<VmState> = m.iter().map(|index| {
        let voldir = cwd.join(index);
        let s = VolumeManager::state(voldir.to_str().unwrap());
        if s == VmState::Done {n_completed += 1};
        let slurm_state = job_states.get(&voldir);
        match slurm_state {
            Some(state) => state_str.push_str(&format!("{} : slurm job : {:?}; volume-manager : {:?}\t\n",index,state,&s)),
            None => state_str.push_str(&format!("{} : slurm job : not submitted; volume-manager : {:?}\t\n",index,&s))
        }
        return s;
    }).collect();

    println!("{}",state_str);
    println!("{} completed out of {}.",n_completed,m.len());
    /*
        Here we save information we want to load up the next time this code runs. Right now, this only has
        to be the slurm job ids of the volume managers
    */
    let vol_man_jobs_str = toml::to_string(&vol_man_jobs).expect("cannot serialize hash");
    utils::write_to_file(volman_jobs_file.to_str().unwrap(),"toml",&vol_man_jobs_str);
    
    // if all work isn't done, schedule this code to run again later (2 minutes seems good?)
    // if n_vols != n_complete{
    //     /* reschedule for later */
    // }
    
}
    