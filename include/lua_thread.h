#ifndef LUA_THREAD_H
#define LUA_THREAD_H

#include <string>
#include <vector>
#include "archive.h"
#include "datamodel.h"
#include "input.h"

class LuaScriptManager {
    struct Impl;
    Impl* m_impl;
    std::string m_archivePath;
    std::vector<ArchiveFileEntry> m_files;
    DataModel* m_dataModel;
    InputState* m_inputState;
public:
    LuaScriptManager(const std::string& archivePath, const std::vector<ArchiveFileEntry>& files, DataModel* dm, InputState* inputState);
    ~LuaScriptManager();
    void startAllScripts();
    void stopAllScripts();
};

#endif