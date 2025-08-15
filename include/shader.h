#pragma once
#include <glad/glad.h>

/**
 * Compile a single shader of type (GL_VERTEX_SHADER / GL_FRAGMENT_SHADER).
 * Returns 0 on failure.
 */
GLuint compileShader(GLenum type, const char* src);

/**
 * Build a program from a vertex and fragment shader source.
 * Returns program GLuint (0 on failure).
 */
GLuint makeProgram(const char* vsSrc, const char* fsSrc);
