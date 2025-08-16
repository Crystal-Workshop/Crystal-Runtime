#pragma once

#if defined(_WIN32) || defined(_WIN64)
  #if defined(CRYSTAL_BUILD_SHARED)
    #define CRYSTAL_API extern "C" __declspec(dllexport)
  #else
    #define CRYSTAL_API extern "C" __declspec(dllimport)
  #endif
#else
  #if __GNUC__ >= 4
    #define CRYSTAL_API extern "C" __attribute__((visibility("default")))
  #else
    #define CRYSTAL_API extern "C"
  #endif
#endif