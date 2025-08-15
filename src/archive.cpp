#include "archive.h"
#include <fstream>
#include <iostream>

bool loadCGame(const std::string &path,
               std::vector<ArchiveFileEntry> &files,
               std::string &sceneXml,
               const std::string &magicExpected) {
    std::ifstream in(path, std::ios::binary);
    if(!in) return false;

    // read magic
    char magic[4];
    in.read(magic, 4);
    if(in.gcount() != 4) return false;
    if(std::string(magic, 4) != magicExpected) {
        std::cerr << "Bad magic\n"; return false;
    }

    uint32_t version;
    in.read(reinterpret_cast<char*>(&version), sizeof(version));
    uint64_t toc_offset;
    in.read(reinterpret_cast<char*>(&toc_offset), sizeof(toc_offset));

    // seek TOC
    in.seekg(toc_offset, std::ios::beg);
    uint32_t num_files;
    in.read(reinterpret_cast<char*>(&num_files), sizeof(num_files));
    files.clear();
    for(uint32_t i=0;i<num_files;++i) {
        uint32_t name_len;
        in.read(reinterpret_cast<char*>(&name_len), sizeof(name_len));
        std::string name(name_len, '\0');
        in.read(&name[0], name_len);
        uint64_t off, sz;
        in.read(reinterpret_cast<char*>(&off), sizeof(off));
        in.read(reinterpret_cast<char*>(&sz), sizeof(sz));
        files.push_back({name, off, sz});
    }
    uint64_t scene_off, scene_sz;
    in.read(reinterpret_cast<char*>(&scene_off), sizeof(scene_off));
    in.read(reinterpret_cast<char*>(&scene_sz), sizeof(scene_sz));
    // read scene xml
    in.seekg(scene_off, std::ios::beg);
    sceneXml.resize((size_t)scene_sz);
    in.read(&sceneXml[0], scene_sz);
    return true;
}

bool extractFileFromArchive(const std::string &archivePath, const ArchiveFileEntry &entry, std::string &outData) {
    std::ifstream in(archivePath, std::ios::binary);
    if(!in) return false;
    in.seekg((std::streamoff)entry.offset, std::ios::beg);
    outData.resize((size_t)entry.size);
    in.read(&outData[0], (std::streamsize)entry.size);
    return true;
}
