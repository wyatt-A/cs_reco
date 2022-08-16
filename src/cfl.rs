use crate::utils;
use std::{collections::HashMap, hash};

pub fn get_dims(path:&str) -> Vec<usize>{
    let h = load_cfl_header(path);
    let d = h.get("# Dimensions").expect("Couldn't find # dimesions").to_owned();
    let dim_str:Vec<&str> = d.split_whitespace().collect();
    let dims:Vec<usize> = dim_str.iter().flat_map(|str| str.to_string().parse()).collect();
    return dims;
}

pub fn load_cfl_header(path:&str) -> HashMap<String,String>{
    let s = utils::read_to_string(path, "hdr");
    let mut h = HashMap::<String,String>::new();
    let lines:Vec<&str> = s.lines().collect();
    lines.iter().enumerate().for_each( |(i,line)|
    {
        if line.starts_with("#"){
            let key = line.to_owned().to_string();
            h.insert(key,lines[i+1].to_string());
        }
    });
    return h;
}

#[test]
fn test(){
    let p = "/Users/Wyatt/cs_recon/rust/cs_recon/zeros.cfl";
    let c = load_cfl_header(p);
    let d = get_dims(p);
    println!("{:?}",d);
}