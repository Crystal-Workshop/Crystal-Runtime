#pragma once
#include <glm/glm.hpp>

extern "C" {
    #include "lua.h"
}

// Base template for vector-like types
template<typename T, int Components>
struct VectorBase {
    T data[Components];
    
    VectorBase() {
        for(int i = 0; i < Components; ++i) {
            data[i] = T(0);
        }
    }
    
    // Constructor for 2+ component vectors
    template<int C = Components>
    VectorBase(T x, T y, typename std::enable_if<(C >= 2), int>::type = 0) {
        data[0] = x;
        data[1] = y;
        for(int i = 2; i < Components; ++i) {
            data[i] = T(0);
        }
    }
    
    // Constructor for 3+ component vectors
    template<int C = Components>
    VectorBase(T x, T y, T z, typename std::enable_if<(C >= 3), int>::type = 0) {
        data[0] = x;
        data[1] = y;
        data[2] = z;
        for(int i = 3; i < Components; ++i) {
            data[i] = T(0);
        }
    }
    
    T& operator[](int index) { return data[index]; }
    const T& operator[](int index) const { return data[index]; }
};

// Vector3 - for positions, rotations, scales
struct Vector3 : VectorBase<float, 3> {
    Vector3() : VectorBase() {}
    Vector3(float x, float y, float z) : VectorBase() {
        data[0] = x;
        data[1] = y;
        data[2] = z;
    }
    explicit Vector3(const glm::vec3& v) : VectorBase() {
        data[0] = v.x;
        data[1] = v.y;
        data[2] = v.z;
    }
    
    float x() const { return data[0]; }
    float y() const { return data[1]; }
    float z() const { return data[2]; }
    
    glm::vec3 toGLM() const { return glm::vec3(data[0], data[1], data[2]); }
    
    // Math operations
    Vector3 operator+(const Vector3& other) const {
        return Vector3(data[0] + other.data[0], data[1] + other.data[1], data[2] + other.data[2]);
    }
    
    Vector3 operator-(const Vector3& other) const {
        return Vector3(data[0] - other.data[0], data[1] - other.data[1], data[2] - other.data[2]);
    }
    
    Vector3 operator*(const Vector3& other) const {
        return Vector3(data[0] * other.data[0], data[1] * other.data[1], data[2] * other.data[2]);
    }
    
    Vector3 operator/(const Vector3& other) const {
        return Vector3(data[0] / other.data[0], data[1] / other.data[1], data[2] / other.data[2]);
    }
    
    Vector3 operator*(float scalar) const {
        return Vector3(data[0] * scalar, data[1] * scalar, data[2] * scalar);
    }
    
    Vector3 operator/(float scalar) const {
        return Vector3(data[0] / scalar, data[1] / scalar, data[2] / scalar);
    }
};

// Vector2 - for viewport and mouse positions
struct Vector2 : VectorBase<float, 2> {
    Vector2() : VectorBase() {}
    Vector2(float x, float y) : VectorBase() {
        data[0] = x;
        data[1] = y;
    }
    explicit Vector2(const glm::vec2& v) : VectorBase() {
        data[0] = v.x;
        data[1] = v.y;
    }
    
    float x() const { return data[0]; }
    float y() const { return data[1]; }
    
    glm::vec2 toGLM() const { return glm::vec2(data[0], data[1]); }
    
    // Math operations
    Vector2 operator+(const Vector2& other) const {
        return Vector2(data[0] + other.data[0], data[1] + other.data[1]);
    }
    
    Vector2 operator-(const Vector2& other) const {
        return Vector2(data[0] - other.data[0], data[1] - other.data[1]);
    }
    
    Vector2 operator*(const Vector2& other) const {
        return Vector2(data[0] * other.data[0], data[1] * other.data[1]);
    }
    
    Vector2 operator/(const Vector2& other) const {
        return Vector2(data[0] / other.data[0], data[1] / other.data[1]);
    }
    
