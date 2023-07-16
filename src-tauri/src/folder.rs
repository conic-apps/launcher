use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

pub struct DataLocation {
    pub instances: PathBuf,
    pub jre: PathBuf,
    pub resources: PathBuf,
}

impl DataLocation {
    pub fn new<S: AsRef<OsStr> + ?Sized>(data_folder: &S) -> Self {
        let data_folder = Path::new(data_folder);
        Self {
            instances: data_folder.join("instances"),
            jre: data_folder.join("jre"),
            resources: data_folder.join("resources"),
        }
    }

    pub fn get_instance_root<P: AsRef<Path>>(&self, instance_name: P) -> PathBuf {
        self.instances.join(instance_name)
    }
}
