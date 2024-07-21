//! Graph data structure.

use core::{
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ptr::NonNull,
};

use crate::{
    bindings,
    error::{to_result_indirect, to_result_indirect_in_place, Error},
    ffi::FFITransferable,
};

/// Index of a node in a [`Graph`].
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeIdx(u64);

/// Index of an edge in a [`Graph`].
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeIdx(u64);

/// Types of external nodes in a [`Graph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExternalsType {
    Source,
    Sink,
}

/// Direction of an edge to a node of a [`Graph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeDirection {
    Inward,
    Outward,
}

/// Helper type for the cloning operation of a [`Graph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdxMapping {
    Node { old: NodeIdx, new: NodeIdx },
    Edge { old: EdgeIdx, new: EdgeIdx },
}

/// Directed Graph.
///
/// A graph is a collection of nodes and edges, represented as an
/// adjacency list. The structure is generic, as it allows some
/// user-defined data with each node/edge. The nodes are referenced
/// by instances of [`NodeIdx`], while the edges are referenced by
/// instances of [`EdgeIdx`]. The node- and edge references are
/// stable within one instance, no matter which operation is performed
/// on it. Operations that create a new graph are allowed to assign
/// new references to the nodes and edges.
#[repr(transparent)]
#[derive(Debug)]
pub struct Graph<N = (), E = ()> {
    ptr: NonNull<bindings::FimoGraph>,
    _n: PhantomData<fn() -> N>,
    _e: PhantomData<fn() -> E>,
}

impl<N, E> Graph<N, E> {
    const _NODE_ALIGNMENT: () = assert!(
        core::mem::align_of::<N>() <= crate::allocator::FimoAllocator::DEFAULT_ALIGNMENT,
        "Node alignment is not supported."
    );
    const _EDGE_ALIGNMENT: () = assert!(
        core::mem::align_of::<E>() <= crate::allocator::FimoAllocator::DEFAULT_ALIGNMENT,
        "Edge alignment is not supported."
    );

    /// Constructs a new empty graph.
    pub fn new() -> Result<Self, Error> {
        unsafe extern "C" fn cleanup<T>(ptr: *mut core::ffi::c_void) {
            let ptr: *mut T = ptr.cast();

            // Safety: The cleanup function is only called with a valid pointer.
            unsafe { ptr.drop_in_place() };
        }

        let node_size = core::mem::size_of::<N>();
        let edge_size = core::mem::size_of::<E>();

        let node_cleanup = if node_size == 0 || !core::mem::needs_drop::<N>() {
            None
        } else {
            Some(cleanup::<N> as unsafe extern "C" fn(*mut core::ffi::c_void))
        };

        let edge_cleanup = if edge_size == 0 || !core::mem::needs_drop::<E>() {
            None
        } else {
            Some(cleanup::<E> as unsafe extern "C" fn(*mut core::ffi::c_void))
        };

        // Safety: The function either sets an error, or initializes the graph.
        let graph = unsafe {
            to_result_indirect_in_place(|error, graph| {
                *error = bindings::fimo_graph_new(
                    node_size,
                    edge_size,
                    node_cleanup,
                    edge_cleanup,
                    graph.as_mut_ptr(),
                );
            })?
        };
        let graph =
            NonNull::new(graph).expect("the construction of the graph did not return an error");

        Ok(Self {
            ptr: graph,
            _n: PhantomData,
            _e: PhantomData,
        })
    }

    /// Returns the number of nodes in the graph.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = Graph::<u32, ()>::new().unwrap();
    ///
    /// let a = graph.add_node(0).unwrap();
    /// let b = graph.add_node(9).unwrap();
    /// let c = graph.add_node(27).unwrap();
    ///
    /// assert_eq!(graph.node_count(), 3);
    /// ```
    pub fn node_count(&self) -> usize {
        // Safety: The graph is initialized.
        unsafe { bindings::fimo_graph_node_count(self.ptr.as_ptr()) }
    }

