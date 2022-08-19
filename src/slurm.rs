use std::io::{Write, Read};
use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs::File;
use crate::utils;

#[derive(PartialEq,Eq,Debug,Clone)]
pub enum JobState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Unknown,
}

pub struct SBatchOpts{
    reservation:String,
    job_name:String,
    no_requeue:bool,
    pub output:String
}

pub struct BatchScript{
    preamble:String,
    pub options:SBatchOpts,
    //bash_file:String,
    pub commands:Vec<String>,
    pub job_id:Option<u32>
}

impl SBatchOpts{
    pub fn new(job_name:&str) -> SBatchOpts {
        return SBatchOpts{
            job_name:job_name.to_string(),
            reservation:"".to_string(),
            no_requeue: true,
            output:"".to_string()
        };
    }
    pub fn print(&self) -> String {
        let mut opts = Vec::<String>::new();
        opts.push(format!("#SBATCH --job-name={}",&self.job_name));
        if !self.reservation.is_empty(){opts.push(format!("#SBATCH --reservation={}",&self.reservation))}
        if self.no_requeue{ opts.push("#SBATCH --no-requeue".to_string())}
        if !self.output.is_empty(){ opts.push(format!("#SBATCH --output={}",&self.output))}
        return opts.join("\n");
    }
}

impl BatchScript{
    pub fn new(job_name:&str) -> BatchScript {
        let preamble = "#!/usr/bin/env bash".to_string();
        let opts = SBatchOpts::new(job_name);
        let command = Vec::<String>::new();
        return BatchScript {
            preamble:preamble,
            options:opts,
            commands:command,
            job_id:None
        }
    }

    pub fn commands(&self) -> String{
        return self.commands.join("\n");
    }

    pub fn print(&self) -> String {
        let mut elems = Vec::<String>::new();
        elems.push(self.preamble.clone());
        elems.push(self.options.print());
        elems.push(self.commands());
        return elems.join("\n");
    }

    pub fn write(&self,location:&str) -> PathBuf{
        let mut fname = Path::new(location).to_owned();
        fname = fname.join(&self.options.job_name).with_extension("bash");
        //fname = fname.with_file_name(&self.options.job_name).with_extension("bash");
        println!("{:?}",fname);
        let mut f = File::create(&fname).expect("cannot create file");
        f.write_all(self.print().as_bytes()).expect("trouble writing to file");
        return fname;
    }

    pub fn submit(&mut self,write_location:&str) -> u32{
        let path = self.write(write_location);
        let mut cmd = Command::new("sbatch");
        cmd.arg(path);
        let o = cmd.output().expect("failed to run command");
        let response = String::from_utf8_lossy(&o.stdout);
        //println!("err::{}",String::from_utf8_lossy(&o.stderr));
        //println!("out::{}",String::from_utf8_lossy(&o.stdout));
        let jid = BatchScript::response_to_job_id(&response);
        println!("job id: {}",jid);
        self.job_id = Some(jid);
        return jid;
    }

    pub fn get_details(&self){
        match self.job_id {
            Some(jid) => {
                let mut cmd = Command::new("squeue");
                cmd.arg("-j");
                cmd.arg(jid.to_string());
                //cmd.arg("--format=avevmsize");
                let o =cmd.output().expect("process failed");
                if o.status.success(){
                    println!("return text: {}",String::from_utf8_lossy(&o.stdout));
                }
            }
            None => {
                println!("{} job has not been successfully submitted",self.options.job_name);
            }
        }
    }

    pub fn check_state(&self){
        match self.job_id {
            Some(jid) => {
                let mut cmd = Command::new("sacct");
                cmd.arg("-j");
                cmd.arg(jid.to_string());
                cmd.arg("--format=state,reqmem");
                let o = cmd.output().expect("process failed");
                if o.status.success(){
                    let r = String::from_utf8_lossy(&o.stdout);
                    println!("{}",r);
                    let strs:Vec<&str> = r.lines().collect();
                    let fields = strs[0];
                    println!("{}",fields);
                }else{
                    panic!("command unsuccessful");
                }
            }
            None => {
                println!("{} job has not been successfully submitted",self.options.job_name);
            }
        }
    }

    pub fn output(&self) -> String{
        let p = Path::new(&self.options.output);
        let mut f = File::open(p).expect("cannot open file");
        let mut s = String::new();
        f.read_to_string(&mut s).expect("problem reading file");
        return s;
    }

    fn response_to_job_id(resp:&str) -> u32{
        let nums:Vec<u32> = resp.split(" ").flat_map(|str| str.replace("\n","").parse()).collect();
        if nums.len() == 0 {panic!("no job ids found in slurm response")}
        if nums.len() != 1 {panic!("multiple ids found in slurm response")};
        return nums[0];
    }

}

pub fn is_running(job_id:u32){
    let mut cmd = Command::new("squeue");
    cmd.arg("-j");
    cmd.arg(job_id.to_string());
    let r = cmd.spawn().unwrap();
    let o =r.wait_with_output().unwrap();
    println!("{:?}",o.stdout);
}

pub fn get_job_state(job_id:u32,n_tries:u16) -> JobState {
    let mut cmd = Command::new("sacct");
    cmd.arg("-j").arg(job_id.to_string()).arg("--format").arg("state");
    let o = cmd.output().unwrap();
    let s = std::str::from_utf8(&o.stdout).unwrap().to_ascii_lowercase();
    let lines:Vec<&str> = s.lines().collect();
    let mut statestr = lines[lines.len()-1];
    statestr = statestr.trim();
    return match statestr {
        "pending" => JobState::Pending,
        "cancelled" => JobState::Cancelled,
        "failed" => JobState::Failed,
        "running" => JobState::Running,
        "completed" => JobState::Completed,
        _ => {
            if n_tries > 0 {
                std::thread::sleep(std::time::Duration::from_millis(1000));
                return get_job_state(job_id,n_tries-1);
            }else{
                println!("gave up waiting for job state for job id: {}",job_id);
                return JobState::Unknown;
            }
        }
    };
}

#[test]
fn test(){
    let cmd = "scp mrs@stejskal:/d/smis/N20220811_00/_03_MGRE/mgre.mrd /home/wa41".to_string();
    let mut j = BatchScript::new("test_job");
    j.commands.push(cmd);
    j.options.output = "/home/wa41/test_log".to_string();
    println!("{}",j.print());
    j.submit("/home/wa41");
    for _ in 0..20{
        j.check_state();
        std::thread::sleep(std::time::Duration::from_millis(100));
        j.get_details();
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    let o = j.output();
    println!("output:\n{}",o);
}