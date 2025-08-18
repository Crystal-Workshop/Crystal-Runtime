#include "input.h"
#include <cctype>
#include <cstdlib>
#include <cstring>
#include <algorithm>
#include <unordered_map>
#include <GLFW/glfw3.h>

// Helper to map named keys
static int mapNamedKey(const std::string& s) {
    if(s == "Space") return GLFW_KEY_SPACE;
if(s == "Enter" || s == "Return") return GLFW_KEY_ENTER;
    if(s == "Tab") return GLFW_KEY_TAB;
    if(s == "Left") return GLFW_KEY_LEFT;
if(s == "Right") return GLFW_KEY_RIGHT;
    if(s == "Up") return GLFW_KEY_UP;
    if(s == "Down") return GLFW_KEY_DOWN;
if(s == "Escape" || s == "Esc") return GLFW_KEY_ESCAPE;
    if(s == "Backspace") return GLFW_KEY_BACKSPACE;
    if(s == "Home") return GLFW_KEY_HOME;
if(s == "End") return GLFW_KEY_END;
    if(s == "PageUp") return GLFW_KEY_PAGE_UP;
    if(s == "PageDown") return GLFW_KEY_PAGE_DOWN;
if(s == "LeftShift" || s == "LShift") return GLFW_KEY_LEFT_SHIFT;
    if(s == "RightShift" || s == "RShift") return GLFW_KEY_RIGHT_SHIFT;
if(s == "LeftCtrl" || s == "LControl") return GLFW_KEY_LEFT_CONTROL;
    if(s == "RightCtrl" || s == "RControl") return GLFW_KEY_RIGHT_CONTROL;
if(s == "LeftAlt" || s == "LAlt") return GLFW_KEY_LEFT_ALT;
    if(s == "RightAlt" || s == "RAlt") return GLFW_KEY_RIGHT_ALT;
    return -1;
}

InputState::InputState() : m_mousePosition(0.0f, 0.0f) {}
InputState::~InputState() {}

void InputState::setKeyDown(int glfwKey) {
    std::lock_guard<std::mutex> lk(m_mutex);
    m_keysDown.insert(glfwKey);
}

void InputState::setKeyUp(int glfwKey) {
    std::lock_guard<std::mutex> lk(m_mutex);
    m_keysDown.erase(glfwKey);
}

void InputState::setMouseButtonDown(int button) {
    std::lock_guard<std::mutex> lk(m_mutex);
    m_mouseDown.insert(button);
}

void InputState::setMouseButtonUp(int button) {
    std::lock_guard<std::mutex> lk(m_mutex);
    m_mouseDown.erase(button);
}

void InputState::setMousePosition(double x, double y) {
    std::lock_guard<std::mutex> lk(m_mutex);
    m_mousePosition.x = (float)x;
    m_mousePosition.y = (float)y;
}

glm::vec2 InputState::getMousePosition() const {
    std::lock_guard<std::mutex> lk(m_mutex);
    return m_mousePosition;
}

std::pair<bool,int> InputState::parseName(const std::string& name) const {
    if(name.empty()) return {false, -1};
// Mouse: "Mouse1", "Mouse2", case-insensitive for "Mouse"
    if(name.size() >= 5 && (name.find("Mouse") == 0 || name.find("mouse") == 0)) {
        std::string num = name.substr(5);
if(num.empty()) return {true, GLFW_MOUSE_BUTTON_1};
        int v = atoi(num.c_str());
        if(v <= 0) v = 1;
return {true, GLFW_MOUSE_BUTTON_1 + (v - 1)};
    }

    // Single char letter
    if(name.size() == 1) {
        char c = name[0];
if(std::isalpha((unsigned char)c)) {
            char upper = (char)std::toupper((unsigned char)c);
return {false, GLFW_KEY_A + (upper - 'A')};
        }
        if(std::isdigit((unsigned char)c)) {
            char d = c;
return {false, GLFW_KEY_0 + (d - '0')};
        }
    }

    // Named keys (case-sensitive expected; you can expand)
    int mapped = mapNamedKey(name);
if(mapped != -1) return {false, mapped};

    // Function keys: F1..F25
    if((name.size() >= 2) && (name[0] == 'F' || name[0] == 'f') && std::isdigit((unsigned char)name[1])) {
        int fn = atoi(name.substr(1).c_str());
if(fn >= 1 && fn <= 25) return {false, GLFW_KEY_F1 + (fn - 1)};
}

    return {false, -1};
}

bool InputState::isKeyDownByName(const std::string& name) const {
    auto parsed = parseName(name);
std::lock_guard<std::mutex> lk(m_mutex);
    if(parsed.second == -1) return false;
    if(parsed.first) {
        return m_mouseDown.find(parsed.second) != m_mouseDown.end();
} else {
        return m_keysDown.find(parsed.second) != m_keysDown.end();
}
}