    /// Returns the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        // Safety: The graph is initialized.
        unsafe { bindings::fimo_graph_edge_count(self.ptr.as_ptr()) }
    }

    /// Adds a new node to the graph.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = Graph::<u32, ()>::new().unwrap();
    ///
    /// let a = graph.add_node(17).unwrap();
    ///
    /// assert_eq!(graph.node_count(), 1);
    /// assert_eq!(graph.node_data(a).unwrap(), &17);
    /// ```
    pub fn add_node(&mut self, data: N) -> Result<NodeIdx, Error> {
        let data = ManuallyDrop::new(data);
        let data_ptr = if core::mem::size_of::<N>() == 0 {
            core::ptr::null()
        } else {
            (&*data as *const N).cast()
        };

        // Safety: The function either sets an error, or initializes the value.
        let node = unsafe {
            to_result_indirect_in_place(|error, node| {
                *error =
                    bindings::fimo_graph_add_node(self.ptr.as_ptr(), data_ptr, node.as_mut_ptr());
            })
        };
        let node = match node {
            Ok(n) => NodeIdx(n),
            Err(e) => {
                drop(ManuallyDrop::into_inner(data));
                return Err(e);
            }
        };

        Ok(node)
    }

    /// Returns the data associated with a node.
    ///
    /// If the node does not exist in the graph, this function
    /// returns an error.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = Graph::<u32, ()>::new().unwrap();
    ///
    /// let a = graph.add_node(0).unwrap();
    /// let b = graph.add_node(9).unwrap();
    /// let c = graph.add_node(27).unwrap();
    ///
    /// assert_eq!(graph.node_data(a).unwrap(), &0);
    /// assert_eq!(graph.node_data(b).unwrap(), &9);
    /// assert_eq!(graph.node_data(c).unwrap(), &27);
    /// ```
    pub fn node_data(&self, node: NodeIdx) -> Result<&N, Error> {
        // Safety: The function either sets an error, or initializes the value.
        let data = unsafe {
            to_result_indirect_in_place(|error, data| {
                *error =
                    bindings::fimo_graph_node_data(self.ptr.as_ptr(), node.0, data.as_mut_ptr());
            })?
        };

        let data = if core::mem::size_of::<N>() == 0 {
            // Safety: `NonNull::dangling` returns a valid pointer for ZST.
            unsafe { &*NonNull::<N>::dangling().as_ptr() }
        } else {
            // Safety: The returned pointer points to a valid `N` instance.
            unsafe { &*data.cast::<N>() }
        };

        Ok(data)
    }

    /// Returns the data associated with a node.
    ///
    /// If the node does not exist in the graph, this function
    /// returns an error.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = Graph::<u32, ()>::new().unwrap();
    ///
    /// let a = graph.add_node(0).unwrap();
    /// let b = graph.add_node(9).unwrap();
    /// let c = graph.add_node(27).unwrap();
    ///
    /// *graph.node_data_mut(a).unwrap() = 2;
    ///
    /// assert_eq!(graph.node_data_mut(a).unwrap(), &2);
    /// assert_eq!(graph.node_data_mut(b).unwrap(), &9);
    /// assert_eq!(graph.node_data_mut(c).unwrap(), &27);
    /// ```
    pub fn node_data_mut(&mut self, node: NodeIdx) -> Result<&mut N, Error> {
        // Safety: The function either sets an error, or initializes the value.
        let data = unsafe {
            to_result_indirect_in_place(|error, data| {
                *error =
                    bindings::fimo_graph_node_data(self.ptr.as_ptr(), node.0, data.as_mut_ptr());
            })?
        };

        let data = if core::mem::size_of::<N>() == 0 {
            // Safety: `NonNull::dangling` returns a valid pointer for ZST.
            unsafe { &mut *NonNull::<N>::dangling().as_ptr() }
        } else {
            // Safety: The returned pointer points to a valid `N` instance.
            unsafe { &mut *data.cast_mut().cast::<N>() }
        };

        Ok(data)
    }

    /// Adds an edge from `src` to `dst`.
    ///
    /// Inserts an edge from `src` to `dst` into the graph, and
    /// associates it with `data`. This function allows for an
    /// edge to be overwritten. In that case, it returns the old
    /// edge index and simply overwrites the associated data.
    /// See [`Self::add_edge_swap`] for a variant of the same
    /// function, which also returns the previously associated
    /// data.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = Graph::<(), u32>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    /// let c = graph.add_node(()).unwrap();
    ///
    /// let e1 = graph.add_edge(a, c, 2).unwrap();
    /// let e2 = graph.add_edge(b, c, 9).unwrap();
    /// let e3 = graph.add_edge(b, c, 11).unwrap();
    ///
    /// assert_eq!(graph.edge_count(), 2);
    /// ```
    pub fn add_edge(&mut self, src: NodeIdx, dst: NodeIdx, data: E) -> Result<EdgeIdx, Error> {
        self.add_edge_swap(src, dst, data).map(|(e, _)| e)
    }

    /// Adds an edge from `src` to `dst`.
    ///
    /// Inserts an edge from `src` to `dst` into the graph, and
    /// associates it with `data`. This function allows for an
    /// edge to be overwritten. In that case, it returns the old
    /// edge index and simply overwrites the associated data.
    /// Unlike [`Self::add_edge`], this function also returns
    /// the previously associated data of the edge.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = Graph::<(), u32>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    /// let c = graph.add_node(()).unwrap();
    ///
    /// let (e1, old_e1) = graph.add_edge_swap(a, c, 2).unwrap();
    /// let (e2, old_e2) = graph.add_edge_swap(b, c, 9).unwrap();
    /// let (e3, old_e3) = graph.add_edge_swap(b, c, 11).unwrap();
    ///
    /// assert_eq!(graph.edge_count(), 2);
    /// assert!(old_e1.is_none());
    /// assert!(old_e2.is_none());
    /// assert_eq!(old_e3.unwrap(), 9);
    /// ```
    pub fn add_edge_swap(
        &mut self,
        src: NodeIdx,
        dst: NodeIdx,
        data: E,
    ) -> Result<(EdgeIdx, Option<E>), Error> {
        // The graph returns `null` for ZST, so we add an additional
        // check to confirm whether an edge already exists.
        let is_update = if core::mem::size_of::<E>() == 0 {
            self.contains_edge(src, dst)
        } else {
            true
        };

        let data = ManuallyDrop::new(data);
        let data_ptr = if core::mem::size_of::<E>() == 0 {
            core::ptr::null()
        } else {
            (&*data as *const E).cast()
        };
        let mut old_data_ptr: *mut E = core::ptr::null_mut();

        // Safety: The function either sets an error, or initializes the value.
        let edge = unsafe {
            to_result_indirect_in_place(|error, edge| {
                *error = bindings::fimo_graph_add_edge(
                    self.ptr.as_ptr(),
                    src.0,
                    dst.0,
                    data_ptr,
                    (&mut old_data_ptr as *mut *mut E).cast(),
                    edge.as_mut_ptr(),
                );
            })
        };
        let edge = match edge {
            Ok(n) => EdgeIdx(n),
            Err(e) => {
                drop(ManuallyDrop::into_inner(data));
                return Err(e);
            }
        };

        let old_data = if !old_data_ptr.is_null() {
            debug_assert_ne!(old_data_ptr, core::ptr::null_mut());
            // Safety: The returned pointer must be valid.
            let old = unsafe { Some(old_data_ptr.read()) };
            // Safety: We own the pointer and it was allocated with the given size,
            // by the given allocator.
            unsafe { bindings::fimo_free_sized(old_data_ptr.cast(), core::mem::size_of::<E>()) };
            old
        } else if core::mem::size_of::<E>() == 0 && is_update {
            debug_assert_ne!(old_data_ptr, core::ptr::null_mut());
            // Safety: This is sound, as `E` is a ZST.
            #[allow(clippy::uninit_assumed_init)]
            unsafe {
                Some(MaybeUninit::<E>::uninit().assume_init())
            }
        } else {
            None
        };

        Ok((edge, old_data))
    }

    /// Updates the data associated with the edge from `src` to `dst`.
    ///
    /// This function updates the data associated with the edge from
    /// `src` to `dst`, and returns the previously associated data.
    /// If the edge does not exist, this function returns an error.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = Graph::<(), u32>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    /// let c = graph.add_node(()).unwrap();
    ///
    /// let e1 = graph.add_edge(a, c, 2).unwrap();
    /// let e2 = graph.add_edge(b, c, 9).unwrap();
    ///
    /// let (e3, old_e3) = graph.update_edge(b, c, 10).unwrap();
    /// assert!(graph.update_edge(a, b, 1).is_err());
    ///
    /// assert_eq!(graph.edge_count(), 2);
    /// assert_eq!(old_e3, 9);
    /// assert_eq!(e2, e3);
    /// ```
    pub fn update_edge(
        &mut self,
        src: NodeIdx,
        dst: NodeIdx,
        data: E,
    ) -> Result<(EdgeIdx, E), Error> {
        let data = ManuallyDrop::new(data);
        let data_ptr = if core::mem::size_of::<E>() == 0 {
            core::ptr::null()
        } else {
            (&*data as *const E).cast()
        };
        let mut old_data_ptr: *mut E = core::ptr::null_mut();

        // Safety: The function either sets an error, or initializes the value.
        let edge = unsafe {
            to_result_indirect_in_place(|error, edge| {
                *error = bindings::fimo_graph_update_edge(
                    self.ptr.as_ptr(),
                    src.0,
                    dst.0,
                    data_ptr,
                    (&mut old_data_ptr as *mut *mut E).cast(),
                    edge.as_mut_ptr(),
                );
            })
        };
        let edge = match edge {
            Ok(n) => EdgeIdx(n),
            Err(e) => {
                drop(ManuallyDrop::into_inner(data));
                return Err(e);
            }
        };

        let old_data = if core::mem::size_of::<E>() == 0 {
            debug_assert_eq!(old_data_ptr, core::ptr::null_mut());
            // Safety: This is generally sound for ZST, if they are
            // noy uninhabited. We know that `E` can not be an
            // uninhabited type at this point, as `update_edge`
            // only succeeds when there already was an edge, which
            // requires an instance of `E` to begin with.
            #[allow(clippy::uninit_assumed_init)]
            unsafe {
                MaybeUninit::<E>::uninit().assume_init()
            }
        } else {
            debug_assert_ne!(old_data_ptr, core::ptr::null_mut());
            // Safety: The returned pointer must be valid.
            let old_data = unsafe { old_data_ptr.read() };
            // Safety: We own the pointer and it was allocated with the given size,
            // by the given allocator.
            unsafe { bindings::fimo_free_sized(old_data_ptr.cast(), core::mem::size_of::<E>()) };
            old_data
        };

        Ok((edge, old_data))
    }

    /// Returns the data associated with a node.
    ///
    /// If the edge does not exist in the graph, this function
    /// returns an error.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = Graph::<(), u32>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    /// let c = graph.add_node(()).unwrap();
    ///
    /// let e1 = graph.add_edge(a, c, 2).unwrap();
    /// let e2 = graph.add_edge(b, c, 9).unwrap();
    ///
    /// assert_eq!(graph.edge_data(e1).unwrap(), &2);
    /// assert_eq!(graph.edge_data(e2).unwrap(), &9);
    /// ```
    pub fn edge_data(&self, edge: EdgeIdx) -> Result<&E, Error> {
        // Safety: The function either sets an error, or initializes the value.
        let data = unsafe {
            to_result_indirect_in_place(|error, data| {
                *error =
                    bindings::fimo_graph_edge_data(self.ptr.as_ptr(), edge.0, data.as_mut_ptr());
            })?
        };

        let data = if core::mem::size_of::<E>() == 0 {
            // Safety: `NonNull::dangling` returns a valid pointer for ZST.
            unsafe { &*NonNull::<E>::dangling().as_ptr() }
        } else {
            // Safety: The returned pointer points to a valid `N` instance.
            unsafe { &*data.cast::<E>() }
        };

        Ok(data)
    }

    /// Returns the data associated with an edge.
    ///
    /// If the edge does not exist in the graph, this function
    /// returns an error.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = Graph::<(), u32>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    /// let c = graph.add_node(()).unwrap();
    ///
    /// let e1 = graph.add_edge(a, c, 2).unwrap();
    /// let e2 = graph.add_edge(b, c, 9).unwrap();
    ///
    /// *graph.edge_data_mut(e1).unwrap() = 5;
    ///
    /// assert_eq!(graph.edge_data_mut(e1).unwrap(), &5);
    /// assert_eq!(graph.edge_data_mut(e2).unwrap(), &9);
    /// ```
    pub fn edge_data_mut(&mut self, edge: EdgeIdx) -> Result<&mut E, Error> {
        // Safety: The function either sets an error, or initializes the value.
        let data = unsafe {
            to_result_indirect_in_place(|error, data| {
                *error =
                    bindings::fimo_graph_edge_data(self.ptr.as_ptr(), edge.0, data.as_mut_ptr());
            })?
        };

        let data = if core::mem::size_of::<E>() == 0 {
            // Safety: `NonNull::dangling` returns a valid pointer for ZST.
            unsafe { &mut *NonNull::<E>::dangling().as_ptr() }
        } else {
            // Safety: The returned pointer points to a valid `N` instance.
            unsafe { &mut *data.cast_mut().cast::<E>() }
        };

        Ok(data)
    }

    /// Returns the node endpoints of an edge.
    ///
    /// Returns the `(src, dst)` endpoint tuple for the edge `edge`.
    /// If the edge does not exist, this function returns an error.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = <Graph>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    /// let c = graph.add_node(()).unwrap();
    ///
    /// let e1 = graph.add_edge(a, c, ()).unwrap();
    /// let e2 = graph.add_edge(b, c, ()).unwrap();
    ///
    /// let (e1_src, e1_dst) = graph.edge_endpoints(e1).unwrap();
    /// let (e2_src, e2_dst) = graph.edge_endpoints(e2).unwrap();
    ///
    /// assert_eq!(e1_src, a);
    /// assert_eq!(e1_dst, c);
    /// assert_eq!(e2_src, b);
    /// assert_eq!(e2_dst, c);
    /// ```
    pub fn edge_endpoints(&self, edge: EdgeIdx) -> Result<(NodeIdx, NodeIdx), Error> {
        let mut src = NodeIdx(0);
        let mut dst = NodeIdx(0);

        to_result_indirect(|error| {
            // Safety: All pointers are valid.
            *error = unsafe {
                bindings::fimo_graph_edge_endpoints(
                    self.ptr.as_ptr(),
                    edge.0,
                    &mut src.0,
                    &mut dst.0,
                )
            };
        })?;

        Ok((src, dst))
    }

    /// Removes a node from the graph.
    ///
    /// Removes a node and all its incoming and outgoing edges from
    /// the graph, and returns the data previously associated with
    /// the node. If the node does not exist, the function returns
    /// an error.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = Graph::<u32, ()>::new().unwrap();
    ///
    /// let a = graph.add_node(0).unwrap();
    /// let b = graph.add_node(1).unwrap();
    /// let c = graph.add_node(2).unwrap();
    /// let d = graph.add_node(3).unwrap();
    ///
    /// let e1 = graph.add_edge(a, c, ()).unwrap();
    /// let e2 = graph.add_edge(b, c, ()).unwrap();
    /// let e3 = graph.add_edge(c, d, ()).unwrap();
    ///
    /// assert_eq!(graph.node_count(), 4);
    /// assert_eq!(graph.edge_count(), 3);
    ///
    /// assert_eq!(graph.remove_node(c).unwrap(), 2);
    /// assert_eq!(graph.node_count(), 3);
    /// assert_eq!(graph.edge_count(), 0);
    /// ```
    pub fn remove_node(&mut self, node: NodeIdx) -> Result<N, Error> {
        // Safety: The function either initializes the data or returns an error.
        let old_data_ptr = unsafe {
            to_result_indirect_in_place(|error, data| {
                *error =
                    bindings::fimo_graph_remove_node(self.ptr.as_ptr(), node.0, data.as_mut_ptr());
            })?
        };
        let old_data_ptr: *mut N = old_data_ptr.cast();

        let old_data = if core::mem::size_of::<N>() == 0 {
            debug_assert!(old_data_ptr.is_null());
            // Safety: See above.
            #[allow(clippy::uninit_assumed_init)]
            unsafe {
                MaybeUninit::<N>::uninit().assume_init()
            }
        } else {
            debug_assert!(!old_data_ptr.is_null());
            // Safety: The pointer is valid.
            let old = unsafe { old_data_ptr.read() };
            // Safety: We own the data.
            unsafe { bindings::fimo_free_sized(old_data_ptr.cast(), core::mem::size_of::<N>()) };
            old
        };

        Ok(old_data)
    }

    /// Removes an edge from the graph.
    ///
    /// Removes an from the graph, and returns the data previously
    /// associated with the edge. If the edge does not exist, the
    /// function returns an error.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = Graph::<(), u32>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    ///
    /// let e1 = graph.add_edge(a, b, 1).unwrap();
    /// let e2 = graph.add_edge(b, a, 2).unwrap();
    ///
    /// assert_eq!(graph.edge_count(), 2);
    /// assert!(graph.contains_edge(a, b));
    /// assert!(graph.contains_edge(b, a));
    ///
    /// assert_eq!(graph.remove_edge(e2).unwrap(), 2);
    /// assert!(graph.contains_edge(a, b));
    /// assert!(!graph.contains_edge(b, a));
    /// ```
    pub fn remove_edge(&mut self, edge: EdgeIdx) -> Result<E, Error> {
        // Safety: The function either initializes the data or returns an error.
        let old_data_ptr = unsafe {
            to_result_indirect_in_place(|error, data| {
                *error =
                    bindings::fimo_graph_remove_edge(self.ptr.as_ptr(), edge.0, data.as_mut_ptr());
            })?
        };
        let old_data_ptr: *mut E = old_data_ptr.cast();

        let old_data = if core::mem::size_of::<E>() == 0 {
            debug_assert!(old_data_ptr.is_null());
            // Safety: See above.
            #[allow(clippy::uninit_assumed_init)]
            unsafe {
                MaybeUninit::<E>::uninit().assume_init()
            }
        } else {
            debug_assert!(!old_data_ptr.is_null());
            // Safety: The pointer is valid.
            let old = unsafe { old_data_ptr.read() };
            // Safety: We own the data.
            unsafe { bindings::fimo_free_sized(old_data_ptr.cast(), core::mem::size_of::<E>()) };
            old
        };

        Ok(old_data)
    }

    /// Checks whether an edge is contained in the graph.
    ///
    /// Checks whether the edge from `src` to `dst` is contained in
    /// the graph.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = <Graph>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    ///
    /// let e1 = graph.add_edge(a, b, ()).unwrap();
    ///
    /// assert!(graph.contains_edge(a, b));
    /// assert!(!graph.contains_edge(b, a));
    /// ```
    pub fn contains_edge(&self, src: NodeIdx, dst: NodeIdx) -> bool {
        // Safety: The function either initializes the value or returns an error.
        unsafe {
            to_result_indirect_in_place(|error, contained| {
                *error = bindings::fimo_graph_contains_edge(
                    self.ptr.as_ptr(),
                    src.0,
                    dst.0,
                    contained.as_mut_ptr(),
                );
            })
            .unwrap_or_default()
        }
    }

    /// Finds the edge index for the edge from `src` to `dst`.
    ///
    /// Returns the index for the edge going from `src` to `dst`,
    /// if it exists. Otherwise, returns `None`.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = <Graph>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    ///
    /// let e1 = graph.add_edge(a, b, ()).unwrap();
    ///
    /// assert_eq!(graph.find_edge(a, b).unwrap(), e1);
    /// assert!(graph.find_edge(b, a).is_none());
    /// ```
    pub fn find_edge(&self, src: NodeIdx, dst: NodeIdx) -> Option<EdgeIdx> {
        let mut edge = EdgeIdx(0);

        // Safety: The function either initializes the value or returns an error.
        let contained = unsafe {
            to_result_indirect_in_place(|error, contained| {
                *error = bindings::fimo_graph_find_edge(
                    self.ptr.as_ptr(),
                    src.0,
                    dst.0,
                    &mut edge.0,
                    contained.as_mut_ptr(),
                );
            })
            .ok()?
        };

        if contained {
            Some(edge)
        } else {
            None
        }
    }

    /// Constructs an iterator over the nodes of the graph.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = Graph::<u32, ()>::new().unwrap();
    ///
    /// let a = graph.add_node(0).unwrap();
    /// let b = graph.add_node(1).unwrap();
    /// let c = graph.add_node(2).unwrap();
    /// let d = graph.add_node(3).unwrap();
    ///
    /// assert_eq!(graph.nodes().unwrap().count(), 4);
    /// ```
    pub fn nodes(&self) -> Result<Nodes<'_, N, E>, Error> {
        let mut has_next = false;

        // Safety: The function either initializes the value or returns an error.
        let iter = unsafe {
            to_result_indirect_in_place(|error, iter| {
                *error = bindings::fimo_graph_nodes_new(
                    self.ptr.as_ptr(),
                    iter.as_mut_ptr(),
                    &mut has_next,
                );
            })?
        };
        let iter = NonNull::new(iter).expect("the pointer should not be null");

        let inner = NodesInner {
            has_next,
            ptr: iter,
        };
        let nodes = Nodes {
            inner,
            _phantom: PhantomData,
        };
        Ok(nodes)
    }

    /// Constructs an iterator over the nodes of the graph.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = Graph::<u32, ()>::new().unwrap();
    ///
    /// let a = graph.add_node(0).unwrap();
    /// let b = graph.add_node(1).unwrap();
    /// let c = graph.add_node(2).unwrap();
    /// let d = graph.add_node(3).unwrap();
    ///
    /// for (_, data) in graph.nodes_mut().unwrap() {
    ///     *data *= 2;
    /// }
    ///
    /// assert_eq!(graph.node_data(a).unwrap(), &0);
    /// assert_eq!(graph.node_data(b).unwrap(), &2);
    /// assert_eq!(graph.node_data(c).unwrap(), &4);
    /// assert_eq!(graph.node_data(d).unwrap(), &6);
    /// ```
    pub fn nodes_mut(&mut self) -> Result<NodesMut<'_, N, E>, Error> {
        let mut has_next = false;

        // Safety: The function either initializes the value or returns an error.
        let iter = unsafe {
            to_result_indirect_in_place(|error, iter| {
                *error = bindings::fimo_graph_nodes_new(
                    self.ptr.as_ptr(),
                    iter.as_mut_ptr(),
                    &mut has_next,
                );
            })?
        };
        let iter = NonNull::new(iter).expect("the pointer should not be null");

        let inner = NodesInner {
            has_next,
            ptr: iter,
        };
        let nodes = NodesMut {
            inner,
            _phantom: PhantomData,
        };
        Ok(nodes)
    }

    /// Constructs an iterator over the edges of a graph.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = Graph::<(), u32>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    /// let c = graph.add_node(()).unwrap();
    /// let d = graph.add_node(()).unwrap();
    ///
    /// let e1 = graph.add_edge(a, c, 1).unwrap();
    /// let e2 = graph.add_edge(b, c, 2).unwrap();
    /// let e3 = graph.add_edge(c, d, 3).unwrap();
    ///
    /// assert_eq!(graph.edges().unwrap().count(), 3);
    /// ```
    pub fn edges(&self) -> Result<Edges<'_, N, E>, Error> {
        let mut has_next = false;

        // Safety: The function either initializes the value or returns an error.
        let iter = unsafe {
            to_result_indirect_in_place(|error, iter| {
                *error = bindings::fimo_graph_edges_new(
                    self.ptr.as_ptr(),
                    iter.as_mut_ptr(),
                    &mut has_next,
                );
            })?
        };
        let iter = NonNull::new(iter).expect("the pointer should not be null");

        let inner = EdgesInner {
            has_next,
            ptr: iter,
        };
        let edges = Edges {
            inner,
            _phantom: PhantomData,
        };
        Ok(edges)
    }

    /// Constructs an iterator over the edges of a graph.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = Graph::<(), u32>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    /// let c = graph.add_node(()).unwrap();
    /// let d = graph.add_node(()).unwrap();
    ///
    /// let e1 = graph.add_edge(a, c, 1).unwrap();
    /// let e2 = graph.add_edge(b, c, 2).unwrap();
    /// let e3 = graph.add_edge(c, d, 3).unwrap();
    ///
    /// for (_, data) in graph.edges_mut().unwrap() {
    ///     *data *= 2;
    /// }
    ///
    /// assert_eq!(graph.edge_data(e1).unwrap(), &2);
    /// assert_eq!(graph.edge_data(e2).unwrap(), &4);
    /// assert_eq!(graph.edge_data(e3).unwrap(), &6);
    /// ```
    pub fn edges_mut(&mut self) -> Result<EdgesMut<'_, N, E>, Error> {
        let mut has_next = false;

        // Safety: The function either initializes the value or returns an error.
        let iter = unsafe {
            to_result_indirect_in_place(|error, iter| {
                *error = bindings::fimo_graph_edges_new(
                    self.ptr.as_ptr(),
                    iter.as_mut_ptr(),
                    &mut has_next,
                );
            })?
        };
        let iter = NonNull::new(iter).expect("the pointer should not be null");

        let inner = EdgesInner {
            has_next,
            ptr: iter,
        };
        let edges = EdgesMut {
            inner,
            _phantom: PhantomData,
        };
        Ok(edges)
    }

    /// Constructs an iterator over the externals of a graph.
    ///
    /// The externals are defines as nodes, that either have
    /// no inward edges ([`ExternalsType::Source`]), or have
    /// no outward edges ([`ExternalsType::Sink`]). The caller
    /// can specify over which nodes to iterate by setting
    /// `externals_type` to the desired type of externals.
    ///
    /// ```
    /// use fimo_std::graph::{ExternalsType, Graph};
    ///
    /// let mut graph = Graph::<u32, ()>::new().unwrap();
    ///
    /// let a = graph.add_node(1).unwrap();
    /// let b = graph.add_node(2).unwrap();
    /// let c = graph.add_node(3).unwrap();
    /// let d = graph.add_node(4).unwrap();
    ///
    /// let e1 = graph.add_edge(a, c, ()).unwrap();
    /// let e2 = graph.add_edge(b, c, ()).unwrap();
    /// let e3 = graph.add_edge(c, d, ()).unwrap();
    ///
    /// assert_eq!(graph.externals(ExternalsType::Source).unwrap().count(), 2);
    /// assert_eq!(graph.externals(ExternalsType::Sink).unwrap().count(), 1);
    /// ```
    pub fn externals(&self, externals_type: ExternalsType) -> Result<Externals<'_, N, E>, Error> {
        let sink = matches!(externals_type, ExternalsType::Sink);
        let mut has_next = false;

        // Safety: The function either initializes the value or returns an error.
        let iter = unsafe {
            to_result_indirect_in_place(|error, iter| {
                *error = bindings::fimo_graph_externals_new(
                    self.ptr.as_ptr(),
                    sink,
                    iter.as_mut_ptr(),
                    &mut has_next,
                );
            })?
        };
        let iter = NonNull::new(iter).expect("the pointer should not be null");

        let inner = ExternalsInner {
            has_next,
            ptr: iter,
        };
        let externals = Externals {
            inner,
            _phantom: PhantomData,
        };
        Ok(externals)
    }

    /// Constructs an iterator over the externals of a graph.
    ///
    /// The externals are defines as nodes, that either have
    /// no inward edges ([`ExternalsType::Source`]), or have
    /// no outward edges ([`ExternalsType::Sink`]). The caller
    /// can specify over which nodes to iterate by setting
    /// `externals_type` to the desired type of externals.
    ///
    /// ```
    /// use fimo_std::graph::{ExternalsType, Graph};
    ///
    /// let mut graph = Graph::<u32, ()>::new().unwrap();
    ///
    /// let a = graph.add_node(1).unwrap();
    /// let b = graph.add_node(2).unwrap();
    /// let c = graph.add_node(3).unwrap();
    /// let d = graph.add_node(4).unwrap();
    ///
    /// let e1 = graph.add_edge(a, c, ()).unwrap();
    /// let e2 = graph.add_edge(b, c, ()).unwrap();
    /// let e3 = graph.add_edge(c, d, ()).unwrap();
    ///
    /// for (_, data) in graph.externals_mut(ExternalsType::Source).unwrap() {
    ///     *data *= 2;
    /// }
    ///
    /// for (_, data) in graph.externals_mut(ExternalsType::Sink).unwrap() {
    ///     *data *= 3;
    /// }
    ///
    /// assert_eq!(graph.node_data(a).unwrap(), &2);
    /// assert_eq!(graph.node_data(b).unwrap(), &4);
    /// assert_eq!(graph.node_data(c).unwrap(), &3);
    /// assert_eq!(graph.node_data(d).unwrap(), &12);
    /// ```
    pub fn externals_mut(
        &mut self,
        externals_type: ExternalsType,
    ) -> Result<ExternalsMut<'_, N, E>, Error> {
        let sink = matches!(externals_type, ExternalsType::Sink);
        let mut has_next = false;

        // Safety: The function either initializes the value or returns an error.
        let iter = unsafe {
            to_result_indirect_in_place(|error, iter| {
                *error = bindings::fimo_graph_externals_new(
                    self.ptr.as_ptr(),
                    sink,
                    iter.as_mut_ptr(),
                    &mut has_next,
                );
            })?
        };
        let iter = NonNull::new(iter).expect("the pointer should not be null");

        let inner = ExternalsInner {
            has_next,
            ptr: iter,
        };
        let externals = ExternalsMut {
            inner,
            _phantom: PhantomData,
        };
        Ok(externals)
    }

    /// Constructs a new iterator over the neighbors of a node.
    ///
    /// The caller can specify whether to iterate over the
    /// nodes that have an edge with `node` as their start
    /// node ([`EdgeDirection::Outward`]), or the nodes that
    /// have an edge with the node `node` as their end node
    /// ([`EdgeDirection::Inward`]).
    ///
    /// ```
    /// use fimo_std::graph::{EdgeDirection, Graph};
    ///
    /// let mut graph = <Graph>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    /// let c = graph.add_node(()).unwrap();
    /// let d = graph.add_node(()).unwrap();
    ///
    /// let e1 = graph.add_edge(a, c, ()).unwrap();
    /// let e2 = graph.add_edge(b, c, ()).unwrap();
    /// let e3 = graph.add_edge(c, d, ()).unwrap();
    ///
    /// assert_eq!(
    ///     graph.neighbors(a, EdgeDirection::Inward).unwrap().count(),
    ///     0
    /// );
    /// assert_eq!(
    ///     graph.neighbors(a, EdgeDirection::Outward).unwrap().count(),
    ///     1
    /// );
    /// assert_eq!(
    ///     graph.neighbors(c, EdgeDirection::Inward).unwrap().count(),
    ///     2
    /// );
    /// assert_eq!(
    ///     graph.neighbors(c, EdgeDirection::Outward).unwrap().count(),
    ///     1
    /// );
    /// assert_eq!(
    ///     graph.neighbors(d, EdgeDirection::Inward).unwrap().count(),
    ///     1
    /// );
    /// assert_eq!(
    ///     graph.neighbors(d, EdgeDirection::Outward).unwrap().count(),
    ///     0
    /// );
    /// ```
    pub fn neighbors(
        &self,
        node: NodeIdx,
        edge_direction: EdgeDirection,
    ) -> Result<Neighbors<'_, N, E>, Error> {
        let inward = matches!(edge_direction, EdgeDirection::Inward);
        let mut has_next = false;

        // Safety: The function either initializes the value or returns an error.
        let iter = unsafe {
            to_result_indirect_in_place(|error, iter| {
                *error = bindings::fimo_graph_neighbors_new(
                    self.ptr.as_ptr(),
                    node.0,
                    inward,
                    iter.as_mut_ptr(),
                    &mut has_next,
                );
            })?
        };
        let iter = NonNull::new(iter).expect("the pointer should not be null");

        let neighbors = Neighbors {
            has_next,
            ptr: iter,
            _phantom: PhantomData,
        };
        Ok(neighbors)
    }

    /// Constructs a new iterator over the edges connecting
    /// a node with its neighbors.
    ///
    /// The caller can specify whether to iterate over the
    /// edges that have the node `node` as their start node
    /// ([`EdgeDirection::Outward`]), or the edges that have
    /// the node `node` as their end node ([`EdgeDirection::Inward`]).
    ///
    /// ```
    /// use fimo_std::graph::{EdgeDirection, Graph};
    ///
    /// let mut graph = <Graph>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    /// let c = graph.add_node(()).unwrap();
    /// let d = graph.add_node(()).unwrap();
    ///
    /// let e1 = graph.add_edge(a, c, ()).unwrap();
    /// let e2 = graph.add_edge(b, c, ()).unwrap();
    /// let e3 = graph.add_edge(c, d, ()).unwrap();
    ///
    /// assert_eq!(
    ///     graph
    ///         .neighbors_edges(a, EdgeDirection::Inward)
    ///         .unwrap()
    ///         .count(),
    ///     0
    /// );
    /// assert_eq!(
    ///     graph
    ///         .neighbors_edges(a, EdgeDirection::Outward)
    ///         .unwrap()
    ///         .count(),
    ///     1
    /// );
    /// assert_eq!(
    ///     graph
    ///         .neighbors_edges(c, EdgeDirection::Inward)
    ///         .unwrap()
    ///         .count(),
    ///     2
    /// );
    /// assert_eq!(
    ///     graph
    ///         .neighbors_edges(c, EdgeDirection::Outward)
    ///         .unwrap()
    ///         .count(),
    ///     1
    /// );
    /// assert_eq!(
    ///     graph
    ///         .neighbors_edges(d, EdgeDirection::Inward)
    ///         .unwrap()
    ///         .count(),
    ///     1
    /// );
    /// assert_eq!(
    ///     graph
    ///         .neighbors_edges(d, EdgeDirection::Outward)
    ///         .unwrap()
    ///         .count(),
    ///     0
    /// );
    /// ```
    pub fn neighbors_edges(
        &self,
        node: NodeIdx,
        edge_direction: EdgeDirection,
    ) -> Result<NeighborsEdges<'_, N, E>, Error> {
        let inward = matches!(edge_direction, EdgeDirection::Inward);
        let mut has_next = false;

        // Safety: The function either initializes the value or returns an error.
        let iter = unsafe {
            to_result_indirect_in_place(|error, iter| {
                *error = bindings::fimo_graph_neighbors_edges_new(
                    self.ptr.as_ptr(),
                    node.0,
                    inward,
                    iter.as_mut_ptr(),
                    &mut has_next,
                );
            })?
        };
        let iter = NonNull::new(iter).expect("the pointer should not be null");

        let neighbors = NeighborsEdges {
            has_next,
            ptr: iter,
            _phantom: PhantomData,
        };
        Ok(neighbors)
    }

    /// Clears all nodes and edges from the graph.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = <Graph>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    /// let c = graph.add_node(()).unwrap();
    /// let d = graph.add_node(()).unwrap();
    ///
    /// let e1 = graph.add_edge(a, c, ()).unwrap();
    /// let e2 = graph.add_edge(b, c, ()).unwrap();
    /// let e3 = graph.add_edge(c, d, ()).unwrap();
    ///
    /// assert_eq!(graph.node_count(), 4);
    /// assert_eq!(graph.edge_count(), 3);
    ///
    /// graph.clear().unwrap();
    ///
    /// assert_eq!(graph.node_count(), 0);
    /// assert_eq!(graph.edge_count(), 0);
    /// ```
    pub fn clear(&mut self) -> crate::error::Result {
        to_result_indirect(|error| {
            // Safety: The graph pointer is valid.
            *error = unsafe { bindings::fimo_graph_clear(self.ptr.as_ptr()) };
        })
    }

    /// Clears all edges from the graph.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = <Graph>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    /// let c = graph.add_node(()).unwrap();
    /// let d = graph.add_node(()).unwrap();
    ///
    /// let e1 = graph.add_edge(a, c, ()).unwrap();
    /// let e2 = graph.add_edge(b, c, ()).unwrap();
    /// let e3 = graph.add_edge(c, d, ()).unwrap();
    ///
    /// assert_eq!(graph.node_count(), 4);
    /// assert_eq!(graph.edge_count(), 3);
    ///
    /// graph.clear_edges().unwrap();
    ///
    /// assert_eq!(graph.node_count(), 4);
    /// assert_eq!(graph.edge_count(), 0);
    /// ```
    pub fn clear_edges(&mut self) -> crate::error::Result {
        to_result_indirect(|error| {
            // Safety: The graph pointer is valid.
            *error = unsafe { bindings::fimo_graph_clear_edges(self.ptr.as_ptr()) };
        })
    }

    /// Reverses the direction of all edges.
    ///
    /// ```
    /// use fimo_std::graph::Graph;
    ///
    /// let mut graph = <Graph>::new().unwrap();
    ///
    /// let a = graph.add_node(()).unwrap();
    /// let b = graph.add_node(()).unwrap();
    /// let c = graph.add_node(()).unwrap();
    ///
    /// let e1 = graph.add_edge(a, b, ()).unwrap();
    /// let e2 = graph.add_edge(c, b, ()).unwrap();
    ///
    /// assert_eq!(graph.edge_endpoints(e1).unwrap(), (a, b));
    /// assert_eq!(graph.edge_endpoints(e2).unwrap(), (c, b));
    ///
    /// graph.reverse().unwrap();
    ///
    /// assert_eq!(graph.edge_endpoints(e1).unwrap(), (b, a));
    /// assert_eq!(graph.edge_endpoints(e2).unwrap(), (b, c));
    /// ```
    pub fn reverse(&mut self) -> crate::error::Result {
        to_result_indirect(|error| {
            // Safety: The graph pointer is valid.
            *error = unsafe { bindings::fimo_graph_reverse(self.ptr.as_ptr()) };
        })
    }

    /// Clones the graph.
    ///
    /// The cloning operation does not guarantee that the
    /// node and edge references remain valid for the clone.
    /// Therefore, the caller may provide a custom mapping
    /// function that allows them to map the old reference to
    /// the new reference.
    ///
    /// ```
    /// use fimo_std::graph::{Graph, IdxMapping};
    ///
    /// let mut graph = Graph::<u32, u32>::new().unwrap();
    ///
    /// let a = graph.add_node(1).unwrap();
    /// let b = graph.add_node(2).unwrap();
    /// let c = graph.add_node(3).unwrap();
    ///
    /// let e1 = graph.add_edge(a, b, 10).unwrap();
    /// let e2 = graph.add_edge(c, b, 20).unwrap();
    ///
    /// let mut new_a = None;
    /// let mut new_b = None;
    /// let mut new_c = None;
    ///
    /// let mut new_e1 = None;
    /// let mut new_e2 = None;
    ///
    /// let mapper = |mapping| {
    ///     match mapping {
    ///         IdxMapping::Node { old, new } => match old {
    ///             x if x == a => new_a = Some(new),
    ///             x if x == b => new_b = Some(new),
    ///             x if x == c => new_c = Some(new),
    ///             _ => unreachable!(),
    ///         },
    ///         IdxMapping::Edge { old, new } => match old {
    ///             x if x == e1 => new_e1 = Some(new),
    ///             x if x == e2 => new_e2 = Some(new),
    ///             _ => unreachable!(),
    ///         },
    ///     }
    ///     Ok(())
    /// };
    ///
    /// let clone = graph.clone_with_mapper(Some(mapper)).unwrap();
    ///
    /// let new_a = new_a.unwrap();
    /// let new_b = new_b.unwrap();
    /// let new_c = new_c.unwrap();
    ///
    /// let new_e1 = new_e1.unwrap();
    /// let new_e2 = new_e2.unwrap();
    ///
    /// assert_eq!(clone.node_data(new_a).unwrap(), &1);
    /// assert_eq!(clone.node_data(new_b).unwrap(), &2);
    /// assert_eq!(clone.node_data(new_c).unwrap(), &3);
    ///
    /// assert_eq!(clone.edge_data(new_e1).unwrap(), &10);
    /// assert_eq!(clone.edge_data(new_e2).unwrap(), &20);
    /// ```
    pub fn clone_with_mapper<T>(&self, mut mapper: Option<T>) -> Result<Self, Error>
    where
        N: Copy,
        E: Copy,
        T: FnMut(IdxMapping) -> crate::error::Result,
    {
        unsafe extern "C" fn node_mapper<T>(
            old: u64,
            new: u64,
            ptr: *mut core::ffi::c_void,
        ) -> bindings::FimoError
        where
            T: FnMut(IdxMapping) -> crate::error::Result,
        {
            let old = NodeIdx(old);
            let new = NodeIdx(new);

            // Safety: The function is only called with valid pointers.
            let mapper = unsafe { &mut *ptr.cast::<T>() };
            (mapper)(IdxMapping::Node { old, new })
                .err()
                .unwrap_or(Error::EOK)
                .into_error()
        }

        unsafe extern "C" fn edge_mapper<T>(
            old: u64,
            new: u64,
            ptr: *mut core::ffi::c_void,
        ) -> bindings::FimoError
        where
            T: FnMut(IdxMapping) -> crate::error::Result,
        {
            let old = EdgeIdx(old);
            let new = EdgeIdx(new);

            // Safety: The function is only called with valid pointers.
            let mapper = unsafe { &mut *ptr.cast::<T>() };
            (mapper)(IdxMapping::Edge { old, new })
                .err()
                .unwrap_or(Error::EOK)
                .into_error()
        }

        let (node, edge, data) = if let Some(mapper) = mapper.as_mut() {
            let node = Some(
                node_mapper::<T>
                    as unsafe extern "C" fn(
                        u64,
                        u64,
                        *mut core::ffi::c_void,
                    ) -> bindings::FimoError,
            );
            let edge = Some(
                edge_mapper::<T>
                    as unsafe extern "C" fn(
                        u64,
                        u64,
                        *mut core::ffi::c_void,
                    ) -> bindings::FimoError,
            );
            let data = (mapper as *mut T).cast::<core::ffi::c_void>();
            (node, edge, data)
        } else {
            (None, None, core::ptr::null_mut())
        };

        // Safety: The function either writes an error, or initializes the value.
        let graph = unsafe {
            to_result_indirect_in_place(|error, graph| {
                *error = bindings::fimo_graph_clone(
                    self.ptr.as_ptr(),
                    graph.as_mut_ptr(),
                    node,
                    edge,
                    data,
                );
            })?
        };
        let graph = NonNull::new(graph).expect("pointer should not be null");

        let graph = Self {
            ptr: graph,
            _n: PhantomData,
            _e: PhantomData,
        };
        Ok(graph)
    }

    /// Clones a reachable subset of the graph.
    ///
    /// The cloned graph contains all nodes and edges, that are
    /// reachable from the start node `start`. Reachable nodes
    /// are defined as all the nodes that have a path to themselves
    /// from the start node. The start node is always reachable. The
    /// cloning operation does not guarantee that the node and edge
    /// references remain valid for the clone. Therefore, the caller
    /// may provide a custom mapping function that allows them to
    /// map the old reference to the new reference.
    ///
    /// ```
    /// use fimo_std::graph::{Graph, IdxMapping};
    ///
    /// let mut graph = Graph::<u32, u32>::new().unwrap();
    ///
    /// let a = graph.add_node(1).unwrap();
    /// let b = graph.add_node(2).unwrap();
    /// let c = graph.add_node(3).unwrap();
    /// let d = graph.add_node(4).unwrap();
    ///
    /// let e1 = graph.add_edge(a, b, 10).unwrap();
    /// let e2 = graph.add_edge(b, c, 20).unwrap();
    /// let e3 = graph.add_edge(b, d, 30).unwrap();
    ///
    /// let mut new_a = None;
    /// let mut new_b = None;
    /// let mut new_c = None;
    /// let mut new_d = None;
    ///
    /// let mut new_e1 = None;
    /// let mut new_e2 = None;
    /// let mut new_e3 = None;
    ///
    /// let mapper = |mapping| {
    ///     match mapping {
    ///         IdxMapping::Node { old, new } => match old {
    ///             x if x == a => new_a = Some(new),
    ///             x if x == b => new_b = Some(new),
    ///             x if x == c => new_c = Some(new),
    ///             x if x == d => new_d = Some(new),
    ///             _ => unreachable!(),
    ///         },
    ///         IdxMapping::Edge { old, new } => match old {
    ///             x if x == e1 => new_e1 = Some(new),
    ///             x if x == e2 => new_e2 = Some(new),
    ///             x if x == e3 => new_e3 = Some(new),
    ///             _ => unreachable!(),
    ///         },
    ///     }
    ///     Ok(())
    /// };
    ///
    /// let clone = graph.clone_reachable_subgraph(b, Some(mapper)).unwrap();
    ///
    /// assert!(new_a.is_none());
    /// let new_b = new_b.unwrap();
    /// let new_c = new_c.unwrap();
    /// let new_d = new_d.unwrap();
    ///
    /// assert!(new_e1.is_none());
    /// let new_e2 = new_e2.unwrap();
    /// let new_e3 = new_e3.unwrap();
    ///
    /// assert_eq!(clone.node_data(new_b).unwrap(), &2);
    /// assert_eq!(clone.node_data(new_c).unwrap(), &3);
    /// assert_eq!(clone.node_data(new_d).unwrap(), &4);
    ///
    /// assert_eq!(clone.edge_data(new_e2).unwrap(), &20);
    /// assert_eq!(clone.edge_data(new_e3).unwrap(), &30);
    /// ```
    pub fn clone_reachable_subgraph<T>(
        &self,
        start: NodeIdx,
        mut mapper: Option<T>,
    ) -> Result<Self, Error>
    where
        N: Copy,
        E: Copy,
        T: FnMut(IdxMapping) -> crate::error::Result,
    {
        unsafe extern "C" fn node_mapper<T>(
            old: u64,
            new: u64,
            ptr: *mut core::ffi::c_void,
        ) -> bindings::FimoError
        where
            T: FnMut(IdxMapping) -> crate::error::Result,
        {
            let old = NodeIdx(old);
            let new = NodeIdx(new);

            // Safety: The function is only called with valid pointers.
            let mapper = unsafe { &mut *ptr.cast::<T>() };
            (mapper)(IdxMapping::Node { old, new })
                .err()
                .unwrap_or(Error::EOK)
                .into_error()
        }

        unsafe extern "C" fn edge_mapper<T>(
            old: u64,
            new: u64,
            ptr: *mut core::ffi::c_void,
        ) -> bindings::FimoError
        where
            T: FnMut(IdxMapping) -> crate::error::Result,
        {
            let old = EdgeIdx(old);
            let new = EdgeIdx(new);

            // Safety: The function is only called with valid pointers.
            let mapper = unsafe { &mut *ptr.cast::<T>() };
            (mapper)(IdxMapping::Edge { old, new })
                .err()
                .unwrap_or(Error::EOK)
                .into_error()
        }

        let (node, edge, data) = if let Some(mapper) = mapper.as_mut() {
            let node = Some(
                node_mapper::<T>
                    as unsafe extern "C" fn(
                        u64,
                        u64,
                        *mut core::ffi::c_void,
                    ) -> bindings::FimoError,
            );
            let edge = Some(
                edge_mapper::<T>
                    as unsafe extern "C" fn(
                        u64,
                        u64,
                        *mut core::ffi::c_void,
                    ) -> bindings::FimoError,
            );
            let data = (mapper as *mut T).cast::<core::ffi::c_void>();
            (node, edge, data)
        } else {
            (None, None, core::ptr::null_mut())
        };

        // Safety: The function either writes an error, or initializes the value.
        let graph = unsafe {
            to_result_indirect_in_place(|error, graph| {
                *error = bindings::fimo_graph_clone_reachable_subgraph(
                    self.ptr.as_ptr(),
                    graph.as_mut_ptr(),
                    start.0,
                    node,
                    edge,
                    data,
                );
            })?
        };
        let graph = NonNull::new(graph).expect("pointer should not be null");

        let graph = Self {
            ptr: graph,
            _n: PhantomData,
            _e: PhantomData,
        };
        Ok(graph)
    }
}

