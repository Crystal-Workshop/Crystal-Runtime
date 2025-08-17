cmake_minimum_required(VERSION 3.16)
project(CrystalRuntime LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

file(GLOB_RECURSE RUNTIME_SOURCES
    src/*.cpp
    include/*.h
)

add_library(CrystalRuntime SHARED ${RUNTIME_SOURCES})

set_target_properties(CrystalRuntime PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY_DEBUG   "${CMAKE_SOURCE_DIR}/Debug"
    RUNTIME_OUTPUT_DIRECTORY_RELEASE "${CMAKE_SOURCE_DIR}/Release"
)

target_compile_definitions(CrystalRuntime PRIVATE CRYSTAL_BUILD_SHARED=1)

target_include_directories(CrystalRuntime PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${CMAKE_CURRENT_SOURCE_DIR}/src
    ${CMAKE_CURRENT_SOURCE_DIR}/third_party/glad/include
    ${CMAKE_CURRENT_SOURCE_DIR}/third_party/glfw/include
    ${CMAKE_CURRENT_SOURCE_DIR}/third_party/glm
)

if(MSVC)
    target_compile_options(CrystalRuntime PRIVATE /W4 /permissive-)
else()
    target_compile_options(CrystalRuntime PRIVATE -Wall -Wextra -Wpedantic)
endif()

include(FetchContent)

# GLM
FetchContent_Declare(
    glm
    GIT_REPOSITORY https://github.com/g-truc/glm.git
    GIT_TAG        1.0.1
)
FetchContent_MakeAvailable(glm)

# GLFW
FetchContent_Declare(
    glfw
    GIT_REPOSITORY https://github.com/glfw/glfw.git
    GIT_TAG        3.4
)
FetchContent_MakeAvailable(glfw)

# Lua

FetchContent_Declare(
    lua
    GIT_REPOSITORY "https://github.com/marovira/lua"
    GIT_TAG "4737fb9"
)

set(LUA_BUILD_AS_DLL OFF CACHE BOOL "" FORCE)
set(LUA_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(LUA_ENABLE_STATIC_LIB ON CACHE BOOL "" FORCE)  # Ensure it builds a static library

FetchContent_MakeAvailable(lua)

# Lua headers
target_include_directories(CrystalRuntime PRIVATE ${lua_SOURCE_DIR}/src)

# Lua library
target_link_libraries(CrystalRuntime PRIVATE lua::lua)

# glad
add_library(glad external/glad/src/glad.c)
target_include_directories(glad PUBLIC external/glad/include)

# Link everything
target_link_libraries(CrystalRuntime PRIVATE glad glfw glm)