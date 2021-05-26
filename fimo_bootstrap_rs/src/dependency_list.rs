use crate::manifest::v0::InterfaceDescriptor;
use crate::{
    get_manifest_path, parse_manifest_from_file, ModuleCallback, ModuleManifest, ValidationError,
};
use emf_core_base_rs::ffi::CBASE_INTERFACE_NAME;
use emf_core_base_rs::ownership::Owned;
use emf_core_base_rs::version::Version;
use emf_core_base_rs::{version, Error};
use fimo_version_rs::{compare_strong, from_string, is_compatible};
use petgraph::graph::NodeIndex;
use petgraph::prelude::EdgeRef;
use petgraph::{Direction, Graph};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

/// Possible dependency errors.
#[derive(Debug)]
#[non_exhaustive]
pub enum DependencyError {
    /// Error originating from the api.
    APIError(Error<Owned>),
    /// Error while validating a module manifest.
    ModuleManifestError(ValidationError),
    /// The provided core module is invalid.
    InvalidCoreModule(PathBuf),
    /// The library does not expose the required interface.
    CoreInterfaceNotFound,
    /// A path does not belong to a module.
    NotAModule(PathBuf),
    /// A module does not export a required interface.
    MissingExport(PathBuf, String),
    /// An interface is exported multiple times.
    DuplicateInterface(String, version::Version, Vec<String>),
    /// Not all dependencies are met.
    MissingDependencies,
    /// No load order exists.
    CyclicDependencies,
}

impl From<ValidationError> for DependencyError {
    fn from(err: ValidationError) -> Self {
        DependencyError::ModuleManifestError(err)
    }
}

impl From<Error<Owned>> for DependencyError {
    fn from(err: Error<Owned>) -> Self {
        DependencyError::APIError(err)
    }
}

pub struct ModuleDependency<'a> {
    pub loader: String,
    pub module_path: PathBuf,
    pub manifest: ModuleManifest,
    pub exports: Vec<InterfaceExport>,
    pub load_fn: Option<Box<ModuleCallback<'a>>>,
    pub init_fn: Option<Box<ModuleCallback<'a>>>,
}

impl Debug for ModuleDependency<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleDependency")
            .field("module_path", &self.module_path)
            .field("manifest", &self.manifest)
            .field("exports", &self.exports)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceExport {
    pub name: String,
    pub version: Version,
    pub extensions: Vec<String>,
}

impl TryFrom<&crate::manifest::v0::InterfaceDescriptor> for InterfaceExport {
    type Error = DependencyError;

    fn try_from(value: &InterfaceDescriptor) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name.clone(),
            version: from_string(&value.version)?,
            extensions: value.extensions.clone().map_or(vec![], |v| v),
        })
    }
}

impl Hash for InterfaceExport {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl PartialEq for InterfaceExport {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && (is_compatible(&self.version, &other.version)
                || is_compatible(&other.version, &self.version))
            && self
                .extensions
                .iter()
                .all(|item| other.extensions.contains(item))
    }
}

impl Eq for InterfaceExport {}

#[derive(Debug, Clone)]
struct ModuleDescription {
    pub name: String,
    pub version: Version,
    pub module_type: String,
    pub module_version: String,
    pub exports: Vec<InterfaceExport>,
    pub load_deps: Vec<InterfaceExport>,
    pub runtime_deps: Vec<InterfaceExport>,
}

impl TryFrom<&ModuleManifest> for ModuleDescription {
    type Error = DependencyError;

    fn try_from(manifest: &ModuleManifest) -> Result<Self, Self::Error> {
        match manifest {
            ModuleManifest::V0 { manifest } => Ok(ModuleDescription {
                name: manifest.name.clone(),
                version: from_string(&manifest.version)?,
                module_type: manifest.module_type.clone(),
                module_version: manifest.module_version.clone(),
                exports: manifest.exports.as_ref().map_or(vec![], |v| {
                    v.iter().map(|v| v.try_into().unwrap()).collect()
                }),
                load_deps: manifest.load_deps.as_ref().map_or(vec![], |v| {
                    v.iter().map(|v| v.try_into().unwrap()).collect()
                }),
                runtime_deps: manifest.runtime_deps.as_ref().map_or(vec![], |v| {
                    v.iter().map(|v| v.try_into().unwrap()).collect()
                }),
            }),
        }
    }
}