// Safety: The only reason why `Send` and `Sync` are not
// implemented by default is that the graph is hidden
// behind a `NonNull`. Since we don't provide access to
// the pointer this is equivalent to some `Box<OpaqueGraph>`,
// and therefore it is safe to implement the traits, if the
// generic types also implement them.
unsafe impl<N, E> Send for Graph<N, E>
where
    N: Send,
    E: Send,
{
}

// Safety: See above.
unsafe impl<N, E> Sync for Graph<N, E>
where
    N: Sync,
    E: Sync,
{
}

impl<N, E> Clone for Graph<N, E>
where
    N: Copy,
    E: Copy,
{
    fn clone(&self) -> Self {
        self.clone_with_mapper::<fn(IdxMapping) -> crate::error::Result>(None)
            .expect("cloning should succeed")
    }
}

impl<N, E> Drop for Graph<N, E> {
    fn drop(&mut self) {
        // Safety: We own the graph pointer.
        unsafe {
            bindings::fimo_graph_free(self.ptr.as_ptr());
        }
    }
}

impl<N, E> FFITransferable<*mut bindings::FimoGraph> for Graph<N, E> {
    fn into_ffi(self) -> *mut bindings::FimoGraph {
        let this = ManuallyDrop::new(self);
        this.ptr.as_ptr()
    }

