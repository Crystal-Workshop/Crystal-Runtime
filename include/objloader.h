#pragma once
#include <vector>
#include <string>

/**
 * Parse an in-memory OBJ file (string) and produce an interleaved vertex buffer and index buffer.
 *
 * The produced vertex layout is interleaved: position (3 floats), normal (3 floats) per vertex.
 * If the OBJ lacks normals, this loader computes per-vertex normals by averaging triangle normals.
 *
 * - data: raw OBJ text
 * - vertices: out vector of floats (pos.xyz, normal.xyz) repeated
 * - indices: out vector of unsigned int indices forming triangles
 *
 * Returns true on success.
 */
bool loadObjFromString(const std::string &data, std::vector<float> &vertices, std::vector<unsigned int> &indices);
