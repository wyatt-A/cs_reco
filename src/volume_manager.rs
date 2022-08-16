use crate::mrd::Mrd;
use crate::pe_table::Petable;
use crate::bart_wrapper::{BartPicsSettings,bart_pics};
use serde::{Deserialize, Serialize};
use std::path::{Path,PathBuf};
use std::fs::{File,create_dir_all,remove_file};
use std::io::{Write,Read};

#[derive(Deserialize, Serialize)]
struct VolumeManager {
    file:String,
    mrd:String,
    phase_table:String,
    mrd_vol_offset:usize,
    reco_settings:String,
    state:VmState,
    kspace:Option<String>,
    imspace:Option<String>,
}

#[derive(Deserialize, Serialize,Clone,PartialEq,Eq)]
enum VmState {
    Idle,
    PreProcessing,
    Reconstructing,
    Done,
}


impl VolumeManager {

    pub fn launch(workdir:&str,mrd:&str,phase_table:&str,vol_offset:usize,reco_settings:&str) -> VolumeManager{
        if VolumeManager::exists(workdir){
            return VolumeManager::advance(workdir);
        }
        let vm_path = Path::new(workdir).join("vol_manager").with_extension("toml");
        let vm_path_str = vm_path.into_os_string().into_string().unwrap();
        let vm = VolumeManager{
            file:vm_path_str,
            mrd:mrd.to_string(),
            phase_table:phase_table.to_string(),
            mrd_vol_offset:vol_offset,
            reco_settings:reco_settings.to_string(),
            state:VmState::Idle,
            kspace:None,
            imspace:None
        };
        VolumeManager::to_file(&vm);
        return vm;
    }

    pub fn open(workdir:&str) -> VolumeManager{
        let vm_path = Path::new(workdir).join("vol_manager").with_extension("toml");
        let mut f = File::open(vm_path).expect("cannot open file");
        let mut sbuf = String::new();
        f.read_to_string(&mut sbuf).expect("problem reading file");
        return toml::from_str(&sbuf).expect("cannot deserialize volume manager. File may be corrupt");
    }

    fn to_file(&self){
        let vm_path = Path::new(&self.file);
        let mut f = File::create(vm_path).expect("cannot create file");
        let vm_str = toml::to_string(&self).expect("cannot serialize data structure");
        f.write_all(vm_str.as_bytes()).expect("trouble writing to file");
    }

    pub fn advance(workdir:&str) -> VolumeManager{
        use VmState::*;
        let mut vm = VolumeManager::open(workdir);
        match vm.state {
            Idle => {
                vm.advance_state();
            }
            PreProcessing => {
                if Path::new(&vm.mrd).exists(){
                    let mut mrd = Mrd::new(&vm.mrd);
                    mrd.load_volume(vm.mrd_vol_offset as u16);
                    let petab = Petable::new(&vm.phase_table);
                    let mrd_name = Path::new(&vm.mrd).with_extension("");
                    let mrd_name = mrd_name.file_name().unwrap().to_str().unwrap();
                    let kspace = Path::new(&workdir).join(&format!("{}_kspace",mrd_name)).with_extension("");
                    mrd.write_zero_filled_cfl(kspace.to_str().unwrap(), &petab);
                    vm.kspace = Some(kspace.to_str().unwrap().to_string());
                    vm.advance_state();
                }
            },
            Reconstructing => {
                let kspace = vm.kspace.clone().unwrap();
                let mrd_name = Path::new(&vm.mrd).with_extension("");
                let mrd_name = mrd_name.file_name().unwrap().to_str().unwrap();
                let imspace = Path::new(&workdir).join(&format!("{}_imspace",mrd_name)).with_extension("");
                bart_pics(&kspace,imspace.to_str().unwrap(),&vm.reco_settings);
                vm.imspace = Some(imspace.to_str().unwrap().to_string());
                vm.advance_state();
            }
            Done => {/*no op*/}
        }
        return vm;
    }

    fn advance_state(&mut self){
        use VmState::*;
        match self.state{
            Idle => self.state = PreProcessing,
            PreProcessing => self.state = Reconstructing,
            Reconstructing => self.state = Done,
            Done => {/*no op*/}
        }
        self.to_file();
    }

    fn fpath(workdir:&str) -> PathBuf{
        return Path::new(workdir).join("vol_manager").with_extension("toml");
    }

    pub fn exists(workdir:&str) -> bool{
        let vm_path = VolumeManager::fpath(workdir);
        return vm_path.exists();
    }

    pub fn remove(workdir:&str){
        if VolumeManager::exists(workdir){
            remove_file(VolumeManager::fpath(workdir)).expect("problem occured trying to delete file");
        }
    }

}

pub fn launch_volume_manager(workdir:&str,mrd:&str,phase_table:&str,vol_offset:usize,bart_settings_file:&str){
    use VmState::*;
    let mut vm = VolumeManager::launch(workdir,mrd,phase_table,vol_offset,bart_settings_file);
    loop {
        let prev_state = vm.state.clone();
        vm = VolumeManager::launch(workdir,mrd,phase_table,vol_offset,bart_settings_file);
        if prev_state == vm.state{
            break
        }

        match vm.state {
            Done => break,
            _ => continue
        }
    }
}

#[test]
fn test(){
    use VmState::*;
    let bart_settings_file = "/home/waustin/vol_00/reco_settings";
    let mrd = "/home/waustin/mrs_test_data/test_data/N20220728_00/_02_ICO61_6b0/220728T16_m00.mrd";
    let ptab = "/home/waustin/mrs_test_data/petableCS_stream/stream_CS480_8x_pa18_pb54";
    let workdirname = "/home/waustin/vol_00";
    let workdir = Path::new(workdirname);
    if !workdir.exists(){
        create_dir_all(workdir).expect("problem creating directory"); 
    }

    let bart_settings = BartPicsSettings::quick();
    bart_settings.to_file(bart_settings_file);

    let mut vm = VolumeManager::launch(workdirname,mrd,ptab,0,bart_settings_file);
    loop {
        let prev_state = vm.state.clone();
        vm = VolumeManager::launch(workdirname,mrd,ptab,0,bart_settings_file);
        if prev_state == vm.state{
            break
        }
        match vm.state {
            Done => break,
            _ => continue
        }
    }
    
}