    unsafe fn from_ffi(ffi: *mut bindings::FimoGraph) -> Self {
        Self {
            ptr: NonNull::new(ffi).expect("the graph should not be null"),
            _n: PhantomData,
            _e: PhantomData,
        }
    }
}

impl<N, E> FFITransferable<*mut bindings::FimoGraph> for Option<Graph<N, E>> {
    fn into_ffi(self) -> *mut bindings::FimoGraph {
        match self {
            Some(x) => x.into_ffi(),
            None => core::ptr::null_mut(),
        }
    }

    unsafe fn from_ffi(ffi: *mut bindings::FimoGraph) -> Self {
        if ffi.is_null() {
            return None;
        }

        // Safety: The caller owns the graph, so it is valid.
        unsafe { Some(Graph::from_ffi(ffi)) }
    }
}

#[derive(Debug)]
struct NodesInner {
    has_next: bool,
    ptr: NonNull<bindings::FimoGraphNodes>,
}

// Safety: Blanket implementation.
unsafe impl Send for NodesInner {}

// Safety: Blanket implementation.
unsafe impl Sync for NodesInner {}

impl Iterator for NodesInner {
    type Item = (NodeIdx, *const core::ffi::c_void);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.has_next {
            return None;
        }

        let mut node = NodeIdx(0);