/// Dependency-Graph node type.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum NodeType {
    /// Load node.
    Load(usize),
    /// Initialisation node.
    Initialize(usize),
    /// Root node.
    Root,
}

type MissingExports = RefCell<Vec<InterfaceExport>>;
pub type DependencyOrder<'a, 'b> = (&'a Vec<ModuleDependency<'b>>, Vec<NodeType>);

/// List of dependencies.
#[derive(Debug)]
pub struct DependencyList<'a> {
    _core_version: Version,
    _modules: Vec<ModuleDependency<'a>>,
    _interfaces: HashMap<InterfaceExport, usize>,
    _incomplete_nodes: Vec<(usize, MissingExports, MissingExports)>,
    _dependency_graph: Graph<NodeType, ()>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> DependencyList<'a> {
    /// Initializes the list with a new root core module.
    pub fn new(
        core_path: &Path,
        target_version: &Version,
        exports: &[(&str, Version)],
    ) -> Result<Self, DependencyError> {
        let manifest_path = match get_manifest_path(core_path) {
            None => return Err(DependencyError::InvalidCoreModule(core_path.to_path_buf())),
            Some(manifest_path) => manifest_path,
        };
        let module_manifest = parse_manifest_from_file(manifest_path.as_path())?;
        let module_desc: ModuleDescription = (&module_manifest).try_into()?;

        if !module_desc.load_deps.is_empty() || !module_desc.runtime_deps.is_empty() {
            Err(DependencyError::InvalidCoreModule(core_path.to_path_buf()))
        } else if let Some(interface) = module_desc
            .exports
            .iter()
            .find(|&v| v.name == CBASE_INTERFACE_NAME && is_compatible(target_version, &v.version))
        {
            let modules = vec![ModuleDependency {
                loader: module_desc.module_type.clone(),
                module_path: core_path.to_path_buf(),
                manifest: module_manifest,
                exports: vec![(*interface).clone()],
                load_fn: None,
                init_fn: None,
            }];

            // Add root interface.
            let mut interfaces = HashMap::new();
            interfaces.insert((*interface).clone(), 0);

            // Add root node.
            let mut deps = Graph::new();
            deps.add_node(NodeType::Root);

            let mut dependency_list = DependencyList {
                _core_version: *target_version,
                _modules: modules,
                _interfaces: interfaces,
                _incomplete_nodes: Vec::new(),
                _dependency_graph: deps,
                _phantom: PhantomData,
            };

            // Fetch and add exports
            let export_desc =
                dependency_list.get_exports_from_module(&module_desc, exports, core_path)?;
            dependency_list.add_exports(export_desc, 0);

            Ok(dependency_list)
        } else {
            Err(DependencyError::CoreInterfaceNotFound)
        }
    }

    fn add_exports(&mut self, exports: Vec<InterfaceExport>, module_idx: usize) {
        for export in exports.into_iter() {
            self._interfaces.insert(export, module_idx);
        }
    }

    fn get_exports_from_module(
        &self,
        module_desc: &ModuleDescription,
        exports: &[(&str, Version)],
        module_path: &Path,
    ) -> Result<Vec<InterfaceExport>, DependencyError> {
        let mut exports_desc = Vec::with_capacity(exports.len());

        // Check if all exports are valid.
        for (export, version) in exports {
            match module_desc
                .exports
                .iter()
                .find(|&e| &e.name == export && is_compatible(version, &e.version))
            {
                Some(export) => {
                    if self._interfaces.contains_key(export) {
                        return Err(DependencyError::DuplicateInterface(
                            export.name.clone(),
                            export.version,
                            export.extensions.clone(),
                        ));
                    }

                    exports_desc.push((*export).clone())
                }
                None => {
                    return Err(DependencyError::MissingExport(
                        module_path.to_path_buf(),
                        export.to_string(),
                    ))
                }
            };
        }

        Ok(exports_desc)
    }

    /// Adds a new module to the list.
    pub fn add_module(
        &mut self,
        module_path: &Path,
        exports: &[(&str, Version)],
        load_fn: Option<Box<ModuleCallback<'a>>>,
        init_fn: Option<Box<ModuleCallback<'a>>>,
    ) -> Result<(), DependencyError> {
        let manifest_path = match get_manifest_path(module_path) {
            None => return Err(DependencyError::NotAModule(module_path.to_path_buf())),
            Some(manifest_path) => manifest_path,
        };
        let module_manifest = parse_manifest_from_file(manifest_path.as_path())?;
        let mut module_desc: ModuleDescription = (&module_manifest).try_into()?;

        // Fetch exports.
        let exports_desc = self.get_exports_from_module(&module_desc, exports, module_path)?;

        // Add required core interface if not already specified.
        if !module_desc.load_deps.iter().any(|v| {
            v.name == CBASE_INTERFACE_NAME
                && compare_strong(&v.version, &module_desc.version) == Ordering::Equal
        }) {
            module_desc.load_deps.push(InterfaceExport {
                name: CBASE_INTERFACE_NAME.to_string(),
                version: module_desc.version,
                extensions: vec![],
            })
        }

        // Add the module to the graph.
        let idx = self._modules.len();
        self._modules.push(ModuleDependency {
            loader: module_desc.module_type.clone(),
            module_path: module_path.to_path_buf(),
            manifest: module_manifest,
            exports: exports_desc.clone(),
            load_fn,
            init_fn,
        });

        // Loading and initialisation are modelled as two nodes.
        let load = self._dependency_graph.add_node(NodeType::Load(idx));
        let init = self._dependency_graph.add_node(NodeType::Initialize(idx));

        // Initialisation always depends on loading.
        self._dependency_graph.add_edge(init, load, ());

        // Update export map
        self.add_exports(exports_desc, idx);

        // Recheck incomplete nodes
        {
            let incomplete = &mut self._incomplete_nodes;
            let interfaces = &self._interfaces;
            let dependency_graph = &mut self._dependency_graph;
            incomplete.retain(|(mod_idx, load_inter, run_inter)| {
                let mut retain = false;

                // Iterate remaining load dependencies.
                load_inter.borrow_mut().retain(|export| {
                    if let Some(interface_idx) = interfaces.get(export) {
                        // Add edge from Load to Init.
                        dependency_graph.update_edge(
                            NodeIndex::new((2 * mod_idx) - 1),
                            NodeIndex::new(2 * interface_idx),
                            (),
                        );
                        false
                    } else {
                        retain = true;
                        true
                    }
                });

                // Iterate remaining load dependencies.
                run_inter.borrow_mut().retain(|export| {
                    if let Some(interface_idx) = interfaces.get(export) {
                        // Add edge from Init to Init.
                        dependency_graph.update_edge(
                            NodeIndex::new(2 * mod_idx),
                            NodeIndex::new(2 * interface_idx),
                            (),
                        );
                        false
                    } else {
                        retain = true;
                        true
                    }
                });

                retain
            });
        }

        // Add edges.
        let mut missing_load_deps = Vec::new();
        let mut missing_init_deps = Vec::new();

        fn update_edges(
            deps: &[InterfaceExport],
            dep_graph: &mut Graph<NodeType, ()>,
            interfaces: &HashMap<InterfaceExport, usize>,
            missing_deps: &mut Vec<InterfaceExport>,
            node: &NodeIndex,
        ) {
            for interface in deps.iter() {
                if let Some(interface_idx) = interfaces.get(interface) {
                    // Add edge from Load to Init.
                    dep_graph.update_edge(*node, NodeIndex::new(2 * interface_idx), ());
                } else {
                    missing_deps.push((*interface).clone());
                }
            }
        }

        // Add load deps.
        update_edges(
            &module_desc.load_deps,
            &mut self._dependency_graph,
            &self._interfaces,
            &mut missing_load_deps,
            &load,
        );

        // Add runtime deps.
        update_edges(
            &module_desc.runtime_deps,
            &mut self._dependency_graph,
            &self._interfaces,
            &mut missing_init_deps,
            &init,
        );

        // Check if the module is complete.
        if !missing_load_deps.is_empty() || !missing_init_deps.is_empty() {
            self._incomplete_nodes.push((
                idx,
                RefCell::new(missing_load_deps),
                RefCell::new(missing_init_deps),
            ));
        }

        Ok(())
    }

    /// Generates a load order for the contained modules.
    pub fn generate_dependency_order(&self) -> Result<DependencyOrder<'_, 'a>, DependencyError> {
        if !self._incomplete_nodes.is_empty() {
            return Err(DependencyError::MissingDependencies);
        }

        // Initialize a counter.
        let mut node_counters = HashMap::new();
        for i in self._dependency_graph.node_indices() {
            let node_type = self._dependency_graph[i];
            let edge_count = self
                ._dependency_graph
                .edges_directed(i, Direction::Outgoing)
                .count();
            node_counters.insert(i, (node_type, edge_count));
        }

        let mut changed = true;
        let mut command_order = Vec::new();

        while changed {
            changed = false;

            if let Some(idx) = node_counters
                .iter()
                .filter_map(|(k, &v)| if v.1 == 0 { Some(*k) } else { None })
                .min_by(|l, r| l.cmp(r))
            {
                let (node_type, _) = node_counters[&idx];
                command_order.push(node_type);
                changed = true;
                node_counters.remove(&idx);

                // Decrease count of all dependent modules.
                for edge in self
                    ._dependency_graph
                    .edges_directed(idx, Direction::Incoming)
                {
                    node_counters.get_mut(&edge.source()).unwrap().1 -= 1;
                }
            } else if !node_counters.is_empty() {
                return Err(DependencyError::CyclicDependencies);
            }
        }

        Ok((&self._modules, command_order))
    }
}

