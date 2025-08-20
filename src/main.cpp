#include <iostream>
#include <vector>
#include <string>
#include <unordered_map>
#include <algorithm>
#include <chrono>

#include <glad/glad.h>
#include <GLFW/glfw3.h>

#define GLM_ENABLE_EXPERIMENTAL
#define GLM_FORCE_RADIANS
#define GLM_FORCE_DEPTH_ZERO_TO_ONE
#include <glm/glm.hpp>
#include <glm/gtc/matrix_transform.hpp>
#include <glm/gtc/constants.hpp>
#include <glm/gtx/euler_angles.hpp>

#include "archive.h"
#include "xmlutil.h"
#include "objloader.h"
#include "mesh.h"
#include "shader.h"
#include "scene.h"
#include "camera.h"
#include "datamodel.h"
#include "lua_thread.h"
#include "input.h"
#include "export.h"

extern "C" {
    #include "lua.h"
    #include "lauxlib.h"
    #include "lualib.h"
}

// Forward declarations (from shader.cpp)
extern const char* getDefaultVS();
extern const char* getDefaultFS();

void registerPlaceGlobal(lua_State* L, DataModel* dataModel);
void registerServiceGlobal(lua_State* L, InputState* inputState);
// Track whether we own the GLFW lifecycle
static bool g_glfwInitializedHere = false;
/// Initialize library (must be called before RunScene if you want manual lifecycle)
extern "C" CRYSTAL_API bool InitializeCrystalRuntime() {
    if (!glfwInit()) {
        std::cerr << "glfwInit failed\n";
return false;
    }
    g_glfwInitializedHere = true;
    return true;
}

/// Shutdown library
extern "C" CRYSTAL_API void ShutdownCrystalRuntime() {
    if (g_glfwInitializedHere) {
        glfwTerminate();
g_glfwInitializedHere = false;
    }
}

/// Run a scene from a .cgame archive (this is your old main())
extern "C" CRYSTAL_API int RunScene(const char* archivePath) {
    if (!archivePath) {
        std::cerr << "Usage: RunScene(<scene.cgame>)\n";
return 1;
    }

    // Load archive and scene xml
    std::vector<ArchiveFileEntry> files;
    std::string sceneXml;
if (!loadCGame(archivePath, files, sceneXml)) {
        std::cerr << "Failed to load .cgame: " << archivePath << "\n";
return 1;
    }
    std::cout << "Loaded archive, scene size: " << sceneXml.size()
              << ", files: " << files.size() << "\n";
// Parse scene
    std::vector<SceneObject> parsedObjects;
    std::vector<Light> lights;
    parseSceneXml(sceneXml, parsedObjects, lights);
    std::cout << "--- Parsed Object Names ---\n";
for (const SceneObject& obj : parsedObjects) {
        std::cout << "Found object with name: '" << obj.name << "' type: '" << obj.type << "'\n";
}
    std::cout << "---------------------------\n";

    // Initialize DataModel from parsed objects
    DataModel dm;
    dm.initializeFrom(parsedObjects);
// Create input state
    InputState inputState;

    // Init GLFW if not already initialized
    if (!glfwInit()) {
        std::cerr << "glfwInit fail\n";
return 1;
    }

    glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, 3);
    glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, 3);
    glfwWindowHint(GLFW_OPENGL_PROFILE, GLFW_OPENGL_CORE_PROFILE);
GLFWwindow* wnd = glfwCreateWindow(1280, 720, "CGame Runtime", nullptr, nullptr);
    if (!wnd) {
        std::cerr << "create window fail\n";
if (g_glfwInitializedHere) {
            glfwTerminate();
            g_glfwInitializedHere = false;
}
        return 1;
    }
    glfwMakeContextCurrent(wnd);
if (!gladLoadGLLoader((GLADloadproc)glfwGetProcAddress)) {
        std::cerr << "glad fail\n";
        glfwDestroyWindow(wnd);
if (g_glfwInitializedHere) {
            glfwTerminate();
            g_glfwInitializedHere = false;
}
        return 1;
}

    // Attach InputState to window
    glfwSetWindowUserPointer(wnd, &inputState);