    Vector2 operator*(float scalar) const {
        return Vector2(data[0] * scalar, data[1] * scalar);
    }
    
    Vector2 operator/(float scalar) const {
        return Vector2(data[0] / scalar, data[1] / scalar);
    }
};

// Color3 - for RGB colors (0-255 range)
struct Color3 : VectorBase<float, 3> {
    Color3() : VectorBase() {}
    Color3(float r, float g, float b) : VectorBase() {
        data[0] = r;
        data[1] = g;
        data[2] = b;
    }
    explicit Color3(const glm::vec3& v) : VectorBase() {
        data[0] = v.x * 255.0f;
        data[1] = v.y * 255.0f;
        data[2] = v.z * 255.0f;
    }
    
    float r() const { return data[0]; }
    float g() const { return data[1]; }
    float b() const { return data[2]; }
    
    glm::vec3 toGLM() const { return glm::vec3(data[0] / 255.0f, data[1] / 255.0f, data[2] / 255.0f); }
    
    // Math operations
    Color3 operator+(const Color3& other) const {
        return Color3(
            glm::clamp(data[0] + other.data[0], 0.0f, 255.0f),
            glm::clamp(data[1] + other.data[1], 0.0f, 255.0f),
            glm::clamp(data[2] + other.data[2], 0.0f, 255.0f)
        );
    }
    
    Color3 operator-(const Color3& other) const {
        return Color3(
            glm::clamp(data[0] - other.data[0], 0.0f, 255.0f),
            glm::clamp(data[1] - other.data[1], 0.0f, 255.0f),
            glm::clamp(data[2] - other.data[2], 0.0f, 255.0f)
        );
    }
    
    Color3 operator*(const Color3& other) const {
        return Color3(
            glm::clamp((data[0] / 255.0f) * (other.data[0] / 255.0f) * 255.0f, 0.0f, 255.0f),
            glm::clamp((data[1] / 255.0f) * (other.data[1] / 255.0f) * 255.0f, 0.0f, 255.0f),
            glm::clamp((data[2] / 255.0f) * (other.data[2] / 255.0f) * 255.0f, 0.0f, 255.0f)
        );
    }
    
    Color3 operator/(const Color3& other) const {
        return Color3(
            other.data[0] != 0 ? glm::clamp((data[0] / 255.0f) / (other.data[0] / 255.0f) * 255.0f, 0.0f, 255.0f) : 0.0f,
            other.data[1] != 0 ? glm::clamp((data[1] / 255.0f) / (other.data[1] / 255.0f) * 255.0f, 0.0f, 255.0f) : 0.0f,
            other.data[2] != 0 ? glm::clamp((data[2] / 255.0f) / (other.data[2] / 255.0f) * 255.0f, 0.0f, 255.0f) : 0.0f
        );
    }
    
    Color3 operator*(float scalar) const {
        return Color3(
            glm::clamp(data[0] * scalar, 0.0f, 255.0f),
            glm::clamp(data[1] * scalar, 0.0f, 255.0f),
            glm::clamp(data[2] * scalar, 0.0f, 255.0f)
        );
    }
    
    Color3 operator/(float scalar) const {
        if (scalar == 0.0f) return Color3(0, 0, 0);
        return Color3(
            glm::clamp(data[0] / scalar, 0.0f, 255.0f),
            glm::clamp(data[1] / scalar, 0.0f, 255.0f),
            glm::clamp(data[2] / scalar, 0.0f, 255.0f)
        );
    }
};

// Lua binding functions
void registerVectorTypes(lua_State* L);
void pushVector3(lua_State* L, const Vector3& vec);
void pushVector2(lua_State* L, const Vector2& vec);
void pushColor3(lua_State* L, const Color3& color);
Vector3* checkVector3(lua_State* L, int idx);
Vector2* checkVector2(lua_State* L, int idx);
Color3* checkColor3(lua_State* L, int idx);