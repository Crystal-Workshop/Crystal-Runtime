#include "vector_types.h"
#include <iostream>
#include <cstring>
#include <type_traits>

extern "C" {
    #include "lauxlib.h"
    #include "lualib.h"
}

// Metatable names
static const char* VECTOR3_META = "CGame.Vector3";
static const char* VECTOR2_META = "CGame.Vector2";
static const char* COLOR3_META = "CGame.Color3";

// Helper macros for common operations
#define VECTOR_BINARY_OP(TYPE, METANAME, OP_NAME, OP_SYMBOL) \
static int TYPE##_##OP_NAME(lua_State* L) { \
    TYPE* a = check##TYPE(L, 1); \
    TYPE* b = check##TYPE(L, 2); \
    if (!a || !b) return 0; \
    TYPE result = (*a) OP_SYMBOL (*b); \
    push##TYPE(L, result); \
    return 1; \
}

#define VECTOR_SCALAR_OP(TYPE, METANAME, OP_NAME, OP_SYMBOL) \
static int TYPE##_scalar_##OP_NAME(lua_State* L) { \
    TYPE* a = check##TYPE(L, 1); \
    float scalar = (float)luaL_checknumber(L, 2); \
    if (!a) return 0; \
    TYPE result = (*a) OP_SYMBOL scalar; \
    push##TYPE(L, result); \
    return 1; \
}

// Individual tostring functions for each type
static int Vector3_tostring(lua_State* L) {
    Vector3* vec = checkVector3(L, 1);
    if (!vec) return 0;
    char buffer[128];
    snprintf(buffer, sizeof(buffer), "Vector3(%.3f, %.3f, %.3f)", vec->x(), vec->y(), vec->z());
    lua_pushstring(L, buffer);
    return 1;
}

static int Vector2_tostring(lua_State* L) {
    Vector2* vec = checkVector2(L, 1);
    if (!vec) return 0;
    char buffer[128];
    snprintf(buffer, sizeof(buffer), "Vector2(%.3f, %.3f)", vec->x(), vec->y());
    lua_pushstring(L, buffer);
    return 1;
}

static int Color3_tostring(lua_State* L) {
    Color3* color = checkColor3(L, 1);
    if (!color) return 0;
    char buffer[128];
    snprintf(buffer, sizeof(buffer), "Color3(%.0f, %.0f, %.0f)", color->r(), color->g(), color->b());
    lua_pushstring(L, buffer);
    return 1;
}

// === VECTOR3 IMPLEMENTATION ===

static int Vector3_new(lua_State* L) {
    float x = (float)luaL_optnumber(L, 1, 0.0);
    float y = (float)luaL_optnumber(L, 2, 0.0);
    float z = (float)luaL_optnumber(L, 3, 0.0);
    
    Vector3* vec = (Vector3*)lua_newuserdata(L, sizeof(Vector3));
    new (vec) Vector3(x, y, z);
    
    luaL_getmetatable(L, VECTOR3_META);
    lua_setmetatable(L, -2);
    return 1;
}

static int Vector3_index(lua_State* L) {
    Vector3* vec = checkVector3(L, 1);
    const char* key = luaL_checkstring(L, 2);
    
    if (!vec) return 0;
    
    if (strcmp(key, "x") == 0) {
        lua_pushnumber(L, vec->x());
        return 1;
    } else if (strcmp(key, "y") == 0) {
        lua_pushnumber(L, vec->y());
        return 1;
    } else if (strcmp(key, "z") == 0) {
        lua_pushnumber(L, vec->z());
        return 1;
    }
    
    // Allow method access
    lua_getmetatable(L, 1);
    lua_getfield(L, -1, key);
    return 1;
}

// Binary operations
VECTOR_BINARY_OP(Vector3, VECTOR3_META, add, +)
VECTOR_BINARY_OP(Vector3, VECTOR3_META, sub, -)
VECTOR_BINARY_OP(Vector3, VECTOR3_META, mul, *)
VECTOR_BINARY_OP(Vector3, VECTOR3_META, div, /)

// Scalar operations
VECTOR_SCALAR_OP(Vector3, VECTOR3_META, mul, *)
VECTOR_SCALAR_OP(Vector3, VECTOR3_META, div, /)

// Special multiplication handler that handles both vector and scalar
static int Vector3_mul_dispatch(lua_State* L) {
    if (lua_isnumber(L, 2)) {
        return Vector3_scalar_mul(L);
    } else {
        return Vector3_mul(L);
    }
}

