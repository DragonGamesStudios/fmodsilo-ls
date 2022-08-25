use std::{collections::HashMap, sync::{Mutex, Arc}, path::PathBuf, fs};

use lsp_json::ClientCapabilities;

use self::fmod::{Mod, LuaFile};

pub mod fmod;

pub struct Workspace {
    client_capabilities: Option<ClientCapabilities>,
    mods: Mutex<HashMap<String, Arc<Mod>>>,
    lua_files: Mutex<HashMap<PathBuf, Arc<LuaFile>>>
}

impl Workspace {
    pub fn new() -> Workspace {
        Workspace {
            client_capabilities: None,
            mods: Mutex::new(HashMap::new()),
            lua_files: Mutex::new(HashMap::new())
        }
    }

    pub fn get_mod(&self, name: &String) -> Option<Arc<Mod>> {
        match self.mods.lock().unwrap().get(name) {
            Some(v) => Some(Arc::clone(v)),
            None => None
        }
    }

    pub fn map_mods<C: FromIterator<T>, T, F: FnMut((&String, &Arc<Mod>)) -> T>(&self, f: F) -> C {
        self.mods.lock().unwrap().iter().map(f).collect()
    }

    pub fn add_lua_file(&self, path: PathBuf) {
        self.lua_files.lock().unwrap().insert(path.to_owned(), Arc::new(LuaFile::new(path)));
    }

    pub fn delete_lua_file(&self, path: &PathBuf) -> Option<Arc<LuaFile>> {
        self.lua_files.lock().unwrap().remove(path)
    }

    pub fn add_mod(&self, name: String, root: PathBuf) {
        self.add_directory(&root);
        self.mods.lock().unwrap().insert(name, Arc::new(Mod::new(root)));
    }

    pub fn remove_mod(&self, name: &String) -> Option<Arc<Mod>> {
        match self.mods.lock().unwrap().remove(name) {
            Some(entry) => {
                self.remove_directory(entry.get_root());
                Some(entry)
            },
            None => None
        }
    }

    pub fn remove_directory(&self, path: &PathBuf) {
        self.lua_files.lock().unwrap().retain(|file_path, _| {
            !file_path.starts_with(path)
        });
    }

    pub fn add_directory(&self, path: &PathBuf) -> Vec<std::io::Error> {
        let mut errors = Vec::new();

        for f in match fs::read_dir(path) {
            Ok(v) => v,
            Err(err) => return vec![err]
        } {
            match f {
                Ok(new_entry) => match new_entry.metadata() {
                    Ok(mt) => {
                        if mt.is_dir() {
                            errors.extend(self.add_directory(&new_entry.path()).into_iter());
                        } else if mt.is_file() {
                            self.add_lua_file(new_entry.path());
                        }
                    },
                    Err(err) => errors.push(err)
                },
                Err(err) => errors.push(err)
            }
        }

        errors
    }

    pub fn set_client_capabilities(&mut self, capabilities: ClientCapabilities) {
        self.client_capabilities = Some(capabilities);
    }

    pub fn get_client_capabilities(&self) -> &Option<ClientCapabilities> {
        &self.client_capabilities
    }
}