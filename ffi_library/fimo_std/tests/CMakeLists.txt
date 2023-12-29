list(APPEND TEST_TARGETS test_empty)

list(APPEND TEST_LIBS "${CMOCKA_LIBRARIES}")
list(APPEND TEST_LIBS fimo_std)

list(APPEND TEST_DIRS "${CMOCKA_INCLUDE_DIRS}")

foreach(TEST_TARGET IN LISTS TEST_TARGETS)
    add_cmocka_test(
        ${TEST_TARGET}
        SOURCES "${TEST_TARGET}.c"
        COMPILE_OPTIONS ${DEFAULT_C_COMPILE_FLAGS}
        LINK_LIBRARIES "${TEST_LIBS}")
    add_cmocka_test_environment(${TEST_TARGET})
    target_include_directories(${TEST_TARGET} PUBLIC "${TEST_DIRS}")
endforeach()
