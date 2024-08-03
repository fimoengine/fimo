#include <catch2/catch_all.hpp>

#include <fimo_std/memory.h>

TEST_CASE("Allocate memory", "[memory]") {
    FimoResult error;

    SECTION("zero size results in a null pointer") {
        void *buffer = fimo_malloc(0, &error);
        REQUIRE(buffer == nullptr);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    }

    SECTION("allocation is properly aligned") {
        void *buffer = fimo_malloc(sizeof(long long), &error);
        REQUIRE(buffer != nullptr);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(reinterpret_cast<FimoUIntPtr>(buffer) % FIMO_MALLOC_ALIGNMENT == 0);
        fimo_free(buffer);
    }

    SECTION("allocation is properly aligned and sized") {
        FimoMallocBuffer buffer = fimo_malloc_sized(1339, &error);
        REQUIRE(buffer.ptr != nullptr);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(buffer.buff_size >= 1339);
        REQUIRE(reinterpret_cast<FimoUIntPtr>(buffer.ptr) % FIMO_MALLOC_ALIGNMENT == 0);
        fimo_free(buffer.ptr);
    }
}

TEST_CASE("Allocate zeroed memory", "[memory]") {
    FimoResult error;

    SECTION("zero size results in a null pointer") {
        void *buffer = fimo_calloc(0, &error);
        REQUIRE(buffer == nullptr);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    }

    SECTION("allocation is properly aligned") {
        long long *buffer = static_cast<long long *>(fimo_calloc(10 * sizeof(long long), &error));
        REQUIRE(buffer != nullptr);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(reinterpret_cast<FimoUIntPtr>(buffer) % FIMO_MALLOC_ALIGNMENT == 0);
        for (FimoUSize i = 0; i < 10; i++) {
            REQUIRE(buffer[i] == 0);
        }
        fimo_free_sized(buffer, 10 * sizeof(long long));
    }

    SECTION("allocation is properly aligned and sized") {
        FimoMallocBuffer buffer = fimo_calloc_sized(1339, &error);
        REQUIRE(buffer.ptr != nullptr);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(buffer.buff_size >= 1339);
        REQUIRE(reinterpret_cast<FimoUIntPtr>(buffer.ptr) % FIMO_MALLOC_ALIGNMENT == 0);
        for (FimoUSize i = 0; i < buffer.buff_size; i++) {
            REQUIRE(reinterpret_cast<char *>(buffer.ptr)[i] == 0);
        }
        fimo_free_sized(buffer.ptr, buffer.buff_size);
    }
}

TEST_CASE("Allocate aligned memory", "[memory]") {
    FimoResult error;

    SECTION("alignment must not be zero") {
        void *buffer = fimo_aligned_alloc(0, 10, &error);
        REQUIRE(buffer == nullptr);
        REQUIRE(FIMO_RESULT_IS_ERROR(error));
        fimo_result_release(error);
    }

    SECTION("alignment must be a power of two") {
        void *buffer = fimo_aligned_alloc(17, 10, &error);
        REQUIRE(buffer == nullptr);
        REQUIRE(FIMO_RESULT_IS_ERROR(error));
        fimo_result_release(error);
    }

    SECTION("zero size results in a null pointer") {
        void *buffer = fimo_aligned_alloc(256, 0, &error);
        REQUIRE(buffer == nullptr);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    }

    SECTION("allocation is properly aligned") {
        void *buffer = static_cast<long long *>(fimo_aligned_alloc(256, sizeof(long long), &error));
        REQUIRE(buffer != nullptr);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(reinterpret_cast<FimoUIntPtr>(buffer) % 256 == 0);
        fimo_free_aligned_sized(buffer, 256, sizeof(long long));
    }

    SECTION("allocation is properly aligned and sized") {
        FimoMallocBuffer buffer = fimo_aligned_alloc_sized(256, 1339, &error);
        REQUIRE(buffer.ptr != nullptr);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(buffer.buff_size >= 1339);
        REQUIRE(reinterpret_cast<FimoUIntPtr>(buffer.ptr) % 256 == 0);
        fimo_free_aligned_sized(buffer.ptr, 256, buffer.buff_size);
    }
}
