#include <catch2/catch_all.hpp>
#include <cstring>

#include <fimo_std/path.h>

TEST_CASE("PathBuf conversions", "[FimoUTF8PathBuf]") {
    SECTION("Empty path") {
        FimoUTF8PathBuf buffer = fimo_utf8_path_buf_new();
        FimoUTF8Path path = fimo_utf8_path_buf_as_path(&buffer);
        REQUIRE_FALSE(path.path == nullptr);
        REQUIRE(path.length == 0);
    }
    SECTION("Non-empty path") {
        FimoUTF8PathBuf buffer = fimo_utf8_path_buf_new();
        FimoResult error = fimo_utf8_path_buf_push_string(&buffer, "/tmp");
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

        FimoUTF8Path path = fimo_utf8_path_buf_as_path(&buffer);
        REQUIRE(path.length == sizeof("/tmp") - 1);
        REQUIRE(strncmp(path.path, "/tmp", path.length) == 0);
        fimo_utf8_path_buf_free(&buffer);
    }
    SECTION("Empty owned path") {
        FimoUTF8PathBuf buffer = fimo_utf8_path_buf_new();
        FimoOwnedUTF8Path path;
        FimoResult error = fimo_utf8_path_buf_into_owned_path(&buffer, &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(path.length == sizeof("") - 1);
        REQUIRE(strncmp(path.path, "", path.length) == 0);
        fimo_owned_utf8_path_free(path);
    }
    SECTION("Non-empty owned path") {
        FimoUTF8PathBuf buffer = fimo_utf8_path_buf_new();
        FimoResult error = fimo_utf8_path_buf_push_string(&buffer, "/tmp");
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

        FimoOwnedUTF8Path path;
        error = fimo_utf8_path_buf_into_owned_path(&buffer, &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(path.length == sizeof("/tmp") - 1);
        REQUIRE(strncmp(path.path, "/tmp", path.length) == 0);
        fimo_owned_utf8_path_free(path);
    }
}

TEST_CASE("Push Path", "[FimoUTF8PathBuf]") {
    FimoUTF8PathBuf buffer = fimo_utf8_path_buf_new();
    FimoResult error = fimo_utf8_path_buf_push_string(&buffer, "/tmp");
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    SECTION("relative path") {
        error = fimo_utf8_path_buf_push_string(&buffer, "file.bk");
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

        FimoUTF8Path path = fimo_utf8_path_buf_as_path(&buffer);
        REQUIRE(path.length == sizeof("/tmp/file.bk") - 1);
#if _WIN32
        REQUIRE(strncmp(path.path, "/tmp\\file.bk", path.length) == 0);
#else
        REQUIRE(strncmp(path.path, "/tmp/file.bk", path.length) == 0);
#endif
    }
    SECTION("absolute path") {
        error = fimo_utf8_path_buf_push_string(&buffer, "/etc");
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

        FimoUTF8Path path = fimo_utf8_path_buf_as_path(&buffer);
        REQUIRE(path.length == sizeof("/etc") - 1);
        REQUIRE(strncmp(path.path, "/etc", path.length) == 0);
    }

    fimo_utf8_path_buf_free(&buffer);
}

TEST_CASE("Pop Path", "[FimoUTF8PathBuf]") {
    FimoUTF8PathBuf buffer = fimo_utf8_path_buf_new();
    FimoResult error = fimo_utf8_path_buf_push_string(&buffer, "/spirited/away.c");
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

    fimo_utf8_path_buf_pop(&buffer);
    FimoUTF8Path path = fimo_utf8_path_buf_as_path(&buffer);
    REQUIRE(path.length == sizeof("/spirited") - 1);
    REQUIRE(strncmp(path.path, "/spirited", path.length) == 0);

    fimo_utf8_path_buf_pop(&buffer);
    path = fimo_utf8_path_buf_as_path(&buffer);
    REQUIRE(path.length == sizeof("/") - 1);
    REQUIRE(strncmp(path.path, "/", path.length) == 0);

    fimo_utf8_path_buf_free(&buffer);
}

TEST_CASE("Create owned Path", "[FimoOwnedUTF8Path]") {
    FimoResult error;
    FimoOwnedUTF8Path path;

    SECTION("NULL string") {
        error = fimo_owned_utf8_path_from_string(nullptr, &path);
        REQUIRE(FIMO_RESULT_IS_ERROR(error));
    }
    SECTION("NULL path") {
        error = fimo_owned_utf8_path_from_string("", nullptr);
        REQUIRE(FIMO_RESULT_IS_ERROR(error));
    }
    SECTION("non UTF-8 string") {
        constexpr char str[] = {'\xc3', '\x28', '\0'};
        error = fimo_owned_utf8_path_from_string(str, &path);
        REQUIRE(FIMO_RESULT_IS_ERROR(error));
    }
    SECTION("UTF-8 string") {
        error = fimo_owned_utf8_path_from_string("foo.txt", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(path.length == sizeof("foo.txt") - 1);
        REQUIRE(strncmp(path.path, "foo.txt", path.length) == 0);
        fimo_owned_utf8_path_free(path);
    }
    SECTION("Path") {
        FimoUTF8Path p;
        error = fimo_utf8_path_new("foo.txt", &p);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

        error = fimo_owned_utf8_path_from_path(p, &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(path.length == sizeof("foo.txt") - 1);
        REQUIRE(strncmp(path.path, "foo.txt", path.length) == 0);
        fimo_owned_utf8_path_free(path);
    }
    SECTION("OSPath") {
#if _WIN32
        FimoOSPath p = fimo_os_path_new(L"foo.txt");
#else
        FimoOSPath p = fimo_os_path_new("foo.txt");
#endif

        error = fimo_owned_utf8_path_from_os_path(p, &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(path.length == sizeof("foo.txt") - 1);
        REQUIRE(strncmp(path.path, "foo.txt", path.length) == 0);
        fimo_owned_utf8_path_free(path);
    }
}

TEST_CASE("Create Path", "[FimoUTF8Path]") {
    FimoResult error;
    FimoUTF8Path path;

    SECTION("NULL string") {
        error = fimo_utf8_path_new(nullptr, &path);
        REQUIRE(FIMO_RESULT_IS_ERROR(error));
    }
    SECTION("NULL path") {
        error = fimo_utf8_path_new("", nullptr);
        REQUIRE(FIMO_RESULT_IS_ERROR(error));
    }
    SECTION("non UTF-8 string") {
        constexpr char str[] = {'\xc3', '\x28', '\0'};
        error = fimo_utf8_path_new(str, &path);
        REQUIRE(FIMO_RESULT_IS_ERROR(error));
    }
    SECTION("UTF-8 string") {
        error = fimo_utf8_path_new("foo.txt", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(path.length == sizeof("foo.txt") - 1);
        REQUIRE(strncmp(path.path, "foo.txt", path.length) == 0);
    }
}

TEST_CASE("Path is absolute", "[FimoUTF8Path]") {
    FimoResult error;
    FimoUTF8Path path;

    SECTION("relative path") {
        error = fimo_utf8_path_new("foo", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE_FALSE(fimo_utf8_path_is_absolute(path));
    }
#ifdef _WIN32
    SECTION("absolute path") {
        error = fimo_utf8_path_new("c:\\windows", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(fimo_utf8_path_is_absolute(path));
    }
#else
    SECTION("root path") {
        error = fimo_utf8_path_new("/foo", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(fimo_utf8_path_is_absolute(path));
    }
#endif
}

TEST_CASE("Path is relative", "[FimoUTF8Path]") {
    FimoResult error;
    FimoUTF8Path path;

    SECTION("relative path") {
        error = fimo_utf8_path_new("foo", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(fimo_utf8_path_is_relative(path));
    }
#ifdef _WIN32
    SECTION("absolute path") {
        error = fimo_utf8_path_new("c:\\windows", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE_FALSE(fimo_utf8_path_is_relative(path));
    }
#else
    SECTION("root path") {
        error = fimo_utf8_path_new("/foo", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE_FALSE(fimo_utf8_path_is_relative(path));
    }
#endif
}

TEST_CASE("Path has root", "[FimoUTF8Path]") {
    FimoResult error;
    FimoUTF8Path path;

    SECTION("no root") {
        error = fimo_utf8_path_new("foo", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE_FALSE(fimo_utf8_path_has_root(path));
    }
#ifdef _WIN32
    SECTION("no prefix with separator") {
        error = fimo_utf8_path_new("\\windows", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(fimo_utf8_path_has_root(path));
    }
    SECTION("prefix with separator") {
        error = fimo_utf8_path_new("c:\\windows", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(fimo_utf8_path_has_root(path));
    }
    SECTION("non-disk prefix") {
        error = fimo_utf8_path_new("\\\\server\\share", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(fimo_utf8_path_has_root(path));
    }
#else
    SECTION("root path") {
        error = fimo_utf8_path_new("/foo", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
        REQUIRE(fimo_utf8_path_has_root(path));
    }
#endif
}

TEST_CASE("Path parent", "[FimoUTF8Path]") {
    SECTION("absolute path") {
        FimoUTF8Path path;
        FimoResult error = fimo_utf8_path_new("/foo/bar", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

        FimoUTF8Path parent;
        bool has_parent = fimo_utf8_path_parent(path, &parent);
        REQUIRE(has_parent);
        REQUIRE(parent.length == sizeof("/foo") - 1);
        REQUIRE(strncmp(parent.path, "/foo", parent.length) == 0);

        FimoUTF8Path grand_parent;
        bool has_grand_parent = fimo_utf8_path_parent(parent, &grand_parent);
        REQUIRE(has_grand_parent);
        REQUIRE(grand_parent.length == sizeof("/") - 1);
        REQUIRE(strncmp(grand_parent.path, "/", grand_parent.length) == 0);

        FimoUTF8Path great_grand_parent;
        REQUIRE_FALSE(fimo_utf8_path_parent(grand_parent, &great_grand_parent));
    }
    SECTION("relative path") {
        FimoUTF8Path path;
        FimoResult error = fimo_utf8_path_new("foo/bar", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

        FimoUTF8Path parent;
        bool has_parent = fimo_utf8_path_parent(path, &parent);
        REQUIRE(has_parent);
        REQUIRE(parent.length == sizeof("foo") - 1);
        REQUIRE(strncmp(parent.path, "foo", parent.length) == 0);

        FimoUTF8Path grand_parent;
        bool has_grand_parent = fimo_utf8_path_parent(parent, &grand_parent);
        REQUIRE(has_grand_parent);
        REQUIRE(grand_parent.length == 0);
        REQUIRE(strncmp(grand_parent.path, "", grand_parent.length) == 0);

        FimoUTF8Path great_grand_parent;
        REQUIRE_FALSE(fimo_utf8_path_parent(grand_parent, &great_grand_parent));
    }
}

TEST_CASE("Path file name", "[FimoUTF8Path]") {
    SECTION("directory") {
        FimoUTF8Path path;
        FimoResult error = fimo_utf8_path_new("/usr/bin/", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

        FimoUTF8Path filename;
        bool has_file_name = fimo_utf8_path_file_name(path, &filename);
        REQUIRE(has_file_name);
        REQUIRE(filename.length == sizeof("bin") - 1);
        REQUIRE(strncmp(filename.path, "bin", filename.length) == 0);
    }
    SECTION("file") {
        FimoUTF8Path path;
        FimoResult error = fimo_utf8_path_new("tmp/foo.txt", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

        FimoUTF8Path filename;
        bool has_file_name = fimo_utf8_path_file_name(path, &filename);
        REQUIRE(has_file_name);
        REQUIRE(filename.length == sizeof("foo.txt") - 1);
        REQUIRE(strncmp(filename.path, "foo.txt", filename.length) == 0);
    }
    SECTION("file non-normalized") {
        FimoUTF8Path path;
        FimoResult error = fimo_utf8_path_new("foo.txt/.", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

        FimoUTF8Path filename;
        bool has_file_name = fimo_utf8_path_file_name(path, &filename);
        REQUIRE(has_file_name);
        REQUIRE(filename.length == sizeof("foo.txt") - 1);
        REQUIRE(strncmp(filename.path, "foo.txt", filename.length) == 0);
    }
    SECTION("file non-normalized 2") {
        FimoUTF8Path path;
        FimoResult error = fimo_utf8_path_new("foo.txt/.//", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

        FimoUTF8Path filename;
        bool has_file_name = fimo_utf8_path_file_name(path, &filename);
        REQUIRE(has_file_name);
        REQUIRE(filename.length == sizeof("foo.txt") - 1);
        REQUIRE(strncmp(filename.path, "foo.txt", filename.length) == 0);
    }
    SECTION("ends with '..'") {
        FimoUTF8Path path;
        FimoResult error = fimo_utf8_path_new("foo.txt/..", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

        FimoUTF8Path filename;
        bool has_file_name = fimo_utf8_path_file_name(path, &filename);
        REQUIRE_FALSE(has_file_name);
    }
    SECTION("root") {
        FimoUTF8Path path;
        FimoResult error = fimo_utf8_path_new("/", &path);
        REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));

        FimoUTF8Path filename;
        bool has_file_name = fimo_utf8_path_file_name(path, &filename);
        REQUIRE_FALSE(has_file_name);
    }
}

TEST_CASE("Path component iteration", "[FimoUTF8Path]") {
    FimoUTF8Path path;
    FimoResult error = fimo_utf8_path_new("/tmp/foo.txt", &path);
    REQUIRE_FALSE(FIMO_RESULT_IS_ERROR(error));
    FimoUTF8PathComponentIterator iterator = fimo_utf8_path_component_iter_new(path);

    SECTION("forwards") {
        FimoUTF8PathComponent component;
        bool has_component = fimo_utf8_path_component_iter_next(&iterator, &component);
        REQUIRE(has_component);
        REQUIRE(component.type == FimoUTF8PathComponent::FIMO_UTF8_PATH_COMPONENT_ROOT_DIR);

        has_component = fimo_utf8_path_component_iter_next(&iterator, &component);
        REQUIRE(has_component);
        REQUIRE(component.type == FimoUTF8PathComponent::FIMO_UTF8_PATH_COMPONENT_NORMAL);
        REQUIRE(component.data.normal.length == sizeof("tmp") - 1);
        REQUIRE(strncmp(component.data.normal.path, "tmp", component.data.normal.length) == 0);

        has_component = fimo_utf8_path_component_iter_next(&iterator, &component);
        REQUIRE(has_component);
        REQUIRE(component.type == FimoUTF8PathComponent::FIMO_UTF8_PATH_COMPONENT_NORMAL);
        REQUIRE(component.data.normal.length == sizeof("foo.txt") - 1);
        REQUIRE(strncmp(component.data.normal.path, "foo.txt", component.data.normal.length) == 0);

        has_component = fimo_utf8_path_component_iter_next(&iterator, &component);
        REQUIRE_FALSE(has_component);
    }
    SECTION("backwards") {
        FimoUTF8PathComponent component;
        bool has_component = fimo_utf8_path_component_iter_next_back(&iterator, &component);
        REQUIRE(has_component);
        REQUIRE(component.type == FimoUTF8PathComponent::FIMO_UTF8_PATH_COMPONENT_NORMAL);
        REQUIRE(component.data.normal.length == sizeof("foo.txt") - 1);
        REQUIRE(strncmp(component.data.normal.path, "foo.txt", component.data.normal.length) == 0);

        has_component = fimo_utf8_path_component_iter_next_back(&iterator, &component);
        REQUIRE(has_component);
        REQUIRE(component.type == FimoUTF8PathComponent::FIMO_UTF8_PATH_COMPONENT_NORMAL);
        REQUIRE(component.data.normal.length == sizeof("tmp") - 1);
        REQUIRE(strncmp(component.data.normal.path, "tmp", component.data.normal.length) == 0);

        has_component = fimo_utf8_path_component_iter_next_back(&iterator, &component);
        REQUIRE(has_component);
        REQUIRE(component.type == FimoUTF8PathComponent::FIMO_UTF8_PATH_COMPONENT_ROOT_DIR);

        has_component = fimo_utf8_path_component_iter_next_back(&iterator, &component);
        REQUIRE_FALSE(has_component);
    }
}
