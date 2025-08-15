#pragma once
#include <vector>
#include <glad/glad.h>

/**
 * Simple OpenGL mesh container:
 * - a VAO, VBO and EBO
 * - indexCount for glDrawElements
 *
 * The destroy() helper releases GL resources.
 */
struct MeshGL {
    GLuint vao=0, vbo=0, ebo=0;
    GLsizei indexCount=0;

    /** Release GL resources (safe to call even if already released). */
    void destroy();
};

/**
 * Create and upload a mesh to GPU from CPU-side vertex/index arrays.
 *
 * - verts: interleaved float array (position vec3, normal vec3)
 * - indices: triangle indices (unsigned int)
 *
 * Returns a MeshGL object containing VAO/VBO/EBO and indexCount.
 */
MeshGL uploadMesh(const std::vector<float> &verts, const std::vector<unsigned int> &indices);