// Input callbacks
    glfwSetKeyCallback(wnd, [](GLFWwindow* w, int key, int, int action, int) {
        auto* s = static_cast<InputState*>(glfwGetWindowUserPointer(w));
        if (!s) return;
        if (action == GLFW_PRESS || action == GLFW_REPEAT) s->setKeyDown(key);
        else if (action == GLFW_RELEASE) s->setKeyUp(key);
    });
glfwSetMouseButtonCallback(wnd, [](GLFWwindow* w, int button, int action, int) {
        auto* s = static_cast<InputState*>(glfwGetWindowUserPointer(w));
        if (!s) return;
        if (action == GLFW_PRESS) s->setMouseButtonDown(button);
        else if (action == GLFW_RELEASE) s->setMouseButtonUp(button);
    });
    glfwSetCursorPosCallback(wnd, [](GLFWwindow* w, double x, double y) {
        auto* s = static_cast<InputState*>(glfwGetWindowUserPointer(w));
        if (s) {
            s->setMousePosition(x, y);
        }
    });
glEnable(GL_DEPTH_TEST);
    GLuint program = makeProgram(getDefaultVS(), getDefaultFS());
    if (!program) {
        std::cerr << "shader fail\n";
glfwDestroyWindow(wnd);
        if (g_glfwInitializedHere) {
            glfwTerminate();
            g_glfwInitializedHere = false;
}
        return 1;
}

    // Light setup - now dynamically find first light object
    glm::vec3 lightPos(0, 10, 0), lightColor(1, 1, 1);
    
    // Look for the first light object in the scene
    auto allObjects = dm.getAllObjects();
    for (const SceneObject& obj : allObjects) {
        if (obj.type == "light") {
            lightPos = obj.position;
            lightColor = obj.color * obj.intensity;
            std::cout << "Using light object '" << obj.name << "' at position (" 
                      << lightPos.x << ", " << lightPos.y << ", " << lightPos.z << ")\n";
            break;
        }
    }

    // Load meshes from archive
    std::unordered_map<std::string, MeshGL> meshCache;
for (const SceneObject& obj : dm.getAllObjects()) {
        if (obj.mesh.empty()) continue;
        if (meshCache.count(obj.mesh)) continue;
auto it = std::find_if(files.begin(), files.end(),
                               [&](const ArchiveFileEntry &e){ return e.name == obj.mesh; });
if (it == files.end()) {
            std::cerr << "Warning: mesh not found in archive: " << obj.mesh << "\n";
continue;
        }

        std::string objData;
if (!extractFileFromArchive(archivePath, *it, objData)) {
            std::cerr << "Failed extract: " << obj.mesh << "\n";
continue;
        }

        std::vector<float> verts;
        std::vector<unsigned int> idxs;
if (!loadObjFromString(objData, verts, idxs)) {
            std::cerr << "Failed parse OBJ: " << obj.mesh << "\n";
continue;
        }

        meshCache[obj.mesh] = uploadMesh(verts, idxs);
std::cout << "Loaded mesh: " << obj.mesh
                  << " verts=" << (verts.size()/6)
                  << " tris=" << (idxs.size()/3) << "\n";
}

    // Lua scripts
    LuaScriptManager luaMgr(archivePath, files, &dm, &inputState, wnd);
    luaMgr.startAllScripts();
// Main loop
    int w = 1280, h = 720;
    glfwGetFramebufferSize(wnd, &w, &h);
    float lastTime = (float)glfwGetTime();
