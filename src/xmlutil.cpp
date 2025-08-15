#include "xmlutil.h"
#include <sstream>

std::string extractTagText(const std::string &block, const std::string &tag) {
    std::string open = "<" + tag + ">";
    std::string close = "</" + tag + ">";
    auto a = block.find(open);
    if(a==std::string::npos) return "";
    a += open.size();
    auto b = block.find(close, a);
    if(b==std::string::npos) return "";
    return block.substr(a, b - a);
}

std::vector<std::string> findBlocks(const std::string &xml, const std::string &blockTag) {
    std::vector<std::string> out;
    std::string open = "<" + blockTag;
    std::string openClose = ">";
    std::string close = "</" + blockTag + ">";
    size_t pos = 0;
    while(true) {
        size_t start = xml.find(open, pos);
        if(start==std::string::npos) break;
        // find end of opening tag '>'
        size_t tagEnd = xml.find(openClose, start);
        if(tagEnd==std::string::npos) break;
        size_t contentStart = tagEnd + 1;
        size_t end = xml.find(close, contentStart);
        if(end==std::string::npos) break;
        out.push_back(xml.substr(contentStart, end - contentStart));
        pos = end + close.size();
    }
    return out;
}

void splitToFloats(const std::string &s, std::vector<float> &out) {
    out.clear();
    std::istringstream ss(s);
    float v;
    while(ss >> v) out.push_back(v);
}
