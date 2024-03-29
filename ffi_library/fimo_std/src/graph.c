#include <fimo_std/graph.h>

#include <fimo_std/array_list.h>
#include <fimo_std/memory.h>

#include <btree/btree.h>
#include <limits.h>
#include <stdalign.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#if defined(_WIN32) || defined(WIN32)
#include <malloc.h>
#elif __APPLE__
#include <malloc/malloc.h>
#elif __ANDROID__
#include <malloc.h>
#elif __linux__
#include <malloc.h>
#endif

#if defined(_WIN32) || defined(WIN32)
#define FIMO_MALLOC_ALIGNMENT 16
#else
#define FIMO_MALLOC_ALIGNMENT _Alignof(max_align_t)
#endif

#define MAX_GRAPH_IDX_ UINT64_MAX

struct FimoGraph {
    size_t node_size;
    size_t edge_size;
    void (*node_free)(void *);
    void (*edge_free)(void *);
    struct btree *nodes;
    struct btree *edges;
    FimoArrayList node_free_list;
    FimoArrayList edge_free_list;
    FimoU64 next_node_idx;
    FimoU64 next_edge_idx;
};

struct FimoGraphNodes {
    struct btree_iter *iter;
    bool has_value;
};

struct FimoGraphEdges {
    struct btree_iter *iter;
    bool has_value;
};

struct FimoGraphExternals {
    struct btree_iter *iter;
    bool has_value;
    bool sink;
};

struct FimoGraphNeighbors {
    struct btree_iter *iter;
    bool has_value;
};

struct FimoGraphNeighborsEdges {
    struct btree_iter *iter;
    bool has_value;
};

struct FimoGraphNode_ {
    FimoU64 key;
    struct btree *adjacency;
    struct btree *inv_adjacency;
    void *data;
};

struct FimoGraphNodeAdj_ {
    FimoU64 key;
    FimoU64 edge;
};

struct FimoGraphEdge_ {
    FimoU64 key;
    FimoU64 src;
    FimoU64 dst;
    void *data;
};

static int fimo_graph_node_compare_(const void *a, const void *b, void *data) {
    (void)data;
    const struct FimoGraphNode_ *na = a;
    const struct FimoGraphNode_ *nb = b;

    FimoI64 x = (FimoI64)(na->key - nb->key);
    if (x < 0) {
        return -1;
    }
    else if (x > 0) {
        return 1;
    }
    return 0;
}

static int fimo_graph_node_adj_compare_(const void *a, const void *b, void *data) {
    (void)data;
    const struct FimoGraphNodeAdj_ *na = a;
    const struct FimoGraphNodeAdj_ *nb = b;

    FimoI64 x = (FimoI64)(na->key - nb->key);
    if (x < 0) {
        return -1;
    }
    else if (x > 0) {
        return 1;
    }
    return 0;
}

static int fimo_graph_edge_compare_(const void *a, const void *b, void *data) {
    (void)data;
    const struct FimoGraphEdge_ *ea = a;
    const struct FimoGraphEdge_ *eb = b;

    FimoI64 x = (FimoI64)(ea->key - eb->key);
    if (x < 0) {
        return -1;
    }
    else if (x > 0) {
        return 1;
    }
    return 0;
}

static bool node_free_(const void *item, void *data) {
    FimoGraph *graph = data;
    struct FimoGraphNode_ *node = (void *)item;
    if (node->data) {
        if (graph->node_free) {
            graph->node_free(node->data);
        }
        fimo_free_sized(node->data, graph->node_size);
        node->data = NULL;
    }
    if (node->adjacency) {
        btree_free(node->adjacency);
    }
    if (node->inv_adjacency) {
        btree_free(node->inv_adjacency);
    }
    return true;
}

static bool edge_free_(const void *item, void *data) {
    FimoGraph *graph = data;
    struct FimoGraphEdge_ *edge = (void *)item;
    if (edge->data) {
        if (graph->edge_free) {
            graph->edge_free(edge->data);
        }
        fimo_free_sized(edge->data, graph->edge_size);
        edge->data = NULL;
    }
    return true;
}

static void *malloc_(size_t size) { return fimo_malloc(size, NULL); }

static void *realloc_(void *ptr, size_t size) {
    if (ptr == NULL) {
        return fimo_malloc(size, NULL);
    }
    if (size == 0) {
        fimo_free(ptr);
        return NULL;
    }

    size_t old_size;
#if defined(_WIN32) || defined(WIN32)
    old_size = _aligned_msize(ptr, FIMO_MALLOC_ALIGNMENT, 0);
    if (old_size == (size_t)-1) {
        return NULL;
    }
#elif __APPLE__
    old_size = malloc_size(ptr);
#elif __ANDROID__
    old_size = malloc_usable_size(ptr);
#elif __linux__
    old_size = malloc_usable_size(ptr);
#else
    old_size = 0;
#endif
    if (old_size >= size) {
        return ptr;
    }

    FimoError error = FIMO_EOK;
    void *new_ptr = fimo_malloc(size, &error);
    if (FIMO_IS_ERROR(error)) {
        return NULL;
    }

    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    memcpy(new_ptr, ptr, old_size);
    fimo_free(ptr);

    return new_ptr;
}

static void free_(void *ptr) { fimo_free(ptr); }

FIMO_MUST_USE
FimoError fimo_graph_new(size_t node_size, size_t edge_size, void (*node_free)(void *), void (*edge_free)(void *),
                         FimoGraph **graph) {
    if (graph == NULL) {
        return FIMO_EINVAL;
    }
    if (node_size == 0 && node_free) {
        return FIMO_EINVAL;
    }
    if (edge_size == 0 && edge_free) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    FimoGraph *g = fimo_aligned_alloc(_Alignof(FimoGraph), sizeof(FimoGraph), &error);
    if (FIMO_IS_ERROR(error)) {
        goto error_graph;
    }

    g->node_size = node_size;
    g->edge_size = edge_size;
    g->node_free = node_free;
    g->edge_free = edge_free;
    g->node_free_list = fimo_array_list_new();
    g->edge_free_list = fimo_array_list_new();
    g->next_node_idx = 0;
    g->next_edge_idx = 0;

    struct btree *nodes = btree_new_with_allocator(malloc_, realloc_, free_, sizeof(struct FimoGraphNode_), 0,
                                                   fimo_graph_node_compare_, NULL);
    if (nodes == NULL) {
        error = FIMO_ENOMEM;
        goto error_nodes;
    }
    g->nodes = nodes;

    struct btree *edges = btree_new_with_allocator(malloc_, realloc_, free_, sizeof(struct FimoGraphEdge_), 0,
                                                   fimo_graph_edge_compare_, NULL);
    if (edges == NULL) {
        error = FIMO_ENOMEM;
        goto error_edges;
    }
    g->edges = edges;

    *graph = g;
    return FIMO_EOK;

error_edges:
    btree_free(nodes);
error_nodes:
    fimo_free_aligned_sized(g, _Alignof(FimoGraph), sizeof(FimoGraph));
error_graph:
    return error;
}

void fimo_graph_free(FimoGraph *graph) {
    if (graph == NULL) {
        perror("graph is null");
        exit(EXIT_FAILURE);
    }

    btree_ascend(graph->nodes, NULL, node_free_, graph);
    btree_free(graph->nodes);

    btree_ascend(graph->edges, NULL, edge_free_, graph);
    btree_free(graph->edges);

    fimo_array_list_free(&graph->node_free_list, sizeof(FimoU64), alignof(FimoU64), NULL);
    fimo_array_list_free(&graph->edge_free_list, sizeof(FimoU64), alignof(FimoU64), NULL);

    fimo_free_aligned_sized(graph, _Alignof(FimoGraph), sizeof(FimoGraph));
}

FIMO_MUST_USE
size_t fimo_graph_node_count(const FimoGraph *graph) {
    if (graph == NULL) {
        perror("graph is null");
        exit(EXIT_FAILURE);
    }

    return btree_count(graph->nodes);
}

