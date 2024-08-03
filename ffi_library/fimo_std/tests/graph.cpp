#include <catch2/catch_all.hpp>

#include <fimo_std/graph.h>
#include <fimo_std/memory.h>

TEST_CASE("Graph initialization", "[graph]") {
    auto dummy_cleanup = [](void *data) { (void)data; };

    // Invalid node size/destructor pair
    FimoGraph *graph;
    FimoResult error = fimo_graph_new(0, 0, dummy_cleanup, nullptr, &graph);
    REQUIRE(FIMO_RESULT_IS_ERROR(error));
    fimo_result_release(error);

    // Invalid edge size/destructor pair
    error = fimo_graph_new(0, 0, nullptr, dummy_cleanup, &graph);
    REQUIRE(FIMO_RESULT_IS_ERROR(error));
    fimo_result_release(error);

    error = fimo_graph_new(0, 0, nullptr, nullptr, &graph);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(sizeof(int), 0, nullptr, nullptr, &graph);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(sizeof(int), 0, dummy_cleanup, nullptr, &graph);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(0, sizeof(int), nullptr, nullptr, &graph);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(0, sizeof(int), nullptr, dummy_cleanup, &graph);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(sizeof(int), sizeof(int), nullptr, nullptr, &graph);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(sizeof(int), sizeof(int), dummy_cleanup, nullptr, &graph);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(sizeof(int), sizeof(int), nullptr, dummy_cleanup, &graph);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(sizeof(int), sizeof(int), dummy_cleanup, dummy_cleanup, &graph);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    fimo_graph_free(graph);
}

TEST_CASE("Graph with zero-sized nodes", "[graph]") {
    FimoGraph *graph;
    FimoResult error = fimo_graph_new(0, 0, nullptr, nullptr, &graph);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(fimo_graph_node_count(graph) == 0);

    // Data when the node expects no data.
    FimoU64 node_a;
    int tmp = 5;
    error = fimo_graph_add_node(graph, &tmp, &node_a);
    REQUIRE(FIMO_RESULT_IS_ERROR(error));
    fimo_result_release(error);

    error = fimo_graph_add_node(graph, nullptr, &node_a);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(fimo_graph_node_count(graph) == 1);

    const int *node_a_data = nullptr;
    error = fimo_graph_node_data(graph, node_a, reinterpret_cast<const void **>(&node_a_data));
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(node_a_data == nullptr);

    FimoU64 node_b;
    error = fimo_graph_add_node(graph, nullptr, &node_b);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(fimo_graph_node_count(graph) == 2);
    REQUIRE_FALSE(node_a == node_b);

    const int *node_b_data = nullptr;
    error = fimo_graph_node_data(graph, node_b, reinterpret_cast<const void **>(&node_b_data));
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(node_b_data == nullptr);

    fimo_graph_free(graph);
}

TEST_CASE("Graph with sized nodes", "[graph]") {
    auto dummy_cleanup = [](void *data) { (void)data; };

    FimoGraph *graph;
    FimoResult error = fimo_graph_new(sizeof(int), 0, dummy_cleanup, nullptr, &graph);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(fimo_graph_node_count(graph) == 0);

    // Empty data when the node expects some data.
    FimoU64 node_a;
    error = fimo_graph_add_node(graph, nullptr, &node_a);
    REQUIRE(FIMO_RESULT_IS_ERROR(error));
    fimo_result_release(error);

    int tmp = 5;
    error = fimo_graph_add_node(graph, &tmp, &node_a);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(fimo_graph_node_count(graph) == 1);

    const int *node_a_data = nullptr;
    error = fimo_graph_node_data(graph, node_a, reinterpret_cast<const void **>(&node_a_data));
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(node_a_data != nullptr);
    REQUIRE(*node_a_data == 5);

    FimoU64 node_b;
    tmp = 10;
    error = fimo_graph_add_node(graph, &tmp, &node_b);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(fimo_graph_node_count(graph) == 2);
    REQUIRE_FALSE(node_a == node_b);

    const int *node_b_data = nullptr;
    error = fimo_graph_node_data(graph, node_b, reinterpret_cast<const void **>(&node_b_data));
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(node_b_data != nullptr);
    REQUIRE(*node_b_data == 10);

    fimo_graph_free(graph);
}