        // Safety: The function either initializes the value or returns an error.
        let data = unsafe {
            to_result_indirect_in_place(|error, data| {
                *error = bindings::fimo_graph_nodes_item(
                    self.ptr.as_ptr(),
                    &mut node.0,
                    data.as_mut_ptr(),
                );
            })
            .expect("the iterator should be valid")
        };

        to_result_indirect(|error| {
            *error =
            // Safety: The pointers are valid.
                unsafe { bindings::fimo_graph_nodes_next(self.ptr.as_ptr(), &mut self.has_next) };
        })
        .expect("the iterator should be valid");

        Some((node, data))
    }
}

impl Drop for NodesInner {
    fn drop(&mut self) {
        // Safety: We own the iterator.
        unsafe {
            bindings::fimo_graph_nodes_free(self.ptr.as_ptr());
        }
    }
}

/// An iterator over the nodes of a [`Graph`].
#[derive(Debug)]
pub struct Nodes<'a, N, E> {
    inner: NodesInner,
    _phantom: PhantomData<&'a Graph<N, E>>,
}

impl<'a, N, E> Iterator for Nodes<'a, N, E> {
    type Item = (NodeIdx, &'a N);

    fn next(&mut self) -> Option<Self::Item> {
        let (node, data) = self.inner.next()?;

        let data = if core::mem::size_of::<N>() == 0 {
            debug_assert!(data.is_null());
            // Safety: See above.
            unsafe { &*NonNull::<N>::dangling().as_ptr() }
        } else {
            debug_assert!(!data.is_null());
            // Safety: See above.
            unsafe { &*data.cast::<N>() }
        };

        Some((node, data))
    }
}

/// An iterator over the nodes of a [`Graph`].
#[derive(Debug)]
pub struct NodesMut<'a, N, E> {
    inner: NodesInner,
    _phantom: PhantomData<&'a mut Graph<N, E>>,
}

