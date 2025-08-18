#ifndef LUA_THREAD_H
#define LUA_THREAD_H

#include <string>
#include <vector>
#include "archive.h"
#include "datamodel.h"
#include "input.h"

// Forward-declare GLFWwindow to avoid including glfw3.h in the header
struct GLFWwindow;

class LuaScriptManager {
    struct Impl;
Impl* m_impl;
    std::string m_archivePath;
    std::vector<ArchiveFileEntry> m_files;
    DataModel* m_dataModel;
    InputState* m_inputState;
    GLFWwindow* m_wnd;
public:
    LuaScriptManager(const std::string& archivePath, const std::vector<ArchiveFileEntry>& files, DataModel* dm, InputState* inputState, GLFWwindow* wnd);
    ~LuaScriptManager();
    void startAllScripts();
    void stopAllScripts();
};
#endif