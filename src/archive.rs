use std::convert::TryFrom;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};

/// File entry extracted from the archive table of contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveFileEntry {
    pub name: String,
    pub offset: u64,
    pub size: u64,
}

/// In-memory representation of a `.cgame` archive.
#[derive(Debug, Clone)]
pub struct CGameArchive {
    backing: ArchiveBacking,
    version: u32,
    files: Vec<ArchiveFileEntry>,
    scene_xml: String,
}

#[derive(Debug, Clone)]
enum ArchiveBacking {
    File(PathBuf),
    Memory { _label: String, data: Arc<[u8]> },
}

impl CGameArchive {
    /// Opens an archive from disk and eagerly loads the scene XML blob.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let mut file = File::open(&path_buf)
            .with_context(|| format!("unable to open {}", path_buf.display()))?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .context("unable to read archive into memory")?;

        let (version, files, scene_xml) = parse_archive_metadata(&data)?;

        Ok(Self {
            backing: ArchiveBacking::File(path_buf),
            version,
            files,
            scene_xml,
        })
    }

    /// Creates an archive from bytes already resident in memory.
    pub fn from_bytes(label: impl Into<String>, data: Vec<u8>) -> Result<Self> {
        let storage: Arc<[u8]> = Arc::from(data.into_boxed_slice());
        let (version, files, scene_xml) = parse_archive_metadata(&storage)?;
        Ok(Self {
            backing: ArchiveBacking::Memory {
                _label: label.into(),
                data: Arc::clone(&storage),
            },
            version,
            files,
            scene_xml,
        })
    }

    /// Returns the engine version stored in the archive header.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Returns the raw scene XML contained in the archive.
    pub fn scene_xml(&self) -> &str {
        &self.scene_xml
    }

    /// Returns the list of files bundled alongside the scene.
    pub fn files(&self) -> &[ArchiveFileEntry] {
        &self.files
    }

    /// Looks up a file entry by name.
    pub fn file(&self, name: &str) -> Option<&ArchiveFileEntry> {
        self.files.iter().find(|entry| entry.name == name)
    }

    /// Extracts the raw bytes for the provided entry name.
    pub fn extract_file(&self, name: &str) -> Result<Vec<u8>> {
        let entry = self
            .file(name)
            .ok_or_else(|| anyhow!("file not found in archive: {name}"))?;
        self.extract_entry(entry)
    }

    /// Extracts the raw bytes for a previously looked-up entry.
    pub fn extract_entry(&self, entry: &ArchiveFileEntry) -> Result<Vec<u8>> {
        match &self.backing {
            ArchiveBacking::File(path) => {
                let mut file = File::open(path)
                    .with_context(|| format!("unable to reopen archive {}", path.display()))?;
                file.seek(SeekFrom::Start(entry.offset))
                    .with_context(|| format!("unable to seek to {}", entry.name))?;
                let mut buffer = vec![0u8; entry.size as usize];
                file.read_exact(&mut buffer)
                    .with_context(|| format!("unable to read {} from archive", entry.name))?;
                Ok(buffer)
            }
            ArchiveBacking::Memory { data, .. } => {
                let start = entry.offset as usize;
                let end = start + entry.size as usize;
                if end > data.len() {
                    return Err(anyhow!(
                        "entry {} extends past archive bounds ({} > {})",
                        entry.name,
                        end,
                        data.len()
                    ));
                }
                Ok(data[start..end].to_vec())
            }
        }
    }
}

