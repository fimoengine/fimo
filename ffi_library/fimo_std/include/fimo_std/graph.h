#ifndef FIMO_GRAPH_H
#define FIMO_GRAPH_H

#include <stddef.h>

#include <fimo_std/error.h>
#include <fimo_std/utils.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * A directed graph data structure.
 *
 * A graph is a collection of nodes and edges, represented as an adjacency list.
 * The structure is generic, as it allows some user-defined data with each node/
 * edge.
 */
typedef struct FimoGraph FimoGraph;

/**
 * An iterator over the nodes of a graph.
 */
typedef struct FimoGraphNodes FimoGraphNodes;

/**
 * An iterator over the edges of a graph.
 */
typedef struct FimoGraphEdges FimoGraphEdges;

/**
 * An iterator over the source/sink nodes of a graph.
 */
typedef struct FimoGraphExternals FimoGraphExternals;

/**
 * An iterator over the neighbors of a node.
 */
typedef struct FimoGraphNeighbors FimoGraphNeighbors;

/**
 * An iterator over the edges connecting a node with its neighbors.
 */
typedef struct FimoGraphNeighborsEdges FimoGraphNeighborsEdges;

/**
 * Constructs a new graph.
 *
 * The caller can use `0` as the size of the node/edge, in that case they must
 * pass `NULL` as the cleanup function. If the size is greater than `0`, this
 * function requires that a cleanup function be defined.
 *
 * @param node_size size of a node element
 * @param edge_size size of an edge element
 * @param node_free node cleanup function
 * @param edge_free edge cleanup function
 * @param graph resulting graph
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_new(size_t node_size, size_t edge_size,
    void (*node_free)(void*), void (*edge_free)(void*),
    FimoGraph** graph);

/**
 * Destroys the graph.
 *
 * Clears the graph and frees up all the memory it owns.
 *
 * @param graph graph to destroy
 */
void fimo_graph_free(FimoGraph* graph);

/**
 * Returns the number of nodes in the graph.
 *
 * @param graph the graph
 *
 * @return Node count.
 */
FIMO_MUST_USE
size_t fimo_graph_node_count(const FimoGraph* graph);

/**
 * Returns the number of edges in the graph.
 *
 * @param graph the graph
 *
 * @return Edge count.
 */
FIMO_MUST_USE
size_t fimo_graph_edge_count(const FimoGraph* graph);