static int Vector3_div_dispatch(lua_State* L) {
    if (lua_isnumber(L, 2)) {
        return Vector3_scalar_div(L);
    } else {
        return Vector3_div(L);
    }
}

// Individual tostring functions for each type

// === VECTOR2 IMPLEMENTATION ===

static int Vector2_new(lua_State* L) {
    float x = (float)luaL_optnumber(L, 1, 0.0);
    float y = (float)luaL_optnumber(L, 2, 0.0);
    
    Vector2* vec = (Vector2*)lua_newuserdata(L, sizeof(Vector2));
    new (vec) Vector2(x, y);
    
    luaL_getmetatable(L, VECTOR2_META);
    lua_setmetatable(L, -2);
    return 1;
}

static int Vector2_index(lua_State* L) {
    Vector2* vec = checkVector2(L, 1);
    const char* key = luaL_checkstring(L, 2);
    
    if (!vec) return 0;
    
    if (strcmp(key, "x") == 0) {
        lua_pushnumber(L, vec->x());
        return 1;
    } else if (strcmp(key, "y") == 0) {
        lua_pushnumber(L, vec->y());
        return 1;
    }
    
    // Allow method access
    lua_getmetatable(L, 1);
    lua_getfield(L, -1, key);
    return 1;
}

// Binary operations for Vector2
VECTOR_BINARY_OP(Vector2, VECTOR2_META, add, +)
VECTOR_BINARY_OP(Vector2, VECTOR2_META, sub, -)
VECTOR_BINARY_OP(Vector2, VECTOR2_META, mul, *)
VECTOR_BINARY_OP(Vector2, VECTOR2_META, div, /)

// Scalar operations for Vector2
VECTOR_SCALAR_OP(Vector2, VECTOR2_META, mul, *)
VECTOR_SCALAR_OP(Vector2, VECTOR2_META, div, /)

static int Vector2_mul_dispatch(lua_State* L) {
    if (lua_isnumber(L, 2)) {
        return Vector2_scalar_mul(L);
    } else {
        return Vector2_mul(L);
    }
}

static int Vector2_div_dispatch(lua_State* L) {
    if (lua_isnumber(L, 2)) {
        return Vector2_scalar_div(L);
    } else {
        return Vector2_div(L);
    }
}

// === COLOR3 IMPLEMENTATION ===

static int Color3_new(lua_State* L) {
    float r = (float)luaL_optnumber(L, 1, 0.0);
    float g = (float)luaL_optnumber(L, 2, 0.0);
    float b = (float)luaL_optnumber(L, 3, 0.0);
    
    Color3* color = (Color3*)lua_newuserdata(L, sizeof(Color3));
    new (color) Color3(r, g, b);
    
    luaL_getmetatable(L, COLOR3_META);
    lua_setmetatable(L, -2);
    return 1;
}

static int Color3_index(lua_State* L) {
    Color3* color = checkColor3(L, 1);
    const char* key = luaL_checkstring(L, 2);
    
    if (!color) return 0;
    
    if (strcmp(key, "r") == 0) {
        lua_pushnumber(L, color->r());
        return 1;
    } else if (strcmp(key, "g") == 0) {
        lua_pushnumber(L, color->g());
        return 1;
    } else if (strcmp(key, "b") == 0) {
        lua_pushnumber(L, color->b());
        return 1;
    }
    
    // Allow method access
    lua_getmetatable(L, 1);
    lua_getfield(L, -1, key);
    return 1;
}

// Binary operations for Color3
VECTOR_BINARY_OP(Color3, COLOR3_META, add, +)
VECTOR_BINARY_OP(Color3, COLOR3_META, sub, -)
VECTOR_BINARY_OP(Color3, COLOR3_META, mul, *)
VECTOR_BINARY_OP(Color3, COLOR3_META, div, /)

// Scalar operations for Color3
VECTOR_SCALAR_OP(Color3, COLOR3_META, mul, *)
VECTOR_SCALAR_OP(Color3, COLOR3_META, div, /)

static int Color3_mul_dispatch(lua_State* L) {
    if (lua_isnumber(L, 2)) {
        return Color3_scalar_mul(L);
    } else {
        return Color3_mul(L);
    }
}

static int Color3_div_dispatch(lua_State* L) {
    if (lua_isnumber(L, 2)) {
        return Color3_scalar_div(L);
    } else {
        return Color3_div(L);
    }
}

