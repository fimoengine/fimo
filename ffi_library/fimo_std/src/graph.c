#include <fimo_std/graph.h>

#include <fimo_std/array_list.h>
#include <fimo_std/memory.h>

#include <btree/btree.h>
#include <limits.h>
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
    void (*node_free)(void*);
    void (*edge_free)(void*);
    struct btree* nodes;
    struct btree* edges;
    FimoArrayList node_free_list;
    FimoArrayList edge_free_list;
    FimoU64 next_node_idx;
    FimoU64 next_edge_idx;
};

struct FimoGraphNodes {
    struct btree_iter* iter;
    bool has_value;
};

struct FimoGraphEdges {
    struct btree_iter* iter;
    bool has_value;
};

struct FimoGraphExternals {
    struct btree_iter* iter;
    bool has_value;
    bool sink;
};

struct FimoGraphNeighbors {
    struct btree_iter* iter;
    bool has_value;
};

struct FimoGraphNeighborsEdges {
    struct btree_iter* iter;
    bool has_value;
};

struct FimoGraphNode_ {
    FimoU64 key;
    struct btree* adjacency;
    struct btree* inv_adjacency;
    void* data;
};

struct FimoGraphNodeAdj_ {
    FimoU64 key;
    FimoU64 edge;
};

struct FimoGraphEdge_ {
    FimoU64 key;
    FimoU64 src;
    FimoU64 dst;
    void* data;
};

static int fimo_graph_node_compare_(const void* a, const void* b, void* data)
{
    (void)data;
    const struct FimoGraphNode_* na = a;
    const struct FimoGraphNode_* nb = b;

    FimoI64 x = (FimoI64)(na->key - nb->key);
    if (x < 0) {
        return -1;
    } else if (x > 0) {
        return 1;
    }
    return 0;
}

static int fimo_graph_node_adj_compare_(const void* a, const void* b, void* data)
{
    (void)data;
    const struct FimoGraphNodeAdj_* na = a;
    const struct FimoGraphNodeAdj_* nb = b;

    FimoI64 x = (FimoI64)(na->key - nb->key);
    if (x < 0) {
        return -1;
    } else if (x > 0) {
        return 1;
    }
    return 0;
}

static int fimo_graph_edge_compare_(const void* a, const void* b, void* data)
{
    (void)data;
    const struct FimoGraphEdge_* ea = a;
    const struct FimoGraphEdge_* eb = b;

    FimoI64 x = (FimoI64)(ea->key - eb->key);
    if (x < 0) {
        return -1;
    } else if (x > 0) {
        return 1;
    }
    return 0;
}

static bool node_free_(const void* item, void* data)
{
    FimoGraph* graph = data;
    struct FimoGraphNode_* node = (void*)item;
    if (graph->node_free) {
        graph->node_free(node->data);
        fimo_free_sized(node->data, graph->node_size);
        node->data = NULL;
    }
    btree_free(node->adjacency);
    btree_free(node->inv_adjacency);
    return true;
}

static bool edge_free_(const void* item, void* data)
{
    FimoGraph* graph = data;
    struct FimoGraphEdge_* edge = (void*)item;
    if (graph->edge_free) {
        graph->edge_free(edge->data);
        fimo_free_sized(edge->data, graph->edge_size);
        edge->data = NULL;
    }
    return true;
}

static void* fimo_graph_malloc_(size_t size)
{
    return fimo_malloc(size, NULL);
}