/**
 * Adds a new node to the graph.
 *
 * The caller may pass a pointer to the node data that will be copied inside
 * the graph, if the graph was initialized with a node size greater than `0`.
 * The resulting node index is written back into `node`.
 *
 * @param graph the graph
 * @param node_data optional data to copy into the node
 * @param node resulting node index
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_add_node(FimoGraph* graph, const void* node_data,
    FimoU64* node);

/**
 * Access the data associated with a node.
 *
 * If the node does not exist in the graph, this function returns en error.
 *
 * @param graph the graph
 * @param node node inside of the graph
 * @param node_data resulting node data
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_node_data(const FimoGraph* graph, FimoU64 node,
    const void** node_data);

/**
 * Adds an edge from `src_node` to `dst_node`.
 *
 * The caller may pass a pointer to the edge data that will be copied inside
 * the graph, if the graph was initialized with a edge size greater than `0`.
 * The resulting edge index is written back into `edge`. If the edge already
 * exists, this function optionally returns the old data.
 *
 * @param graph the graph
 * @param src_node start node
 * @param dst_node destination node
 * @param edge_data optional edge data to copy
 * @param old_edge_data pointer where to store the old data
 * @param edge resulting edge
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_add_edge(FimoGraph* graph, FimoU64 src_node,
    FimoU64 dst_node, const void* edge_data, void** old_edge_data,
    FimoU64* edge);

/**
 * Updates the edge from `src_node` to `dst_node`.
 *
 * The caller may pass a pointer to the edge data that will be copied inside
 * the graph, if the graph was initialized with a edge size greater than `0`.
 * The resulting edge index is written back into `edge`. Returns an error, if
 * the edge does not already exist.
 *
 * @param graph the graph
 * @param src_node start node
 * @param dst_node destination node
 * @param edge_data optional edge data to copy
 * @param old_edge_data pointer where to store the old data
 * @param edge resulting edge
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_update_edge(FimoGraph* graph, FimoU64 src_node,
    FimoU64 dst_node, const void* edge_data, void** old_edge_data,
    FimoU64* edge);

/**
 * Access the data associated with an edge.
 *
 * If the edge does not exist in the graph, this function returns an error.
 *
 * @param graph the graph
 * @param edge edge inside of the graph
 * @param edge_data resulting edge data
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_edge_data(const FimoGraph* graph, FimoU64 edge,
    const void** edge_data);

/**
 * Returns the node endpoints of an edge.
 *
 * If the edge does not exist in the graph, this function returns an error.
 *
 * @param graph the graph
 * @param edge edge inside of the graph
 * @param start_node resulting start node
 * @param end_node resulting end node
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_edge_endpoints(const FimoGraph* graph, FimoU64 edge,
    FimoU64* start_node, FimoU64* end_node);

/**
 * Removes a node and all its edges from the graph.
 *
 * If the node does not exist in the graph, this function returns an error.
 *
 * @param graph the graph
 * @param node node to remove
 * @param node_data old node data
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_remove_node(FimoGraph* graph, FimoU64 node,
    void** node_data);

/**
 * Removes an edge from the graph.
 *
 * If the edge does not exist in the graph, this function returns an error.
 *
 * @param graph the graph
 * @param edge edge inside of the graph
 * @param edge_data resulting edge data
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_remove_edge(FimoGraph* graph, FimoU64 edge,
    void** edge_data);

/**
 * Checks whether an edge exists from `src_node` to `dst_node`.
 *
 * If any of the two nodes does not exist, this function returns an error.
 *
 * @param graph the graph
 * @param src_node start node
 * @param dst_node end node
 * @param contained query result
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_contains_edge(const FimoGraph* graph, FimoU64 src_node,
    FimoU64 dst_node, bool* contained);

/**
 * Finds the edge index from `src_node` to `dst_node`.
 *
 * If any of the two nodes does not exist, this function returns an error.
 * If the edge does not exist, this function sets `contained` to `false`.
 *
 * @param graph the graph
 * @param src_node start node
 * @param dst_node end node
 * @param edge queried edge
 * @param contained edge presence
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_find_edge(const FimoGraph* graph, FimoU64 src_node,
    FimoU64 dst_node, FimoU64* edge, bool* contained);

/**
 * Constructs a new iterator over the nodes of a graph.
 *
 * The parameter `has_value` is set to true, if there is
 * at least one node to iterate over.
 *
 * @param graph the graph
 * @param iter resulting iterator
 * @param has_value whether the iterator is empty
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_nodes_new(const FimoGraph* graph,
    FimoGraphNodes** iter, bool* has_value);

/**
 * Performs an iteration step to the next node.
 *
 * Goes to the next node of the iterator. The iterator must not have
 * reached the end before calling this function. Returns whether the
 * iteration has been completed, by writing a value into `has_value`.
 *
 * @param iter the iterator
 * @param has_value whether the iterator has not jet reached the end
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_nodes_next(FimoGraphNodes* iter,
    bool* has_value);

/**
 * Queries the node index and data at the current iterator position.
 *
 * The node index is written into `node`, while the node data is
 * written into `node_data`, if the iterator has not yet reached
 * the end position and the parameters are not `NULL`.
 *
 * @param iter the iterator
 * @param node resulting node index.
 * @param node_data resulting node data.
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_nodes_item(const FimoGraphNodes* iter,
    FimoU64* node, const void** node_data);

/**
 * Frees up a nodes iterator.
 *
 * @param iter the iterator
 */
void fimo_graph_nodes_free(FimoGraphNodes* iter);

