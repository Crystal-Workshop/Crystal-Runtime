#pragma once
extern "C" {
#include <lua.h>
}
#include "archive.h" 
#include <string>
#include <vector>

class DataModel;

/**
 * Create a Lua state, bind `place` to the provided DataModel, and run scripts from
 * the archive's `files` list that are under the "scripts/" prefix.
 *
 * - archivePath: path to archive (used by extractFileFromArchive)
 * - files: list of ArchiveFileEntry found by loadCGame
 * - dataModel: pointer to the datamodel (lifetime must outlive the Lua state)
 *
 * Returns a pointer to a lua_State (owned by the caller) or nullptr on failure.
 */
lua_State* createLuaAndRunScripts(const std::string &archivePath,
                                  const std::vector<ArchiveFileEntry> &files,
                                  DataModel *dataModel);

/** Close/destroy a lua_State created above. */
void destroyLua(lua_State* L);
