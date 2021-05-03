use serde::Deserialize;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

/// Module manifest.
#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
#[serde(tag = "schema")]
pub enum ModuleManifest {
    /// Version `0` manifest schema.
    #[serde(rename = "0")]
    V0 {
        /// Manifest contents.
        #[serde(flatten)]
        manifest: v0::ModuleManifest,
    },
}

/// Possible validation errors.
#[derive(Debug)]
#[non_exhaustive]
pub enum ValidationError {
    /// An overflow error.
    LengthOverflow(String, usize, usize),
    /// Error indicating an invalid version string.
    InvalidVersionFormat(String, String),
    /// An IO error.
    IOError(std::io::Error),
    /// A serde error.
    SerdeError(serde_json::Error),
    /// The path does not contain a module.
    NotAModule(PathBuf),
    /// The path does not point to a manifest.
    NoManifest(PathBuf),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::LengthOverflow(name, length, max) => {
                write!(
                    f,
                    "Overflow: field: {}, length: {}, maximum length: {}",
                    name, length, max
                )
            }
            ValidationError::InvalidVersionFormat(field, version) => {
                write!(f, "Invalid version: field: {}, version: {}", field, version)
            }
            ValidationError::IOError(err) => {
                write!(f, "IO error: {}", err)
            }
            ValidationError::SerdeError(err) => {
                write!(f, "Serde error: {}", err)
            }
            ValidationError::NotAModule(path) => {
                write!(f, "The path contains no module: path: {}", path.display())
            }
            ValidationError::NoManifest(path) => {
                write!(
                    f,
                    "The path does not point to a manifest file: path: {}",
                    path.display()
                )
            }
        }
    }
}

pub mod v0 {
    use super::ValidationError;
    use fimo_version_rs::string_is_valid;
    use serde::Deserialize;
    use std::convert::TryFrom;

    /// Module manifest.
    #[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
    #[serde(try_from = "ModuleManifestShadow")]
    pub struct ModuleManifest {
        pub name: String,
        pub version: String,
        pub module_type: String,
        pub module_version: String,
        pub load_deps: Option<Vec<InterfaceDescriptor>>,
        pub runtime_deps: Option<Vec<InterfaceDescriptor>>,
        pub exports: Option<Vec<InterfaceDescriptor>>,
    }

    #[derive(Deserialize)]
    struct ModuleManifestShadow {
        name: String,
        version: String,
        module_type: String,
        module_version: String,
        #[serde(default)]
        load_deps: Option<Vec<InterfaceDescriptor>>,
        #[serde(default)]
        runtime_deps: Option<Vec<InterfaceDescriptor>>,
        #[serde(default)]
        exports: Option<Vec<InterfaceDescriptor>>,
    }

    /// Interface description.
    #[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
    #[serde(try_from = "InterfaceDescriptorShadow")]
    pub struct InterfaceDescriptor {
        pub name: String,
        pub version: String,
        pub extensions: Option<Vec<String>>,
    }

    #[derive(Deserialize)]
    struct InterfaceDescriptorShadow {
        pub name: String,
        pub version: String,
        #[serde(default)]
        pub extensions: Option<Vec<String>>,
    }

    impl TryFrom<ModuleManifestShadow> for ModuleManifest {
        type Error = ValidationError;

        fn try_from(shadow: ModuleManifestShadow) -> Result<Self, Self::Error> {
            if shadow.name.as_bytes().len() > 32 {
                return Err(ValidationError::LengthOverflow(
                    String::from("name"),
                    shadow.name.as_bytes().len(),
                    32,
                ));
            }

            if !string_is_valid(&shadow.version.as_str()) {
                return Err(ValidationError::InvalidVersionFormat(
                    String::from("version"),
                    shadow.version,
                ));
            }

            if shadow.module_type.as_bytes().len() > 64 {
                return Err(ValidationError::LengthOverflow(
                    String::from("module_type"),
                    shadow.module_type.as_bytes().len(),
                    64,
                ));
            }

            if shadow.module_version.as_bytes().len() > 32 {
                return Err(ValidationError::LengthOverflow(
                    String::from("module_version"),
                    shadow.module_version.as_bytes().len(),
                    32,
                ));
            }

            Ok(ModuleManifest {
                name: shadow.name,
                version: shadow.version,
                module_type: shadow.module_type,
                module_version: shadow.module_version,
                load_deps: shadow.load_deps,
                runtime_deps: shadow.runtime_deps,
                exports: shadow.exports,
            })
        }
    }

    impl TryFrom<InterfaceDescriptorShadow> for InterfaceDescriptor {
        type Error = ValidationError;

        fn try_from(shadow: InterfaceDescriptorShadow) -> Result<Self, Self::Error> {
            let InterfaceDescriptorShadow {
                name,
                version,
                extensions,
            } = shadow;

            if name.as_bytes().len() > 32 {
                return Err(ValidationError::LengthOverflow(
                    String::from("name"),
                    name.as_bytes().len(),
                    32,
                ));
            }

            if !string_is_valid(&version.as_str()) {
                return Err(ValidationError::InvalidVersionFormat(
                    String::from("version"),
                    version,
                ));
            }

            if let Some(extensions) = extensions.as_ref() {
                for extension in extensions.iter() {
                    if extension.as_bytes().len() > 32 {
                        return Err(ValidationError::LengthOverflow(
                            String::from("extensions::") + extension.as_str(),
                            extension.as_bytes().len(),
                            32,
                        ));
                    }
                }
            }

            Ok(InterfaceDescriptor {
                name,
                version,
                extensions,
            })
        }
    }
}

/// Parses the manifest using a reader.
pub fn parse_manifest<R: Read>(reader: BufReader<R>) -> Result<ModuleManifest, ValidationError> {
    serde_json::from_reader(reader).map_err(ValidationError::SerdeError)
}

/// Parses the manifest from a module.
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use fimo_bootstrap_rs::{parse_manifest_from_file, get_manifest_path};
///
/// let manifest_path = get_manifest_path(Path::new("module/")).unwrap();
/// let manifest = parse_manifest_from_file(Path::new("module/module.json"));
/// ```
pub fn parse_manifest_from_file(manifest_path: &Path) -> Result<ModuleManifest, ValidationError> {
    if !manifest_path.exists() || !manifest_path.is_file() {
        Err(ValidationError::NotAModule(manifest_path.to_path_buf()))
    } else {
        let file = match File::open(manifest_path) {
            Ok(file) => file,
            Err(e) => return Err(ValidationError::IOError(e)),
        };
        let reader = BufReader::new(file);
        parse_manifest(reader)
    }
}

/// Returns the path to the module manifest.
///
/// No value will be returned if `path` is not a module.
pub fn get_manifest_path(path: &Path) -> Option<PathBuf> {
    if !path.is_dir() {
        None
    } else {
        let manifest_path = path.join("module.json");
        if !manifest_path.exists() || !manifest_path.is_file() {
            None
        } else {
            Some(manifest_path)
        }
    }
}