/**
 * Constructs a new iterator over the edges of a graph.
 *
 * The parameter `has_value` is set to true, if there is
 * at least one edge to iterate over.
 *
 * @param graph the graph
 * @param iter resulting iterator
 * @param has_value whether the iterator is empty
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_edges_new(const FimoGraph* graph,
    FimoGraphEdges** iter, bool* has_value);

/**
 * Performs an iteration step to the next edge.
 *
 * Goes to the next edge of the iterator. The iterator must not have
 * reached the end before calling this function. Returns whether the
 * iteration has been completed, by writing a value into `has_value`.
 *
 * @param iter the iterator
 * @param has_value whether the iterator has not jet reached the end
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_edges_next(FimoGraphEdges* iter,
    bool* has_value);

/**
 * Queries the edge index and data at the current iterator position.
 *
 * The edge index is written into `edge`, while the edge data is
 * written into `edge_data`, if the iterator has not yet reached
 * the end position and the parameters are not `NULL`.
 *
 * @param iter the iterator
 * @param edge resulting edge index.
 * @param edge_data resulting edge data.
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_edges_item(const FimoGraphEdges* iter,
    FimoU64* edge, const void** edge_data);

/**
 * Frees up an edges iterator.
 *
 * @param iter the iterator
 */
void fimo_graph_edges_free(FimoGraphEdges* iter);

/**
 * Constructs a new iterator over the externals of a graph.
 *
 * The externals are defined as nodes, that either have no
 * inward edges (sources), or have no outward edges (sinks).
 * The caller can specify whether to iterate over the source
 * or sink nodes by setting the `sink` flag. The parameter
 * `has_value` is set to true, if there is at least one
 * external node to iterate over.
 *
 * @param graph the graph
 * @param sink whether to iterate over the source or sink nodes
 * @param iter resulting iterator
 * @param has_value whether the iterator is empty
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_externals_new(const FimoGraph* graph, bool sink,
    FimoGraphExternals** iter, bool* has_value);

/**
 * Performs an iteration step to the next external node.
 *
 * Goes to the next external of the iterator. The iterator must not have
 * reached the end before calling this function. Returns whether the
 * iteration has been completed, by writing a value into `has_value`.
 *
 * @param iter the iterator
 * @param has_value whether the iterator has not jet reached the end
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_externals_next(FimoGraphExternals* iter,
    bool* has_value);

/**
 * Queries the node index and data at the current iterator position.
 *
 * The node index is written into `node`, while the node data is
 * written into `node_data`, if the iterator has not yet reached
 * the end position and the parameters are not `NULL`.
 *
 * @param iter the iterator
 * @param node resulting node index.
 * @param node_data resulting node data.
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_externals_item(const FimoGraphExternals* iter,
    FimoU64* node, const void** node_data);

/**
 * Frees up a externals iterator.
 *
 * @param iter the iterator
 */
void fimo_graph_externals_free(FimoGraphExternals* iter);

/**
 * Constructs a new iterator over the neighbors of a node.
 *
 * Returns an iterator over all neighbors of `node`. If
 * `inward` is set to `true`, the iterator will iterate
 * over all nodes that have an edge to `node`, otherwise
 * the iterator will iterate over all edges starting from
 * `node`. If `node` does not exist, this function will
 * return an error.
 *
 * @param graph the graph
 * @param node the node to query
 * @param inward whether to interpret `node` as the starting or end node
 * @param iter resulting iterator
 * @param has_value iterator is empty
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_neighbors_new(const FimoGraph* graph, FimoU64 node,
    bool inward, FimoGraphNeighbors** iter, bool* has_value);

/**
 * Performs an iteration step.
 *
 * Goes to the next neighbor of the iterator. The iterator must not have
 * reached the end before calling this function. Returns whether the
 * iteration has been completed, by writing a value into `has_value`.
 *
 * @param iter the iterator
 * @param has_value whether the iterator has not jet reached the end
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_neighbors_next(FimoGraphNeighbors* iter,
    bool* has_value);

/**
 * Queries the node index at the current iterator position.
 *
 * The node index is written into `node`, if the iterator has not
 * yet reached the end position.
 *
 * @param iter the iterator
 * @param node resulting node index.
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_neighbors_item(const FimoGraphNeighbors* iter,
    FimoU64* node);

/**
 * Frees up a neighbors iterator.
 *
 * @param iter the iterator
 */
