#include "lua_thread.h"
#include <thread>
#include <atomic>
#include <chrono>
#include <iostream>
#include "datamodel.h"
#include "archive.h"
#include "input.h"
extern "C" {
    #include "lua.h"
    #include "lauxlib.h"
    #include "lualib.h"
}

// Forward declaration from lua_bind.cpp
void registerPlaceGlobal(lua_State* L, DataModel* dataModel);
void registerServiceGlobal(lua_State* L, InputState* inputState);
// ---------------------- cooperative stop support ----------------------
static std::atomic<bool>* g_running_ptr = nullptr;
static void lua_stop_hook(lua_State* L, lua_Debug* ar) {
    (void)ar;
    if (g_running_ptr && !g_running_ptr->load(std::memory_order_acquire)) {
        luaL_error(L, "script stopped by host");
    }
}
// print bound to stdout
static int lua_print_impl(lua_State* L) {
    int nargs = lua_gettop(L);
    for (int i = 1; i <= nargs; i++) {
        if (lua_isstring(L, i)) {
            std::cout << lua_tostring(L, i);
        } else if (lua_isnil(L, i)) {
            std::cout << "nil";
        } else if (lua_isboolean(L, i)) {
            std::cout << (lua_toboolean(L, i) ? "true" : "false");
        } else {
            lua_getglobal(L, "tostring");
            lua_pushvalue(L, i);
            if (lua_pcall(L, 1, 1, 0) == LUA_OK) {
                const char* s = lua_tostring(L, -1);
                if (s) std::cout << s;
            }
            lua_pop(L, 1);
        }
        if (i < nargs) std::cout << "\t";
    }
    std::cout << std::endl;
    std::cout.flush();
    return 0;
}
// wait(milliseconds) - sleeps this script's thread, but also returns early if host asked stop.
static int lua_wait_impl(lua_State* L) {
    int ms = (int)luaL_optinteger(L, 1, 0);
    if (ms <= 0) {
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
        return 0;
    }
    const int chunk = 10;
    int remaining = ms;
    while (remaining > 0) {
        int sleepFor = (remaining > chunk) ? chunk : remaining;
        std::this_thread::sleep_for(std::chrono::milliseconds(sleepFor));
        remaining -= sleepFor;
        if (g_running_ptr && !g_running_ptr->load(std::memory_order_acquire)) {
            luaL_error(L, "wait interrupted by host stop");
        }
    }
    return 0;
}
// Per-script runner
static void runScriptThread(const std::string archivePath, ArchiveFileEntry entry, DataModel* dm, InputState* inputState) {
    lua_State* L = luaL_newstate();
    luaL_openlibs(L);
    // Install cooperative stop hook: run every 1000 VM instructions
    lua_sethook(L, lua_stop_hook, LUA_MASKCOUNT, 1000);
    // Register print and wait
    lua_pushcfunction(L, lua_print_impl);
    lua_setglobal(L, "print");
    lua_pushcfunction(L, lua_wait_impl);
    lua_setglobal(L, "wait");
    registerPlaceGlobal(L, dm);
    registerServiceGlobal(L, inputState);
    std::cout << "[LuaScriptManager] Registered place & service globals for script: " << entry.name << "\n";
    // Extract script
    std::string scriptSrc;
    if (!extractFileFromArchive(archivePath, entry, scriptSrc)) {
        std::cerr << "Failed to extract script: " << entry.name << "\n";
        lua_close(L);
        return;
    }
    // Load & run
    int loadStatus = luaL_loadbuffer(L, scriptSrc.c_str(), scriptSrc.size(), entry.name.c_str());
    if (loadStatus != LUA_OK) {
        std::cerr << "Error loading script " << entry.name << ": " << lua_tostring(L, -1) << "\n";
        lua_close(L);
        return;
    }
    int pcallStatus = lua_pcall(L, 0, LUA_MULTRET, 0);
    if (pcallStatus != LUA_OK) {
        std::cerr << "Lua runtime error in " << entry.name << ": " << lua_tostring(L, -1) << "\n";
        lua_close(L);
        return;
    }
    lua_close(L);
}
struct LuaScriptManager::Impl {
    std::vector<std::thread> threads;
    std::atomic<bool> running{false};
};
LuaScriptManager::LuaScriptManager(const std::string& archivePath,
                                 const std::vector<ArchiveFileEntry>& files,
                                 DataModel* dm,
                                 InputState* inputState)
    : m_archivePath(archivePath), m_files(files), m_dataModel(dm), m_inputState(inputState), m_impl(new Impl())
{}
LuaScriptManager::~LuaScriptManager() {
    stopAllScripts();
}
void LuaScriptManager::startAllScripts() {
    if (m_impl->running.exchange(true)) return;
    g_running_ptr = &m_impl->running;
    for (const auto& entry : m_files) {
        if (entry.name.rfind("scripts/", 0) == 0) {
            std::string archiveCopy = m_archivePath;
            ArchiveFileEntry e = entry;
            DataModel* dm = m_dataModel;
            InputState* inputState = m_inputState;
            m_impl->threads.emplace_back([archiveCopy, e, dm, inputState]() {
                runScriptThread(archiveCopy, e, dm, inputState);
            });
        }
    }
}
void LuaScriptManager::stopAllScripts() {
    if (!m_impl->running.exchange(false)) {
        // was not running
    }
    for (auto& t : m_impl->threads) {
        if (t.joinable()) t.join();
    }
    m_impl->threads.clear();
    g_running_ptr = nullptr;
}