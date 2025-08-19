#include <iostream>
#include <string>
#include <cstring>
#include "datamodel.h"
#include "scene.h"
#include "input.h"
#include "vector_types.h"
#include <glm/glm.hpp>
#include <GLFW/glfw3.h>

extern "C" {
    #include "lua.h"
    #include "lauxlib.h"
    #include "lualib.h"
}

// Global pointer (declared in datamodel.cpp).
extern DataModel* m_dataModel;
static InputState* g_inputState = nullptr;

// ObjectRef: userdata for an object
struct ObjectRef {
    std::string name;
    DataModel* dm;
};

// Forward declarations
static int objectref_index(lua_State* L);
static int objectref_newindex(lua_State* L);
static int place_index(lua_State* L);

// -------------------- ObjectRef metamethods --------------------
static int objectref_index(lua_State* L) {
    ObjectRef* ref = (ObjectRef*)luaL_checkudata(L, 1, "CGame.ObjectRef");
    const char* key = luaL_checkstring(L, 2);
    if(!ref || !ref->dm) { lua_pushnil(L); return 1; }

    auto maybeObj = ref->dm->getObjectCopy(ref->name);
    if(!maybeObj) {
        lua_pushnil(L);
        return 1;
    }

    // Return Vector3 objects for position, rotation, scale
    if(strcmp(key,"position")==0) {
        Vector3 pos(maybeObj->position);
        pushVector3(L, pos);
        return 1;
    } else if(strcmp(key,"rotation")==0) {
        Vector3 rot(maybeObj->rotation);
        pushVector3(L, rot);
        return 1;
    } else if(strcmp(key,"scale")==0) {
        Vector3 scale(maybeObj->scale);
        pushVector3(L, scale);
        return 1;
    } else if(strcmp(key,"color")==0) {
        Color3 color(maybeObj->color);
        pushColor3(L, color);
        return 1;
    }

    if(strcmp(key, "fov") == 0) {
        lua_pushnumber(L, maybeObj->fov);
        return 1;
    }

    // also return name property for convenience
    if(strcmp(key,"name")==0) {
        lua_pushstring(L, maybeObj->name.c_str());
        return 1;
    }

    lua_pushnil(L);
    return 1;
}

static int objectref_newindex(lua_State* L) {
    ObjectRef* ref = (ObjectRef*)luaL_checkudata(L, 1, "CGame.ObjectRef");
    const char* key = luaL_checkstring(L, 2);
    if(!ref || !ref->dm) return 0;

    if(strcmp(key,"position")==0) {
        Vector3* vec = checkVector3(L, 3);
        if(vec) {
            ref->dm->setPosition(ref->name, vec->toGLM());
        } else {
            luaL_error(L, "position must be a Vector3");
        }
        return 0;
    } else if(strcmp(key,"rotation")==0) {
        Vector3* vec = checkVector3(L, 3);
        if(vec) {
            ref->dm->setRotation(ref->name, vec->toGLM());
        } else {
            luaL_error(L, "rotation must be a Vector3");
        }
        return 0;
    } else if(strcmp(key,"scale")==0) {
        Vector3* vec = checkVector3(L, 3);
        if(vec) {
            ref->dm->setScale(ref->name, vec->toGLM());
        } else {
            luaL_error(L, "scale must be a Vector3");
        }
        return 0;
    } else if(strcmp(key,"color")==0) {
        Color3* color = checkColor3(L, 3);
        if(color) {
            ref->dm->setColor(ref->name, color->toGLM());
        } else {
            luaL_error(L, "color must be a Color3");
        }
        return 0;
    } else if(strcmp(key, "fov") == 0) {
        float fov = (float)luaL_checknumber(L, 3);
        ref->dm->setFov(ref->name, fov);
        return 0;
    }

    // ignore other assignments
    return 0;
}

// -------------------- place.__index --------------------
static int place_index(lua_State* L) {
    const char* name = luaL_checkstring(L, 2);
    // use global pointer provided by registerPlaceGlobal
    if(m_dataModel == nullptr) {
        lua_pushnil(L);
        return 1;
    }

    // Try find object
    auto maybeObj = m_dataModel->getObjectCopy(name);
    if (!maybeObj) {
        lua_pushnil(L); // Return nil for this lookup
        return 1;
    }

    // push ObjectRef userdata
    ObjectRef* ref = (ObjectRef*)lua_newuserdata(L, sizeof(ObjectRef));
    new (ref) ObjectRef();
    ref->name = name;
    ref->dm = m_dataModel;
    luaL_getmetatable(L, "CGame.ObjectRef");
    lua_setmetatable(L, -2);
    return 1;
}

