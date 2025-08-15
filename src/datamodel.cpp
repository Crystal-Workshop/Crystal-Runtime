#include "datamodel.h"
#include <algorithm>

DataModel* m_dataModel = nullptr;

void DataModel::initializeFrom(const std::vector<SceneObject>& objs) {
    std::lock_guard<std::mutex> lk(m_mutex);
    m_objects = objs; // copy
}

std::vector<SceneObject> DataModel::getAllObjects() {
    std::lock_guard<std::mutex> lk(m_mutex);
    return m_objects; // copy
}

std::optional<SceneObject> DataModel::getObjectCopy(const std::string& name) {
    std::lock_guard<std::mutex> lk(m_mutex);
    auto it = std::find_if(m_objects.begin(), m_objects.end(),
        [&](const SceneObject &o){ return o.name == name; });
    if(it == m_objects.end()) return std::nullopt;
    return *it;
}

void DataModel::setColor(const std::string& name, const glm::vec3& color) {
    std::lock_guard<std::mutex> lk(m_mutex);
    for(auto &o : m_objects) {
        if(o.name == name) { o.color = color; break; }
    }
}

void DataModel::setPosition(const std::string& name, const glm::vec3& pos) {
    std::lock_guard<std::mutex> lk(m_mutex);
    for(auto &o : m_objects) {
        if(o.name == name) { o.position = pos; break; }
    }
}

void DataModel::setScale(const std::string& name, const glm::vec3& scale) {
    std::lock_guard<std::mutex> lk(m_mutex);
    for(auto &o : m_objects) {
        if(o.name == name) { o.scale = scale; break; }
    }
}

void DataModel::setRotation(const std::string& name, const glm::vec3& rot) {
    std::lock_guard<std::mutex> lk(m_mutex);
    for(auto &o : m_objects) {
        if(o.name == name) { o.rotation = rot; break; }
    }
}

void DataModel::setFov(const std::string& name, float fov) {
    std::lock_guard<std::mutex> lk(m_mutex);
    for(auto &o : m_objects) {
        if(o.name == name) { o.fov = fov; break; }
    }
}

float DataModel::getFov(const std::string& name) const {
    std::lock_guard<std::mutex> lk(m_mutex);
    auto it = std::find_if(m_objects.begin(), m_objects.end(),
        [&](const SceneObject &o){ return o.name == name; });
    if(it == m_objects.end()) return 45.0f;
    return it->fov;
}

void DataModel::setIntensity(const std::string& name, float intensity) {
    std::lock_guard<std::mutex> lk(m_mutex);
    for(auto &o : m_objects) {
        if(o.name == name) { o.intensity = intensity; break; }
    }
}

float DataModel::getIntensity(const std::string& name) const {
    std::lock_guard<std::mutex> lk(m_mutex);
    auto it = std::find_if(m_objects.begin(), m_objects.end(),
        [&](const SceneObject &o){ return o.name == name; });
    if(it == m_objects.end()) return 1.0f;
    return it->intensity;
}