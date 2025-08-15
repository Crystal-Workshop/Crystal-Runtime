#ifndef DATAMODEL_H
#define DATAMODEL_H

#include <mutex>
#include <vector>
#include <optional>
#include "scene.h"
#include <string>

class DataModel {
private:
    mutable std::mutex m_mutex; // Added mutable to allow locking in const methods
    std::vector<SceneObject> m_objects;
public:
    void initializeFrom(const std::vector<SceneObject>& objs);
    std::vector<SceneObject> getAllObjects();
    std::optional<SceneObject> getObjectCopy(const std::string& name);
    void setColor(const std::string& name, const glm::vec3& color);
    void setPosition(const std::string& name, const glm::vec3& pos);
    void setScale(const std::string& name, const glm::vec3& scale);
    void setRotation(const std::string& name, const glm::vec3& rot);
    void setFov(const std::string& name, float fov);
    float getFov(const std::string& name) const;
    void setIntensity(const std::string& name, float intensity);
    float getIntensity(const std::string& name) const;
};
extern DataModel* m_dataModel;

#endif