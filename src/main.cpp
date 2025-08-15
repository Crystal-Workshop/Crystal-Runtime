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
extern "C" {
#include "lua.h"
#include "lauxlib.h"
#include "lualib.h"
#include <windows.h>
#include <cstdlib>
}
// Forward declarations to get shader sources defined in shader.cpp
extern const char* getDefaultVS();
extern const char* getDefaultFS();
void registerPlaceGlobal(lua_State* L, DataModel* dataModel);
void registerServiceGlobal(lua_State* L, InputState* inputState);
int APIENTRY WinMain(HINSTANCE /*hInstance*/, // Fixed: Remove unused parameter names
                     HINSTANCE /*hPrevInstance*/,
                     LPSTR     /*lpCmdLine*/,
                     int       /*nCmdShow*/)
{
    int argc = __argc;
    char** argv = __argv;
    if(argc < 2) {
        std::cout << "Usage: Crystal - <scene.cgame>\n";
        return 1;
    }
    std::string archivePath = argv[1];
    // load archive and scene xml
    std::vector<ArchiveFileEntry> files;
    std::string sceneXml;
    if(!loadCGame(archivePath, files, sceneXml)) {
        std::cerr << "Failed to load .cgame: " << archivePath << "\n"; return 1;
    }
    std::cout << "Loaded archive, scene size: " << sceneXml.size() << ", files: " << files.size() << "\n";
    // parse scene
    std::vector<SceneObject> parsedObjects;
    std::vector<Light> lights;
    parseSceneXml(sceneXml, parsedObjects, lights);
    std::cout << "--- Parsed Object Names ---\n";
    for (const SceneObject& obj : parsedObjects) {
        std::cout << "Found object with name: '" << obj.name << "'\n";
    }
    std::cout << "---------------------------\n";
    std::cout << "Parsed scene: objects=" << parsedObjects.size() << " lights=" << lights.size() << "\n";
    // initialize DataModel from parsed objects
    DataModel dm;
    dm.initializeFrom(parsedObjects);
    // create input state
    InputState inputState;
    // init glfw/glad
    if(!glfwInit()) { std::cerr << "glfwInit fail\n"; return 1; }
    glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, 3);
    glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, 3);
    glfwWindowHint(GLFW_OPENGL_PROFILE, GLFW_OPENGL_CORE_PROFILE);
    GLFWwindow* wnd = glfwCreateWindow(1280, 720, "CGame Runtime", nullptr, nullptr);
    if(!wnd) { std::cerr << "create window fail\n"; glfwTerminate(); return 1; }
    glfwMakeContextCurrent(wnd);
    if(!gladLoadGLLoader((GLADloadproc)glfwGetProcAddress)) { std::cerr << "glad fail\n"; return 1; }
    // attach inputState to window user pointer so callbacks can access it
    glfwSetWindowUserPointer(wnd, &inputState);
    // set up callbacks to maintain InputState (main thread)
    glfwSetKeyCallback(wnd, [](GLFWwindow* w, int key, int /*scancode*/, int action, int /*mods*/) { // Fixed: Remove unused parameter names
        void* p = glfwGetWindowUserPointer(w);
        if(!p) return;
        InputState* s = (InputState*)p;
        if(action == GLFW_PRESS || action == GLFW_REPEAT) s->setKeyDown(key);
        else if(action == GLFW_RELEASE) s->setKeyUp(key);
    });
    glfwSetMouseButtonCallback(wnd, [](GLFWwindow* w, int button, int action, int /*mods*/) { // Fixed: Remove unused parameter name
        void* p = glfwGetWindowUserPointer(w);
        if(!p) return;
        InputState* s = (InputState*)p;
        if(action == GLFW_PRESS) s->setMouseButtonDown(button);
        else if(action == GLFW_RELEASE) s->setMouseButtonUp(button);
    });
    glEnable(GL_DEPTH_TEST);
    GLuint program = makeProgram(getDefaultVS(), getDefaultFS());
    if(!program) { std::cerr << "shader fail\n"; return 1; }
    // Find light to use (simple: first light or fallback)
    glm::vec3 lightPos(0,10,0), lightColor(1,1,1);
    if(!lights.empty()) { lightPos = lights[0].position; lightColor = lights[0].color * lights[0].intensity; }
    // For each distinct mesh filename referenced by objects, extract and load mesh
    std::unordered_map<std::string, MeshGL> meshCache;
    for(const SceneObject& obj : dm.getAllObjects()) {
        std::string meshName = obj.mesh;
        if(meshName.empty()) continue;
        if(meshCache.find(meshName) != meshCache.end()) continue;
        // find file entry in archive
        auto it = std::find_if(files.begin(), files.end(), [&](const ArchiveFileEntry &e){ return e.name == meshName; });
        if(it == files.end()) {
            std::cerr << "Warning: mesh not found in archive: " << meshName << "\n";
            continue;
        }
        std::string objData;
        if(!extractFileFromArchive(archivePath, *it, objData)) {
            std::cerr << "Failed extract: " << meshName << "\n"; continue;
        }
        std::vector<float> verts;
        std::vector<unsigned int> idxs;
        if(!loadObjFromString(objData, verts, idxs)) {
            std::cerr << "Failed parse OBJ: " << meshName << "\n"; continue;
        }
        MeshGL mg = uploadMesh(verts, idxs);
        meshCache[meshName] = mg;
        std::cout << "Loaded mesh: " << meshName << " verts=" << (verts.size()/6) << " tris=" << (idxs.size()/3) << "\n";
    }
    // --- Start Lua scripts in their own threads ---
    LuaScriptManager luaMgr(archivePath, files, &dm, &inputState);
    luaMgr.startAllScripts();
    // Main loop
    int w=1280,h=720;
    glfwGetFramebufferSize(wnd,&w,&h);
    float lastTime = (float)glfwGetTime();
    while(!glfwWindowShouldClose(wnd)) {
        float now = (float)glfwGetTime();
        float dt = now - lastTime; // Fixed: Keep dt for potential future use
        lastTime = now;
        glfwGetFramebufferSize(wnd,&w,&h);
        glViewport(0,0,w,h);
        glClearColor(0.1f, 0.12f, 0.15f, 1.0f);
        glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);
        glUseProgram(program);
        // Get camera from DataModel (assume name="camera")
        auto camObj = dm.getObjectCopy("camera");
        glm::vec3 camPos = glm::vec3(0,0,0);
        glm::vec3 camRot = glm::vec3(0,0,0); // Euler degrees
        float camFov = 45.0f;
        static Camera fallbackCamera; // Fixed: Rename to avoid shadowing global 'camera'
        if (camObj.has_value()) {
            camPos = camObj.value().position;
            camRot = camObj.value().rotation;
            camFov = camObj.value().fov;
        } else {
            // Fallback to initial global camera if no object (no inputs to update it)
            camPos = fallbackCamera.pos;
            camRot = glm::vec3(fallbackCamera.pitch * (180.0f / glm::pi<float>()), fallbackCamera.yaw * (180.0f / glm::pi<float>()), 0.0f);
            camFov = fallbackCamera.fovy * (180.0f / glm::pi<float>());
        }
        // Compute view from position and rotation
        glm::mat4 rotMat = glm::eulerAngleXYZ(glm::radians(camRot.x), glm::radians(camRot.y), glm::radians(camRot.z));
        glm::vec3 forward = rotMat * glm::vec4(0, 0, -1, 0); // Z-forward
        glm::vec3 up = rotMat * glm::vec4(0, 1, 0, 0);
        glm::mat4 view = glm::lookAt(camPos, camPos + forward, up);
        glm::mat4 proj = glm::perspective(glm::radians(camFov), (float)w/(float)h, 0.1f, 100.0f);
        GLint locModel = glGetUniformLocation(program, "uModel");
        GLint locView = glGetUniformLocation(program, "uView");
        GLint locProj = glGetUniformLocation(program, "uProj");
        GLint locColor = glGetUniformLocation(program, "uColor");
        GLint locCamPos = glGetUniformLocation(program, "uCamPos");
        GLint locLightPos = glGetUniformLocation(program, "uLightPos");
        GLint locLightCol = glGetUniformLocation(program, "uLightColor");
        glUniformMatrix4fv(locView, 1, GL_FALSE, &view[0][0]);
        glUniformMatrix4fv(locProj, 1, GL_FALSE, &proj[0][0]);
        glUniform3fv(locCamPos, 1, &camPos[0]);
        glUniform3fv(locLightPos, 1, &lightPos[0]);
        glUniform3fv(locLightCol, 1, &lightColor[0]);
        // draw objects — read current state from DataModel so script changes are visible
        for(const SceneObject& obj : dm.getAllObjects()) {
            glm::mat4 M(1.0f);
            M = glm::translate(M, obj.position);
            glm::vec3 r = glm::radians(obj.rotation);
            M = M * glm::eulerAngleXYZ(r.x, r.y, r.z);
            M = glm::scale(M, obj.scale);
            glUniformMatrix4fv(locModel, 1, GL_FALSE, &M[0][0]);
            glUniform3fv(locColor, 1, &obj.color[0]);
            if(obj.mesh.empty()) continue;
            auto it = meshCache.find(obj.mesh);
            if(it == meshCache.end()) continue;
            MeshGL &mg = it->second;
            glBindVertexArray(mg.vao);
            glDrawElements(GL_TRIANGLES, mg.indexCount, GL_UNSIGNED_INT, 0);
            glBindVertexArray(0);
        }
        glfwSwapBuffers(wnd);
        glfwPollEvents();
        if(glfwGetKey(wnd, GLFW_KEY_ESCAPE) == GLFW_PRESS) glfwSetWindowShouldClose(wnd, GLFW_TRUE);
    }
    // cleanup
    luaMgr.stopAllScripts();
    for(auto &kv : meshCache) kv.second.destroy();
    glDeleteProgram(program);
    glfwDestroyWindow(wnd);
    glfwTerminate();
    return 0;
}