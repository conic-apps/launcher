#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JavaExec {
    pub binary: PathBuf,
    // pub version: String,
    // pub version_major: String,
}

impl JavaExec {
    pub async fn new<P: AsRef<OsStr> + ?Sized>(home: &P) -> Self {
        let home = Path::new(home).to_path_buf();
        // let release = tokio::fs::read_to_string(home.join("release"))
        //     .await
        //     .unwrap();
        // let version = release
        //     .lines()
        //     .find(|line| line.starts_with("JAVA_VERSION"))
        //     .unwrap()
        //     .split("=")
        //     .collect::<Vec<&str>>()
        //     .get(1)
        //     .unwrap()
        //     .trim()
        //     .to_string();
        Self {
            binary: home.join("bin").join("java"),
            // version_major: version.split(".").collect::<Vec<&str>>().get(0).unwrap().to_string(),
            // version,
        }
    }
}