impl<'a, N, E> Iterator for NodesMut<'a, N, E> {
    type Item = (NodeIdx, &'a mut N);

    fn next(&mut self) -> Option<Self::Item> {
        let (node, data) = self.inner.next()?;

        let data = if core::mem::size_of::<N>() == 0 {
            debug_assert!(data.is_null());
            // Safety: See above.
            unsafe { &mut *NonNull::<N>::dangling().as_ptr() }
        } else {
            debug_assert!(!data.is_null());
            // Safety: See above.
            unsafe { &mut *data.cast_mut().cast::<N>() }
        };

        Some((node, data))
    }
}

#[derive(Debug)]
struct EdgesInner {
    has_next: bool,
    ptr: NonNull<bindings::FimoGraphEdges>,
}

// Safety: Blanket implementation.
unsafe impl Send for EdgesInner {}

// Safety: Blanket implementation.
unsafe impl Sync for EdgesInner {}

impl Iterator for EdgesInner {
    type Item = (EdgeIdx, *const core::ffi::c_void);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.has_next {
            return None;
        }

        let mut edge = EdgeIdx(0);

        // Safety: The function either initializes the value or returns an error.
        let data = unsafe {
            to_result_indirect_in_place(|error, data| {
                *error = bindings::fimo_graph_edges_item(
                    self.ptr.as_ptr(),
                    &mut edge.0,
                    data.as_mut_ptr(),
                );
            })
            .expect("the iterator should be valid")
        };