// -------------------- registration --------------------
void registerPlaceGlobal(lua_State* L, DataModel* dataModel) {
    // set global pointer
    m_dataModel = dataModel;
    
    // Register vector types first
    registerVectorTypes(L);
    
    // ObjectRef metatable
    if(luaL_newmetatable(L, "CGame.ObjectRef")) {
        lua_pushcfunction(L, objectref_index);
        lua_setfield(L, -2, "__index");
        lua_pushcfunction(L, objectref_newindex); 
        lua_setfield(L, -2, "__newindex");
    }
    lua_pop(L, 1);

    // Create `place` global with a metatable __index = place_index
    lua_newtable(L);
    lua_newtable(L);
    lua_pushcfunction(L, place_index);
    lua_setfield(L, -2, "__index");
    lua_setmetatable(L, -2);
    lua_setglobal(L, "place");
}

// -------------------- service.input:GetKeyDown  --------------------
// C function exposed as service.input:GetKeyDown(name) -> boolean
static int lua_service_input_GetKeyDown(lua_State* L) {
    // upvalue 1 should be lightuserdata InputState*
    void* ud = lua_touserdata(L, lua_upvalueindex(1));
    InputState* input = (InputState*)ud;
    if(!input) {
        lua_pushboolean(L, 0);
        return 1;
    }

    const char* name = nullptr;
    if(lua_isstring(L, 1)) {
        name = lua_tostring(L, 1);
    } else if(lua_isstring(L, 2)) {
        name = lua_tostring(L, 2);
    } else {
        lua_pushboolean(L, 0);
        return 1;
    }

    bool down = input->isKeyDownByName(std::string(name));
    lua_pushboolean(L, down ? 1 : 0);
    return 1;
}

// -------------------- service.input:GetMousePosition  --------------------
static int lua_service_input_GetMousePosition(lua_State* L) {
    void* ud = lua_touserdata(L, lua_upvalueindex(1));
    InputState* input = (InputState*)ud;
    if (!input) {
        lua_pushnil(L);
        return 1;
    }
    
    glm::vec2 pos = input->getMousePosition();
    Vector2 mousePos(pos);
    pushVector2(L, mousePos);
    return 1;
}

void registerServiceGlobal(lua_State* L, InputState* inputState) {
    g_inputState = inputState;
    // Make service table
    lua_newtable(L); // service

    // make input table
    lua_newtable(L);
    // input

    // Push input pointer as lightuserdata upvalue for GetKeyDown
    lua_pushlightuserdata(L, (void*)inputState);
    lua_pushcclosure(L, lua_service_input_GetKeyDown, 1);
    lua_setfield(L, -2, "GetKeyDown");

    // Push input pointer as lightuserdata upvalue for GetMousePosition
    lua_pushlightuserdata(L, (void*)inputState);
    lua_pushcclosure(L, lua_service_input_GetMousePosition, 1);
    lua_setfield(L, -2, "GetMousePosition");

    // set input table on service
    lua_setfield(L, -2, "input");
    // set service global
    lua_setglobal(L, "service");
}

// -------------------- screen:GetViewportSize --------------------
static int lua_screen_GetViewportSize(lua_State* L) {
    void* ud = lua_touserdata(L, lua_upvalueindex(1));
    GLFWwindow* wnd = (GLFWwindow*)ud;
    if (!wnd) {
        lua_pushnil(L);
        return 1;
    }

    int width, height;
    glfwGetFramebufferSize(wnd, &width, &height);

    Vector2 viewportSize((float)width, (float)height);
    pushVector2(L, viewportSize);
    return 1;
}

void registerScreenGlobal(lua_State* L, GLFWwindow* wnd) {
    // Make screen table
    lua_newtable(L);

    // Push GetViewportSize function
    lua_pushlightuserdata(L, (void*)wnd);
    lua_pushcclosure(L, lua_screen_GetViewportSize, 1);
    lua_setfield(L, -2, "GetViewportSize");

    // set screen global
    lua_setglobal(L, "screen");
}