TEST_CASE("Graph with zero-sized edges", "[graph]") {
    FimoGraph *graph;
    FimoResult error = fimo_graph_new(0, 0, nullptr, nullptr, &graph);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(fimo_graph_edge_count(graph) == 0);

    FimoU64 node_a;
    error = fimo_graph_add_node(graph, nullptr, &node_a);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    FimoU64 node_b;
    error = fimo_graph_add_node(graph, nullptr, &node_b);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    FimoU64 node_c;
    error = fimo_graph_add_node(graph, nullptr, &node_c);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    int tmp = 0;
    void *old_data = nullptr;

    FimoU64 edge_ab;
    error = fimo_graph_add_edge(graph, node_a, node_b, &tmp, &old_data, &edge_ab);
    REQUIRE(FIMO_RESULT_IS_ERROR(error));
    fimo_result_release(error);

    error = fimo_graph_add_edge(graph, node_a, node_b, nullptr, &old_data, &edge_ab);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(old_data == nullptr);
    REQUIRE(fimo_graph_edge_count(graph) == 1);

    const int *edge_ab_data = nullptr;
    error = fimo_graph_edge_data(graph, edge_ab, reinterpret_cast<const void **>(&edge_ab_data));
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(edge_ab_data == nullptr);

    FimoU64 edge_ab_new;
    error = fimo_graph_add_edge(graph, node_a, node_b, nullptr, &old_data, &edge_ab_new);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(old_data == nullptr);
    REQUIRE(fimo_graph_edge_count(graph) == 1);
    REQUIRE(edge_ab == edge_ab_new);

    error = fimo_graph_edge_data(graph, edge_ab, reinterpret_cast<const void **>(&edge_ab_data));
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(edge_ab_data == nullptr);

    FimoU64 edge_bc;
    error = fimo_graph_add_edge(graph, node_b, node_c, nullptr, &old_data, &edge_bc);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(old_data == nullptr);
    REQUIRE(fimo_graph_edge_count(graph) == 2);
    REQUIRE_FALSE(edge_ab == edge_bc);

    const int *edge_bc_data = nullptr;
    error = fimo_graph_edge_data(graph, edge_bc, reinterpret_cast<const void **>(&edge_bc_data));
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(edge_bc_data == nullptr);

    fimo_graph_free(graph);
}

TEST_CASE("Graph with sized edged", "[graph]") {
    auto dummy_cleanup = [](void *data) { (void)data; };

    FimoGraph *graph;
    FimoResult error = fimo_graph_new(0, sizeof(int), nullptr, dummy_cleanup, &graph);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(fimo_graph_edge_count(graph) == 0);

    FimoU64 node_a;
    error = fimo_graph_add_node(graph, nullptr, &node_a);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    FimoU64 node_b;
    error = fimo_graph_add_node(graph, nullptr, &node_b);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    FimoU64 node_c;
    error = fimo_graph_add_node(graph, nullptr, &node_c);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    void *old_data = nullptr;

    FimoU64 edge_ab;
    error = fimo_graph_add_edge(graph, node_a, node_b, nullptr, &old_data, &edge_ab);
    REQUIRE(FIMO_RESULT_IS_ERROR(error));
    fimo_result_release(error);

    int tmp = 0;
    error = fimo_graph_add_edge(graph, node_a, node_b, &tmp, &old_data, &edge_ab);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(old_data == nullptr);
    REQUIRE(fimo_graph_edge_count(graph) == 1);

    const int *edge_ab_data = nullptr;
    error = fimo_graph_edge_data(graph, edge_ab, reinterpret_cast<const void **>(&edge_ab_data));
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(edge_ab_data != nullptr);
    REQUIRE(*edge_ab_data == 0);

    tmp = 1;
    FimoU64 edge_ab_new;
    error = fimo_graph_add_edge(graph, node_a, node_b, &tmp, &old_data, &edge_ab_new);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(old_data != nullptr);
    REQUIRE(fimo_graph_edge_count(graph) == 1);
    REQUIRE(edge_ab == edge_ab_new);
    REQUIRE(*(int *)old_data == 0);
    fimo_free_sized(old_data, sizeof(int));

    error = fimo_graph_edge_data(graph, edge_ab, reinterpret_cast<const void **>(&edge_ab_data));
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(edge_ab_data != nullptr);
    REQUIRE(*edge_ab_data == 1);

    tmp = 2;
    FimoU64 edge_bc;
    error = fimo_graph_add_edge(graph, node_b, node_c, &tmp, &old_data, &edge_bc);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(old_data == nullptr);
    REQUIRE(fimo_graph_edge_count(graph) == 2);
    REQUIRE_FALSE(edge_ab == edge_bc);

    const int *edge_bc_data = nullptr;
    error = fimo_graph_edge_data(graph, edge_bc, reinterpret_cast<const void **>(&edge_bc_data));
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    REQUIRE(edge_bc_data != nullptr);
    REQUIRE(*edge_bc_data == 2);

    fimo_graph_free(graph);
}
