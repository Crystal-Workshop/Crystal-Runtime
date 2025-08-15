#include "scene.h"
#include "xmlutil.h"
#include <glm/gtc/constants.hpp>
#include <iostream>

void parseSceneXml(const std::string &xml, std::vector<SceneObject> &outObjects, std::vector<Light> &outLights) {
    outObjects.clear();
    outLights.clear();
    auto objBlocks = findBlocks(xml, "object");
    for(auto &b : objBlocks) {
        SceneObject obj;
        obj.name = extractTagText(b, "name");
        obj.type = extractTagText(b, "type");
        obj.mesh = extractTagText(b, "mesh");
        { 
            std::string s = extractTagText(b, "color"); 
            std::vector<float> v; 
            splitToFloats(s,v); 
            if(v.size()>=3) obj.color = glm::vec3(v[0]/255.0f, v[1]/255.0f, v[2]/255.0f); 
        }
        { 
            std::string s = extractTagText(b, "position"); 
            std::vector<float> v; 
            splitToFloats(s,v); 
            if(v.size()>=3) obj.position = glm::vec3(v[0],v[1],v[2]); 
        }
        { 
            std::string s = extractTagText(b, "rotation"); 
            std::vector<float> v; 
            splitToFloats(s,v); 
            if(v.size()>=3) obj.rotation = glm::vec3(v[0],v[1],v[2]); 
        }
        { 
            std::string s = extractTagText(b, "scale");    
            std::vector<float> v; 
            splitToFloats(s,v); 
            if(v.size()>=3) obj.scale = glm::vec3(v[0],v[1],v[2]); 
        }
        { 
            std::string s = extractTagText(b, "fov"); 
            if(!s.empty()) obj.fov = std::stof(s); 
        }
        std::string inten = extractTagText(b, "intensity");
        if(!inten.empty()) { 
            obj.intensity = std::stof(inten); 
        }
        if(obj.type == "light") {
            Light light;
            light.position = obj.position;
            light.color = obj.color;
            light.intensity = obj.intensity;
            outLights.push_back(light);
        } else {
            outObjects.push_back(obj);
        }
    }
}