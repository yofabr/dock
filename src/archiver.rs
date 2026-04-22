use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::{self, File};
use std::io;
use std::path::Path;

pub fn create_tar_gz(source: &Path, archive: &Path) -> io::Result<u64> {
    let file = File::create(archive)?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = tar::Builder::new(enc);

    let name = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("archive");

    if source.is_dir() {
        tar.append_dir_all(name, source)?;
    } else {
        tar.append_path_with_name(source, name)?;
    }

    tar.finish()?;
    let enc = tar.into_inner()?;
    enc.finish()?;
    fs::metadata(archive).map(|m| m.len())
}

pub fn extract_tar_gz(archive: &Path, target: &Path) -> io::Result<()> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    fs::create_dir_all(target)?;
    let file = File::open(archive)?;
    let mut dec = GzDecoder::new(file);

    let mut ar = Archive::new(&mut dec);

    for entry in ar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let entry_type = entry.header().entry_type();

        if entry_type.is_dir() {
            let stripped: std::path::PathBuf = path.components().skip(1).collect();
            fs::create_dir_all(&target.join(&stripped))?;
        } else {
            let stripped: std::path::PathBuf = path.components().skip(1).collect();
            let file_path = target.join(&stripped);
            fs::create_dir_all(file_path.parent().unwrap_or(target))?;
            entry.unpack(&file_path)?;
        }
    }

    Ok(())
}

#[cfg(windows)]
pub mod windows_archiver {
    use std::fs::{self, File};
    use std::io::{self, BufReader};
    use std::path::Path;
    use zip::write::FileOptions;
    use zip::ZipWriter;

    pub fn create_zip(source: &Path, archive: &Path) -> io::Result<u64> {
        let file = File::create(archive)?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        if source.is_dir() {
            walkdir(source, source, &mut zip, &options)?;
        } else {
            let name = source.file_name().and_then(|n| n.to_str()).unwrap_or("archive");
            zip.start_file(name, options.clone())?;
            let mut f = File::open(source)?;
            io::copy(&mut f, &mut zip)?;
        }

        zip.finish()?;
        Ok(fs::metadata(archive).map(|m| m.len()).unwrap_or(0))
    }

    fn walkdir(dir: &Path, base: &Path, zip: &mut ZipWriter<File>, options: &FileOptions<()>) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = path.strip_prefix(base).unwrap_or(&path);
            let name_str = name.to_string_lossy().replace('\\', "/");

            if path.is_dir() {
                zip.add_directory(&name_str, options.clone())?;
                walkdir(&path, base, zip, options)?;
            } else {
                zip.start_file(&name_str, options.clone())?;
                let mut f = File::open(&path)?;
                io::copy(&mut f, zip)?;
            }
        }
        Ok(())
    }

    pub fn extract_zip(archive: &Path, target: &Path) -> io::Result<()> {
        let file = File::open(archive)?;
        let reader = BufReader::new(file);
        let mut ar = zip::ZipArchive::new(reader)?;
        ar.extract(target)
    }
}