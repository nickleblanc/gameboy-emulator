use memmap2::{Mmap, MmapMut};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

pub struct Save {
    pub ram: MmapMut,
}

impl Save {
    pub fn new(path: &Path, capacity: usize) -> Save {
        let mut path = PathBuf::from(path);
        path.set_extension("sav");

        let filename = path.file_name().unwrap().to_str().unwrap();
        println!("Filename: {}", filename);

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(false)
            .create(true)
            .open(&path)
            .unwrap();

        file.set_len(capacity as u64).unwrap();

        let mmap = unsafe { Mmap::map(&file).unwrap().make_mut().unwrap() };

        Save { ram: mmap }
    }
}
