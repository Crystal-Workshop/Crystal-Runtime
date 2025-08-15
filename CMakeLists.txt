cmake_minimum_required(VERSION 3.16)
project(CrystalRuntime LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

file(GLOB_RECURSE RUNTIME_SOURCES
    src/*.cpp
    include/*.h
)

if(WIN32)
    list(APPEND RUNTIME_SOURCES ${CMAKE_SOURCE_DIR}/app.rc)
endif()

add_executable(CrystalRuntime ${RUNTIME_SOURCES})

target_include_directories(CrystalRuntime PRIVATE
    ${CMAKE_SOURCE_DIR}/include
    third_party/glad/include
    third_party/glfw/include
    third_party/glm
)

if(MSVC)
    target_compile_options(CrystalRuntime PRIVATE /W4 /permissive-)
    set_target_properties(CrystalRuntime PROPERTIES WIN32_EXECUTABLE YES)
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

# Lua (marovira/lua with static build and no tests)
set(LUA_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(LUA_BUILD_AS_DLL OFF CACHE BOOL "" FORCE)
FetchContent_Declare(
    lua
    GIT_REPOSITORY https://github.com/marovira/lua
    GIT_TAG        4737fb9
)
FetchContent_MakeAvailable(lua)
# set(LUA_BUILD_TESTS OFF CACHE BOOL "" FORCE)
# set(LUA_BUILD_AS_DLL OFF CACHE BOOL "" FORCE)

# glad
add_library(glad external/glad/src/glad.c)
target_include_directories(glad PUBLIC external/glad/include)

# Link everything
target_link_libraries(CrystalRuntime PRIVATE glad glfw glm lua)