#[cfg(test)]
mod tests {
    use crate::dependency_list::DependencyOrder;
    use crate::{DependencyError, DependencyList, NodeType};
    use emf_core_base_rs::version::Version;
    use fimo_version_rs::new_short;
    use std::path::PathBuf;

    const CORE_VERSION: Version = new_short(0, 1, 0);

    macro_rules! assert_err {
        ($expression:expr, $($pattern:tt)+) => {
            match $expression {
                $($pattern)+ => (),
                ref e => panic!("expected `{}` but got `{:?}`", stringify!($($pattern)+), e),
            }
        }
    }

    fn create_dependency_order<'a, 'b>(
        list: &'a mut DependencyList<'b>,
        modules: &[(PathBuf, &[(&str, Version)])],
    ) -> Result<DependencyOrder<'a, 'b>, DependencyError> {
        for module in modules {
            list.add_module(module.0.as_path(), module.1, None, None)?;
        }

        list.generate_dependency_order()
    }

    #[test]
    fn cyclic_dependencies() {
        let resource_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/tests/dependency_list/cyclic_dependencies");

        let core_module = resource_path.join("core");
        let modules: &[(PathBuf, &[(&str, Version)])] = &[
            (resource_path.join("module_1"), &[("A", new_short(0, 1, 0))]),
            (resource_path.join("module_2"), &[("B", new_short(0, 1, 0))]),
        ];

        let mut dependency_list =
            DependencyList::new(core_module.as_path(), &CORE_VERSION, &[]).unwrap();
        let order = create_dependency_order(&mut dependency_list, modules);
        assert_err!(order, Err(DependencyError::CyclicDependencies));
    }

