#include "mesh.h"
#include <vector>

void MeshGL::destroy() {
    if(ebo) { glDeleteBuffers(1, &ebo); ebo=0; }
    if(vbo) { glDeleteBuffers(1, &vbo); vbo=0; }
    if(vao) { glDeleteVertexArrays(1, &vao); vao=0; }
}

MeshGL uploadMesh(const std::vector<float> &verts, const std::vector<unsigned int> &indices) {
    MeshGL m;
    glGenVertexArrays(1, &m.vao);
    glGenBuffers(1, &m.vbo);
    glGenBuffers(1, &m.ebo);
    glBindVertexArray(m.vao);

    glBindBuffer(GL_ARRAY_BUFFER, m.vbo);
    glBufferData(GL_ARRAY_BUFFER, verts.size()*sizeof(float), verts.data(), GL_STATIC_DRAW);

    glBindBuffer(GL_ELEMENT_ARRAY_BUFFER, m.ebo);
    glBufferData(GL_ELEMENT_ARRAY_BUFFER, indices.size()*sizeof(unsigned int), indices.data(), GL_STATIC_DRAW);

    glEnableVertexAttribArray(0); // pos
    glVertexAttribPointer(0, 3, GL_FLOAT, GL_FALSE, 6 * sizeof(float), (void*)0);
    glEnableVertexAttribArray(1); // normal
    glVertexAttribPointer(1, 3, GL_FLOAT, GL_FALSE, 6 * sizeof(float), (void*)(3 * sizeof(float)));

    glBindVertexArray(0);
    m.indexCount = (GLsizei)indices.size();
    return m;
}
