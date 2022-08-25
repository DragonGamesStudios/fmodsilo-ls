use std::path::PathBuf;

pub struct FileData {
    pub path: PathBuf
}

pub struct LuaFile {
    pub file: FileData
}

impl LuaFile {
    pub fn new(path: PathBuf) -> LuaFile {
        LuaFile {
            file: FileData { path }
        }
    }
}

pub struct Mod {
    root: PathBuf
}

impl Mod {
    pub fn new(root_path: PathBuf) -> Mod {
        Mod {
            root: PathBuf::from(root_path)
        }
    }

    pub fn get_root(&self) -> &PathBuf {
        &self.root
    }
}