void fimo_graph_neighbors_free(FimoGraphNeighbors* iter);

/**
 * Constructs a new iterator over the edges connecting
 * a node with its neighbors.
 *
 * Returns an iterator over all edges connecting the neighbors
 * of `node` with `node`. If `inward` is set to `true`, the
 * iterator will iterate over all edges to `node`, otherwise
 * the iterator will iterate over all edges starting from
 * `node`. If `node` does not exist, this function will
 * return an error.
 *
 * @param graph the graph
 * @param node the node to query
 * @param inward whether to interpret `node` as the starting or end node
 * @param iter resulting iterator
 * @param has_value iterator is empty
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_neighbors_edges_new(const FimoGraph* graph, FimoU64 node,
    bool inward, FimoGraphNeighborsEdges** iter, bool* has_value);

/**
 * Performs an iteration step.
 *
 * Goes to the next edge of the iterator. The iterator must not have
 * reached the end before calling this function. Returns whether the
 * iteration has been completed, by writing a value into `has_value`.
 *
 * @param iter the iterator
 * @param has_value whether the iterator has not jet reached the end
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_neighbors_edges_next(FimoGraphNeighborsEdges* iter,
    bool* has_value);

/**
 * Queries the edge index at the current iterator position.
 *
 * The edge index is written into `edge`, if the iterator has not
 * yet reached the end position.
 *
 * @param iter the iterator
 * @param edge resulting edge index.
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_neighbors_edges_item(const FimoGraphNeighborsEdges* iter,
    FimoU64* edge);

/**
 * Frees up a edges iterator.
 *
 * @param iter the iterator
 */
void fimo_graph_neighbors_edges_free(FimoGraphNeighborsEdges* iter);

/**
 * Removes all nodes and edges from the graph.
 *
 * @param graph the graph
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_clear(FimoGraph* graph);

/**
 * Removes all edges from the graph.
 *
 * @param graph the graph
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_clear_edges(FimoGraph* graph);

/**
 * Inverts the direction of all edges in the graph.
 *
 * @param graph the graph
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_reverse(FimoGraph* graph);

/**
 * Initializes a new graph by cloning another one.
 *
 * Performs a deep copy of the graph structure, and the contained
 * node and edge data into the uninitialized `new_graph`. The new
 * graph may use different node and edge indices than the original
 * one, therefore, the caller can provide two callback functions
 * that are called with the old and the new index of the node/edge.
 *
 * @param graph the graph to clone
 * @param new_graph cloned graph
 * @param node_mapper callback for mapped node indices
 * @param edge_mapper callback for mapped edge indices
 * @param user_data data pointer for the callbacks
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_clone(const FimoGraph* graph, FimoGraph** new_graph,
    FimoError (*node_mapper)(FimoU64, FimoU64, void*),
    FimoError (*edge_mapper)(FimoU64, FimoU64, void*),
    void* user_data);

/**
 * Initializes a new subgraph containing all reachable nodes.
 *
 * Performs a deep copy of the graph structure, and the contained
 * node and edge data into the uninitialized `sub_graph`, including
 * all nodes and edges reachable from the start node `start_node`.
 * It is an error to pass in a nonexistent start node as a parameter,
 * The new graph may use different node and edge indices than the
 * original one, therefore, the caller can provide two callback
 * functions that are called with the old and the new index of the
 * node/edge.
 *
 * @param graph the graph to clone
 * @param sub_graph resulting sub-graph
 * @param start_node start node
 * @param node_mapper callback for mapped node indices
 * @param edge_mapper callback for mapped edge indices
 * @param user_data data pointer for the callbacks
 *
 * @return Status code.
 */
FIMO_MUST_USE
FimoError fimo_graph_clone_reachable_subgraph(const FimoGraph* graph,
    FimoGraph** sub_graph, FimoU64 start_node,
    FimoError (*node_mapper)(FimoU64, FimoU64, void*),
    FimoError (*edge_mapper)(FimoU64, FimoU64, void*),
    void* user_data);

#ifdef __cplusplus
}
#endif

#endif // FIMO_GRAPH_H
