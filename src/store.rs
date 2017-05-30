use keepass::{Database, OpenDBError, Group};
use std::fs;
use std::path::{PathBuf, Path};
use errors::*;

const KEEPASS_FILE_ENDING: &'static str = ".kdbx";

// scan for keepass files
pub fn find_all_keepass_files<P: AsRef<Path>>(directory: P) -> Result<Vec<KeePassFile<PathBuf>>> {
    let mut ret = Vec::new();

    for entry in fs::read_dir(directory)? {
        let path: PathBuf = entry?.path();

        // whoooaaaah
        if path.is_file() {
            if let Some(os_filename) = path.file_name() {
                if let Some(filename) = os_filename.to_str() {
                    if filename.ends_with(KEEPASS_FILE_ENDING) {
                        ret.push(KeePassFile::new(path.clone(), filename.to_owned()));
                    }
                }
            }
        }
    }
    Ok(ret)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeePassEntry {
    pub title: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub url: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug)]
pub struct KeePassFile<P: AsRef<Path>> {
    pub filepath: P,
    pub filename: String,    
}

impl<P: AsRef<Path>> KeePassFile<P> {
    
    pub fn new(filepath: P, filename: String) -> Self {
        KeePassFile {
            filepath: filepath,
            filename: filename,
        }
    }

    pub fn get_entries(&self, master_password: &str) -> Result<Vec<KeePassEntry>> {
        let database = &open_database(&self.filepath, master_password)?;
        Ok(find_entries("", &database.root))
    }
}

fn find_entries(rootnavpath: &str, group: &Group) -> Vec<KeePassEntry> {
    let navpath = format!("{}/{}", rootnavpath, group.name);
    let mut ret = Vec::new();

    for e in &group.entries {
        let keepass_entry = KeePassEntry {
            title: format!("{}/{}", &navpath, e.get_title().unwrap_or("")),
            username: e.get_username().map(ToOwned::to_owned),
            password: e.get_password().map(ToOwned::to_owned),
            url: e.get("URL").map(ToOwned::to_owned),
            notes: e.get("Notes").map(ToOwned::to_owned),
        };

        ret.push(keepass_entry);
    }

    for child_group in &group.child_groups {
        ret.append(&mut find_entries(&navpath, child_group));
    }

    ret
}

fn open_database<P: AsRef<Path>>(filepath: P, password: &str) -> Result<Database> {
    fs::File::open(filepath)
        .map_err(OpenDBError::Io)
        .and_then(|mut db_file| Database::open(&mut db_file, password))
        .chain_err(|| "could not open database")
}