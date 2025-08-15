#pragma once
#include <string>
#include <vector>

/**
 * Extract inner text for a simple tag <tag>text</tag>.
 *
 * This helper is simple and assumes the tag appears once inside the block.
 * Returns empty string if not found.
 */
std::string extractTagText(const std::string &block, const std::string &tag);

/**
 * Find all top-level blocks of the form <blockTag ...> ... </blockTag>
 * and return the inner contents (between > and </blockTag>).
 *
 * This is a minimal, ad-hoc helper intended for the controlled scene XML format.
 */
std::vector<std::string> findBlocks(const std::string &xml, const std::string &blockTag);

/**
 * Split a whitespace-separated list of floats into a vector<float>.
 * Useful for reading "x y z" style attributes.
 */
void splitToFloats(const std::string &s, std::vector<float> &out);
