#include "shader.h"
#include <iostream>

GLuint compileShader(GLenum type, const char* src) {
    GLuint s = glCreateShader(type);
    glShaderSource(s, 1, &src, nullptr);
    glCompileShader(s);
    GLint ok; glGetShaderiv(s, GL_COMPILE_STATUS, &ok);
    if(!ok) {
        char buf[1024]; glGetShaderInfoLog(s, 1024, nullptr, buf);
        std::cerr << "Shader compile error: " << buf << "\n";
        glDeleteShader(s); return 0;
    }
    return s;
}
GLuint makeProgram(const char* vsSrc, const char* fsSrc) {
    GLuint vs = compileShader(GL_VERTEX_SHADER, vsSrc);
    GLuint fs = compileShader(GL_FRAGMENT_SHADER, fsSrc);
    if(!vs || !fs) return 0;
    GLuint p = glCreateProgram();
    glAttachShader(p, vs);
    glAttachShader(p, fs);
    glLinkProgram(p);
    GLint ok; glGetProgramiv(p, GL_LINK_STATUS, &ok);
    if(!ok) {
        char buf[1024]; glGetProgramInfoLog(p, 1024, nullptr, buf);
        std::cerr << "Program link error: " << buf << "\n";
    }
    glDeleteShader(vs);
    glDeleteShader(fs);
    return p;
}

// shader sources (kept here so shader.cpp compiles standalone)
const char* vsSrc = R"glsl(
#version 330 core
layout(location=0) in vec3 aPos;
layout(location=1) in vec3 aNormal;
uniform mat4 uModel;
uniform mat4 uView;
uniform mat4 uProj;
out vec3 vNormal;
out vec3 vWorldPos;
void main() {
    vec4 w = uModel * vec4(aPos,1.0);
    vWorldPos = w.xyz;
    vNormal = mat3(transpose(inverse(uModel))) * aNormal;
    gl_Position = uProj * uView * w;
}
)glsl";

const char* fsSrc = R"glsl(
#version 330 core
in vec3 vNormal;
in vec3 vWorldPos;
out vec4 FragColor;
uniform vec3 uColor;
uniform vec3 uCamPos;
uniform vec3 uLightPos;
uniform vec3 uLightColor;
void main() {
    vec3 N = normalize(vNormal);
    vec3 L = normalize(uLightPos - vWorldPos);
    float diff = max(dot(N,L), 0.0);
    vec3 viewDir = normalize(uCamPos - vWorldPos);
    vec3 h = normalize(L + viewDir);
    float spec = pow(max(dot(N,h),0.0), 32.0);
    vec3 col = uColor * (0.2 + 0.8*diff) + uLightColor * spec * 0.3;
    FragColor = vec4(col,1.0);
}
)glsl";

// Small helper to expose the builtin sources if desired by other modules
const char* getDefaultVS() { return vsSrc; }
const char* getDefaultFS() { return fsSrc; }
