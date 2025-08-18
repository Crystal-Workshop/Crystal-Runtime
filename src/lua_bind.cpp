// lua_bind.cpp
// Single-file, robust binding that exposes `place.<name>` and live Vec3 proxies
// for position/rotation/scale/color.
#include <iostream>
#include <string>
#include <cstring>
#include "datamodel.h"
#include "scene.h"
#include "input.h"
#include <glm/glm.hpp>
#include <GLFW/glfw3.h>

extern "C" {
    #include "lua.h"
    #include "lauxlib.h"
    #include "lualib.h"
}


// Global pointer (declared in datamodel.cpp).
extern DataModel* m_dataModel;
// We'll keep a local global pointer for InputState in this translation unit.
static InputState* g_inputState = nullptr;
// ObjectRef: userdata for an object
struct ObjectRef {
    std::string name;
    DataModel* dm;
};
// Vec3Ref: userdata for a specific vec3 property of an object
struct Vec3Ref {
    std::string name;
    DataModel* dm;
    enum Field { POSITION, ROTATION, SCALE, COLOR } field;
};

// Forward declarations
static int objectref_index(lua_State* L);
static int objectref_newindex(lua_State* L);
static int vec3ref_index(lua_State* L);
static int vec3ref_newindex(lua_State* L);
static int place_index(lua_State* L);
// -------------------- helpers --------------------
static bool parseVec3FromLua(lua_State* L, int idx, glm::vec3 &out) {
    if(lua_type(L, idx) == LUA_TTABLE) {
        // numeric indices [1],[2],[3]
        lua_geti(L, idx, 1);
        if(lua_isnumber(L, -1)) {
            float x = (float)lua_tonumber(L, -1); lua_pop(L,1);
            lua_geti(L, idx, 2); float y = (float)lua_tonumber(L, -1); lua_pop(L,1);
            lua_geti(L, idx, 3); float z = (float)lua_tonumber(L, -1); lua_pop(L,1);
            out = glm::vec3(x,y,z);
            return true;
        }
        lua_pop(L,1);
        // named fields x,y,z
        lua_getfield(L, idx, "x"); if(!lua_isnumber(L,-1)) { lua_pop(L,1); return false; }
        float x = (float)lua_tonumber(L,-1); lua_pop(L,1);
        lua_getfield(L, idx, "y"); if(!lua_isnumber(L,-1)) { lua_pop(L,1); return false; }
        float y = (float)lua_tonumber(L,-1); lua_pop(L,1);
        lua_getfield(L, idx, "z");
        if(!lua_isnumber(L,-1)) { lua_pop(L,1); return false; }
        float z = (float)lua_tonumber(L,-1); lua_pop(L,1);
        out = glm::vec3(x,y,z);
        return true;
    } else if(lua_isstring(L, idx)) {
        const char* s = lua_tostring(L, idx);
        float x=0,y=0,z=0;
        if(sscanf(s, "%f %f %f", &x, &y, &z) == 3) {
            out = glm::vec3(x,y,z);
            return true;
        }
    }
    return false;
}

// -------------------- Vec3Ref metamethods --------------------
static int vec3ref_index(lua_State* L) {
    Vec3Ref* ref = (Vec3Ref*)luaL_checkudata(L, 1, "CGame.Vec3Ref");
    const char* key = luaL_checkstring(L, 2);
    if(!ref || !ref->dm) { lua_pushnil(L); return 1; }

    auto maybeObj = ref->dm->getObjectCopy(ref->name);
    if(!maybeObj) { lua_pushnil(L); return 1; }

    glm::vec3 val;
    switch(ref->field) {
        case Vec3Ref::POSITION: val = maybeObj->position; break;
        case Vec3Ref::ROTATION: val = maybeObj->rotation; break;
        case Vec3Ref::SCALE:    val = maybeObj->scale;    break;
        case Vec3Ref::COLOR:    val = maybeObj->color;    break;
    }

    if(strcmp(key,"x")==0) lua_pushnumber(L, val.x);
    else if(strcmp(key,"y")==0) lua_pushnumber(L, val.y);
    else if(strcmp(key,"z")==0) lua_pushnumber(L, val.z);
    else lua_pushnil(L);
    return 1;
}