        to_result_indirect(|error| {
            *error =
            // Safety: The pointers are valid.
                unsafe { bindings::fimo_graph_edges_next(self.ptr.as_ptr(), &mut self.has_next) };
        })
        .expect("the iterator should be valid");

        Some((edge, data))
    }
}

impl Drop for EdgesInner {
    fn drop(&mut self) {
        // Safety: We own the iterator.
        unsafe {
            bindings::fimo_graph_edges_free(self.ptr.as_ptr());
        }
    }
}

/// An iterator over the edges of a [`Graph`].
#[derive(Debug)]
pub struct Edges<'a, N, E> {
    inner: EdgesInner,
    _phantom: PhantomData<&'a Graph<N, E>>,
}

impl<'a, N, E> Iterator for Edges<'a, N, E> {
    type Item = (EdgeIdx, &'a E);

    fn next(&mut self) -> Option<Self::Item> {
        let (edge, data) = self.inner.next()?;

        let data = if core::mem::size_of::<E>() == 0 {
            debug_assert!(data.is_null());
            // Safety: See above.
            unsafe { &*NonNull::<E>::dangling().as_ptr() }
        } else {
            debug_assert!(!data.is_null());
            // Safety: See above.
            unsafe { &*data.cast::<E>() }
        };

        Some((edge, data))
    }
}

/// An iterator over the edges of a [`Graph`].
#[derive(Debug)]
pub struct EdgesMut<'a, N, E> {
    inner: EdgesInner,
    _phantom: PhantomData<&'a mut Graph<N, E>>,
}