static void* fimo_graph_realloc_(void* ptr, size_t size)
{
    if (!ptr) {
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
    void* new_ptr = fimo_malloc(size, &error);
    if (FIMO_IS_ERROR(error)) {
        return NULL;
    }

    // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
    memcpy(new_ptr, ptr, old_size);
    fimo_free(ptr);

    return new_ptr;
}

static void fimo_graph_free_(void* ptr)
{
    fimo_free(ptr);
}

FIMO_MUST_USE
FimoError fimo_graph_new(size_t node_size, size_t edge_size,
    void (*node_free)(void*), void (*edge_free)(void*),
    FimoGraph** graph)
{
    if (!graph) {
        return FIMO_EINVAL;
    }
    if ((node_size > 0 && !node_free) || (node_size == 0 && node_free)) {
        return FIMO_EINVAL;
    }
    if ((edge_size > 0 && !edge_free) || (edge_size == 0 && edge_free)) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    FimoGraph* g = fimo_aligned_alloc(_Alignof(FimoGraph), sizeof(FimoGraph), &error);
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

    struct btree* nodes = btree_new_with_allocator(fimo_graph_malloc_, fimo_graph_realloc_,
        fimo_graph_free_, sizeof(struct FimoGraphNode_), 0, fimo_graph_node_compare_, NULL);
    if (!nodes) {
        error = FIMO_ENOMEM;
        goto error_nodes;
    }
    g->nodes = nodes;

    struct btree* edges = btree_new_with_allocator(fimo_graph_malloc_, fimo_graph_realloc_,
        fimo_graph_free_, sizeof(struct FimoGraphEdge_), 0, fimo_graph_edge_compare_, NULL);
    if (!edges) {
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

void fimo_graph_free(FimoGraph* graph)
{
    if (!graph) {
        perror("graph is null");
        exit(EXIT_FAILURE);
    }

    btree_ascend(graph->nodes, NULL, node_free_, graph);
    btree_free(graph->nodes);

    btree_ascend(graph->edges, NULL, edge_free_, graph);
    btree_free(graph->edges);

    fimo_array_list_free(&graph->node_free_list, sizeof(FimoU64));
    fimo_array_list_free(&graph->edge_free_list, sizeof(FimoU64));

    fimo_free_aligned_sized(graph, _Alignof(FimoGraph),
        sizeof(FimoGraph));
}

FIMO_MUST_USE
size_t fimo_graph_node_count(const FimoGraph* graph)
{
    if (!graph) {
        perror("graph is null");
        exit(EXIT_FAILURE);
    }

    return btree_count(graph->nodes);
}

FIMO_MUST_USE
size_t fimo_graph_edge_count(const FimoGraph* graph)
{
    if (!graph) {
        perror("graph is null");
        exit(EXIT_FAILURE);
    }

    return btree_count(graph->edges);
}

FIMO_MUST_USE
FimoError fimo_graph_add_node(FimoGraph* graph, const void* node_data,
    FimoU64* node)
{
    if (!graph || !node) {
        return FIMO_EINVAL;
    }
    if ((node_data && graph->node_size == 0)
        || (!node_data && graph->node_size != 0)) {
        return FIMO_EINVAL;
    }

    FimoU64 node_idx;
    FimoError error = FIMO_EOK;
    bool node_from_free_list = true;
    if (!fimo_array_list_is_empty(&graph->node_free_list)) {
        error = fimo_array_list_pop_back(&graph->node_free_list,
            sizeof(FimoU64), &node_idx);
        if (FIMO_IS_ERROR(error)) {
            return error;
        }
    } else {
        if (graph->next_node_idx == MAX_GRAPH_IDX_) {
            return FIMO_EINVAL;
        }
        node_idx = graph->next_node_idx++;
        node_from_free_list = false;
    }

    void* node_data_copy = NULL;
    if (node_data) {
        node_data_copy = fimo_malloc(graph->node_size, &error);
        if (FIMO_IS_ERROR(error)) {
            goto error_data_alloc;
        }
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(node_data_copy, node_data, graph->node_size);
    }

    btree_set(graph->nodes, &(struct FimoGraphNode_) {
                                .key = node_idx,
                                .adjacency = NULL,
                                .inv_adjacency = NULL,
                                .data = node_data_copy,
                            });
    if (btree_oom(graph->nodes)) {
        error = FIMO_ENOMEM;
        goto error_node_alloc;
    }

    return FIMO_EOK;

error_node_alloc:
    if (node_data_copy) {
        fimo_free_sized(node_data_copy, graph->node_size);
    }
error_data_alloc:
    if (node_from_free_list) {
        fimo_array_list_push(&graph->node_free_list, sizeof(FimoU64),
            &node_idx);
    } else {
        graph->next_node_idx--;
    }

    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_node_data(const FimoGraph* graph, FimoU64 node,
    const void** node_data)
{
    if (!graph || !node_data) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNode_* n = btree_get(graph->nodes, &(struct FimoGraphNode_) {
                                                                 .key = node,
                                                             });
    if (!n) {
        return FIMO_EINVAL;
    }
    *node_data = n->data;

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_add_edge(FimoGraph* graph, FimoU64 src_node,
    FimoU64 dst_node, const void* edge_data, void** old_edge_data,
    FimoU64* edge)
{
    if (!graph || !edge) {
        return FIMO_EINVAL;
    }
    if ((edge_data && graph->edge_size == 0)
        || (!edge_data && graph->edge_size != 0)) {
        return FIMO_EINVAL;
    }

    struct FimoGraphNode_* src = (void*)btree_get(graph->nodes, &(struct FimoGraphNode_) {
                                                                    .key = src_node,
                                                                });
    if (!src) {
        return FIMO_EINVAL;
    }

    struct FimoGraphNode_* dst = (void*)btree_get(graph->nodes, &(struct FimoGraphNode_) {
                                                                    .key = dst_node,
                                                                });
    if (!dst) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    struct btree* adjacency = src->adjacency;
    if (!adjacency) {
        adjacency = btree_new_with_allocator(fimo_graph_malloc_, fimo_graph_realloc_,
            fimo_graph_free_, sizeof(struct FimoGraphNodeAdj_), 0, fimo_graph_node_adj_compare_, NULL);
        if (!adjacency) {
            error = FIMO_ENOMEM;
            goto error_adjacency;
        }
    }

    struct btree* inv_adjacency = dst->inv_adjacency;
    if (!inv_adjacency) {
        inv_adjacency = btree_new_with_allocator(fimo_graph_malloc_, fimo_graph_realloc_,
            fimo_graph_free_, sizeof(struct FimoGraphNodeAdj_), 0, fimo_graph_node_adj_compare_, NULL);
        if (!inv_adjacency) {
            error = FIMO_ENOMEM;
            goto error_inv_adjacency;
        }
    }

    void* edge_data_copy = NULL;
    if (edge_data) {
        edge_data_copy = fimo_malloc(graph->edge_size, &error);
        if (FIMO_IS_ERROR(error)) {
            goto error_edge_data_alloc;
        }
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(edge_data_copy, edge_data, graph->edge_size);
    }

    const struct FimoGraphNodeAdj_* src_adj = btree_get(adjacency, &(struct FimoGraphNodeAdj_) {
                                                                       .key = dst_node,
                                                                   });
    if (src_adj) {
        FimoU64 edge_idx = src_adj->edge;
        struct FimoGraphEdge_* edge_node = (void*)btree_get(graph->edges, &(struct FimoGraphEdge_) {
                                                                              .key = edge_idx,
                                                                          });
        if (old_edge_data) {
            *old_edge_data = edge_node->data;
        } else {
            edge_free_(edge_node, graph);
        }
        *edge = edge_idx;
        edge_node->data = edge_data_copy;
        return FIMO_EOK;
    }

    FimoU64 edge_idx;
    bool edge_idx_from_free_list = true;
    if (!fimo_array_list_is_empty(&graph->edge_free_list)) {
        error = fimo_array_list_pop_back(&graph->edge_free_list, sizeof(FimoU64), &edge_idx);
        if (FIMO_IS_ERROR(error)) {
            goto error_edge_idx_fetch;
        }
    } else {
        if (graph->next_edge_idx == MAX_GRAPH_IDX_) {
            error = FIMO_EINVAL;
            goto error_edge_idx_fetch;
        }
        edge_idx = graph->next_edge_idx++;
        edge_idx_from_free_list = false;
    }

    btree_set(adjacency, &(struct FimoGraphNodeAdj_) {
                             .key = dst_node,
                             .edge = edge_idx,
                         });
    if (btree_oom(adjacency)) {
        error = FIMO_ENOMEM;
        goto error_adjacency_set;
    }

    btree_set(inv_adjacency, &(struct FimoGraphNodeAdj_) {
                                 .key = src_node,
                                 .edge = edge_idx,
                             });
    if (btree_oom(inv_adjacency)) {
        error = FIMO_ENOMEM;
        goto error_inv_adjacency_set;
    }

    btree_set(graph->edges, &(struct FimoGraphEdge_) {
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
    fimo_free((void*)btree_delete(inv_adjacency, &(struct FimoGraphNodeAdj_) {
                                                     .key = src_node,
                                                 }));
error_inv_adjacency_set:
    fimo_free((void*)btree_delete(adjacency, &(struct FimoGraphNodeAdj_) {
                                                 .key = dst_node,
                                             }));
error_adjacency_set:
    if (edge_idx_from_free_list) {
        (void)fimo_array_list_push(&graph->edge_free_list, sizeof(FimoU64),
            &edge_idx);
    } else {
        graph->next_edge_idx--;
    }
error_edge_idx_fetch:
    if (edge_data_copy) {
        fimo_free_sized(edge_data_copy, graph->edge_size);
    }
error_edge_data_alloc:
    if (inv_adjacency && !dst->inv_adjacency) {
        btree_free(inv_adjacency);
    }
error_inv_adjacency:
    if (adjacency && !src->adjacency) {
        btree_free(adjacency);
    }
error_adjacency:
    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_update_edge(FimoGraph* graph, FimoU64 src_node,
    FimoU64 dst_node, const void* edge_data, void** old_edge_data,
    FimoU64* edge)
{
    if (!graph || !edge) {
        return FIMO_EINVAL;
    }
    if ((edge_data && graph->edge_size == 0)
        || (!edge_data && graph->edge_size != 0)) {
        return FIMO_EINVAL;
    }

    struct FimoGraphNode_* src = (void*)btree_get(graph->nodes, &(struct FimoGraphNode_) {
                                                                    .key = src_node,
                                                                });
    if (!src || !src->adjacency) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNodeAdj_* src_edge = btree_get(src->adjacency, &(struct FimoGraphNodeAdj_) {
                                                                             .key = dst_node,
                                                                         });
    if (!src_edge) {
        return FIMO_EINVAL;
    }

    void* edge_data_copy = NULL;
    if (edge_data) {
        FimoError error = FIMO_EOK;
        edge_data_copy = fimo_malloc(graph->edge_size, &error);
        if (FIMO_IS_ERROR(error)) {
            return error;
        }
        // NOLINTNEXTLINE(clang-analyzer-security.insecureAPI.DeprecatedOrUnsafeBufferHandling)
        memcpy(edge_data_copy, edge_data, graph->edge_size);
    }

    struct FimoGraphEdge_* edge_node = (void*)btree_get(graph->edges, &(struct FimoGraphEdge_) {
                                                                          .key = src_edge->edge,
                                                                      });
    if (old_edge_data) {
        *old_edge_data = edge_node->data;
    } else {
        edge_free_(edge_node, graph);
    }
    edge_node->data = edge_data_copy;
    *edge = src_edge->edge;

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_edge_data(const FimoGraph* graph, FimoU64 edge,
    const void** edge_data)
{
    if (!graph || !edge_data) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphEdge_* e = btree_get(graph->edges, &(struct FimoGraphEdge_) {
                                                                 .key = edge,
                                                             });
    if (!e) {
        return FIMO_EINVAL;
    }
    *edge_data = e->data;

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_edge_endpoints(const FimoGraph* graph, FimoU64 edge,
    FimoU64* start_node, FimoU64* end_node)
{
    if (!graph || !start_node || !end_node) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphEdge_* e = btree_get(graph->edges, &(struct FimoGraphEdge_) {
                                                                 .key = edge,
                                                             });
    if (!e) {
        return FIMO_EINVAL;
    }
    *start_node = e->src;
    *end_node = e->dst;

    return FIMO_EOK;
}

struct CollectEdgesData_ {
    FimoArrayList* edges;
    FimoError error;
};

static bool collect_edges_(const void* item, void* data)
{
    struct CollectEdgesData_* d = data;
    const struct FimoGraphNodeAdj_* adj = item;
    d->error = fimo_array_list_push(d->edges, sizeof(FimoU64), &adj->edge);
    if (FIMO_IS_ERROR(d->error)) {
        return false;
    }
    return true;
}

FIMO_MUST_USE
FimoError fimo_graph_remove_node(FimoGraph* graph, FimoU64 node,
    void** node_data)
{
    if (!graph || !node_data) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    const struct FimoGraphNode_* n = btree_get(graph->nodes, &(struct FimoGraphNode_) {
                                                                 .key = node,
                                                             });
    if (!n) {
        error = FIMO_EINVAL;
        goto error_node_not_found;
    }

    error = fimo_array_list_reserve(&graph->node_free_list, sizeof(FimoU64), 1);
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

    error = fimo_array_list_reserve(&graph->edge_free_list, sizeof(FimoU64),
        edge_count);
    if (FIMO_IS_ERROR(error)) {
        goto error_edge_free_list_resize;
    }

    if (edge_count != 0) {
        FimoArrayList edge_buffer;
        error = fimo_array_list_with_capacity(edge_count, sizeof(FimoU64),
            &edge_buffer);
        if (FIMO_IS_ERROR(error)) {
            goto error_edge_buffer_alloc;
        }

        struct CollectEdgesData_ data = {
            .edges = &edge_buffer,
            .error = FIMO_EOK,
        };
        btree_ascend(n->adjacency, NULL, collect_edges_, &data);
        if (FIMO_IS_ERROR(data.error)) {
            goto error_collect_edges;
        }
        btree_ascend(n->inv_adjacency, NULL, collect_edges_, &data);
        if (FIMO_IS_ERROR(data.error)) {
            goto error_collect_edges;
        }

        while (!fimo_array_list_is_empty(&edge_buffer)) {
            FimoU64 edge;
            (void)fimo_array_list_pop_back(&edge_buffer, sizeof(FimoU64), &edge);
            void* edge_data = NULL;
            error = fimo_graph_remove_edge(graph, edge, &edge_data);
            if (FIMO_IS_ERROR(error)) {
                goto error_remove_edge;
            }
            fimo_free_sized(edge_data, graph->edge_size);
        }

        fimo_array_list_free(&edge_buffer, sizeof(FimoU64));
        goto success_edges_removed;

    error_remove_edge:;
        perror("critical error while removing the edges from the graph");
        exit(EXIT_FAILURE);
    error_collect_edges:
        fimo_array_list_free(&edge_buffer, sizeof(FimoU64));
        goto error_edge_buffer_alloc;
    }
success_edges_removed:;

    n = btree_delete(graph->nodes, &(struct FimoGraphNode_) {
                                       .key = node,
                                   });
    *node_data = n->data;

    error = fimo_array_list_push(&graph->node_free_list, sizeof(FimoU64),
        &node);
    fimo_free((void*)n);

    return error;

error_edge_buffer_alloc:;
error_edge_free_list_resize:;
error_node_free_list_resize:;
error_node_not_found:
    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_remove_edge(FimoGraph* graph, FimoU64 edge,
    void** edge_data)
{
    if (!graph || !edge_data) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    error = fimo_array_list_reserve(&graph->edge_free_list, sizeof(FimoU64), 1);
    if (FIMO_IS_ERROR(error)) {
        goto error_free_list_resize;
    }

    const struct FimoGraphEdge_* e = btree_delete(graph->edges, &(struct FimoGraphEdge_) {
                                                                    .key = edge,
                                                                });
    if (!e) {
        error = FIMO_EINVAL;
        goto error_edge_delete;
    }

    FimoU64 src = e->src;
    FimoU64 dst = e->dst;
    *edge_data = e->data;

    (void)fimo_array_list_push(&graph->edge_free_list, sizeof(FimoU64), &edge);
    fimo_free((void*)e);

    struct FimoGraphNode_* src_node = (void*)btree_get(graph->nodes, &(struct FimoGraphNode_) {
                                                                         .key = src,
                                                                     });
    struct FimoGraphNode_* dst_node = (void*)btree_get(graph->nodes, &(struct FimoGraphNode_) {
                                                                         .key = dst,
                                                                     });

    fimo_free((void*)btree_delete(src_node->adjacency, &(struct FimoGraphNodeAdj_) {
                                                           .key = dst,
                                                       }));
    if (btree_count(src_node->adjacency) == 0) {
        btree_free(src_node->adjacency);
        src_node->adjacency = NULL;
    }

    fimo_free((void*)btree_delete(dst_node->inv_adjacency, &(struct FimoGraphNodeAdj_) {
                                                               .key = src,
                                                           }));
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
FimoError fimo_graph_contains_edge(const FimoGraph* graph, FimoU64 src_node,
    FimoU64 dst_node, bool* contained)
{
    FimoU64 edge;
    return fimo_graph_find_edge(graph, src_node, dst_node, &edge, contained);
}

FIMO_MUST_USE
FimoError fimo_graph_find_edge(const FimoGraph* graph, FimoU64 src_node,
    FimoU64 dst_node, FimoU64* edge, bool* contained)
{
    if (!graph || !edge || !contained) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNode_* src = (void*)btree_get(graph->nodes, &(struct FimoGraphNode_) {
                                                                          .key = src_node,
                                                                      });
    if (!src) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNode_* dst = btree_get(graph->nodes, &(struct FimoGraphNode_) {
                                                                   .key = dst_node,
                                                               });
    if (!dst) {
        return FIMO_EINVAL;
    }

    if (!src->adjacency) {
        *contained = false;
        return FIMO_EOK;
    }

    const struct FimoGraphNodeAdj_* adj = btree_get(src->adjacency, &(struct FimoGraphNodeAdj_) {
                                                                        .key = dst_node,
                                                                    });
    if (adj) {
        *edge = adj->edge;
        *contained = true;
    } else {
        *contained = false;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_nodes_new(const FimoGraph* graph,
    FimoGraphNodes** iter, bool* has_value)
{
    if (!graph || !iter || !has_value) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    FimoGraphNodes* tmp_iter = fimo_aligned_alloc(_Alignof(FimoGraphNodes),
        sizeof(FimoGraphNodes), &error);
    if (FIMO_IS_ERROR(error)) {
        goto error_iter_alloc;
    }

    if (btree_count(graph->nodes) == 0) {
        tmp_iter->iter = NULL;
        tmp_iter->has_value = false;
        return FIMO_EOK;
    }

    tmp_iter->iter = btree_iter_new(graph->nodes);
    if (!tmp_iter->iter) {
        error = FIMO_ENOMEM;
        goto error_nodes_iter;
    }
    tmp_iter->has_value = btree_iter_first(tmp_iter->iter);

    *iter = tmp_iter;
    *has_value = tmp_iter->has_value;

    return FIMO_EOK;

error_nodes_iter:
    fimo_free_aligned_sized(tmp_iter, _Alignof(FimoGraphNodes),
        sizeof(FimoGraphNodes));
error_iter_alloc:
    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_nodes_next(FimoGraphNodes* iter,
    bool* has_value)
{
    if (!iter || !has_value || !iter->has_value) {
        return FIMO_EINVAL;
    }

    iter->has_value = btree_iter_next(iter->iter);
    if (!iter->has_value) {
        btree_iter_free(iter->iter);
        iter->iter = NULL;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_nodes_item(const FimoGraphNodes* iter,
    FimoU64* node, const void** node_data)
{
    if (!iter || !iter->has_value) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNode_* n = btree_iter_item(iter->iter);
    if (node) {
        *node = n->key;
    }
    if (node_data) {
        *node_data = n->data;
    }

    return FIMO_EOK;
}

void fimo_graph_nodes_free(FimoGraphNodes* iter)
{
    if (!iter) {
        perror("invalid nodes iter");
        exit(EXIT_FAILURE);
    }

    if (iter->iter) {
        btree_iter_free(iter->iter);
    }
    fimo_free_aligned_sized(iter, _Alignof(FimoGraphNodes),
        sizeof(FimoGraphNodes));
}

FIMO_MUST_USE
FimoError fimo_graph_edges_new(const FimoGraph* graph,
    FimoGraphEdges** iter, bool* has_value)
{
    if (!graph || !iter || !has_value) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    FimoGraphEdges* tmp_iter = fimo_aligned_alloc(_Alignof(FimoGraphEdges),
        sizeof(FimoGraphEdges), &error);
    if (FIMO_IS_ERROR(error)) {
        goto error_iter_alloc;
    }

    if (btree_count(graph->edges) == 0) {
        tmp_iter->iter = NULL;
        tmp_iter->has_value = false;
        return FIMO_EOK;
    }

    tmp_iter->iter = btree_iter_new(graph->edges);
    if (!tmp_iter->iter) {
        error = FIMO_ENOMEM;
        goto error_edges_iter;
    }
    tmp_iter->has_value = btree_iter_first(tmp_iter->iter);

    *iter = tmp_iter;
    *has_value = tmp_iter->has_value;

    return FIMO_EOK;

error_edges_iter:
    fimo_free_aligned_sized(tmp_iter, _Alignof(FimoGraphEdges),
        sizeof(FimoGraphEdges));
error_iter_alloc:
    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_edges_next(FimoGraphEdges* iter,
    bool* has_value)
{
    if (!iter || !has_value || !iter->has_value) {
        return FIMO_EINVAL;
    }

    iter->has_value = btree_iter_next(iter->iter);
    if (!iter->has_value) {
        btree_iter_free(iter->iter);
        iter->iter = NULL;
    }

    return FIMO_EOK;
}

FIMO_MUST_USE
FimoError fimo_graph_edges_item(const FimoGraphEdges* iter,
    FimoU64* edge, const void** edge_data)
{
    if (!iter || !iter->has_value) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphEdge_* e = btree_iter_item(iter->iter);
    if (edge) {
        *edge = e->key;
    }
    if (edge_data) {
        *edge_data = e->data;
    }

    return FIMO_EOK;
}

void fimo_graph_edges_free(FimoGraphEdges* iter)
{
    if (!iter) {
        perror("invalid edges iter");
        exit(EXIT_FAILURE);
    }

    if (iter->iter) {
        btree_iter_free(iter->iter);
    }
    fimo_free_aligned_sized(iter, _Alignof(FimoGraphEdges),
        sizeof(FimoGraphEdges));
}

FIMO_MUST_USE
FimoError fimo_graph_externals_new(const FimoGraph* graph, bool sink,
    FimoGraphExternals** iter, bool* has_value)
{
    if (!graph || !iter || !has_value) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    FimoGraphExternals* tmp_iter = fimo_aligned_alloc(_Alignof(FimoGraphExternals),
        sizeof(FimoGraphExternals), &error);
    if (FIMO_IS_ERROR(error)) {
        goto error_iter_alloc;
    }

    tmp_iter->sink = sink;
    if (btree_count(graph->nodes) == 0) {
        tmp_iter->iter = NULL;
        tmp_iter->has_value = false;
        return FIMO_EOK;
    }

    tmp_iter->iter = btree_iter_new(graph->nodes);
    if (!tmp_iter->iter) {
        error = FIMO_ENOMEM;
        goto error_nodes_iter;
    }

    tmp_iter->has_value = btree_iter_first(tmp_iter->iter);
    while (tmp_iter->has_value) {
        const struct FimoGraphNode_* n = btree_iter_item(tmp_iter->iter);
        if ((sink && btree_count(n->adjacency) == 0)
            || (!sink && btree_count(n->inv_adjacency) == 0)) {
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
    fimo_free_aligned_sized(tmp_iter, _Alignof(FimoGraphExternals),
        sizeof(FimoGraphExternals));
error_iter_alloc:
    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_externals_next(FimoGraphExternals* iter,
    bool* has_value)
{
    if (!iter || !has_value || !iter->has_value) {
        return FIMO_EINVAL;
    }

    while (iter->has_value) {
        const struct FimoGraphNode_* n = btree_iter_item(iter->iter);
        if ((iter->sink && btree_count(n->adjacency) == 0)
            || (!iter->sink && btree_count(n->inv_adjacency) == 0)) {
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
FimoError fimo_graph_externals_item(const FimoGraphExternals* iter,
    FimoU64* node, const void** node_data)
{
    if (!iter || !iter->has_value) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNode_* n = btree_iter_item(iter->iter);
    if (node) {
        *node = n->key;
    }
    if (node_data) {
        *node_data = n->data;
    }

    return FIMO_EOK;
}

void fimo_graph_externals_free(FimoGraphExternals* iter)
{
    if (!iter) {
        perror("invalid neighbors iterator");
        exit(EXIT_FAILURE);
    }

    if (iter->iter) {
        btree_iter_free(iter->iter);
    }
    fimo_free_aligned_sized(iter, _Alignof(FimoGraphExternals),
        sizeof(FimoGraphExternals));
}

FIMO_MUST_USE
FimoError fimo_graph_neighbors_new(const FimoGraph* graph, FimoU64 node,
    bool inward, FimoGraphNeighbors** iter, bool* has_value)
{
    if (!graph || !iter || !has_value) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    const struct FimoGraphNode_* n = btree_get(graph->nodes, &(struct FimoGraphNode_) {
                                                                 .key = node,
                                                             });
    if (!n) {
        error = FIMO_EINVAL;
        goto error_node_not_found;
    }

    FimoGraphNeighbors* tmp_iter = fimo_aligned_alloc(_Alignof(FimoGraphNeighbors),
        sizeof(FimoGraphNeighbors), &error);
    if (FIMO_IS_ERROR(error)) {
        goto error_iter_alloc;
    }

    const struct btree* neighbors = n->adjacency;
    if (inward) {
        neighbors = n->inv_adjacency;
    }

    if (!neighbors || btree_count(neighbors) == 0) {
        tmp_iter->has_value = false;
        tmp_iter->iter = NULL;
        *iter = tmp_iter;
        return FIMO_EOK;
    }

    tmp_iter->iter = btree_iter_new(neighbors);
    if (!tmp_iter) {
        error = FIMO_ENOMEM;
        goto error_neighbors_iter_alloc;
    }
    tmp_iter->has_value = btree_iter_first(tmp_iter->iter);
    *iter = tmp_iter;

    return FIMO_EOK;

error_neighbors_iter_alloc:;
    fimo_free_aligned_sized(tmp_iter, _Alignof(FimoGraphNeighbors),
        sizeof(FimoGraphNeighbors));
error_iter_alloc:;
error_node_not_found:
    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_neighbors_next(FimoGraphNeighbors* iter,
    bool* has_value)
{
    if (!iter || !has_value || !iter->has_value) {
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
FimoError fimo_graph_neighbors_item(const FimoGraphNeighbors* iter,
    FimoU64* node)
{
    if (!iter || !iter->has_value || !node) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNodeAdj_* n = btree_iter_item(iter->iter);
    *node = n->key;

    return FIMO_EOK;
}

void fimo_graph_neighbors_free(FimoGraphNeighbors* iter)
{
    if (!iter) {
        perror("invalid neighbors iterator");
        exit(EXIT_FAILURE);
    }

    if (iter->iter) {
        btree_iter_free(iter->iter);
    }
    fimo_free_aligned_sized(iter, _Alignof(FimoGraphNeighbors),
        sizeof(FimoGraphNeighbors));
}

FIMO_MUST_USE
FimoError fimo_graph_neighbors_edges_new(const FimoGraph* graph, FimoU64 node,
    bool inward, FimoGraphNeighborsEdges** iter, bool* has_value)
{
    if (!graph || !iter || !has_value) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    const struct FimoGraphNode_* n = btree_get(graph->nodes, &(struct FimoGraphNode_) {
                                                                 .key = node,
                                                             });
    if (!n) {
        error = FIMO_EINVAL;
        goto error_node_not_found;
    }

    FimoGraphNeighborsEdges* tmp_iter = fimo_aligned_alloc(_Alignof(FimoGraphNeighborsEdges),
        sizeof(FimoGraphNeighborsEdges), &error);
    if (FIMO_IS_ERROR(error)) {
        goto error_iter_alloc;
    }

    const struct btree* neighbors = n->adjacency;
    if (inward) {
        neighbors = n->inv_adjacency;
    }

    if (!neighbors || btree_count(neighbors) == 0) {
        tmp_iter->has_value = false;
        tmp_iter->iter = NULL;
        *iter = tmp_iter;
        return FIMO_EOK;
    }

    tmp_iter->iter = btree_iter_new(neighbors);
    if (!tmp_iter) {
        error = FIMO_ENOMEM;
        goto error_neighbors_iter_alloc;
    }
    tmp_iter->has_value = btree_iter_first(tmp_iter->iter);
    *iter = tmp_iter;

    return FIMO_EOK;

error_neighbors_iter_alloc:;
    fimo_free_aligned_sized(tmp_iter, _Alignof(FimoGraphNeighborsEdges),
        sizeof(FimoGraphNeighborsEdges));
error_iter_alloc:;
error_node_not_found:
    return error;
}

FIMO_MUST_USE
FimoError fimo_graph_neighbors_edges_next(FimoGraphNeighborsEdges* iter,
    bool* has_value)
{
    if (!iter || !has_value || !iter->has_value) {
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
FimoError fimo_graph_neighbors_edges_item(const FimoGraphNeighborsEdges* iter,
    FimoU64* edge)
{
    if (!iter || !iter->has_value || !edge) {
        return FIMO_EINVAL;
    }

    const struct FimoGraphNodeAdj_* n = btree_iter_item(iter->iter);
    *edge = n->edge;

    return FIMO_EOK;
}

void fimo_graph_neighbors_edges_free(FimoGraphNeighborsEdges* iter)
{
    if (!iter) {
        perror("invalid neighbors iterator");
        exit(EXIT_FAILURE);
    }

    if (iter->iter) {
        btree_iter_free(iter->iter);
    }
    fimo_free_aligned_sized(iter, _Alignof(FimoGraphNeighborsEdges),
        sizeof(FimoGraphNeighborsEdges));
}

FIMO_MUST_USE
FimoError fimo_graph_clear(FimoGraph* graph)
{
    if (!graph) {
        return FIMO_EINVAL;
    }

    if (btree_count(graph->nodes) == 0 && btree_count(graph->edges) == 0) {
        return FIMO_EOK;
    }

    btree_ascend(graph->nodes, NULL, node_free_, graph);
    btree_clear(graph->nodes);

    btree_ascend(graph->edges, NULL, edge_free_, graph);
    btree_clear(graph->edges);

    (void)fimo_array_list_resize(&graph->node_free_list, sizeof(FimoU64), 0);
    graph->next_node_idx = 0;

    (void)fimo_array_list_resize(&graph->edge_free_list, sizeof(FimoU64), 0);
    graph->next_edge_idx = 0;

    return FIMO_EOK;
}

static bool node_clear_edges_(const void* item, void* data)
{
    (void)data;
    struct FimoGraphNode_* node = (void*)item;
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
FimoError fimo_graph_clear_edges(FimoGraph* graph)
{
    if (!graph) {
        return FIMO_EINVAL;
    }

    if (btree_count(graph->edges) == 0) {
        return FIMO_EOK;
    }

    btree_ascend(graph->nodes, NULL, node_clear_edges_, NULL);

    btree_ascend(graph->edges, NULL, edge_free_, graph);
    btree_clear(graph->edges);

    (void)fimo_array_list_resize_exact(&graph->edge_free_list, sizeof(FimoU64), 0);
    graph->next_edge_idx = 0;

    return FIMO_EOK;
}

static bool invert_node_edge_(const void* item, void* data)
{
    (void)data;
    struct FimoGraphNode_* n = (void*)item;
    struct btree* tmp = n->adjacency;
    n->adjacency = n->inv_adjacency;
    n->inv_adjacency = tmp;
    return true;
}

static bool invert_edge_(const void* item, void* data)
{
    (void)data;
    struct FimoGraphEdge_* e = (void*)item;
    FimoU64 tmp = e->src;
    e->src = e->dst;
    e->dst = tmp;
    return true;
}

FIMO_MUST_USE
FimoError fimo_graph_reverse(FimoGraph* graph)
{
    if (!graph) {
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
    FimoGraph* g;
    struct btree* node_map;
    FimoError (*node_mapper)(FimoU64, FimoU64, void*);
    FimoError (*edge_mapper)(FimoU64, FimoU64, void*);
    void* user_data;
    FimoError error;
};

static int index_map_compare_(const void* a, const void* b, void* data)
{
    (void)data;
    const struct IndexMap_* ma = a;
    const struct IndexMap_* mb = b;

    FimoI64 x = (FimoI64)(ma->key - mb->key);
    if (x < 0) {
        return -1;
    } else if (x > 0) {
        return 1;
    }
    return 0;
}

static bool node_clone_(const void* item, void* data)
{
    struct GraphCloneData_* d = data;
    const struct FimoGraphNode_* n = item;

    FimoU64 node = (FimoU64)-1;
    d->error = fimo_graph_add_node(d->g, n->data, &node);
    if (FIMO_IS_ERROR(d->error)) {
        return false;
    }

    btree_set(d->node_map, &(struct IndexMap_) {
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

static bool edge_clone_(const void* item, void* data)
{
    struct GraphCloneData_* d = data;
    const struct FimoGraphEdge_* e = item;

    const struct IndexMap_* src = btree_get(d->node_map, &(struct IndexMap_) {
                                                             .key = e->src,
                                                         });
    const struct IndexMap_* dst = btree_get(d->node_map, &(struct IndexMap_) {
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
FimoError fimo_graph_clone(const FimoGraph* graph, FimoGraph** new_graph,
    FimoError (*node_mapper)(FimoU64, FimoU64, void*),
    FimoError (*edge_mapper)(FimoU64, FimoU64, void*),
    void* user_data)
{
    if (!graph || !new_graph) {
        return FIMO_EINVAL;
    }

    FimoGraph* g = NULL;
    FimoError error = fimo_graph_new(graph->node_size, graph->edge_size,
        graph->node_free, graph->edge_free, &g);
    if (FIMO_IS_ERROR(error)) {
        goto error_graph_alloc;
    }

    struct btree* node_map = btree_new_with_allocator(fimo_graph_malloc_, fimo_graph_realloc_,
        fimo_graph_free_, sizeof(struct IndexMap_), 0, index_map_compare_, NULL);
    if (!node_map) {
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
    FimoArrayList* node_stack;
    struct btree* node_map;
    const FimoGraph* graph;
    FimoGraph* sub_graph;
    FimoError (*node_mapper)(FimoU64, FimoU64, void*);
    FimoError (*edge_mapper)(FimoU64, FimoU64, void*);
    void* user_data;
    FimoU64 current_node;
    FimoError error;
};

static int index_compare_(const void* a, const void* b, void* data)
{
    (void)data;
    const FimoU64* ia = a;
    const FimoU64* ib = b;

    FimoI64 x = (FimoI64)(*ia - *ib);
    if (x < 0) {
        return -1;
    } else if (x > 0) {
        return 1;
    }
    return 0;
}

static bool clone_adjacency_(const void* item, void* data)
{
    const struct FimoGraphNodeAdj_* adj = item;
    struct ReachableSubgraphData_* d = data;

    const void* edge_data;
    d->error = fimo_graph_edge_data(d->graph, adj->edge, &edge_data);
    if (FIMO_IS_ERROR(d->error)) {
        return false;
    }

    const struct IndexMap_* src_node_mapping = btree_get(d->node_map, &(struct IndexMap_) {
                                                                          .key = d->current_node,
                                                                      });
    FimoU64 mapped_src_node = src_node_mapping->mapped;

    FimoU64 mapped_dst_node = (FimoU64)-1;
    const struct IndexMap_* dst_node_mapping = btree_get(d->node_map, &(struct IndexMap_) {
                                                                          .key = adj->key,
                                                                      });
    if (!dst_node_mapping) {
        const void* node_data;
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

        btree_set(d->node_map, &(struct IndexMap_) {
                                   .key = adj->key,
                                   .mapped = mapped_dst_node,
                               });
        if (btree_oom(d->node_map)) {
            d->error = FIMO_ENOMEM;
            return false;
        }

        d->error = fimo_array_list_push(d->node_stack, sizeof(FimoU64), &adj->key);
        if (FIMO_IS_ERROR(d->error)) {
            return false;
        }
    } else {
        mapped_dst_node = dst_node_mapping->mapped;
    }

    FimoU64 mapped_edge = (FimoU64)-1;
    d->error = fimo_graph_add_edge(d->sub_graph, mapped_src_node, mapped_dst_node,
        edge_data, NULL, &mapped_edge);

    if (d->edge_mapper) {
        d->edge_mapper(adj->edge, mapped_edge, d->user_data);
    }

    return true;
}

FIMO_MUST_USE
FimoError fimo_graph_clone_reachable_subgraph(const FimoGraph* graph,
    FimoGraph** sub_graph, FimoU64 start_node,
    FimoError (*node_mapper)(FimoU64, FimoU64, void*),
    FimoError (*edge_mapper)(FimoU64, FimoU64, void*),
    void* user_data)
{
    if (!graph | !sub_graph) {
        return FIMO_EINVAL;
    }

    FimoError error = FIMO_EOK;
    const struct FimoGraphNode_* start = btree_get(graph->nodes, &(struct FimoGraphNode_) {
                                                                     .key = start_node,
                                                                 });
    if (!start) {
        error = FIMO_EINVAL;
        goto error_start_node_not_found;
    }

    FimoArrayList node_stack;
    error = fimo_array_list_with_capacity(1, sizeof(FimoU64), &node_stack);
    if (FIMO_IS_ERROR(error)) {
        goto error_node_stack_alloc;
    }

    error = fimo_array_list_push(&node_stack, sizeof(FimoU64), &start_node);
    if (FIMO_IS_ERROR(error)) {
        goto error_node_stack_init;
    }

    struct btree* node_map = btree_new_with_allocator(fimo_graph_malloc_, fimo_graph_realloc_,
        fimo_graph_free_, sizeof(struct IndexMap_), 0, index_compare_, NULL);
    if (!node_map) {
        error = FIMO_ENOMEM;
        goto error_node_map_alloc;
    }

    FimoGraph* g;
    error = fimo_graph_new(graph->node_size, graph->edge_size,
        graph->node_free, graph->edge_free, &g);
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

    btree_set(node_map, &(struct IndexMap_) {
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
        error = fimo_array_list_pop_back(&node_stack, sizeof(FimoU64), &node);
        if (FIMO_IS_ERROR(error)) {
            goto error_construct_sub_graph;
        }

        reachable_data.current_node = node;
        const struct FimoGraphNode_* n = btree_get(graph->nodes, &(struct FimoGraphNode_) {
                                                                     .key = node,
                                                                 });
        btree_ascend(n->adjacency, NULL, clone_adjacency_, &reachable_data);
        if (FIMO_IS_ERROR(reachable_data.error)) {
            goto error_construct_sub_graph;
        }
    }

    *sub_graph = g;

    btree_free(node_map);
    fimo_array_list_free(&node_stack, sizeof(FimoU64));

    return FIMO_EOK;

error_construct_sub_graph:;
    fimo_graph_free(g);
error_sub_graph_alloc:
    btree_free(node_map);
error_node_map_alloc:;
error_node_stack_init:
    fimo_array_list_free(&node_stack, sizeof(FimoU64));
error_node_stack_alloc:;
error_start_node_not_found:
    return error;
}