// === PUBLIC API IMPLEMENTATION ===

void registerVectorTypes(lua_State* L) {
    // Register Vector3
    if (luaL_newmetatable(L, VECTOR3_META)) {
        lua_pushcfunction(L, Vector3_index);
        lua_setfield(L, -2, "__index");
        
        lua_pushcfunction(L, Vector3_add);
        lua_setfield(L, -2, "__add");
        
        lua_pushcfunction(L, Vector3_sub);
        lua_setfield(L, -2, "__sub");
        
        lua_pushcfunction(L, Vector3_mul_dispatch);
        lua_setfield(L, -2, "__mul");
        
        lua_pushcfunction(L, Vector3_div_dispatch);
        lua_setfield(L, -2, "__div");
        
        lua_pushcfunction(L, Vector3_tostring);
        lua_setfield(L, -2, "__tostring");
    }
    lua_pop(L, 1);
    
    // Create Vector3 constructor
    lua_newtable(L);
    lua_pushcfunction(L, Vector3_new);
    lua_setfield(L, -2, "new");
    lua_setglobal(L, "Vector3");
    
    // Register Vector2
    if (luaL_newmetatable(L, VECTOR2_META)) {
        lua_pushcfunction(L, Vector2_index);
        lua_setfield(L, -2, "__index");
        
        lua_pushcfunction(L, Vector2_add);
        lua_setfield(L, -2, "__add");
        
        lua_pushcfunction(L, Vector2_sub);
        lua_setfield(L, -2, "__sub");
        
        lua_pushcfunction(L, Vector2_mul_dispatch);
        lua_setfield(L, -2, "__mul");
        
        lua_pushcfunction(L, Vector2_div_dispatch);
        lua_setfield(L, -2, "__div");
        
        lua_pushcfunction(L, Vector2_tostring);
        lua_setfield(L, -2, "__tostring");
    }
    lua_pop(L, 1);
    
    // Create Vector2 constructor
    lua_newtable(L);
    lua_pushcfunction(L, Vector2_new);
    lua_setfield(L, -2, "new");
    lua_setglobal(L, "Vector2");
    
    // Register Color3
    if (luaL_newmetatable(L, COLOR3_META)) {
        lua_pushcfunction(L, Color3_index);
        lua_setfield(L, -2, "__index");
        
        lua_pushcfunction(L, Color3_add);
        lua_setfield(L, -2, "__add");
        
        lua_pushcfunction(L, Color3_sub);
        lua_setfield(L, -2, "__sub");
        
        lua_pushcfunction(L, Color3_mul_dispatch);
        lua_setfield(L, -2, "__mul");
        
        lua_pushcfunction(L, Color3_div_dispatch);
        lua_setfield(L, -2, "__div");
        
        lua_pushcfunction(L, Color3_tostring);
        lua_setfield(L, -2, "__tostring");
    }
    lua_pop(L, 1);
    
    // Create Color3 constructor
    lua_newtable(L);
    lua_pushcfunction(L, Color3_new);
    lua_setfield(L, -2, "new");
    lua_setglobal(L, "Color3");
}

void pushVector3(lua_State* L, const Vector3& vec) {
    Vector3* udata = (Vector3*)lua_newuserdata(L, sizeof(Vector3));
    new (udata) Vector3(vec);
    luaL_getmetatable(L, VECTOR3_META);
    lua_setmetatable(L, -2);
}

void pushVector2(lua_State* L, const Vector2& vec) {
    Vector2* udata = (Vector2*)lua_newuserdata(L, sizeof(Vector2));
    new (udata) Vector2(vec);
    luaL_getmetatable(L, VECTOR2_META);
    lua_setmetatable(L, -2);
}

void pushColor3(lua_State* L, const Color3& color) {
    Color3* udata = (Color3*)lua_newuserdata(L, sizeof(Color3));
    new (udata) Color3(color);
    luaL_getmetatable(L, COLOR3_META);
    lua_setmetatable(L, -2);
}

Vector3* checkVector3(lua_State* L, int idx) {
    return (Vector3*)luaL_checkudata(L, idx, VECTOR3_META);
}

Vector2* checkVector2(lua_State* L, int idx) {
    return (Vector2*)luaL_checkudata(L, idx, VECTOR2_META);
}

Color3* checkColor3(lua_State* L, int idx) {
    return (Color3*)luaL_checkudata(L, idx, COLOR3_META);
}