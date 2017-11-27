extern crate blake2;
extern crate time;
extern crate filebuffer;

use std::io::{self, Write};
use std::collections::HashMap;
use std::fs::{self, DirEntry, remove_file, OpenOptions};
use std::path::{Path, PathBuf};
use time::PreciseTime;

use blake2::Blake2b;
use blake2::digest::{Input, VariableOutput};
use filebuffer::FileBuffer;


fn visit_dirs(dir: &Path, cb: &Fn(&mut HashMap<u64, Vec<PathBuf>>, &DirEntry), dict: &mut HashMap<u64, Vec<PathBuf>>) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                match visit_dirs(&path, cb, dict) {
                    Ok(v) => v,
                    Err(err) => println!("Error: {:?} - {:?}", &path, err),
                };
            } else {
                cb(dict, &entry);
            }
        }
    }
    Ok(())
}

fn add_to_dict(dict: &mut HashMap<u64, Vec<PathBuf>>, file: &DirEntry) {
    if let Ok(metadata) = file.metadata() {
        dict.entry(metadata.len()).or_insert(Vec::new()).push(file.path());
    }
}

fn remove_uniques(dict: &mut HashMap<u64, Vec<PathBuf>>) {
    let copy: HashMap<u64, Vec<PathBuf>> = dict.clone();
    for (size, paths) in copy {
        if paths.len() == 1 {
            dict.remove(&size);
        }
    }
}

fn remove_uniques_(dict: &mut HashMap<String, Vec<PathBuf>>) {
    let copy = dict.clone();
    for (key, paths) in copy {
        if paths.len() == 1 {
            dict.remove(&key);
        }
    }
}

fn blake2_hash(fbuffer: &FileBuffer) -> String {
    let mut hasher = Blake2b::new(64).unwrap();
    hasher.process(&fbuffer);
    let mut buf = [0u8; 64];
    let bytes: Vec<u8> = hasher.variable_result(&mut buf).unwrap().to_vec();
    let strs:Vec<String> = bytes.iter().map(|b| format!("{:02X}", b)).collect();
    return strs.join("");
}

fn find_duplicates(files: &HashMap<u64, Vec<PathBuf>>, duplicates: &mut HashMap<String, Vec<PathBuf>>) {
    for (_size, paths) in files {
        for path in paths {
            let fbuffer = match FileBuffer::open(&path) {
                Ok(v) => v,
                Err(err) => {
                    println!("failed to open file: {:?}", err);
                    continue;
                },
            };
            let hash: String = blake2_hash(&fbuffer);
            duplicates.entry(hash).or_insert(Vec::new()).push(path.to_path_buf());
        }
    }
}

fn search(path: &Path) {
    let start = PreciseTime::now();
    println!("[+] find files in {:?}", path);    
    let mut dict = HashMap::new();
    match visit_dirs(path, &add_to_dict, &mut dict) {
        Ok(n) => n,
        Err(err) => println!("Error: {:?} - {:?}", &path, err),
    }
    remove_uniques(&mut dict);
    println!("\t[+] found {} different file sizes", dict.len());

    println!("[+] hashing");

    let mut duplicates = HashMap::new();
    find_duplicates(&dict, &mut duplicates);
    remove_uniques_(&mut duplicates);
    println!("[+] found {} duplicate hashes", duplicates.len());
    println!("\t[+] printing duplicates\n");

    let path = Path::new("log.txt");
    if path.exists() {
        match remove_file(path) {
            Ok(n) => n,
            Err(err) => println!("Error: {:?}", err)
        }
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path).unwrap();
    for (hash, paths) in &duplicates {
        let strs: Vec<String> = paths.iter().map(|b| format!("\t{:?}\n", b)).collect();
        println!("{}\n{}", hash, strs.join(""));
        if let Err(e) =  writeln!(file, "{}\n{}", hash, strs.join("")) {
            println!("Error {:?}", e);
        }
    }

    let end = PreciseTime::now();
    println!("it took {:?} seconds", start.to(end));
}

fn main() {
    let path = Path::new("/run/media/nyx/Daten/porn/");
    search(&path);    
}
