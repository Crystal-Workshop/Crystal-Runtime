#pragma once
#include <glm/glm.hpp>
#include <GLFW/glfw3.h>

/**
 * Simple fly-camera used by the runtime.
 * - Camera instance is declared in camera.cpp and is accessible via the extern variable `camera`.
 * - Cursor and mouse callbacks are provided to integrate with GLFW.
 *
 * Use processKeys(window, dt) every frame to move the camera.
 */
struct Camera {
    glm::vec3 pos = {0,2,6};
    float pitch = 0.0f;
    float yaw = -3.14159f/2.0f;
    float fovy = glm::radians(60.0f);

    /** Compute a view matrix from the camera parameters. */
    glm::mat4 view() const;
};

/** Global camera instance used by main runtime. */
extern Camera camera;

/** GLFW cursor position callback (handles mouse look while right-button is down). */
void cursorCallback(GLFWwindow* wnd, double x, double y);

/** GLFW mouse button callback (captures/releases mouse when right button is pressed). */
void mouseButton(GLFWwindow* wnd, int button, int action, int mods);

/** Process WASD/QE for translation; call each frame with elapsed dt. */
void processKeys(GLFWwindow* wnd, float dt);