fn parse_archive_metadata(data: &[u8]) -> Result<(u32, Vec<ArchiveFileEntry>, String)> {
    if data.len() < 16 {
        return Err(anyhow!(
            "archive too small to contain header (len={})",
            data.len()
        ));
    }

    let magic = &data[..4];
    if magic != b"CGME" {
        return Err(anyhow!(
            "invalid archive magic: expected CGME, found {:?}",
            magic
        ));
    }

    let version_bytes: [u8; 4] = data[4..8].try_into().expect("slice length verified above");
    let toc_bytes: [u8; 8] = data[8..16].try_into().expect("slice length verified above");

    let (_endian, version, _toc_offset, files, scene_xml) =
        parse_archive_bytes(data, version_bytes, toc_bytes)?;
    Ok((version, files, scene_xml))
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum ArchiveEndian {
    Little,
    Big,
}

impl ArchiveEndian {
    fn decode_u32(self, bytes: [u8; 4]) -> u32 {
        match self {
            ArchiveEndian::Little => u32::from_le_bytes(bytes),
            ArchiveEndian::Big => u32::from_be_bytes(bytes),
        }
    }

    fn decode_u64(self, bytes: [u8; 8]) -> u64 {
        match self {
            ArchiveEndian::Little => u64::from_le_bytes(bytes),
            ArchiveEndian::Big => u64::from_be_bytes(bytes),
        }
    }

    #[cfg(test)]
    fn encode_u32(self, value: u32) -> [u8; 4] {
        match self {
            ArchiveEndian::Little => value.to_le_bytes(),
            ArchiveEndian::Big => value.to_be_bytes(),
        }
    }

    #[cfg(test)]
    fn encode_u64(self, value: u64) -> [u8; 8] {
        match self {
            ArchiveEndian::Little => value.to_le_bytes(),
            ArchiveEndian::Big => value.to_be_bytes(),
        }
    }
}

fn parse_archive_bytes(
    data: &[u8],
    version_bytes: [u8; 4],
    toc_bytes: [u8; 8],
) -> Result<(ArchiveEndian, u32, u64, Vec<ArchiveFileEntry>, String)> {
    let mut last_error = None;
    let file_len = data.len() as u64;

    for endian in [ArchiveEndian::Little, ArchiveEndian::Big] {
        let version = endian.decode_u32(version_bytes);
        let toc_offset = endian.decode_u64(toc_bytes);
        if (16..=file_len.saturating_sub(16)).contains(&toc_offset) {
            match parse_toc_block(data, endian, toc_offset) {
                Ok((files, scene_offset, scene_size)) => {
                    let scene_xml = extract_scene(data, scene_offset, scene_size)?;
                    return Ok((endian, version, toc_offset, files, scene_xml));
                }
                Err(err) => last_error = Some(err),
            }
        }
    }

    for endian in [ArchiveEndian::Little, ArchiveEndian::Big] {
        let version = endian.decode_u32(version_bytes);
        match locate_toc_by_scanning(data, endian) {
            Ok((toc_offset, files, scene_offset, scene_size)) => {
                let scene_xml = extract_scene(data, scene_offset, scene_size)?;
                return Ok((endian, version, toc_offset, files, scene_xml));
            }
            Err(err) => last_error = Some(err),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        anyhow!(
            "unable to locate archive table of contents (file length {file_len}, little {}, big {})",
            u64::from_le_bytes(toc_bytes),
            u64::from_be_bytes(toc_bytes)
        )
    }))
}

fn parse_toc_block(
    data: &[u8],
    endian: ArchiveEndian,
    toc_offset: u64,
) -> Result<(Vec<ArchiveFileEntry>, u64, u64)> {
    let len = data.len();
    let toc_start = toc_offset as usize;
    if toc_start < 16 || toc_start >= len {
        return Err(anyhow!(
            "archive TOC offset {toc_offset} is outside file bounds"
        ));
    }
    if len < 16 {
        return Err(anyhow!("archive is too small to contain TOC"));
    }

    let toc_end = len - 16;
    if toc_start > toc_end {
        return Err(anyhow!(
            "archive TOC offset {toc_offset} overlaps scene metadata"
        ));
    }

    let mut cursor = toc_start;
    let num_files = read_u32_from_slice(data, &mut cursor, endian)?;
    let mut files = Vec::with_capacity(num_files as usize);

    for _ in 0..num_files {
        let name_len = read_u32_from_slice(data, &mut cursor, endian)? as usize;
        if cursor
            .checked_add(name_len)
            .filter(|end| *end <= toc_end)
            .is_none()
        {
            return Err(anyhow!(
                "archive file name extends past TOC region (offset={toc_offset})"
            ));
        }
        let name_bytes = &data[cursor..cursor + name_len];
        let name = String::from_utf8(name_bytes.to_vec())
            .map_err(|err| anyhow!("invalid UTF-8 in file name: {err}"))?;
        cursor += name_len;

        let offset = read_u64_from_slice(data, &mut cursor, endian)?;
        let size = read_u64_from_slice(data, &mut cursor, endian)?;
        if offset
            .checked_add(size)
            .filter(|end| *end <= len as u64)
            .is_none()
        {
            return Err(anyhow!(
                "file entry {name} points outside archive bounds (offset={offset}, size={size}, len={})",
                len
            ));
        }
        files.push(ArchiveFileEntry { name, offset, size });
    }

    if cursor != toc_end {
        return Err(anyhow!(
            "archive TOC parsing ended at {cursor}, expected {toc_end}"
        ));
    }

    let scene_offset = endian.decode_u64(
        data[toc_end..toc_end + 8]
            .try_into()
            .expect("slice length verified"),
    );
    let scene_size = endian.decode_u64(
        data[toc_end + 8..len]
            .try_into()
            .expect("slice length verified"),
    );

    if scene_offset
        .checked_add(scene_size)
        .filter(|end| *end <= len as u64)
        .is_none()
    {
        return Err(anyhow!(
            "scene blob points outside archive bounds (offset={scene_offset}, size={scene_size}, len={len})"
        ));
    }

    Ok((files, scene_offset, scene_size))
}

