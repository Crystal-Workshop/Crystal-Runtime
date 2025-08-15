#ifndef REGISTER_FUNCTIONS_H
#define REGISTER_FUNCTIONS_H

extern "C" {
#include "lua.h"
}

// Declare registration functions without definitions
void registerPrint(lua_State* L);
void registerWait(lua_State* L);

#endif // REGISTER_FUNCTIONS_H