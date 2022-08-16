use std::fs::File;
use std::path::Path;
use std::io::{BufReader,Read};

pub struct Petable {
    pub file:File,
    pub size:usize,
}

impl Petable {
    pub fn new(path:&str,size:usize) -> Petable{
        let pe_table_path = Path::new(path);
        let file = File::open(pe_table_path).expect("cannot open pe_table");
        return Petable{file:file,size:size};
    }

    fn read_values(&self) -> Vec<i32> {
        println!("reading phase encode table ...");
        let mut file_reader = BufReader::new(&self.file);
        let mut strbuff = String::new();
        file_reader.read_to_string(&mut strbuff).expect("oops. Something went wrong reading the pe table");
        return strbuff.split("\r\n").flat_map(|x| x.parse()).collect();
    }

    pub fn coordinates(&self) -> Vec<(i32,i32)>{
        let vals = self.read_values();
        let n = vals.len()/2;
        let mut indices:Vec<(i32,i32)> = Vec::with_capacity(n);
        for i in 0..n {
            indices.push((vals[2*i],vals[2*i+1]));
        };
        return indices;
    }

    pub fn indices(&self) -> Vec<(usize,usize)> {
        let coords = self.coordinates();
        let offset = (self.size/2) as i32;
        return coords.iter().map(|coord| (
            (coord.0 + offset) as usize,
            (coord.1 + offset) as usize
        )).collect();
    }
}

#[test]
fn test(){
    let ptab = Petable::new("/Users/Wyatt/cs_recon/test_data/petableCS_stream/stream_CS480_8x_pa18_pb54",480);
}