fn locate_toc_by_scanning(
    data: &[u8],
    endian: ArchiveEndian,
) -> Result<(u64, Vec<ArchiveFileEntry>, u64, u64)> {
    let len = data.len();
    if len < 32 {
        return Err(anyhow!("archive too small to locate TOC"));
    }

    let toc_end = len - 16;
    let mut last_error = None;
    let mut empty_result = None;

    for candidate in (16..=toc_end).rev() {
        if candidate + 4 > toc_end {
            continue;
        }
        let num_files = endian.decode_u32(
            data[candidate..candidate + 4]
                .try_into()
                .expect("slice length checked"),
        );
        if num_files > 1_000_000 {
            continue;
        }

        match parse_toc_block(data, endian, candidate as u64) {
            Ok((files, scene_offset, scene_size)) => {
                if files.is_empty() {
                    if empty_result.is_none() {
                        empty_result = Some((candidate as u64, files, scene_offset, scene_size));
                    }
                } else {
                    return Ok((candidate as u64, files, scene_offset, scene_size));
                }
            }
            Err(err) => last_error = Some(err),
        }
    }

    if let Some(result) = empty_result {
        return Ok(result);
    }

    Err(last_error.unwrap_or_else(|| anyhow!("unable to find TOC by scanning")))
}

fn read_u32_from_slice(data: &[u8], cursor: &mut usize, endian: ArchiveEndian) -> Result<u32> {
    if *cursor + 4 > data.len() {
        return Err(anyhow!(
            "unexpected end of archive while reading 32-bit value"
        ));
    }
    let value = endian.decode_u32(
        data[*cursor..*cursor + 4]
            .try_into()
            .expect("slice length verified"),
    );
    *cursor += 4;
    Ok(value)
}

fn read_u64_from_slice(data: &[u8], cursor: &mut usize, endian: ArchiveEndian) -> Result<u64> {
    if *cursor + 8 > data.len() {
        return Err(anyhow!(
            "unexpected end of archive while reading 64-bit value"
        ));
    }
    let value = endian.decode_u64(
        data[*cursor..*cursor + 8]
            .try_into()
            .expect("slice length verified"),
    );
    *cursor += 8;
    Ok(value)
}

