use std::io::Write;
use std::process::Command;
use std::{path::Path, io::Read};
use std::fs::{File,self};
use serde::{Deserialize, Serialize};

#[derive(Serialize,Deserialize,Debug,Hash,Eq,Clone)]
pub struct Resource{
    src:String,
    dest:String,
    state:RState,
    host:Option<Host>
}

impl std::cmp::PartialEq<Resource> for Resource {
    fn eq(&self, other: &Resource) -> bool {
        other.src == self.src &&
        other.dest == self.dest &&
        other.host == self.host
        // omitting state
    }
}

#[derive(Serialize,Deserialize,Debug)]
pub struct ResourceList{
    workdir:String,
    item:Vec<Resource>,
    host:Option<Host>
}

#[derive(Serialize,Deserialize,Clone,Debug,Hash,PartialEq,Eq)]
pub struct Host {
    pub name:String,
    pub user:String
}

#[derive(Serialize,Deserialize,Debug,Hash,PartialEq,Eq,Clone)]
enum RState {
    Remote,
    Local,
    Succeeded,
}

impl Host{
    pub fn new(user:&str,name:&str) -> Host{
        return Host{user:user.to_string(),name:name.to_string()}
    }
}

 
impl ResourceList{

    pub fn open(work_dir:&str) -> ResourceList {
        let cwd = Path::new(work_dir).to_owned();
        let fname = cwd.join("resource_list").with_extension("toml");
        let mut rl:ResourceList;
        if fname.exists() {
            let mut f = File::open(fname).expect("cannot open file");
            let mut buf = String::new();
            f.read_to_string(&mut buf).expect("cannot read from file");
            rl = toml::from_str(&buf).expect("cannot deserialize.. file may be corrupt");

        }else{
            rl = ResourceList::new(work_dir);
            rl.workdir = work_dir.to_string();
        }
        rl.update_file();
        return rl;
    }

    fn update_file(&mut self){
        let cwd = Path::new(&self.workdir).to_owned();
        let fname = cwd.join("resource_list").with_extension("toml");
        let mut f = File::create(&fname).expect("cannot create file");
        let s = toml::to_string(&self).expect("cannot serialize data structure");
        f.write_all(s.as_bytes()).expect("trouble writing to file");
    }

    pub fn add_resource(&mut self,res:Resource){
        // add resource if it doesn't already exist in the collection
        // note** we're not using a hash here because we cant serialize it
        let mut res = res;
        res.set_host(self.host.clone());
        let p = Path::new(&self.workdir).join(res.dest);
        res.dest = p.into_os_string().into_string().unwrap();
        let identical:Vec<&Resource> = self.item.iter().filter(|it| **it == res).collect();
        let exists = identical.len() != 0;
        if !exists{
            self.item.push(res);
            self.update_file();
        }
    }

    fn new(work_dir:&str) -> ResourceList {
        return ResourceList { item:Vec::<Resource>::new(),host:None,workdir:work_dir.to_string() };
    }

    pub fn set_host(&mut self,host:&Host){
        self.host = Some(host.clone());
        self.item.iter_mut().for_each(|item| {
            item.set_host(self.host.clone());
        });
        self.update_file();
    }

    pub fn start_transfer(&mut self){
        // couldn't figure out how to make this more "rusty". Long story short ... damn borrow checker!
        // a for-loop will have to do for now
        let n = self.item.len();
        for i in 0..n{
            if self.item[i].state != RState::Succeeded {
                self.item[i].fetch();
                self.update_file();
            }
        }
    }
}


impl Resource{
    pub fn new(source:&str,destination:&str) -> Resource{
        return Resource {
            src: source.to_string(),
            dest: destination.to_string(),
            state:RState::Local,
            host:None,
         };
    }

    pub fn local_path(&self) -> String{
        let sp = Path::new(&self.src);
        let dp = Path::new(&self.dest);
        let fname = sp.file_name().unwrap();
        return dp.join(fname).into_os_string().into_string().unwrap();
    }

    pub fn set_remote_host(&mut self,host:&Host){
        let h = Some(host.clone());
        self.set_host(h)
    }

    fn set_host(&mut self,host:Option<Host>){
        if self.state != RState::Succeeded{
            match host {
                Some(_) => self.state = RState::Remote,
                None => self.state = RState::Local,
            }
        }
        self.host = host;
    }

    pub fn fetch(&mut self){
        match self.state{
            RState::Remote => {
                let local = false;
                self.start_transfer(local);
            },
            RState::Local => {
                let local = true;
                self.start_transfer(local);
            }
            RState::Succeeded => {
                println!("fetch already succeeded. use update to update file from other another location");
            },
            
        }
    }

    pub fn update(&mut self,local:bool){
        self.start_transfer(local);
    }

    fn start_transfer(&mut self,local:bool){
        let mut cmd:Command;
        let mut src:String;
        match local {
            true => {
                cmd = Command::new("cp");cmd.arg("-p");
                src = self.src.clone();
            }
            false => {
                cmd = Command::new("scp");cmd.arg("-Bp");
                match &self.host {
                    None => panic!("remote host and user must be set"),
                    Some(host) => {
                        src = format!("{}@{}:",host.user,host.name);
                        src.push_str(&self.src);
                    }
                }
            }
        }
        cmd.arg(src);
        cmd.arg(&self.dest);

        // check that dest exists.
        let p = Path::new(&self.dest);
        if !p.exists() {fs::create_dir_all(p).expect("failed to create directory");}
        println!("{:?}",cmd);
        let r = cmd.spawn().expect("failed to launch cp command");
        let o = r.wait_with_output().expect("failed to wait for execution");
        if o.status.success() {self.state = RState::Succeeded}
    }
}



#[test]
fn test(){
    use crate::volume_index::VolumeIndex;
    let scanner = Host::new("mrs","stejskal");
    let workdir = "/home/wa41/transfer";
    let vpath = "/d/smis/N20220811_00/_02_ICO61_6b0/volume_index.txt";
    let vol_prefix = "m";
    
    let mut vol_index = Resource::new(vpath,workdir);
    let vol_index_local = false;
    vol_index.set_remote_host(&scanner);
    vol_index.update(vol_index_local);   
    let vhash = VolumeIndex::read(&vol_index.local_path());
    let mut r = ResourceList::open(workdir);
    r.set_host(&scanner);
    for (key, value) in vhash.into_iter() {
        let src_path = Path::new(vpath).with_file_name(&key).into_os_string().into_string().unwrap();
        let mut dest = vol_prefix.to_string();
        dest.push_str(&value);
        // if a resource has already been added, it will not be added again despite the method call
        r.add_resource(Resource::new(&src_path,&dest));
    }
    r.start_transfer();
}