#ifndef SCENE_H
#define SCENE_H

#include <string>
#include <vector>
#include "xmlutil.h"
#include <glm/glm.hpp>

struct SceneObject {
    std::string name;
    std::string type;
    std::string mesh;
    glm::vec3 color = glm::vec3(1,1,1);
    glm::vec3 position = glm::vec3(0,0,0);
    glm::vec3 rotation = glm::vec3(0,0,0);
    glm::vec3 scale = glm::vec3(1,1,1);
    float fov = 45.0f;
    float intensity = 1.0f; // For light type objects
};

struct Light {
    glm::vec3 position = glm::vec3(0,0,0);
    glm::vec3 color = glm::vec3(1,1,1);
    float intensity = 1.0f;
};

void parseSceneXml(const std::string &xml, std::vector<SceneObject> &outObjects, std::vector<Light> &outLights);

#endif