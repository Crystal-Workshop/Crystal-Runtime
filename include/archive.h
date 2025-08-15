#pragma once
#include <string>
#include <vector>
#include <cstdint>

struct ArchiveFileEntry {
    std::string name;
    uint64_t offset;
    uint64_t size;
};

/**
 * Load a simple .cgame archive.
 *
 * - path: path to .cgame file
 * - files: out parameter receives file table entries
 * - sceneXml: out parameter receives the scene XML blob extracted from the archive
 * - magicExpected: magic header expected (defaults to "CGME")
 *
 * Returns true on success.
 *
 * NOTE: This parser assumes little-endian layout matching the writer.
 */
bool loadCGame(const std::string &path,
               std::vector<ArchiveFileEntry> &files,
               std::string &sceneXml,
               const std::string &magicExpected = "CGME");

/**
 * Extract a file stored in the archive to a memory string.
 *
 * - archivePath: path to the .cgame archive
 * - entry: an ArchiveFileEntry previously read from the archive TOC
 * - outData: receives the raw file bytes
 *
 * Returns true on success.
 */
bool extractFileFromArchive(const std::string &archivePath, const ArchiveFileEntry &entry, std::string &outData);