    #[test]
    fn invalid_module() {
        let resource_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/tests/dependency_list/invalid_module");

        let core_module = resource_path.join("core");
        let modules: &[(PathBuf, &[(&str, Version)])] = &[(resource_path.join("no_module"), &[])];
        let mut dependency_list =
            DependencyList::new(core_module.as_path(), &CORE_VERSION, &[]).unwrap();
        let order = create_dependency_order(&mut dependency_list, modules);
        assert_err!(order, Err(DependencyError::NotAModule(_)));

        let core_module = resource_path.join("core");
        let modules: &[(PathBuf, &[(&str, Version)])] = &[(
            resource_path.join("missing_export"),
            &[("A", new_short(0, 1, 0))],
        )];
        let mut dependency_list =
            DependencyList::new(core_module.as_path(), &CORE_VERSION, &[]).unwrap();
        let order = create_dependency_order(&mut dependency_list, modules);
        assert_err!(order, Err(DependencyError::MissingExport(_, _)));

        let core_module = resource_path.join("core");
        let modules: &[(PathBuf, &[(&str, Version)])] = &[
            (
                resource_path.join("duplicate_interface_1"),
                &[("A", new_short(0, 1, 0))],
            ),
            (
                resource_path.join("duplicate_interface_2"),
                &[("A", new_short(0, 1, 5))],
            ),
        ];
        let mut dependency_list =
            DependencyList::new(core_module.as_path(), &CORE_VERSION, &[]).unwrap();
        let order = create_dependency_order(&mut dependency_list, modules);
        assert_err!(order, Err(DependencyError::DuplicateInterface(_, _, _)));
    }

