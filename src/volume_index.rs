use std::io::Read;
use std::path::{Path,PathBuf};
use std::collections::HashMap;
use std::fs::File;

pub struct VolumeIndex{

}

impl VolumeIndex{
    pub fn read(path:&str) -> HashMap<String,String>{
        let mut h = HashMap::<String,String>::new();
        let fpath = Path::new(path);
        let mut f = File::open(fpath).expect("cannot open file");
        let mut strbuff = String::new();
        f.read_to_string(&mut strbuff).expect("problem reading file");
        //let mut strvec:Vec<&str>;
        strbuff.lines().for_each(|line| {
            let strvec:Vec<&str> = line.split_whitespace().collect();
            if strvec.len() == 2 {
                h.insert(strvec[1].to_string(),strvec[0].to_string());
            }
        }
        );
        return h;
    }
}