FIMO_MUST_USE
size_t fimo_graph_edge_count(const FimoGraph *graph) {
    if (graph == NULL) {
        perror("graph is null");
        exit(EXIT_FAILURE);
    }

    return btree_count(graph->edges);
}

FIMO_MUST_USE
FimoError fimo_graph_neighbors_count(const FimoGraph *graph, FimoU64 node, bool inward, size_t *count) {
    if (graph == NULL || count == NULL) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNode_ *node_ptr = btree_get(graph->nodes, &(struct FimoGraphNode_){
                                                                            .key = node,
                                                                    });
    if (node_ptr == NULL) {
        return FIMO_EINVAL;
    }

    const struct btree *neighbors = inward ? node_ptr->inv_adjacency : node_ptr->adjacency;
    if (neighbors == NULL) {
        *count = 0;
    }
    else {
        *count = btree_count(neighbors);
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_add_node(FimoGraph *graph, const void *node_data, FimoU64 *node) {
    if (graph == NULL || node == NULL) {
        return FIMO_EINVAL;
    }
    if ((node_data && graph->node_size == 0) || (node_data == NULL && graph->node_size != 0)) {
        return FIMO_EINVAL;
    }

    FimoU64 node_idx;
    FimoError error = FIMO_EOK;
    bool node_from_free_list = true;
    if (!fimo_array_list_is_empty(&graph->node_free_list)) {
        error = fimo_array_list_pop_back(&graph->node_free_list, sizeof(FimoU64), &node_idx, NULL);
        if (FIMO_IS_ERROR(error)) {
            return error;
        }
    }
    else {
        if (graph->next_node_idx == MAX_GRAPH_IDX_) {
            return FIMO_EINVAL;
        }
        node_idx = graph->next_node_idx++;
        node_from_free_list = false;
    }

    void *node_data_copy = NULL;
    if (node_data) {
        node_data_copy = fimo_malloc(graph->node_size, &error);
        if (FIMO_IS_ERROR(error)) {
            goto error_data_alloc;
        }
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(node_data_copy, node_data, graph->node_size);
    }

    btree_set(graph->nodes, &(struct FimoGraphNode_){
                                    .key = node_idx,
                                    .adjacency = NULL,
                                    .inv_adjacency = NULL,
                                    .data = node_data_copy,
                            });
    if (btree_oom(graph->nodes)) {
        error = FIMO_ENOMEM;
        goto error_node_alloc;
    }

    *node = node_idx;

    return FIMO_EOK;

error_node_alloc:
    if (node_data_copy) {
        fimo_free_sized(node_data_copy, graph->node_size);
    }
error_data_alloc:
    if (node_from_free_list) {
        FIMO_IGNORE(fimo_array_list_push(&graph->node_free_list, sizeof(FimoU64), alignof(FimoU64), &node_idx, NULL));
    }
    else {
        graph->next_node_idx--;
    }

    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_node_data(const FimoGraph *graph, FimoU64 node, const void **node_data) {
    if (graph == NULL || node_data == NULL) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNode_ *n = btree_get(graph->nodes, &(struct FimoGraphNode_){
                                                                     .key = node,
                                                             });
    if (n == NULL) {
        return FIMO_EINVAL;
    }
    *node_data = n->data;

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_add_edge(FimoGraph *graph, FimoU64 src_node, FimoU64 dst_node, const void *edge_data,
                              void **old_edge_data, FimoU64 *edge) {
    if (graph == NULL || edge == NULL) {
        return FIMO_EINVAL;
    }
    if ((edge_data && graph->edge_size == 0) || (edge_data == NULL && graph->edge_size != 0)) {
        return FIMO_EINVAL;
    }

    struct FimoGraphNode_ *src = (void *)btree_get(graph->nodes, &(struct FimoGraphNode_){
                                                                         .key = src_node,
                                                                 });
    if (src == NULL) {
        return FIMO_EINVAL;
    }

    struct FimoGraphNode_ *dst = (void *)btree_get(graph->nodes, &(struct FimoGraphNode_){
                                                                         .key = dst_node,
                                                                 });
    if (dst == NULL) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    struct btree *adjacency = src->adjacency;
    if (adjacency == NULL) {
        adjacency = btree_new_with_allocator(malloc_, realloc_, free_, sizeof(struct FimoGraphNodeAdj_), 0,
                                             fimo_graph_node_adj_compare_, NULL);
        if (adjacency == NULL) {
            error = FIMO_ENOMEM;
            goto error_adjacency;
        }
    }

    struct btree *inv_adjacency = dst->inv_adjacency;
    if (inv_adjacency == NULL) {
        inv_adjacency = btree_new_with_allocator(malloc_, realloc_, free_, sizeof(struct FimoGraphNodeAdj_), 0,
                                                 fimo_graph_node_adj_compare_, NULL);
        if (inv_adjacency == NULL) {
            error = FIMO_ENOMEM;
            goto error_inv_adjacency;
        }
    }

    void *edge_data_copy = NULL;
    if (edge_data) {
        edge_data_copy = fimo_malloc(graph->edge_size, &error);
        if (FIMO_IS_ERROR(error)) {
            goto error_edge_data_alloc;
        }
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(edge_data_copy, edge_data, graph->edge_size);
    }

    const struct FimoGraphNodeAdj_ *src_adj = btree_get(adjacency, &(struct FimoGraphNodeAdj_){
                                                                           .key = dst_node,
                                                                   });
    if (src_adj) {
        FimoU64 edge_idx = src_adj->edge;
        struct FimoGraphEdge_ *edge_node = (void *)btree_get(graph->edges, &(struct FimoGraphEdge_){
                                                                                   .key = edge_idx,
                                                                           });
        if (old_edge_data) {
            *old_edge_data = edge_node->data;
        }
        else {
            edge_free_(edge_node, graph);
        }
        *edge = edge_idx;
        edge_node->data = edge_data_copy;
        return FIMO_EOK;
    }

    FimoU64 edge_idx;
    bool edge_idx_from_free_list = true;
    if (!fimo_array_list_is_empty(&graph->edge_free_list)) {
        error = fimo_array_list_pop_back(&graph->edge_free_list, sizeof(FimoU64), &edge_idx, NULL);
        if (FIMO_IS_ERROR(error)) {
            goto error_edge_idx_fetch;
        }
    }
    else {
        if (graph->next_edge_idx == MAX_GRAPH_IDX_) {
            error = FIMO_EINVAL;
            goto error_edge_idx_fetch;
        }
        edge_idx = graph->next_edge_idx++;
        edge_idx_from_free_list = false;
    }

    btree_set(adjacency, &(struct FimoGraphNodeAdj_){
                                 .key = dst_node,
                                 .edge = edge_idx,
                         });
    if (btree_oom(adjacency)) {
        error = FIMO_ENOMEM;
        goto error_adjacency_set;
    }

    btree_set(inv_adjacency, &(struct FimoGraphNodeAdj_){
                                     .key = src_node,
                                     .edge = edge_idx,
                             });
    if (btree_oom(inv_adjacency)) {
        error = FIMO_ENOMEM;
        goto error_inv_adjacency_set;
    }

    btree_set(graph->edges, &(struct FimoGraphEdge_){
                                    .key = edge_idx,
                                    .src = src_node,
                                    .dst = dst_node,
                                    .data = edge_data_copy,
                            });
    if (btree_oom(graph->edges)) {
        error = FIMO_ENOMEM;
        goto error_edges_set;
    }

    src->adjacency = adjacency;
    dst->inv_adjacency = inv_adjacency;
    if (old_edge_data) {
        *old_edge_data = NULL;
    }
    *edge = edge_idx;

    return FIMO_EOK;

error_edges_set:
    btree_delete(inv_adjacency, &(struct FimoGraphNodeAdj_){
                                        .key = src_node,
                                });
error_inv_adjacency_set:
    btree_delete(adjacency, &(struct FimoGraphNodeAdj_){
                                    .key = dst_node,
                            });
error_adjacency_set:
    if (edge_idx_from_free_list) {
        FIMO_IGNORE(fimo_array_list_push(&graph->edge_free_list, sizeof(FimoU64), alignof(FimoU64), &edge_idx, NULL));
    }
    else {
        graph->next_edge_idx--;
    }
error_edge_idx_fetch:
    if (edge_data_copy) {
        fimo_free_sized(edge_data_copy, graph->edge_size);
    }
error_edge_data_alloc:
    if (dst->inv_adjacency == NULL) {
        btree_free(inv_adjacency);
    }
error_inv_adjacency:
    if (src->adjacency == NULL) {
        btree_free(adjacency);
    }
error_adjacency:
    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_update_edge(FimoGraph *graph, FimoU64 src_node, FimoU64 dst_node, const void *edge_data,
                                 void **old_edge_data, FimoU64 *edge) {
    if (graph == NULL || edge == NULL) {
        return FIMO_EINVAL;
    }
    if ((edge_data && graph->edge_size == 0) || (edge_data == NULL && graph->edge_size != 0)) {
        return FIMO_EINVAL;
    }

    struct FimoGraphNode_ *src = (void *)btree_get(graph->nodes, &(struct FimoGraphNode_){
                                                                         .key = src_node,
                                                                 });
    if (src == NULL || src->adjacency == NULL) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNodeAdj_ *src_edge = btree_get(src->adjacency, &(struct FimoGraphNodeAdj_){
                                                                                 .key = dst_node,
                                                                         });
    if (src_edge == NULL) {
        return FIMO_EINVAL;
    }

    void *edge_data_copy = NULL;
    if (edge_data) {
        FimoError error = FIMO_EOK;
        edge_data_copy = fimo_malloc(graph->edge_size, &error);
        if (FIMO_IS_ERROR(error)) {
            return error;
        }
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(edge_data_copy, edge_data, graph->edge_size);
    }

    struct FimoGraphEdge_ *edge_node = (void *)btree_get(graph->edges, &(struct FimoGraphEdge_){
                                                                               .key = src_edge->edge,
                                                                       });
    if (old_edge_data) {
        *old_edge_data = edge_node->data;
    }
    else {
        edge_free_(edge_node, graph);
    }
    edge_node->data = edge_data_copy;
    *edge = src_edge->edge;

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_edge_data(const FimoGraph *graph, FimoU64 edge, const void **edge_data) {
    if (graph == NULL || edge_data == NULL) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphEdge_ *e = btree_get(graph->edges, &(struct FimoGraphEdge_){
                                                                     .key = edge,
                                                             });
    if (e == NULL) {
        return FIMO_EINVAL;
    }
    *edge_data = e->data;

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_edge_endpoints(const FimoGraph *graph, FimoU64 edge, FimoU64 *start_node, FimoU64 *end_node) {
    if (graph == NULL || start_node == NULL || end_node == NULL) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphEdge_ *e = btree_get(graph->edges, &(struct FimoGraphEdge_){
                                                                     .key = edge,
                                                             });
    if (e == NULL) {
        return FIMO_EINVAL;
    }
    *start_node = e->src;
    *end_node = e->dst;

    return FIMO_EOK;
}

struct CollectEdgesData_ {
    FimoArrayList *edges;
    FimoError error;
};

static bool collect_edges_(const void *item, void *data) {
    struct CollectEdgesData_ *d = data;
    const struct FimoGraphNodeAdj_ *adj = item;
    d->error = fimo_array_list_push(d->edges, sizeof(FimoU64), alignof(FimoU64), (void *)&adj->edge, NULL);
    if (FIMO_IS_ERROR(d->error)) {
        return false;
    }
    return true;
}

FIMO_MUST_USE
FimoError fimo_graph_remove_node(FimoGraph *graph, FimoU64 node, void **node_data) {
    if (graph == NULL || node_data == NULL) {
        return FIMO_EINVAL;
    }

    FimoError error;
    const struct FimoGraphNode_ *n = btree_get(graph->nodes, &(struct FimoGraphNode_){
                                                                     .key = node,
                                                             });
    if (n == NULL) {
        error = FIMO_EINVAL;
        goto error_node_not_found;
    }

    error = fimo_array_list_reserve(&graph->node_free_list, sizeof(FimoU64), alignof(FimoU64), 1, NULL);
    if (FIMO_IS_ERROR(error)) {
        goto error_node_free_list_resize;
    }

    size_t edge_count = 0;
    if (n->adjacency) {
        edge_count += btree_count(n->adjacency);
    }
    if (n->inv_adjacency) {
        edge_count += btree_count(n->inv_adjacency);
    }

    error = fimo_array_list_reserve(&graph->edge_free_list, sizeof(FimoU64), alignof(FimoU64), edge_count, NULL);
    if (FIMO_IS_ERROR(error)) {
        goto error_edge_free_list_resize;
    }

    if (edge_count != 0) {
        FimoArrayList edge_buffer;
        error = fimo_array_list_with_capacity(edge_count, sizeof(FimoU64), alignof(FimoU64), &edge_buffer);
        if (FIMO_IS_ERROR(error)) {
            goto error_edge_buffer_alloc;
        }

        struct CollectEdgesData_ data = {
                .edges = &edge_buffer,
                .error = FIMO_EOK,
        };
        if (n->adjacency) {
            btree_ascend(n->adjacency, NULL, collect_edges_, &data);
            if (FIMO_IS_ERROR(data.error)) {
                goto error_collect_edges;
            }
        }
        if (n->inv_adjacency) {
            btree_ascend(n->inv_adjacency, NULL, collect_edges_, &data);
            if (FIMO_IS_ERROR(data.error)) {
                goto error_collect_edges;
            }
        }

        while (!fimo_array_list_is_empty(&edge_buffer)) {
            FimoU64 edge;
            FIMO_IGNORE(fimo_array_list_pop_back(&edge_buffer, sizeof(FimoU64), &edge, NULL));
            void *edge_data = NULL;
            error = fimo_graph_remove_edge(graph, edge, &edge_data);
            if (FIMO_IS_ERROR(error)) {
                goto error_remove_edge;
            }
            fimo_free_sized(edge_data, graph->edge_size);
        }

        fimo_array_list_free(&edge_buffer, sizeof(FimoU64), alignof(FimoU64), NULL);
        goto success_edges_removed;

    error_remove_edge:;
        perror("critical error while removing the edges from the graph");
        exit(EXIT_FAILURE);
    error_collect_edges:
        fimo_array_list_free(&edge_buffer, sizeof(FimoU64), alignof(FimoU64), NULL);
        goto error_edge_buffer_alloc;
    }
success_edges_removed:;

    n = btree_delete(graph->nodes, &(struct FimoGraphNode_){
                                           .key = node,
                                   });
    *node_data = n->data;

    error = fimo_array_list_push(&graph->node_free_list, sizeof(FimoU64), alignof(FimoU64), &node, NULL);

    return error;

error_edge_buffer_alloc:;
error_edge_free_list_resize:;
error_node_free_list_resize:;
error_node_not_found:
    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_remove_edge(FimoGraph *graph, FimoU64 edge, void **edge_data) {
    if (graph == NULL || edge_data == NULL) {
        return FIMO_EINVAL;
    }

    FimoError error = fimo_array_list_reserve(&graph->edge_free_list, sizeof(FimoU64), alignof(FimoU64), 1, NULL);
    if (FIMO_IS_ERROR(error)) {
        goto error_free_list_resize;
    }

    const struct FimoGraphEdge_ *e = btree_delete(graph->edges, &(struct FimoGraphEdge_){
                                                                        .key = edge,
                                                                });
    if (e == NULL) {
        error = FIMO_EINVAL;
        goto error_edge_delete;
    }

    FimoU64 src = e->src;
    FimoU64 dst = e->dst;
    *edge_data = e->data;

    FIMO_IGNORE(fimo_array_list_push(&graph->edge_free_list, sizeof(FimoU64), alignof(FimoU64), &edge, NULL));

    struct FimoGraphNode_ *src_node = (void *)btree_get(graph->nodes, &(struct FimoGraphNode_){
                                                                              .key = src,
                                                                      });
    btree_delete(src_node->adjacency, &(struct FimoGraphNodeAdj_){
                                              .key = dst,
                                      });
    if (btree_count(src_node->adjacency) == 0) {
        btree_free(src_node->adjacency);
        src_node->adjacency = NULL;
    }

    struct FimoGraphNode_ *dst_node = (void *)btree_get(graph->nodes, &(struct FimoGraphNode_){
                                                                              .key = dst,
                                                                      });
    btree_delete(dst_node->inv_adjacency, &(struct FimoGraphNodeAdj_){
                                                  .key = src,
                                          });
    if (btree_count(dst_node->inv_adjacency) == 0) {
        btree_free(dst_node->inv_adjacency);
        dst_node->inv_adjacency = NULL;
    }

    return FIMO_EOK;

error_edge_delete:;
error_free_list_resize:
    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_contains_edge(const FimoGraph *graph, FimoU64 src_node, FimoU64 dst_node, bool *contained) {
    FimoU64 edge;
    return fimo_graph_find_edge(graph, src_node, dst_node, &edge, contained);
}

FIMO_MUST_USE
FimoError fimo_graph_find_edge(const FimoGraph *graph, FimoU64 src_node, FimoU64 dst_node, FimoU64 *edge,
                               bool *contained) {
    if (graph == NULL || edge == NULL || contained == NULL) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNode_ *src = (void *)btree_get(graph->nodes, &(struct FimoGraphNode_){
                                                                               .key = src_node,
                                                                       });
    if (src == NULL) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNode_ *dst = btree_get(graph->nodes, &(struct FimoGraphNode_){
                                                                       .key = dst_node,
                                                               });
    if (dst == NULL) {
        return FIMO_EINVAL;
    }

    if (src->adjacency == NULL) {
        *contained = false;
        return FIMO_EOK;
    }

    const struct FimoGraphNodeAdj_ *adj = btree_get(src->adjacency, &(struct FimoGraphNodeAdj_){
                                                                            .key = dst_node,
                                                                    });
    if (adj) {
        *edge = adj->edge;
        *contained = true;
    }
    else {
        *contained = false;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_nodes_new(const FimoGraph *graph, FimoGraphNodes **iter, bool *has_value) {
    if (graph == NULL || iter == NULL || has_value == NULL) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    FimoGraphNodes *tmp_iter = fimo_aligned_alloc(_Alignof(FimoGraphNodes), sizeof(FimoGraphNodes), &error);
    if (FIMO_IS_ERROR(error)) {
        goto error_iter_alloc;
    }

    if (btree_count(graph->nodes) == 0) {
        tmp_iter->iter = NULL;
        tmp_iter->has_value = false;
        *iter = tmp_iter;
        *has_value = tmp_iter->has_value;
        return FIMO_EOK;
    }

    tmp_iter->iter = btree_iter_new(graph->nodes);
    if (tmp_iter->iter == NULL) {
        error = FIMO_ENOMEM;
        goto error_nodes_iter;
    }
    tmp_iter->has_value = btree_iter_first(tmp_iter->iter);

    *iter = tmp_iter;
    *has_value = tmp_iter->has_value;

    return FIMO_EOK;

error_nodes_iter:
    fimo_free_aligned_sized(tmp_iter, _Alignof(FimoGraphNodes), sizeof(FimoGraphNodes));
error_iter_alloc:
    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_nodes_next(FimoGraphNodes *iter, bool *has_value) {
    if (iter == NULL || has_value == NULL || !iter->has_value) {
        return FIMO_EINVAL;
    }

    iter->has_value = btree_iter_next(iter->iter);
    if (!iter->has_value) {
        btree_iter_free(iter->iter);
        iter->iter = NULL;
    }
    *has_value = iter->has_value;

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_nodes_item(const FimoGraphNodes *iter, FimoU64 *node, const void **node_data) {
    if (iter == NULL || !iter->has_value) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNode_ *n = btree_iter_item(iter->iter);
    if (node) {
        *node = n->key;
    }
    if (node_data) {
        *node_data = n->data;
    }

    return FIMO_EOK;
}

void fimo_graph_nodes_free(FimoGraphNodes *iter) {
    if (iter == NULL) {
        perror("invalid nodes iter");
        exit(EXIT_FAILURE);
    }

    if (iter->iter) {
        btree_iter_free(iter->iter);
    }
    fimo_free_aligned_sized(iter, _Alignof(FimoGraphNodes), sizeof(FimoGraphNodes));
}

FIMO_MUST_USE
FimoError fimo_graph_edges_new(const FimoGraph *graph, FimoGraphEdges **iter, bool *has_value) {
    if (graph == NULL || iter == NULL || has_value == NULL) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    FimoGraphEdges *tmp_iter = fimo_aligned_alloc(_Alignof(FimoGraphEdges), sizeof(FimoGraphEdges), &error);
    if (FIMO_IS_ERROR(error)) {
        goto error_iter_alloc;
    }

    if (btree_count(graph->edges) == 0) {
        tmp_iter->iter = NULL;
        tmp_iter->has_value = false;
        *iter = tmp_iter;
        *has_value = false;
        return FIMO_EOK;
    }

    tmp_iter->iter = btree_iter_new(graph->edges);
    if (tmp_iter->iter == NULL) {
        error = FIMO_ENOMEM;
        goto error_edges_iter;
    }
    tmp_iter->has_value = btree_iter_first(tmp_iter->iter);

    *iter = tmp_iter;
    *has_value = tmp_iter->has_value;

    return FIMO_EOK;

error_edges_iter:
    fimo_free_aligned_sized(tmp_iter, _Alignof(FimoGraphEdges), sizeof(FimoGraphEdges));
error_iter_alloc:
    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_edges_next(FimoGraphEdges *iter, bool *has_value) {
    if (iter == NULL || has_value == NULL || !iter->has_value) {
        return FIMO_EINVAL;
    }

    iter->has_value = btree_iter_next(iter->iter);
    if (!iter->has_value) {
        btree_iter_free(iter->iter);
        iter->iter = NULL;
    }
    *has_value = iter->has_value;

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_edges_item(const FimoGraphEdges *iter, FimoU64 *edge, const void **edge_data) {
    if (iter == NULL || !iter->has_value) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphEdge_ *e = btree_iter_item(iter->iter);
    if (edge) {
        *edge = e->key;
    }
    if (edge_data) {
        *edge_data = e->data;
    }

    return FIMO_EOK;
}

void fimo_graph_edges_free(FimoGraphEdges *iter) {
    if (iter == NULL) {
        perror("invalid edges iter");
        exit(EXIT_FAILURE);
    }

    if (iter->iter) {
        btree_iter_free(iter->iter);
    }
    fimo_free_aligned_sized(iter, _Alignof(FimoGraphEdges), sizeof(FimoGraphEdges));
}

FIMO_MUST_USE
FimoError fimo_graph_externals_new(const FimoGraph *graph, bool sink, FimoGraphExternals **iter, bool *has_value) {
    if (graph == NULL || iter == NULL || has_value == NULL) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    FimoGraphExternals *tmp_iter = fimo_aligned_alloc(_Alignof(FimoGraphExternals), sizeof(FimoGraphExternals), &error);
    if (FIMO_IS_ERROR(error)) {
        goto error_iter_alloc;
    }

    tmp_iter->sink = sink;
    if (btree_count(graph->nodes) == 0) {
        tmp_iter->iter = NULL;
        tmp_iter->has_value = false;
        *iter = tmp_iter;
        *has_value = false;
        return FIMO_EOK;
    }

    tmp_iter->iter = btree_iter_new(graph->nodes);
    if (tmp_iter->iter == NULL) {
        error = FIMO_ENOMEM;
        goto error_nodes_iter;
    }

    tmp_iter->has_value = btree_iter_first(tmp_iter->iter);
    while (tmp_iter->has_value) {
        const struct FimoGraphNode_ *n = btree_iter_item(tmp_iter->iter);
        if ((sink && (n->adjacency == NULL || btree_count(n->adjacency) == 0)) ||
            (!sink && (n->inv_adjacency == NULL || btree_count(n->inv_adjacency) == 0))) {
            break;
        }
        tmp_iter->has_value = btree_iter_next(tmp_iter->iter);
    }

    if (!tmp_iter->has_value) {
        btree_iter_free(tmp_iter->iter);
        tmp_iter->iter = NULL;
    }

    *iter = tmp_iter;
    *has_value = tmp_iter->has_value;

    return FIMO_EOK;

error_nodes_iter:
    fimo_free_aligned_sized(tmp_iter, _Alignof(FimoGraphExternals), sizeof(FimoGraphExternals));
error_iter_alloc:
    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_externals_next(FimoGraphExternals *iter, bool *has_value) {
    if (iter == NULL || has_value == NULL || !iter->has_value) {
        return FIMO_EINVAL;
    }

    iter->has_value = btree_iter_next(iter->iter);
    while (iter->has_value) {
        const struct FimoGraphNode_ *n = btree_iter_item(iter->iter);
        if ((iter->sink && (n->adjacency == NULL || btree_count(n->adjacency) == 0)) ||
            (!iter->sink && (n->inv_adjacency == NULL || btree_count(n->inv_adjacency) == 0))) {
            break;
        }
        iter->has_value = btree_iter_next(iter->iter);
    }

    if (!iter->has_value) {
        btree_iter_free(iter->iter);
        iter->iter = NULL;
    }

    *has_value = iter->has_value;

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_externals_item(const FimoGraphExternals *iter, FimoU64 *node, const void **node_data) {
    if (iter == NULL || !iter->has_value) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNode_ *n = btree_iter_item(iter->iter);
    if (node) {
        *node = n->key;
    }
    if (node_data) {
        *node_data = n->data;
    }

    return FIMO_EOK;
}

void fimo_graph_externals_free(FimoGraphExternals *iter) {
    if (iter == NULL) {
        perror("invalid neighbors iterator");
        exit(EXIT_FAILURE);
    }

    if (iter->iter) {
        btree_iter_free(iter->iter);
    }
    fimo_free_aligned_sized(iter, _Alignof(FimoGraphExternals), sizeof(FimoGraphExternals));
}

FIMO_MUST_USE
FimoError fimo_graph_neighbors_new(const FimoGraph *graph, FimoU64 node, bool inward, FimoGraphNeighbors **iter,
                                   bool *has_value) {
    if (graph == NULL || iter == NULL || has_value == NULL) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    const struct FimoGraphNode_ *n = btree_get(graph->nodes, &(struct FimoGraphNode_){
                                                                     .key = node,
                                                             });
    if (n == NULL) {
        error = FIMO_EINVAL;
        goto error_node_not_found;
    }

    FimoGraphNeighbors *tmp_iter = fimo_aligned_alloc(_Alignof(FimoGraphNeighbors), sizeof(FimoGraphNeighbors), &error);
    if (FIMO_IS_ERROR(error)) {
        goto error_iter_alloc;
    }

    const struct btree *neighbors = n->adjacency;
    if (inward) {
        neighbors = n->inv_adjacency;
    }

    if (neighbors == NULL || btree_count(neighbors) == 0) {
        tmp_iter->has_value = false;
        tmp_iter->iter = NULL;
        *iter = tmp_iter;
        *has_value = false;
        return FIMO_EOK;
    }

    tmp_iter->iter = btree_iter_new(neighbors);
    if (tmp_iter->iter == NULL) {
        error = FIMO_ENOMEM;
        goto error_neighbors_iter_alloc;
    }
    tmp_iter->has_value = btree_iter_first(tmp_iter->iter);
    *iter = tmp_iter;
    *has_value = tmp_iter->has_value;

    return FIMO_EOK;

error_neighbors_iter_alloc:;
    fimo_free_aligned_sized(tmp_iter, _Alignof(FimoGraphNeighbors), sizeof(FimoGraphNeighbors));
error_iter_alloc:;
error_node_not_found:
    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_neighbors_next(FimoGraphNeighbors *iter, bool *has_value) {
    if (iter == NULL || has_value == NULL || !iter->has_value) {
        return FIMO_EINVAL;
    }

    bool has_next = btree_iter_next(iter->iter);
    iter->has_value = has_next;
    *has_value = has_next;

    if (!has_next) {
        btree_iter_free(iter->iter);
        iter->iter = NULL;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_neighbors_item(const FimoGraphNeighbors *iter, FimoU64 *node) {
    if (iter == NULL || !iter->has_value || node == NULL) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNodeAdj_ *n = btree_iter_item(iter->iter);
    *node = n->key;

    return FIMO_EOK;
}

void fimo_graph_neighbors_free(FimoGraphNeighbors *iter) {
    if (iter == NULL) {
        perror("invalid neighbors iterator");
        exit(EXIT_FAILURE);
    }

    if (iter->iter) {
        btree_iter_free(iter->iter);
    }
    fimo_free_aligned_sized(iter, _Alignof(FimoGraphNeighbors), sizeof(FimoGraphNeighbors));
}

FIMO_MUST_USE
FimoError fimo_graph_neighbors_edges_new(const FimoGraph *graph, FimoU64 node, bool inward,
                                         FimoGraphNeighborsEdges **iter, bool *has_value) {
    if (graph == NULL || iter == NULL || has_value == NULL) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    const struct FimoGraphNode_ *n = btree_get(graph->nodes, &(struct FimoGraphNode_){
                                                                     .key = node,
                                                             });
    if (n == NULL) {
        error = FIMO_EINVAL;
        goto error_node_not_found;
    }

    FimoGraphNeighborsEdges *tmp_iter =
            fimo_aligned_alloc(_Alignof(FimoGraphNeighborsEdges), sizeof(FimoGraphNeighborsEdges), &error);
    if (FIMO_IS_ERROR(error)) {
        goto error_iter_alloc;
    }

    const struct btree *neighbors = n->adjacency;
    if (inward) {
        neighbors = n->inv_adjacency;
    }

    if (neighbors == NULL || btree_count(neighbors) == 0) {
        tmp_iter->has_value = false;
        tmp_iter->iter = NULL;
        *iter = tmp_iter;
        *has_value = false;
        return FIMO_EOK;
    }

    tmp_iter->iter = btree_iter_new(neighbors);
    if (tmp_iter->iter == NULL) {
        error = FIMO_ENOMEM;
        goto error_neighbors_iter_alloc;
    }
    tmp_iter->has_value = btree_iter_first(tmp_iter->iter);
    *iter = tmp_iter;
    *has_value = tmp_iter->has_value;

    return FIMO_EOK;

error_neighbors_iter_alloc:;
    fimo_free_aligned_sized(tmp_iter, _Alignof(FimoGraphNeighborsEdges), sizeof(FimoGraphNeighborsEdges));
error_iter_alloc:;
error_node_not_found:
    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_neighbors_edges_next(FimoGraphNeighborsEdges *iter, bool *has_value) {
    if (iter == NULL || has_value == NULL || !iter->has_value) {
        return FIMO_EINVAL;
    }

    bool has_next = btree_iter_next(iter->iter);
    iter->has_value = has_next;
    *has_value = has_next;

    if (!has_next) {
        btree_iter_free(iter->iter);
        iter->iter = NULL;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_neighbors_edges_item(const FimoGraphNeighborsEdges *iter, FimoU64 *edge) {
    if (iter == NULL || !iter->has_value || edge == NULL) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNodeAdj_ *n = btree_iter_item(iter->iter);
    *edge = n->edge;

    return FIMO_EOK;
}

void fimo_graph_neighbors_edges_free(FimoGraphNeighborsEdges *iter) {
    if (iter == NULL) {
        perror("invalid neighbors iterator");
        exit(EXIT_FAILURE);
    }

    if (iter->iter) {
        btree_iter_free(iter->iter);
    }
    fimo_free_aligned_sized(iter, _Alignof(FimoGraphNeighborsEdges), sizeof(FimoGraphNeighborsEdges));
}

FIMO_MUST_USE
FimoError fimo_graph_clear(FimoGraph *graph) {
    if (graph == NULL) {
        return FIMO_EINVAL;
    }

    if (btree_count(graph->nodes) == 0 && btree_count(graph->edges) == 0) {
        return FIMO_EOK;
    }

    btree_ascend(graph->nodes, NULL, node_free_, graph);
    btree_clear(graph->nodes);

    btree_ascend(graph->edges, NULL, edge_free_, graph);
    btree_clear(graph->edges);

    FIMO_IGNORE(fimo_array_list_set_capacity(&graph->node_free_list, sizeof(FimoU64), alignof(FimoU64), 0, NULL, NULL));
    graph->next_node_idx = 0;

    FIMO_IGNORE(fimo_array_list_set_capacity(&graph->edge_free_list, sizeof(FimoU64), alignof(FimoU64), 0, NULL, NULL));
    graph->next_edge_idx = 0;

    return FIMO_EOK;
}

static bool node_clear_edges_(const void *item, void *data) {
    (void)data;
    struct FimoGraphNode_ *node = (void *)item;
    if (node->adjacency) {
        btree_free(node->adjacency);
        node->adjacency = NULL;
    }
    if (node->inv_adjacency) {
        btree_free(node->inv_adjacency);
        node->inv_adjacency = NULL;
    }
    return true;
}

FIMO_MUST_USE
FimoError fimo_graph_clear_edges(FimoGraph *graph) {
    if (graph == NULL) {
        return FIMO_EINVAL;
    }

    if (btree_count(graph->edges) == 0) {
        return FIMO_EOK;
    }

    btree_ascend(graph->nodes, NULL, node_clear_edges_, NULL);

    btree_ascend(graph->edges, NULL, edge_free_, graph);
    btree_clear(graph->edges);

    FIMO_IGNORE(fimo_array_list_set_capacity(&graph->edge_free_list, sizeof(FimoU64), alignof(FimoU64), 0, NULL, NULL));
    graph->next_edge_idx = 0;

    return FIMO_EOK;
}

static bool invert_node_edge_(const void *item, void *data) {
    (void)data;
    struct FimoGraphNode_ *n = (void *)item;
    struct btree *tmp = n->adjacency;
    n->adjacency = n->inv_adjacency;
    n->inv_adjacency = tmp;
    return true;
}

static bool invert_edge_(const void *item, void *data) {
    (void)data;
    struct FimoGraphEdge_ *e = (void *)item;
    FimoU64 tmp = e->src;
    e->src = e->dst;
    e->dst = tmp;
    return true;
}

FIMO_MUST_USE
FimoError fimo_graph_reverse(FimoGraph *graph) {
    if (graph == NULL) {
        return FIMO_EINVAL;
    }

    btree_ascend(graph->nodes, NULL, invert_node_edge_, NULL);
    btree_ascend(graph->edges, NULL, invert_edge_, NULL);

    return FIMO_EOK;
}

struct IndexMap_ {
    FimoU64 key;
    FimoU64 mapped;
};

struct GraphCloneData_ {
    FimoGraph *g;
    struct btree *node_map;
    FimoError (*node_mapper)(FimoU64, FimoU64, void *);
    FimoError (*edge_mapper)(FimoU64, FimoU64, void *);
    void *user_data;
    FimoError error;
};

static int index_map_compare_(const void *a, const void *b, void *data) {
    (void)data;
    const struct IndexMap_ *ma = a;
    const struct IndexMap_ *mb = b;

    FimoI64 x = (FimoI64)(ma->key - mb->key);
    if (x < 0) {
        return -1;
    }
    else if (x > 0) {
        return 1;
    }
    return 0;
}

static bool node_clone_(const void *item, void *data) {
    struct GraphCloneData_ *d = data;
    const struct FimoGraphNode_ *n = item;

    FimoU64 node = (FimoU64)-1;
    d->error = fimo_graph_add_node(d->g, n->data, &node);
    if (FIMO_IS_ERROR(d->error)) {
        return false;
    }

    btree_set(d->node_map, &(struct IndexMap_){
                                   .key = n->key,
                                   .mapped = node,
                           });
    if (btree_oom(d->node_map)) {
        d->error = FIMO_ENOMEM;
        return false;
    }

    if (d->node_mapper) {
        d->error = d->node_mapper(n->key, node, d->user_data);
        if (FIMO_IS_ERROR(d->error)) {
            return false;
        }
    }

    return true;
}

static bool edge_clone_(const void *item, void *data) {
    struct GraphCloneData_ *d = data;
    const struct FimoGraphEdge_ *e = item;

    const struct IndexMap_ *src = btree_get(d->node_map, &(struct IndexMap_){
                                                                 .key = e->src,
                                                         });
    const struct IndexMap_ *dst = btree_get(d->node_map, &(struct IndexMap_){
                                                                 .key = e->dst,
                                                         });

    FimoU64 src_node = src->mapped;
    FimoU64 dst_node = dst->mapped;

    FimoU64 edge = (FimoU64)-1;
    d->error = fimo_graph_add_edge(d->g, src_node, dst_node, e->data, NULL, &edge);
    if (FIMO_IS_ERROR(d->error)) {
        return false;
    }

    if (d->edge_mapper) {
        d->error = d->edge_mapper(e->key, edge, d->user_data);
        if (FIMO_IS_ERROR(d->error)) {
            return false;
        }
    }

    return true;
}

FIMO_MUST_USE
FimoError fimo_graph_clone(const FimoGraph *graph, FimoGraph **new_graph,
                           FimoError (*node_mapper)(FimoU64, FimoU64, void *),
                           FimoError (*edge_mapper)(FimoU64, FimoU64, void *), void *user_data) {
    if (graph == NULL || new_graph == NULL) {
        return FIMO_EINVAL;
    }

    FimoGraph *g = NULL;
    FimoError error = fimo_graph_new(graph->node_size, graph->edge_size, graph->node_free, graph->edge_free, &g);
    if (FIMO_IS_ERROR(error)) {
        goto error_graph_alloc;
    }

    struct btree *node_map =
            btree_new_with_allocator(malloc_, realloc_, free_, sizeof(struct IndexMap_), 0, index_map_compare_, NULL);
    if (node_map == NULL) {
        error = FIMO_ENOMEM;
        goto error_node_map_alloc;
    }

    struct GraphCloneData_ clone_data = {
            .g = g,
            .node_map = node_map,
            .node_mapper = node_mapper,
            .edge_mapper = edge_mapper,
            .user_data = user_data,
            .error = FIMO_EOK,
    };
    btree_ascend(graph->nodes, NULL, node_clone_, &clone_data);
    if (FIMO_IS_ERROR(clone_data.error)) {
        error = clone_data.error;
        goto error_nodes_clone;
    }

    btree_ascend(graph->edges, NULL, edge_clone_, &clone_data);
    if (FIMO_IS_ERROR(clone_data.error)) {
        error = clone_data.error;
        goto error_edges_clone;
    }

    btree_free(node_map);
    *new_graph = g;

    return FIMO_EOK;

error_edges_clone:;
error_nodes_clone:
    btree_free(node_map);
error_node_map_alloc:
    fimo_graph_free(g);
error_graph_alloc:
    return error;
}

struct ReachableSubgraphData_ {
    FimoArrayList *node_stack;
    struct btree *node_map;
    const FimoGraph *graph;
    FimoGraph *sub_graph;
    FimoError (*node_mapper)(FimoU64, FimoU64, void *);
    FimoError (*edge_mapper)(FimoU64, FimoU64, void *);
    void *user_data;
    FimoU64 current_node;
    FimoError error;
};

static int index_compare_(const void *a, const void *b, void *data) {
    (void)data;
    const FimoU64 *ia = a;
    const FimoU64 *ib = b;

    FimoI64 x = (FimoI64)(*ia - *ib);
    if (x < 0) {
        return -1;
    }
    else if (x > 0) {
        return 1;
    }
    return 0;
}

static bool clone_adjacency_(const void *item, void *data) {
    const struct FimoGraphNodeAdj_ *adj = item;
    struct ReachableSubgraphData_ *d = data;

    const void *edge_data;
    d->error = fimo_graph_edge_data(d->graph, adj->edge, &edge_data);
    if (FIMO_IS_ERROR(d->error)) {
        return false;
    }

    const struct IndexMap_ *src_node_mapping = btree_get(d->node_map, &(struct IndexMap_){
                                                                              .key = d->current_node,
                                                                      });
    FimoU64 mapped_src_node = src_node_mapping->mapped;

    FimoU64 mapped_dst_node = (FimoU64)-1;
    const struct IndexMap_ *dst_node_mapping = btree_get(d->node_map, &(struct IndexMap_){
                                                                              .key = adj->key,
                                                                      });
    if (dst_node_mapping == NULL) {
        const void *node_data;
        d->error = fimo_graph_node_data(d->graph, adj->key, &node_data);
        if (FIMO_IS_ERROR(d->error)) {
            return false;
        }

        d->error = fimo_graph_add_node(d->sub_graph, node_data, &mapped_dst_node);
        if (FIMO_IS_ERROR(d->error)) {
            return false;
        }

        if (d->node_mapper) {
            d->node_mapper(adj->key, mapped_dst_node, d->user_data);
        }

        btree_set(d->node_map, &(struct IndexMap_){
                                       .key = adj->key,
                                       .mapped = mapped_dst_node,
                               });
        if (btree_oom(d->node_map)) {
            d->error = FIMO_ENOMEM;
            return false;
        }

        d->error = fimo_array_list_push(d->node_stack, sizeof(FimoU64), alignof(FimoU64), (void *)&adj->key, NULL);
        if (FIMO_IS_ERROR(d->error)) {
            return false;
        }
    }
    else {
        mapped_dst_node = dst_node_mapping->mapped;
    }

    FimoU64 mapped_edge = (FimoU64)-1;
    d->error = fimo_graph_add_edge(d->sub_graph, mapped_src_node, mapped_dst_node, edge_data, NULL, &mapped_edge);

    if (d->edge_mapper) {
        d->edge_mapper(adj->edge, mapped_edge, d->user_data);
    }

    return true;
}

FIMO_MUST_USE
FimoError fimo_graph_clone_reachable_subgraph(const FimoGraph *graph, FimoGraph **sub_graph, FimoU64 start_node,
                                              FimoError (*node_mapper)(FimoU64, FimoU64, void *),
                                              FimoError (*edge_mapper)(FimoU64, FimoU64, void *), void *user_data) {
    if (graph == NULL || sub_graph == NULL) {
        return FIMO_EINVAL;
    }

    FimoError error;
    const struct FimoGraphNode_ *start = btree_get(graph->nodes, &(struct FimoGraphNode_){
                                                                         .key = start_node,
                                                                 });
    if (start == NULL) {
        error = FIMO_EINVAL;
        goto error_start_node_not_found;
    }

    FimoArrayList node_stack;
    error = fimo_array_list_with_capacity(1, sizeof(FimoU64), alignof(FimoU64), &node_stack);
    if (FIMO_IS_ERROR(error)) {
        goto error_node_stack_alloc;
    }

    error = fimo_array_list_push(&node_stack, sizeof(FimoU64), alignof(FimoU64), &start_node, NULL);
    if (FIMO_IS_ERROR(error)) {
        goto error_node_stack_init;
    }

    struct btree *node_map =
            btree_new_with_allocator(malloc_, realloc_, free_, sizeof(struct IndexMap_), 0, index_compare_, NULL);
    if (node_map == NULL) {
        error = FIMO_ENOMEM;
        goto error_node_map_alloc;
    }

    FimoGraph *g;
    error = fimo_graph_new(graph->node_size, graph->edge_size, graph->node_free, graph->edge_free, &g);
    if (FIMO_IS_ERROR(error)) {
        goto error_sub_graph_alloc;
    }

    FimoU64 mapped = (FimoU64)-1;
    error = fimo_graph_add_node(g, start->data, &mapped);
    if (FIMO_IS_ERROR(error)) {
        goto error_construct_sub_graph;
    }

    if (node_mapper) {
        error = node_mapper(start_node, mapped, user_data);
        if (FIMO_IS_ERROR(error)) {
            goto error_construct_sub_graph;
        }
    }

    btree_set(node_map, &(struct IndexMap_){
                                .key = start_node,
                                .mapped = mapped,
                        });
    if (btree_oom(node_map)) {
        error = FIMO_ENOMEM;
        goto error_construct_sub_graph;
    }

    struct ReachableSubgraphData_ reachable_data = {
            .node_stack = &node_stack,
            .node_map = node_map,
            .graph = graph,
            .sub_graph = g,
            .node_mapper = node_mapper,
            .edge_mapper = edge_mapper,
            .user_data = user_data,
            .error = FIMO_EOK,
    };
    while (!fimo_array_list_is_empty(&node_stack)) {
        FimoU64 node;
        error = fimo_array_list_pop_back(&node_stack, sizeof(FimoU64), &node, NULL);
        if (FIMO_IS_ERROR(error)) {
            goto error_construct_sub_graph;
        }

        reachable_data.current_node = node;
        const struct FimoGraphNode_ *n = btree_get(graph->nodes, &(struct FimoGraphNode_){
                                                                         .key = node,
                                                                 });
        if (n->adjacency) {
            btree_ascend(n->adjacency, NULL, clone_adjacency_, &reachable_data);
            if (FIMO_IS_ERROR(reachable_data.error)) {
                goto error_construct_sub_graph;
            }
        }
    }

    *sub_graph = g;

    btree_free(node_map);
    fimo_array_list_free(&node_stack, sizeof(FimoU64), alignof(FimoU64), NULL);

    return FIMO_EOK;

error_construct_sub_graph:;
    fimo_graph_free(g);
error_sub_graph_alloc:
    btree_free(node_map);
error_node_map_alloc:;
error_node_stack_init:
    fimo_array_list_free(&node_stack, sizeof(FimoU64), alignof(FimoU64), NULL);
error_node_stack_alloc:;
error_start_node_not_found:
    return error;
}

struct PathExistsNodeMapperData_ {
    FimoU64 old_start;
    FimoU64 old_end;
    FimoU64 new_start;
    bool end_contained;
};

static FimoError path_exists_node_mapper_(FimoU64 old, FimoU64 new, void *data) {
    struct PathExistsNodeMapperData_ *data_ = data;
    if (old == data_->old_start) {
        data_->new_start = new;
    }
    if (old == data_->old_end) {
        data_->end_contained = true;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_path_exists(const FimoGraph *graph, FimoU64 start_node, FimoU64 end_node, bool *path_exists) {
    if (graph == NULL || path_exists == NULL) {
        return FIMO_EINVAL;
    }

    FimoGraph *subgraph = NULL;
    struct PathExistsNodeMapperData_ data = {
            .old_start = start_node,
            .old_end = end_node,
            .new_start = 0,
            .end_contained = false,
    };
    FimoError error =
            fimo_graph_clone_reachable_subgraph(graph, &subgraph, start_node, path_exists_node_mapper_, NULL, &data);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }

    if (start_node != end_node) {
        *path_exists = data.end_contained;
        return FIMO_EOK;
    }

    FimoUSize count = 0;
    error = fimo_graph_neighbors_count(subgraph, data.new_start, true, &count);
    if (FIMO_IS_ERROR(error)) {
        return error;
    }

    *path_exists = count != 0;
    return FIMO_EOK;
}

struct IsCyclicData_ {
    struct btree *nodes;
    struct btree *discovered;
    struct btree *finished;
    bool is_cyclic;
    FimoError error;
};

static bool is_cyclic_helper_(const void *item, void *data);

static bool is_cyclic_(const void *item, void *data) {
    const struct FimoGraphNode_ *node = item;
    struct IsCyclicData_ *d = data;

    if (btree_get(d->discovered, &node->key) == NULL && btree_get(d->finished, &node->key) == NULL) {
        btree_set(d->discovered, &node->key);
        if (btree_oom(d->discovered)) {
            d->error = FIMO_ENOMEM;
            return false;
        }

        if (node->adjacency != NULL) {
            btree_ascend(node->adjacency, NULL, is_cyclic_helper_, d);
            if (d->is_cyclic || FIMO_IS_ERROR(d->error)) {
                return false;
            }
        }

        btree_delete(d->discovered, &node->key);
        if (btree_oom(d->discovered)) {
            d->error = FIMO_ENOMEM;
            return false;
        }
        btree_set(d->finished, &node->key);
        if (btree_oom(d->finished)) {
            d->error = FIMO_ENOMEM;
            return false;
        }
    }

    return true;
}

static bool is_cyclic_helper_(const void *item, void *data) {
    const struct FimoGraphNodeAdj_ *adj = item;
    struct IsCyclicData_ *d = data;

    if (btree_get(d->discovered, &adj->key) != NULL) {
        d->is_cyclic = true;
        return false;
    }

    if (btree_get(d->finished, &adj->key) == NULL) {
        const struct FimoGraphNode_ *node = btree_get(d->nodes, &(struct FimoGraphNode_){
                                                                        .key = adj->key,
                                                                });
        is_cyclic_(node, d);
        if (d->is_cyclic || FIMO_IS_ERROR(d->error)) {
            return false;
        }
    }

    return true;
}

static int is_cyclic_compare_(const void *a, const void *b, void *data) {
    (void)data;
    FimoU64 a_ = *(const FimoU64 *)a;
    FimoU64 b_ = *(const FimoU64 *)b;

    if (a_ < b_) {
        return -1;
    }
    if (a_ > b_) {
        return 1;
    }
    return 0;
}

FIMO_MUST_USE
FimoError fimo_graph_is_cyclic(const FimoGraph *graph, bool *is_cyclic) {
    if (graph == NULL || is_cyclic == NULL) {
        return FIMO_EINVAL;
    }

    struct btree *discovered =
            btree_new_with_allocator(malloc_, realloc_, free_, sizeof(FimoU64), 0, is_cyclic_compare_, NULL);
    if (discovered == NULL) {
        return FIMO_ENOMEM;
    }

    struct btree *finished =
            btree_new_with_allocator(malloc_, realloc_, free_, sizeof(FimoU64), 0, is_cyclic_compare_, NULL);
    if (finished == NULL) {
        btree_free(discovered);
        return FIMO_ENOMEM;
    }

    struct IsCyclicData_ data = {
            .nodes = graph->nodes,
            .discovered = discovered,
            .finished = finished,
            .is_cyclic = false,
            .error = FIMO_EOK,
    };
    btree_ascend(graph->nodes, NULL, is_cyclic_, &data);
    btree_free(finished);
    btree_free(discovered);

    *is_cyclic = data.is_cyclic;
    return data.error;
}

struct TopologicalSortMarker_ {
    FimoU64 node;
    bool permanent;
};

struct TopologicalSortData_ {
    struct btree *nodes;
    struct btree *markers;
    FimoArrayList *order;
    bool inward;
    FimoError error;
};

static bool topological_sort_visit_helper_(const void *item, void *data);

static bool topological_sort_visit_(const void *item, void *data) {
    const struct FimoGraphNode_ *node = item;
    struct TopologicalSortData_ *d = data;

    // Check if there is a cycle or if we can skip the node.
    {
        const struct TopologicalSortMarker_ *marker = btree_get(d->markers, &(struct TopologicalSortMarker_){
                                                                                    .node = node->key,
                                                                            });
        if (marker != NULL) {
            if (marker->permanent) {
                return true;
            }

            // Cycle detected
            d->error = FIMO_EINVAL;
            return false;
        }
    }


    // Mark with a temporary marker.
    {
        btree_set(d->markers, &(struct TopologicalSortMarker_){
                                      .node = node->key,
                                      .permanent = false,
                              });
        if (btree_oom(d->markers)) {
            d->error = FIMO_ENOMEM;
            return false;
        }
    }

    const struct btree *adj = d->inward ? node->inv_adjacency : node->adjacency;
    if (adj != NULL) {
        btree_ascend(adj, NULL, topological_sort_visit_helper_, d);
        if (FIMO_IS_ERROR(d->error)) {
            return false;
        }
    }

    // Mark with a permanent marker.
    struct TopologicalSortMarker_ *marker = (void *)btree_get(d->markers, &(struct TopologicalSortMarker_){
                                                                                  .node = node->key,
                                                                          });
    marker->permanent = true;

    // Append the node to the front.
    d->error = fimo_array_list_insert(d->order, 0, sizeof(FimoU64), _Alignof(FimoU64), (void *)&node->key, NULL);
    if (FIMO_IS_ERROR(d->error)) {
        return false;
    }

    return true;
}

static bool topological_sort_visit_helper_(const void *item, void *data) {
    const struct FimoGraphNodeAdj_ *adj = item;
    struct TopologicalSortData_ *d = data;

    const struct FimoGraphNode_ *node = btree_get(d->nodes, &(struct FimoGraphNode_){
                                                                    .key = adj->key,
                                                            });
    topological_sort_visit_(node, d);
    if (FIMO_IS_ERROR(d->error)) {
        return false;
    }

    return true;
}

static int topological_sort_compare_(const void *a, const void *b, void *data) {
    (void)data;
    const struct TopologicalSortMarker_ *a_ = a;
    const struct TopologicalSortMarker_ *b_ = b;

    if (a_->node < b_->node) {
        return -1;
    }
    if (a_->node > b_->node) {
        return 1;
    }
    return 0;
}

FIMO_MUST_USE
FimoError fimo_graph_topological_sort(const FimoGraph *graph, const bool inward, FimoArrayList *nodes) {
    if (graph == NULL || nodes == NULL) {
        return FIMO_EINVAL;
    }

    struct btree *markers = btree_new_with_allocator(malloc_, realloc_, free_, sizeof(struct TopologicalSortMarker_), 0,
                                                     topological_sort_compare_, NULL);
    if (markers == NULL) {
        return FIMO_ENOMEM;
    }

    FimoError error =
            fimo_array_list_with_capacity(btree_count(graph->nodes), sizeof(FimoU64), _Alignof(FimoU64), nodes);
    if (FIMO_IS_ERROR(error)) {
        btree_free(markers);
        return error;
    }

    struct TopologicalSortData_ data = {
            .nodes = graph->nodes, .markers = markers, .order = nodes, .inward = inward, .error = FIMO_EOK};
    btree_ascend(graph->nodes, NULL, topological_sort_visit_, &data);
    if (FIMO_IS_ERROR(data.error)) {
        fimo_array_list_free(nodes, sizeof(FimoU64), _Alignof(FimoU64), NULL);
        return data.error;
    }

    btree_free(markers);
    return FIMO_EOK;
}
