#include <setjmp.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>

#include <cmocka.h>

#include <fimo_std/graph.h>
#include <fimo_std/memory.h>

static void graph_init_test(void** state)
{
    (void)state;

    // Invalid node size/destructor pair
    FimoGraph* graph;
    FimoError error = fimo_graph_new(0, 0, dummy_cleanup_, NULL, &graph);
    assert_true(FIMO_IS_ERROR(error));

    // Invalid edge size/destructor pair
    error = fimo_graph_new(0, 0, NULL, dummy_cleanup_, &graph);
    assert_true(FIMO_IS_ERROR(error));

    error = fimo_graph_new(0, 0, NULL, NULL, &graph);
    assert_false(FIMO_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(sizeof(int), 0, NULL, NULL, &graph);
    assert_false(FIMO_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(sizeof(int), 0, dummy_cleanup_, NULL, &graph);
    assert_false(FIMO_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(0, sizeof(int), NULL, NULL, &graph);
    assert_false(FIMO_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(0, sizeof(int), NULL, dummy_cleanup_, &graph);
    assert_false(FIMO_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(sizeof(int), sizeof(int), NULL, NULL, &graph);
    assert_false(FIMO_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(sizeof(int), sizeof(int), dummy_cleanup_, NULL, &graph);
    assert_false(FIMO_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(sizeof(int), sizeof(int), NULL, dummy_cleanup_, &graph);
    assert_false(FIMO_IS_ERROR(error));
    fimo_graph_free(graph);

    error = fimo_graph_new(sizeof(int), sizeof(int), dummy_cleanup_, dummy_cleanup_, &graph);
    assert_false(FIMO_IS_ERROR(error));
    fimo_graph_free(graph);
}

static void add_nodes_empty_test(void** state)
{
    (void)state;

    FimoGraph* graph;
    FimoError error = fimo_graph_new(0, 0, NULL, NULL, &graph);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(fimo_graph_node_count(graph) == 0);

    // Data when the node expects no data.
    FimoU64 node_a;
    int tmp = 5;
    error = fimo_graph_add_node(graph, &tmp, &node_a);
    assert_true(FIMO_IS_ERROR(error));

    error = fimo_graph_add_node(graph, NULL, &node_a);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(fimo_graph_node_count(graph) == 1);

    const int* node_a_data = NULL;
    error = fimo_graph_node_data(graph, node_a, &node_a_data);
    assert_false(FIMO_IS_ERROR(error));
    assert_null(node_a_data);

    FimoU64 node_b;
    error = fimo_graph_add_node(graph, NULL, &node_b);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(fimo_graph_node_count(graph) == 2);
    assert_false(node_a == node_b);

    const int* node_b_data = NULL;
    error = fimo_graph_node_data(graph, node_b, &node_b_data);
    assert_false(FIMO_IS_ERROR(error));
    assert_null(node_b_data);

    fimo_graph_free(graph);
}

static void add_nodes_test(void** state)
{
    (void)state;

    FimoGraph* graph;
    FimoError error = fimo_graph_new(sizeof(int), 0, dummy_cleanup_, NULL, &graph);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(fimo_graph_node_count(graph) == 0);

    // Empty data when the node expects some data.
    FimoU64 node_a;
    error = fimo_graph_add_node(graph, NULL, &node_a);
    assert_true(FIMO_IS_ERROR(error));

    int tmp = 5;
    error = fimo_graph_add_node(graph, &tmp, &node_a);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(fimo_graph_node_count(graph) == 1);

    const int* node_a_data = NULL;
    error = fimo_graph_node_data(graph, node_a, &node_a_data);
    assert_false(FIMO_IS_ERROR(error));
    assert_non_null(node_a_data);
    assert_true(*node_a_data == 5);

    FimoU64 node_b;
    tmp = 10;
    error = fimo_graph_add_node(graph, &tmp, &node_b);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(fimo_graph_node_count(graph) == 2);
    assert_false(node_a == node_b);

    const int* node_b_data = NULL;
    error = fimo_graph_node_data(graph, node_b, &node_b_data);
    assert_false(FIMO_IS_ERROR(error));
    assert_non_null(node_b_data);
    assert_true(*node_b_data == 10);

    fimo_graph_free(graph);
}

static void add_edges_empty_test(void** state)
{
    (void)state;

    FimoGraph* graph;
    FimoError error = fimo_graph_new(0, 0, NULL, NULL, &graph);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(fimo_graph_edge_count(graph) == 0);

    FimoU64 node_a;
    error = fimo_graph_add_node(graph, NULL, &node_a);
    assert_false(FIMO_IS_ERROR(error));

    FimoU64 node_b;
    error = fimo_graph_add_node(graph, NULL, &node_b);
    assert_false(FIMO_IS_ERROR(error));

    FimoU64 node_c;
    error = fimo_graph_add_node(graph, NULL, &node_c);
    assert_false(FIMO_IS_ERROR(error));

    int tmp = 0;
    void* old_data = NULL;

    FimoU64 edge_ab;
    error = fimo_graph_add_edge(graph, node_a, node_b, &tmp, &old_data, &edge_ab);
    assert_true(FIMO_IS_ERROR(error));

    error = fimo_graph_add_edge(graph, node_a, node_b, NULL, &old_data, &edge_ab);
    assert_false(FIMO_IS_ERROR(error));
    assert_null(old_data);
    assert_true(fimo_graph_edge_count(graph) == 1);

    int* edge_ab_data = NULL;
    error = fimo_graph_edge_data(graph, edge_ab, &edge_ab_data);
    assert_false(FIMO_IS_ERROR(error));
    assert_null(edge_ab_data);

    FimoU64 edge_ab_new;
    error = fimo_graph_add_edge(graph, node_a, node_b, NULL, &old_data, &edge_ab_new);
    assert_false(FIMO_IS_ERROR(error));
    assert_null(old_data);
    assert_true(fimo_graph_edge_count(graph) == 1);
    assert_true(edge_ab == edge_ab_new);

    error = fimo_graph_edge_data(graph, edge_ab, &edge_ab_data);
    assert_false(FIMO_IS_ERROR(error));
    assert_null(edge_ab_data);

    FimoU64 edge_bc;
    error = fimo_graph_add_edge(graph, node_b, node_c, NULL, &old_data, &edge_bc);
    assert_false(FIMO_IS_ERROR(error));
    assert_null(old_data);
    assert_true(fimo_graph_edge_count(graph) == 2);
    assert_false(edge_ab == edge_bc);

    int* edge_bc_data = NULL;
    error = fimo_graph_edge_data(graph, edge_bc, &edge_bc_data);
    assert_false(FIMO_IS_ERROR(error));
    assert_null(edge_bc_data);

    fimo_graph_free(graph);
}

static void add_edges_test(void** state)
{
    (void)state;

    FimoGraph* graph;
    FimoError error = fimo_graph_new(0, sizeof(int), NULL, dummy_cleanup_, &graph);
    assert_false(FIMO_IS_ERROR(error));
    assert_true(fimo_graph_edge_count(graph) == 0);

    FimoU64 node_a;
    error = fimo_graph_add_node(graph, NULL, &node_a);
    assert_false(FIMO_IS_ERROR(error));

    FimoU64 node_b;
    error = fimo_graph_add_node(graph, NULL, &node_b);
    assert_false(FIMO_IS_ERROR(error));

    FimoU64 node_c;
    error = fimo_graph_add_node(graph, NULL, &node_c);
    assert_false(FIMO_IS_ERROR(error));

    void* old_data = NULL;

    FimoU64 edge_ab;
    error = fimo_graph_add_edge(graph, node_a, node_b, NULL, &old_data, &edge_ab);
    assert_true(FIMO_IS_ERROR(error));

    int tmp = 0;
    error = fimo_graph_add_edge(graph, node_a, node_b, &tmp, &old_data, &edge_ab);
    assert_false(FIMO_IS_ERROR(error));
    assert_null(old_data);
    assert_true(fimo_graph_edge_count(graph) == 1);

    int* edge_ab_data = NULL;
    error = fimo_graph_edge_data(graph, edge_ab, &edge_ab_data);
    assert_false(FIMO_IS_ERROR(error));
    assert_non_null(edge_ab_data);
    assert_true(*edge_ab_data == 0);

    tmp = 1;
    FimoU64 edge_ab_new;
    error = fimo_graph_add_edge(graph, node_a, node_b, &tmp, &old_data, &edge_ab_new);
    assert_false(FIMO_IS_ERROR(error));
    assert_non_null(old_data);
    assert_true(fimo_graph_edge_count(graph) == 1);
    assert_true(edge_ab == edge_ab_new);
    assert_true(*(int*)old_data == 0);
    fimo_free_sized(old_data, sizeof(int));

    error = fimo_graph_edge_data(graph, edge_ab, &edge_ab_data);
    assert_false(FIMO_IS_ERROR(error));
    assert_non_null(edge_ab_data);
    assert_true(*edge_ab_data == 1);

    tmp = 2;
    FimoU64 edge_bc;
    error = fimo_graph_add_edge(graph, node_b, node_c, &tmp, &old_data, &edge_bc);
    assert_false(FIMO_IS_ERROR(error));
    assert_null(old_data);
    assert_true(fimo_graph_edge_count(graph) == 2);
    assert_false(edge_ab == edge_bc);

    int* edge_bc_data = NULL;
    error = fimo_graph_edge_data(graph, edge_bc, &edge_bc_data);
    assert_false(FIMO_IS_ERROR(error));
    assert_non_null(edge_bc_data);
    assert_true(*edge_bc_data == 2);

    fimo_graph_free(graph);
}

int main(void)
{
    const struct CMUnitTest tests[] = {
        cmocka_unit_test(graph_init_test),
        cmocka_unit_test(add_nodes_empty_test),
        cmocka_unit_test(add_nodes_test),
        cmocka_unit_test(add_edges_empty_test),
        cmocka_unit_test(add_edges_test),
    };

    return cmocka_run_group_tests(tests, NULL, NULL);
}
