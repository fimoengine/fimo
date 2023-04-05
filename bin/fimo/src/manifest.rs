use fimo_ffi::Version;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    error::Error,
    path::{Path, PathBuf},
};
use wax::Glob;

#[derive(Serialize, Deserialize, Debug)]
struct FimoManifest {
    workspace: FimoManifestWorkspace,
    test: Option<FimoManifestTest>,
}

#[derive(Serialize, Deserialize, Debug)]
struct FimoManifestWorkspace {
    name: String,
    modules: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct FimoManifestTest {
    runner: String,
    include: Vec<String>,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Clone)]
pub struct FimoWorkspace {
    pub name: String,
    pub modules: Vec<(PathBuf, FimoModuleManifest)>,
    pub test: Option<FimoWorkspaceTest>,
}

#[derive(Clone)]
pub struct FimoWorkspaceTest {
    pub runner: String,
    pub include: Vec<PathBuf>,
    pub args: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FimoModuleManifest {
    pub module: FimoModule,
    pub profile: Vec<FimoModuleProfile>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FimoModule {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FimoModuleProfile {
    pub name: String,
    pub loader: String,
    pub builder: FimoModuleBuilder,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub export: Vec<FimoModuleInterface>,
    #[serde(default)]
    pub dependency: Vec<FimoModuleInterface>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FimoModuleBuilder {
    pub name: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FimoModuleInterface {
    pub name: InterfaceName,
    pub version: Version,
    #[serde(default)]
    pub extensions: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InterfaceName {
    pub name: String,
    #[serde(default)]
    pub namespace: Option<String>,
}

pub fn load_fimo_manifest(manifest_path: &Path) -> Result<FimoWorkspace, Box<dyn Error>> {
    let manifest_file = std::fs::read(manifest_path)?;
    let manifest: FimoManifest = toml::from_slice(&manifest_file)?;

    let manifest_dir = manifest_path.parent().unwrap();
    let mut module_names = BTreeSet::new();
    let mut modules: Vec<(PathBuf, FimoModuleManifest)> =
        Vec::with_capacity(manifest.workspace.modules.len());

    for pattern in &manifest.workspace.modules {
        let glob = Glob::new(pattern).expect("Failed to read glob pattern");
        for entry in glob.walk(manifest_dir) {
            let entry = entry.unwrap();
            let module = entry.path();
            let module_manifest_path = module.join("FimoModule.toml");
            if !module_manifest_path.exists() {
                continue;
            }

            let module_manifest = std::fs::read(&module_manifest_path)?;
            let module_manifest = module_manifest_from_slice(&module_manifest)?;

            if !module_names.insert(module_manifest.module.name.clone()) {
                return Err(
                    format!("duplicate module name {}", module_manifest.module.name).into(),
                );
            }

            modules.push((module.to_path_buf(), module_manifest));
        }
    }

    let test = if let Some(test) = manifest.test {
        let mut include = Vec::new();
        for pattern in &test.include {
            let glob = Glob::new(pattern).expect("Failed to read glob pattern");
            for entry in glob.walk(manifest_dir) {
                let entry = entry.unwrap();
                let path = entry.path();
                if !path.exists() || !path.is_dir() {
                    continue;
                }

                include.push(path.to_path_buf());
            }
        }

        Some(FimoWorkspaceTest {
            runner: test.runner,
            include,
            args: test.args,
        })
    } else {
        None
    };

    Ok(FimoWorkspace {
        name: manifest.workspace.name,
        modules,
        test,
    })
}

pub fn module_manifest_from_slice(bytes: &[u8]) -> Result<FimoModuleManifest, Box<dyn Error>> {
    Ok(toml::from_slice(bytes)?)
}

#[allow(dead_code)]
pub fn module_manifest_from_str(s: &str) -> Result<FimoModuleManifest, Box<dyn Error>> {
    Ok(toml::from_str(s)?)
}
