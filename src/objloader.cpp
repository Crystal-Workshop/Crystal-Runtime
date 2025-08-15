#include "objloader.h"
#include <glm/glm.hpp>
#include <sstream>
#include <string>
#include <vector>
#include <unordered_map>
#include <cmath>

struct FaceIdx { int v, vt, vn; };

bool loadObjFromString(const std::string &data, std::vector<float> &vertices, std::vector<unsigned int> &indices) {
    vertices.clear();
    indices.clear();
    std::vector<glm::vec3> positions;
    std::vector<glm::vec3> normals;
    std::vector<glm::vec2> texcoords;

    std::vector<std::vector<FaceIdx>> faces; // triangulated face lists

    std::istringstream in(data);
    std::string line;
    while(std::getline(in, line)) {
        if(line.size() == 0) continue;
        // trim leading spaces
        size_t p = line.find_first_not_of(" \t\r\n");
        if(p==std::string::npos) continue;
        if(line[p] == '#') continue;
        std::istringstream ls(line.substr(p));
        std::string tok;
        ls >> tok;
        if(tok == "v") {
            float x,y,z; ls >> x >> y >> z; positions.push_back({x,y,z});
        } else if(tok == "vn") {
            float x,y,z; ls >> x >> y >> z; normals.push_back({x,y,z});
        } else if(tok == "vt") {
            float u,v; ls >> u >> v; texcoords.push_back({u,v});
        } else if(tok == "f") {
            std::vector<FaceIdx> f;
            std::string part;
            while(ls >> part) {
                FaceIdx idx{0,0,0};
                int vi=0, vti=0, vni=0;
                size_t p1 = part.find('/');
                if(p1 == std::string::npos) {
                    vi = std::stoi(part);
                } else {
                    vi = std::stoi(part.substr(0,p1));
                    size_t p2 = part.find('/', p1+1);
                    if(p2 == std::string::npos) {
                        vti = std::stoi(part.substr(p1+1));
                    } else {
                        if(p2 == p1+1) {
                            // v//vn
                            vni = std::stoi(part.substr(p2+1));
                        } else {
                            vti = std::stoi(part.substr(p1+1, p2 - (p1+1)));
                            vni = std::stoi(part.substr(p2+1));
                        }
                    }
                }
                auto fixIndex = [](int idx, int size)->int {
                    if(idx > 0) return idx - 1;
                    if(idx < 0) return size + idx;
                    return -1;
                };
                idx.v = fixIndex(vi, (int)positions.size());
                idx.vt = vti==0 ? -1 : fixIndex(vti, (int)texcoords.size());
                idx.vn = vni==0 ? -1 : fixIndex(vni, (int)normals.size());
                f.push_back(idx);
            }
            if(f.size() < 3) continue;
            for(size_t i=1;i+1<f.size();++i) {
                faces.push_back({f[0], f[i], f[i+1]});
            }
        }
    }

    struct Key { int v, vt, vn; bool operator==(Key const& o) const { return v==o.v && vt==o.vt && vn==o.vn; } };
    struct KeyHash { size_t operator()(Key const& k) const noexcept {
        return ((size_t)k.v * 73856093) ^ ((size_t)(k.vt+1) * 19349663) ^ ((size_t)(k.vn+2) * 83492791);
    }};

    std::unordered_map<Key, unsigned int, KeyHash> mapIdx;
    std::vector<float> verts; std::vector<unsigned int> idxs;

    for(auto &face : faces) {
        for(int k=0;k<3;++k) {
            Key key{face[k].v, face[k].vt, face[k].vn};
            auto it = mapIdx.find(key);
            if(it != mapIdx.end()) {
                idxs.push_back(it->second);
            } else {
                unsigned int newIdx = (unsigned int)(verts.size() / 6);
                mapIdx[key] = newIdx;
                // position
                glm::vec3 p = (face[k].v >=0 ? positions[face[k].v] : glm::vec3(0.0f));
                verts.push_back(p.x); verts.push_back(p.y); verts.push_back(p.z);
                // normal: prefer vn, otherwise 0 for now
                glm::vec3 n = (face[k].vn>=0 ? normals[face[k].vn] : glm::vec3(0.0f));
                verts.push_back(n.x); verts.push_back(n.y); verts.push_back(n.z);
                idxs.push_back(newIdx);
            }
        }
    }

    // compute normals if missing
    bool needNormals = false;
    for(size_t i=0;i<verts.size(); i+=6) {
        if(std::fabs(verts[i+3]) < 1e-6f && std::fabs(verts[i+4]) < 1e-6f && std::fabs(verts[i+5]) < 1e-6f) { needNormals = true; break; }
    }
    if(needNormals) {
        std::vector<glm::vec3> accum(verts.size()/6, glm::vec3(0.0f));
        for(size_t i=0;i<idxs.size(); i+=3) {
            unsigned int i0 = idxs[i+0], i1 = idxs[i+1], i2 = idxs[i+2];
            glm::vec3 p0(verts[i0*6+0], verts[i0*6+1], verts[i0*6+2]);
            glm::vec3 p1(verts[i1*6+0], verts[i1*6+1], verts[i1*6+2]);
            glm::vec3 p2(verts[i2*6+0], verts[i2*6+1], verts[i2*6+2]);
            glm::vec3 n = glm::cross(p1 - p0, p2 - p0);
            if(glm::length(n) > 1e-6f) n = glm::normalize(n);
            accum[i0] += n; accum[i1] += n; accum[i2] += n;
        }
        for(size_t vi=0; vi<accum.size(); ++vi) {
            glm::vec3 n = glm::normalize(accum[vi]);
            verts[vi*6 + 3] = n.x; verts[vi*6 + 4] = n.y; verts[vi*6 + 5] = n.z;
        }
    }

    vertices.swap(verts);
    indices.swap(idxs);
    return true;
}