    #[test]
    fn missing_core() {
        let resource_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/tests/dependency_list/missing_core");

        assert_err!(
            DependencyList::new(
                resource_path.join("invalid_core_1").as_path(),
                &CORE_VERSION,
                &[]
            ),
            Err(DependencyError::InvalidCoreModule(_))
        );
        assert_err!(
            DependencyList::new(
                resource_path.join("invalid_core_2").as_path(),
                &CORE_VERSION,
                &[]
            ),
            Err(DependencyError::InvalidCoreModule(_))
        );
        assert_err!(
            DependencyList::new(
                resource_path.join("missing_interface").as_path(),
                &CORE_VERSION,
                &[]
            ),
            Err(DependencyError::CoreInterfaceNotFound)
        );
        assert_err!(
            DependencyList::new(
                resource_path.join("missing_module").as_path(),
                &CORE_VERSION,
                &[]
            ),
            Err(DependencyError::InvalidCoreModule(_))
        );
    }

    #[test]
    fn missing_dependencies() {
        let resource_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/tests/dependency_list/missing_dependencies");

        let core_module = resource_path.join("core");
        let modules: &[(PathBuf, &[(&str, Version)])] = &[
            (resource_path.join("module_1"), &[("A", new_short(0, 1, 0))]),
            (resource_path.join("module_2"), &[]),
        ];

        let mut dependency_list =
            DependencyList::new(core_module.as_path(), &CORE_VERSION, &[]).unwrap();
        let order = create_dependency_order(&mut dependency_list, modules);
        assert_err!(order, Err(DependencyError::MissingDependencies));
    }

    #[test]
    fn single_dependency() {
        let resource_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/tests/dependency_list/single_dependency");

        let core_module = resource_path.join("core");
        let modules: &[(PathBuf, &[(&str, Version)])] = &[(resource_path.join("module_1"), &[])];

        let mut dependency_list =
            DependencyList::new(core_module.as_path(), &CORE_VERSION, &[]).unwrap();
        let (_, ops) = create_dependency_order(&mut dependency_list, modules).unwrap();
        assert_eq!(
            ops,
            vec![NodeType::Root, NodeType::Load(1), NodeType::Initialize(1)]
        )
    }

    #[test]
    fn multiple_dependencies() {
        let resource_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/tests/dependency_list/multiple_dependencies");

        let core_module = resource_path.join("core");
        let modules: &[(PathBuf, &[(&str, Version)])] = &[
            (resource_path.join("module_1"), &[("A", new_short(0, 1, 0))]),
            (resource_path.join("module_2"), &[("B", new_short(0, 1, 0))]),
            (resource_path.join("module_3"), &[("C", new_short(0, 1, 0))]),
            (resource_path.join("module_4"), &[("D", new_short(0, 1, 0))]),
            (resource_path.join("module_5"), &[("E", new_short(0, 1, 0))]),
            (resource_path.join("module_6"), &[("F", new_short(0, 1, 0))]),
        ];

        let mut dependency_list =
            DependencyList::new(core_module.as_path(), &CORE_VERSION, &[]).unwrap();
        let (_, ops) = create_dependency_order(&mut dependency_list, modules).unwrap();
        assert_eq!(
            ops,
            vec![
                NodeType::Root,
                NodeType::Load(4),
                NodeType::Initialize(4),
                NodeType::Load(3),
                NodeType::Initialize(3),
                NodeType::Load(2),
                NodeType::Initialize(2),
                NodeType::Load(6),
                NodeType::Initialize(6),
                NodeType::Load(1),
                NodeType::Load(5),
                NodeType::Initialize(5),
                NodeType::Initialize(1)
            ]
        )
    }

    #[test]
    fn multiple_dependencies_per_module() {
        let resource_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/tests/dependency_list/multiple_dependencies_per_module");

        let core_module = resource_path.join("core");
        let modules: &[(PathBuf, &[(&str, Version)])] = &[
            (
                resource_path.join("module_1"),
                &[("A", new_short(0, 1, 0)), ("B", new_short(0, 1, 0))],
            ),
            (resource_path.join("module_2"), &[("C", new_short(0, 1, 0))]),
            (resource_path.join("module_3"), &[]),
        ];

        let mut dependency_list =
            DependencyList::new(core_module.as_path(), &CORE_VERSION, &[]).unwrap();
        let (_, ops) = create_dependency_order(&mut dependency_list, modules).unwrap();
        assert_eq!(
            ops,
            vec![
                NodeType::Root,
                NodeType::Load(1),
                NodeType::Initialize(1),
                NodeType::Load(2),
                NodeType::Initialize(2),
                NodeType::Load(3),
                NodeType::Initialize(3)
            ]
        )
    }
}
