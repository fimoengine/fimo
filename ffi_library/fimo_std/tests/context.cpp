#include <catch2/catch_all.hpp>

#include <fimo_std/context.h>

TEST_CASE("Context initalization", "[context]") {
    FimoContext ctx;
    FimoError error = fimo_context_init(nullptr, &ctx);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    error = fimo_context_check_version(ctx);
    REQUIRE_FALSE(FIMO_IS_ERROR(error));

    fimo_context_acquire(ctx);
    fimo_context_release(ctx);
    fimo_context_release(ctx);
}