static Camera fallbackCamera;

    while (!glfwWindowShouldClose(wnd)) {
        float now = (float)glfwGetTime();
float dt = now - lastTime;
        lastTime = now;

        glfwGetFramebufferSize(wnd, &w, &h);
        glViewport(0, 0, w, h);
        glClearColor(0.1f, 0.12f, 0.15f, 1.0f);
glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

        glUseProgram(program);

        // Camera
        auto camObj = dm.getObjectCopy("camera");
        glm::vec3 camPos, camRot;
float camFov;

        if (camObj.has_value()) {
            camPos = camObj->position;
camRot = camObj->rotation;
            camFov = camObj->fov;
        } else {
            camPos = fallbackCamera.pos;
camRot = glm::vec3(fallbackCamera.pitch * (180.0f / glm::pi<float>()),
                               fallbackCamera.yaw * (180.0f / glm::pi<float>()), 0.0f);
camFov = fallbackCamera.fovy * (180.0f / glm::pi<float>());
        }

        glm::mat4 rotMat = glm::eulerAngleXYZ(glm::radians(camRot.x),
                                              glm::radians(camRot.y),
                                    
          glm::radians(camRot.z));
        glm::vec3 forward = rotMat * glm::vec4(0, 0, -1, 0);
glm::vec3 up = rotMat * glm::vec4(0, 1, 0, 0);
        glm::mat4 view = glm::lookAt(camPos, camPos + forward, up);
glm::mat4 proj = glm::perspective(glm::radians(camFov), (float)w/(float)h, 0.1f, 100.0f);

        // Update light position dynamically from scene objects
        auto currentObjects = dm.getAllObjects();
        for (const SceneObject& obj : currentObjects) {
            if (obj.type == "light") {
                lightPos = obj.position;
                lightColor = obj.color * obj.intensity;
                break; // Use first light found
            }
        }

        GLint locModel = glGetUniformLocation(program, "uModel");
        GLint locView  = glGetUniformLocation(program, "uView");
GLint locProj  = glGetUniformLocation(program, "uProj");
        GLint locColor = glGetUniformLocation(program, "uColor");
        GLint locCamPos = glGetUniformLocation(program, "uCamPos");
GLint locLightPos = glGetUniformLocation(program, "uLightPos");
        GLint locLightCol = glGetUniformLocation(program, "uLightColor");

        glUniformMatrix4fv(locView, 1, GL_FALSE, &view[0][0]);
        glUniformMatrix4fv(locProj, 1, GL_FALSE, &proj[0][0]);
glUniform3fv(locCamPos, 1, &camPos[0]);
        glUniform3fv(locLightPos, 1, &lightPos[0]);
        glUniform3fv(locLightCol, 1, &lightColor[0]);

        // Draw objects (including lights if they have meshes)
        for (const SceneObject& obj : dm.getAllObjects()) {
            glm::mat4 M(1.0f);
M = glm::translate(M, obj.position);
            glm::vec3 r = glm::radians(obj.rotation);
            M = M * glm::eulerAngleXYZ(r.x, r.y, r.z);
            M = glm::scale(M, obj.scale);
glUniformMatrix4fv(locModel, 1, GL_FALSE, &M[0][0]);
            glUniform3fv(locColor, 1, &obj.color[0]);

            if (obj.mesh.empty()) continue;
            auto it = meshCache.find(obj.mesh);
            if (it == meshCache.end()) continue;
MeshGL &mg = it->second;
            glBindVertexArray(mg.vao);
            glDrawElements(GL_TRIANGLES, mg.indexCount, GL_UNSIGNED_INT, 0);
            glBindVertexArray(0);
        }

        glfwSwapBuffers(wnd);
        glfwPollEvents();
if (glfwGetKey(wnd, GLFW_KEY_ESCAPE) == GLFW_PRESS) {
            glfwSetWindowShouldClose(wnd, GLFW_TRUE);
}
    }

    // Cleanup
    luaMgr.stopAllScripts();
    for (auto &kv : meshCache) kv.second.destroy();
glDeleteProgram(program);
    glfwDestroyWindow(wnd);

    if (g_glfwInitializedHere) {
        glfwTerminate();
        g_glfwInitializedHere = false;
}

    return 0;
}