static int vec3ref_newindex(lua_State* L) {
    Vec3Ref* ref = (Vec3Ref*)luaL_checkudata(L, 1, "CGame.Vec3Ref");
    const char* key = luaL_checkstring(L, 2);
    if(!ref || !ref->dm) return 0;

    auto maybeObj = ref->dm->getObjectCopy(ref->name);
    if(!maybeObj) return 0; // object disappeared

    glm::vec3 value;
    switch(ref->field) {
        case Vec3Ref::POSITION: value = maybeObj->position; break;
        case Vec3Ref::ROTATION: value = maybeObj->rotation; break;
        case Vec3Ref::SCALE:    value = maybeObj->scale;    break;
        case Vec3Ref::COLOR:    value = maybeObj->color;    break;
    }

    if(strcmp(key,"x")==0) value.x = (float)luaL_checknumber(L, 3);
    else if(strcmp(key,"y")==0) value.y = (float)luaL_checknumber(L, 3);
    else if(strcmp(key,"z")==0) value.z = (float)luaL_checknumber(L, 3);
    else return 0;

    switch(ref->field) {
        case Vec3Ref::POSITION: ref->dm->setPosition(ref->name, value); break;
        case Vec3Ref::ROTATION: ref->dm->setRotation(ref->name, value); break;
        case Vec3Ref::SCALE:    ref->dm->setScale(ref->name, value);    break;
        case Vec3Ref::COLOR:    ref->dm->setColor(ref->name, value);    break;
    }
    return 0;
}

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

    // Provide Vec3Ref for these properties (live proxy)
    if(strcmp(key,"position")==0 || strcmp(key,"rotation")==0 ||
       strcmp(key,"scale")==0    || strcmp(key,"color")==0) {
        Vec3Ref* vref = (Vec3Ref*)lua_newuserdata(L, sizeof(Vec3Ref));
        new (vref) Vec3Ref(); // placement new to construct strings
        vref->dm = ref->dm;
        vref->name = ref->name;
        if(strcmp(key,"position")==0) vref->field = Vec3Ref::POSITION;
        else if(strcmp(key,"rotation")==0) vref->field = Vec3Ref::ROTATION;
        else if(strcmp(key,"scale")==0) vref->field = Vec3Ref::SCALE;
        else vref->field = Vec3Ref::COLOR;

        // attach Vec3Ref metatable (must be registered before scripts)
        luaL_getmetatable(L, "CGame.Vec3Ref");
        lua_setmetatable(L, -2);
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

    glm::vec3 v;
    if(strcmp(key,"position")==0) {
        if(parseVec3FromLua(L, 3, v)) {
            ref->dm->setPosition(ref->name, v);
        } else {
            luaL_error(L, "invalid position assignment");
        }
        return 0;
    } else if(strcmp(key,"rotation")==0) {
        if(parseVec3FromLua(L, 3, v)) ref->dm->setRotation(ref->name, v);
        return 0;
    } else if(strcmp(key,"scale")==0) {
        if(parseVec3FromLua(L, 3, v)) ref->dm->setScale(ref->name, v);
        return 0;
    } else if(strcmp(key,"color")==0) {
        if(parseVec3FromLua(L, 3, v)) ref->dm->setColor(ref->name, v);
        return 0;
    }else if(strcmp(key, "fov") == 0) {
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
    // and do not modify the DataModel
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
    // ObjectRef metatable
    if(luaL_newmetatable(L, "CGame.ObjectRef")) {
        lua_pushcfunction(L, objectref_index);
        lua_setfield(L, -2, "__index");
        lua_pushcfunction(L, objectref_newindex); lua_setfield(L, -2, "__newindex");
    }
    lua_pop(L, 1);
    // Vec3Ref metatable
    if(luaL_newmetatable(L, "CGame.Vec3Ref")) {
        lua_pushcfunction(L, vec3ref_index);
        lua_setfield(L, -2, "__index");
        lua_pushcfunction(L, vec3ref_newindex); lua_setfield(L, -2, "__newindex");

        // __tostring for nice prints
        lua_pushcfunction(L, [](lua_State* L) -> int {
            Vec3Ref* ref = (Vec3Ref*)luaL_checkudata(L, 1, "CGame.Vec3Ref");
            if(!ref || !ref->dm) { lua_pushstring(L, "nil"); return 1; }
            auto maybeObj = ref->dm->getObjectCopy(ref->name);
            if(!maybeObj) { lua_pushstring(L, "nil"); return 1; }
     
           glm::vec3 val;
            switch(ref->field) {
                case Vec3Ref::POSITION: val = maybeObj->position; break;
                case Vec3Ref::ROTATION: val = maybeObj->rotation; break;
                case Vec3Ref::SCALE:    val = maybeObj->scale;    break;
          
              case Vec3Ref::COLOR:    val = maybeObj->color;    break;
            }
            char buf[64];
            snprintf(buf, sizeof(buf), "(%.3f, %.3f, %.3f)", val.x, val.y, val.z);
            lua_pushstring(L, buf);
            return 1;
        });
        lua_setfield(L, -2, "__tostring");
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
    lua_newtable(L);
    lua_pushnumber(L, pos.x);
    lua_setfield(L, -2, "x");
    lua_pushnumber(L, pos.y);
    lua_setfield(L, -2, "y");
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

    lua_newtable(L);
    lua_pushnumber(L, width);
    lua_setfield(L, -2, "x");
    lua_pushnumber(L, height);
    lua_setfield(L, -2, "y");
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