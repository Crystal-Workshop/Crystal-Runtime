#pragma once
#include <string>
#include <unordered_set>
#include <mutex>
#include <utility>      // <<< This line is critical for std::pair
#include <glm/glm.hpp>

// Thread-safe input state used by main thread (GLFW callbacks) and read by Lua threads.
class InputState {
public:
    InputState();
    ~InputState();

    // Called by GLFW callbacks (main thread)
    void setKeyDown(int glfwKey);
    void setKeyUp(int glfwKey);
    void setMouseButtonDown(int button);
    void setMouseButtonUp(int button);
    void setMousePosition(double x, double y);

    // Query by name from any thread (Lua script threads)
    bool isKeyDownByName(const std::string& name) const;
    glm::vec2 getMousePosition() const;

private:
    mutable std::mutex m_mutex;
    std::unordered_set<int> m_keysDown;
    std::unordered_set<int> m_mouseDown;
    glm::vec2 m_mousePosition;

    // The compiler error points here. It fails to recognize std::pair.
    std::pair<bool, int> parseName(const std::string& name) const;
};