fn extract_scene(data: &[u8], scene_offset: u64, scene_size: u64) -> Result<String> {
    let start = usize::try_from(scene_offset)
        .map_err(|_| anyhow!("scene offset exceeds usize range: {scene_offset}"))?;
    let size = usize::try_from(scene_size)
        .map_err(|_| anyhow!("scene size exceeds usize range: {scene_size}"))?;
    if start
        .checked_add(size)
        .filter(|end| *end <= data.len())
        .is_none()
    {
        return Err(anyhow!(
            "scene blob points outside archive bounds (offset={scene_offset}, size={scene_size}, len={})",
            data.len()
        ));
    }

    let bytes = &data[start..start + size];
    String::from_utf8(bytes.to_vec()).map_err(|err| anyhow!("scene XML is not valid UTF-8: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::io::Write;
    use tempfile::NamedTempFile;

    static SCENE_XML: Lazy<String> = Lazy::new(|| {
        "<scene>\n  <object>\n    <name>Cube</name>\n    <type>mesh</type>\n  </object>\n</scene>\n"
            .to_string()
    });

    fn build_archive_buffer(endian: ArchiveEndian, files: &[(&str, &[u8])]) -> Vec<u8> {
        let scene_bytes = SCENE_XML.as_bytes();

        let mut buffer = Vec::new();
        buffer.extend_from_slice(b"CGME");
        buffer.extend_from_slice(&endian.encode_u32(1));
        buffer.extend_from_slice(&endian.encode_u64(0)); // placeholder for toc

        let header_len = buffer.len() as u64;
        let mut entries = Vec::new();
        let mut cursor = header_len;

        for (name, data) in files {
            entries.push((name.to_string(), cursor, data.len() as u64));
            buffer.extend_from_slice(data);
            cursor += data.len() as u64;
        }

        let scene_offset = cursor;
        buffer.extend_from_slice(scene_bytes);
        cursor += scene_bytes.len() as u64;

        let toc_offset = cursor;
        buffer.extend_from_slice(&endian.encode_u32(files.len() as u32));
        for (name, offset, size) in &entries {
            buffer.extend_from_slice(&endian.encode_u32(name.len() as u32));
            buffer.extend_from_slice(name.as_bytes());
            buffer.extend_from_slice(&endian.encode_u64(*offset));
            buffer.extend_from_slice(&endian.encode_u64(*size));
        }
        buffer.extend_from_slice(&endian.encode_u64(scene_offset));
        buffer.extend_from_slice(&endian.encode_u64(scene_bytes.len() as u64));

        let toc_offset_bytes = endian.encode_u64(toc_offset);
        buffer[8..16].copy_from_slice(&toc_offset_bytes);

        buffer
    }

    fn write_archive(buffer: &[u8]) -> NamedTempFile {
        let mut tmp = NamedTempFile::new().expect("tmp file");
        tmp.write_all(buffer).expect("write archive");
        tmp
    }

    fn create_archive(
        endian: ArchiveEndian,
        files: &[(&str, &[u8])],
    ) -> (NamedTempFile, CGameArchive) {
        let buffer = build_archive_buffer(endian, files);
        let tmp = write_archive(&buffer);
        let archive = CGameArchive::open(tmp.path()).expect("open archive");
        (tmp, archive)
    }

    #[test]
    fn open_archive_reads_scene_and_files() {
        let (_tmp, archive) = create_archive(
            ArchiveEndian::Little,
            &[("scripts/test.lua", b"print('hi')")],
        );
        assert_eq!(archive.version(), 1);
        assert_eq!(archive.scene_xml(), SCENE_XML.as_str());
        assert_eq!(archive.files().len(), 1);
        assert_eq!(archive.files()[0].name, "scripts/test.lua");
    }

    #[test]
    fn extract_file_returns_bytes() {
        let (_tmp, archive) =
            create_archive(ArchiveEndian::Little, &[("scripts/test.lua", b"return 42")]);
        let bytes = archive.extract_file("scripts/test.lua").unwrap();
        assert_eq!(bytes, b"return 42");
    }

    #[test]
    fn extract_missing_file_is_error() {
        let (_tmp, archive) = create_archive(ArchiveEndian::Little, &[]);
        assert!(archive.extract_file("missing.lua").is_err());
    }

    #[test]
    fn open_big_endian_archive() {
        let (_tmp, archive) =
            create_archive(ArchiveEndian::Big, &[("scripts/test.lua", b"print('hi')")]);
        assert_eq!(archive.version(), 1);
        assert_eq!(archive.files().len(), 1);
    }

    #[test]
    fn recover_from_corrupted_toc_header() {
        let mut buffer = build_archive_buffer(
            ArchiveEndian::Little,
            &[("scripts/test.lua", b"print('hi')")],
        );
        buffer[8..16].copy_from_slice(b"\x00\x00\x00\x00<pla");

        let tmp = write_archive(&buffer);
        let archive = CGameArchive::open(tmp.path()).expect("open archive");
        assert_eq!(archive.files().len(), 1);
        assert_eq!(archive.files()[0].name, "scripts/test.lua");
    }
}
