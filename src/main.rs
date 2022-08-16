use cs_reco::volume_manager::launch_volume_manager;
use cs_reco::test::{main_test_cluster,main_test_local};
use clap::Parser;

/*
    Main entry point for arguments: looking for a sub-command and their args
*/
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args{
    sub_cmd:String,
    vargs:Vec<String>
}

/*
    Volume manager arguments
*/
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct VolumeManagerArgs{
    parent:String,
    working_directory:String,
    mrd_file:String,
    phase_encode_stream_table:String,
    volume_offset:usize,
    bart_settings_file:String
}

/*
    Mrd to cfl args
*/
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct MrdToCflArgs{
    parent:String,
    cmd2:String,
    cmd3:String
}

fn main(){
    let args = Args::parse();
    match args.sub_cmd.as_str(){
        "volume-manager" => {
            let a = VolumeManagerArgs::parse();
            launch_volume_manager(
                &a.working_directory,
                &a.mrd_file,
                &a.phase_encode_stream_table,
                a.volume_offset,
                &a.bart_settings_file
            );
        }
        "cluster-test" => {
            main_test_cluster();
        },
        "local-test" => {
            main_test_local();
        },
        _ => println!("sub-command not recognized")
    }
}