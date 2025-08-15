// camera.cpp
// Minimal Camera struct without input controls (removed cursorCallback, mouseButton, processKeys)
// Camera properties are now managed via DataModel "camera" object

#include "camera.h"
#define GLM_ENABLE_EXPERIMENTAL
#include <glm/gtc/matrix_transform.hpp>
#include <glm/gtx/euler_angles.hpp>
#include <algorithm>

Camera camera; // Retained as fallback, but not updated by inputs

glm::mat4 Camera::view() const {
    glm::vec3 dir = glm::normalize(glm::vec3(cos(pitch)*cos(yaw), sin(pitch), cos(pitch)*sin(yaw)));
    return glm::lookAt(pos, pos + dir, glm::vec3(0,1,0));
}