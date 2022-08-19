use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path,PathBuf};

pub struct Headfile{
    items:HashMap<String,String>
}

impl Headfile{
    pub fn from_mrd_meta(mrd_meta_file:&Path) -> Headfile{
        let mut f = File::open(mrd_meta_file).expect("cannot open file");
        let mut strbuff = String::new();
        f.read_to_string(&mut strbuff).expect("trouble reading file");
        return Headfile{items:Headfile::txt_to_hash(strbuff)}
    }

    pub fn append_field<T,U>(&mut self,key:T,value:U)
    where T:std::string::ToString, U:std::string::ToString
    {
        let old_val = self.items.insert(key.to_string(),value.to_string());
        if old_val.is_some(){
            println!("value {} updated to {}",old_val.unwrap(),value.to_string());
        }
    }

    pub fn write_headfile(&self,headfile:&Path){
        let mut f = File::create(headfile).expect("cannot create file");
        let mut strbuf = String::new();
        for (key, val) in self.items.iter() {
            strbuf.push_str(key);
            strbuf.push('=');
            strbuf.push_str(val);
            strbuf.push('\n');
        }
        f.write_all(strbuf.as_bytes()).expect("problem writing to file");
    }

    // pub fn to_hash_map(&mut self) -> HashMap<String,String>{
    //     let mut rawstr = String::new();
    //     self.file.read_to_string(&mut rawstr).expect("problem reading file");
    //     return Headfile::txt_to_hash(rawstr);
    // }

    pub fn to_hash(headfile:PathBuf) -> HashMap<String,String>{
        let mut f = File::open(headfile).expect("cannot open file");
        let mut strbuff = String::new();
        f.read_to_string(&mut strbuff).expect("issue reading file");
        return Headfile::txt_to_hash(strbuff);
    }

    pub fn txt_to_hash(headfile_str:String) -> HashMap<String,String>{
        let mut hf = HashMap::<String,String>::new();
        headfile_str.lines().for_each(|line|{
            // split on the first = we find
            match line.find("="){
                Some(index) => {
                    let (key,val) = line.split_at(index);
                    let key = key.to_string();
                    let mut val = val.to_string();
                    val.remove(0);// remove leading "="
                    hf.insert(key.to_string(),val);
                },
                None => () // do not add to hash if "=" not found
            }
        });
        Headfile::translate_field_names(&mut hf);
        return hf;
    }

    fn translate_field_names(meta_hash:&mut HashMap<String,String>){
        println!("transcribing fields ...");
        transcribe_numeric(meta_hash,"fov_read","fovx",1000.0 as f32);
        transcribe_numeric(meta_hash,"fov_phase","fovy",1000.0 as f32);
        transcribe_numeric(meta_hash,"fov_slice","fovz",1000.0 as f32);
        transcribe_numeric(meta_hash,"echo_time","te",1000.0 as f32);
        transcribe_numeric(meta_hash,"rep_time","tr",1000000.0 as f32);
        transcribe_numeric(meta_hash,"flip","alpha",1.0 as f32);
        transcribe_numeric(meta_hash,"bandwidth","bw",0.5 as f32);
        transcribe_numeric(meta_hash,"ppr_no_echoes","ne",1 as i32);
        transcribe_string(meta_hash,"acq_Sequence","S_PSDname");
        meta_hash.insert("F_imgformat".to_string(),"raw".to_string());
    }

}

fn transcribe_numeric<T>(hash:&mut HashMap<String,String>,old_name:&str,new_name:&str,scale:T)
where T: std::fmt::Display + std::str::FromStr + std::ops::MulAssign,
<T as std::str::FromStr>::Err: std::fmt::Debug
{
    match hash.get(old_name){
        Some(string) => {
            let mut num:T = string.parse().expect("cannot parse value");
            num *= scale;
            let str = num.to_string();
            hash.insert(new_name.to_string(),str);
        }
        None => {println!("{} field not found... not transcribing",old_name);}
    }
}

fn transcribe_string(hash:&mut HashMap<String,String>,old_name:&str,new_name:&str)
{
    match hash.get(old_name){
        Some(str) => {
            hash.insert(new_name.to_string(),str.to_string());
        },
        None => {
            println!("{} field not found... not transcribing",old_name);
        }
    }
}

#[test]
fn test_make_headfile() {
    let test_file = "/Users/Wyatt/cs_recon/test_data/N20220808_00/_02_ICO61_6b0/220808T12_m00_meta.txt";
    let headfile = "test.headfile";
    let mut hf = Headfile::from_mrd_meta(&Path::new(test_file));
    hf.write_headfile(Path::new(headfile));
    hf.append_field("DUMMYFIELD",6.5);
    hf.write_headfile(&Path::new(test_file));
}