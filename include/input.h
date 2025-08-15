#pragma once
#include <string>
#include <unordered_set>
#include <mutex>
#include <utility>

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

    // Query by name from any thread (Lua script threads)
    // Examples: "A", "Space", "Enter", "Left", "Right", "Up", "Down", "F1", "0".."9", "Mouse1"
    bool isKeyDownByName(const std::string& name) const;

private:
    mutable std::mutex m_mutex;
    std::unordered_set<int> m_keysDown;   // GLFW key codes
    std::unordered_set<int> m_mouseDown;  // GLFW mouse buttons

    // parse name; returns pair<isMouse, code>, code == -1 on failure
    std::pair<bool,int> parseName(const std::string& name) const;
};