impl<'a, N, E> Iterator for EdgesMut<'a, N, E> {
    type Item = (EdgeIdx, &'a mut E);

    fn next(&mut self) -> Option<Self::Item> {
        let (edge, data) = self.inner.next()?;

        let data = if core::mem::size_of::<E>() == 0 {
            debug_assert!(data.is_null());
            // Safety: See above.
            unsafe { &mut *NonNull::<E>::dangling().as_ptr() }
        } else {
            debug_assert!(!data.is_null());
            // Safety: See above.
            unsafe { &mut *data.cast_mut().cast::<E>() }
        };

        Some((edge, data))
    }
}

#[derive(Debug)]
struct ExternalsInner {
    has_next: bool,
    ptr: NonNull<bindings::FimoGraphExternals>,
}

// Safety: Blanket implementation.
unsafe impl Send for ExternalsInner {}

// Safety: Blanket implementation.
unsafe impl Sync for ExternalsInner {}

impl Iterator for ExternalsInner {
    type Item = (NodeIdx, *const core::ffi::c_void);

    fn next(&mut self) -> Option<Self::Item> {
        if !self.has_next {
            return None;
        }

        let mut node = NodeIdx(0);

        // Safety: The function either initializes the value or returns an error.
        let data = unsafe {
            to_result_indirect_in_place(|error, data| {
                *error = bindings::fimo_graph_externals_item(
                    self.ptr.as_ptr(),
                    &mut node.0,
                    data.as_mut_ptr(),
                );
            })
            .expect("the iterator should be valid")
        };

        to_result_indirect(|error| {
            *error =
            // Safety: The pointers are valid.
                unsafe { bindings::fimo_graph_externals_next(self.ptr.as_ptr(), &mut self.has_next) };
        }).expect("the iterator should be valid");

        Some((node, data))
    }
}

impl Drop for ExternalsInner {
    fn drop(&mut self) {
        // Safety: We own the iterator.
        unsafe {
            bindings::fimo_graph_externals_free(self.ptr.as_ptr());
        }
    }
}

/// An iterator over the externals of a node in a [`Graph`].
#[derive(Debug)]
pub struct Externals<'a, N, E> {
    inner: ExternalsInner,
    _phantom: PhantomData<&'a Graph<N, E>>,
}

// Safety: Blanket implementation.
unsafe impl<N, E> Send for Externals<'_, N, E> where Graph<N, E>: Send {}

// Safety: Blanket implementation.
unsafe impl<N, E> Sync for Externals<'_, N, E> where Graph<N, E>: Sync {}

impl<'a, N, E> Iterator for Externals<'a, N, E> {
    type Item = (NodeIdx, &'a N);

    fn next(&mut self) -> Option<Self::Item> {
        let (node, data) = self.inner.next()?;

        let data = if core::mem::size_of::<N>() == 0 {
            debug_assert!(data.is_null());
            // Safety: See above.
            unsafe { &*NonNull::<N>::dangling().as_ptr() }
        } else {
            debug_assert!(!data.is_null());
            // Safety: See above.
            unsafe { &*data.cast::<N>() }
        };

        Some((node, data))
    }
}

/// An iterator over the externals of a node in a [`Graph`].
#[derive(Debug)]
pub struct ExternalsMut<'a, N, E> {
    inner: ExternalsInner,
    _phantom: PhantomData<&'a mut Graph<N, E>>,
}

// Safety: Blanket implementation.
unsafe impl<N, E> Send for ExternalsMut<'_, N, E> where Graph<N, E>: Send {}

// Safety: Blanket implementation.
unsafe impl<N, E> Sync for ExternalsMut<'_, N, E> where Graph<N, E>: Sync {}

impl<'a, N, E> Iterator for ExternalsMut<'a, N, E> {
    type Item = (NodeIdx, &'a mut N);

    fn next(&mut self) -> Option<Self::Item> {
        let (node, data) = self.inner.next()?;

        let data = if core::mem::size_of::<N>() == 0 {
            debug_assert!(data.is_null());
            // Safety: See above.
            unsafe { &mut *NonNull::<N>::dangling().as_ptr() }
        } else {
            debug_assert!(!data.is_null());
            // Safety: See above.
            unsafe { &mut *data.cast_mut().cast::<N>() }
        };

        Some((node, data))
    }
}

/// An iterator over the neighbors of a node in a [`Graph`].
#[derive(Debug)]
pub struct Neighbors<'a, N, E> {
    has_next: bool,
    ptr: NonNull<bindings::FimoGraphNeighbors>,
    _phantom: PhantomData<&'a Graph<N, E>>,
}

// Safety: Blanket implementation.
unsafe impl<N, E> Send for Neighbors<'_, N, E> where Graph<N, E>: Send {}

// Safety: Blanket implementation.
unsafe impl<N, E> Sync for Neighbors<'_, N, E> where Graph<N, E>: Sync {}

impl<N, E> Iterator for Neighbors<'_, N, E> {
    type Item = NodeIdx;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.has_next {
            return None;
        }

        // Safety: The function either initializes the value or returns an error.
        let node = unsafe {
            to_result_indirect_in_place(|error, node| {
                *error = bindings::fimo_graph_neighbors_item(self.ptr.as_ptr(), node.as_mut_ptr());
            })
            .expect("the iterator should be valid")
        };
        let node = NodeIdx(node);

        to_result_indirect(|error| {
            *error =
            // Safety: The pointers are valid.
                unsafe { bindings::fimo_graph_neighbors_next(self.ptr.as_ptr(), &mut self.has_next) };
        }).expect("the iterator should be valid");

        Some(node)
    }
}

impl<N, E> Drop for Neighbors<'_, N, E> {
    fn drop(&mut self) {
        // Safety: We own the iterator.
        unsafe {
            bindings::fimo_graph_neighbors_free(self.ptr.as_ptr());
        }
    }
}

/// An iterator over the edges in a [`Graph`] connecting a node with its neighbors.
#[derive(Debug)]
pub struct NeighborsEdges<'a, N, E> {
    has_next: bool,
    ptr: NonNull<bindings::FimoGraphNeighborsEdges>,
    _phantom: PhantomData<&'a Graph<N, E>>,
}

// Safety: Blanket implementation.
unsafe impl<N, E> Send for NeighborsEdges<'_, N, E> where Graph<N, E>: Send {}

// Safety: Blanket implementation.
unsafe impl<N, E> Sync for NeighborsEdges<'_, N, E> where Graph<N, E>: Sync {}

impl<N, E> Iterator for NeighborsEdges<'_, N, E> {
    type Item = EdgeIdx;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.has_next {
            return None;
        }

        // Safety: The function either initializes the value or returns an error.
        let edge = unsafe {
            to_result_indirect_in_place(|error, edge| {
                *error =
                    bindings::fimo_graph_neighbors_edges_item(self.ptr.as_ptr(), edge.as_mut_ptr());
            })
            .expect("the iterator should be valid")
        };
        let edge = EdgeIdx(edge);

        to_result_indirect(|error| {
            *error =
            // Safety: The pointers are valid.
                unsafe { bindings::fimo_graph_neighbors_edges_next(self.ptr.as_ptr(), &mut self.has_next) };
        }).expect("the iterator should be valid");

        Some(edge)
    }
}

impl<N, E> Drop for NeighborsEdges<'_, N, E> {
    fn drop(&mut self) {
        // Safety: We own the iterator.
        unsafe {
            bindings::fimo_graph_neighbors_edges_free(self.ptr.as_ptr());
        }
    }
}
