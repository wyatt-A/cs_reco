use crate::resource::*;
use crate::volume_index::VolumeIndex;
use std::path::Path;
use crate::bart_wrapper::BartPicsSettings;
use crate::volume_manager::launch_volume_manager;
use crate::slurm::BatchScript;

#[test]
fn main(){
let cluster = false;
let bart_settings = BartPicsSettings::quick();   
let bart_settings_file = "/home/waustin/vol_00/reco_settings";
//let scanner = Host::new("mrs","stejskal");
let workdir = "/home/waustin/local_recon";
let ptab = "/home/waustin/mrs_test_data/petableCS_stream/stream_CS480_8x_pa18_pb54";
//let vpath = "/d/smis/N20220811_00/_02_ICO61_6b0/volume_index.txt";
let vpath = "/home/waustin/mrs_test_data/test_data/N20220728_00/_02_ICO61_6b0/volume_index.txt";
let vol_index_local = true;
let vol_prefix = "m";
let mrd_vol_offset = 0;

bart_settings.to_file(bart_settings_file);
let mut vol_index = Resource::new(vpath,workdir);

//let vol_index_local = false;
//vol_index.set_remote_host(&scanner);
vol_index.update(vol_index_local);   
let vhash = VolumeIndex::read(&vol_index.local_path());
let mut r = ResourceList::open(workdir);
//r.set_host(&scanner);
for (key, value) in vhash.into_iter() {
    let src_path = Path::new(vpath).with_file_name(&key).into_os_string().into_string().unwrap();
    let mut dest = vol_prefix.to_string();
    dest.push_str(&value);
    // if a resource has already been added, it will not be added again despite the method call
    r.try_add(Resource::new(&src_path,&dest));
    println!("destination: {}",dest);
}
r.start_transfer();

r.item.iter().for_each(|item| {
    let mrd = item.local_path();
    let volworkdir = item.local_dir();
    if !cluster {
    launch_volume_manager(
        &volworkdir,
        &mrd,
        ptab,
        mrd_vol_offset,
        bart_settings_file
    );
}else{
    let cmd = format!("/home/waustin/cs_reco/target/debug/cs_reco {} {} {} {} {}",
    volworkdir,mrd,ptab,mrd_vol_offset,bart_settings_file);
    let mut j = BatchScript::new(&format!("slurm_job"));
    j.commands.push(cmd);
    j.submit(&volworkdir);
}
});

//if not all work done, schedule again for later


}