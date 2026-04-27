use anyhow::Result;
use anyhow::anyhow;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::{fs, io};

/// Checks whether the checksum of the file at path 'a' matches the checksum saved in the file at path 'b'.
/// # Arguments
///
/// * `a` - A reference to a `&Path` object representing the path of the neovim archive.
/// * `b` - A reference to a `&Path` object representing the path of the checksum file.
///
/// # Returns
///
/// This function returns a `Result` that contains a `bool` indicating whether the checksum of the file at path 'a' matches the checksum saved in the file at path 'b'.
/// If there is an error opening or reading the files, the function returns `Err(error)`.
pub fn sha256cmp(a: &Path, b: &Path, filename: &str) -> Result<bool> {
    let checksum_contents = fs::read_to_string(b)?;
    let checksum = checksum_contents
        .lines()
        .find(|line| line.contains(filename))
        .and_then(|line| line.split_whitespace().next())
        .ok_or_else(|| anyhow!("Checksum not found for {filename}"))?;

    let mut hasher = Sha256::new();
    let mut file = fs::File::open(a)?;
    io::copy(&mut file, &mut hasher)?;

    let hash = hasher.finalize();
    let hash = format!("{hash:x}");

    Ok(hash == checksum)
}

/// Computes the SHA-256 checksum of the file at `path`.
/// # Arguments
///
/// * `path` - A reference to a `&Path` representing the file whose checksum will be computed.
///
/// # Returns
///
/// This function returns a `Result` which on success contains the SHA-256 digest produced by
/// the `sha2` crate (the concrete digest type returned by `Sha256::finalize()`). If the file
/// cannot be opened or an I/O error occurs while reading, the function returns `Err(error)`.
fn hash_binary(path: &Path) -> Result<impl PartialEq> {
    let mut hasher = Sha256::new();
    let file = fs::File::open(path)?;
    let mut reader = io::BufReader::new(file);
    io::copy(&mut reader, &mut hasher)?;

    let hash = hasher.finalize();
    Ok(hash)
}

/// Checks whether the SHA-256 checksum of `origin` matches the checksum of `proxy`.
/// # Arguments
///
/// * `origin` - A reference to a `&Path` for the original binary file.
/// * `proxy` - A reference to a `&Path` for the binary file to compare against the origin.
///
/// # Returns
///
/// This function returns a `Result` containing a `bool` which is `true` if the SHA-256 digests
/// of the two files are identical and `false` otherwise. Any I/O or hashing error encountered
/// while reading either file is returned as `Err(error)`.
pub fn compare_binaries(origin: &Path, proxy: &Path) -> Result<bool> {
    let origin_hash = hash_binary(origin)?;
    let proxy_hash = hash_binary(proxy)?;

    Ok(origin_hash == proxy_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::io;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn hash_binary_and_compare_binaries_same() {
        // create two temp files with identical contents
        let mut f1 = NamedTempFile::new().unwrap();
        let mut f2 = NamedTempFile::new().unwrap();
        write!(f1, "hello world").unwrap();
        write!(f2, "hello world").unwrap();
        f1.flush().unwrap();
        f2.flush().unwrap();

        // hash_binary is private, but unit tests in the same module can call it
        let h1 = hash_binary(f1.path()).unwrap();
        let h2 = hash_binary(f2.path()).unwrap();
        assert!(h1 == h2);

        // compare_binaries should report equality
        assert!(compare_binaries(f1.path(), f2.path()).unwrap());
    }

    #[test]
    fn compare_binaries_different() {
        let mut f1 = NamedTempFile::new().unwrap();
        let mut f2 = NamedTempFile::new().unwrap();
        write!(f1, "a").unwrap();
        write!(f2, "b").unwrap();
        f1.flush().unwrap();
        f2.flush().unwrap();

        assert!(!compare_binaries(f1.path(), f2.path()).unwrap());
    }

    #[test]
    fn sha256cmp_with_checksum_file() {
        // create a data file and write content
        let mut data = NamedTempFile::new().unwrap();
        write!(data, "payload").unwrap();
        data.flush().unwrap();

        // compute hex sha256 of the data file (so we can place it in the checksum file)
        let mut hasher = Sha256::new();
        let mut file = fs::File::open(data.path()).unwrap();
        io::copy(&mut file, &mut hasher).unwrap();
        let hex = format!("{:x}", hasher.finalize());

        // prepare a checksum file in the format: "<hex>  <filename>"
        // Use the real filename component so sha256cmp's search-by-filename will match.
        let filename = data.path().file_name().unwrap().to_str().unwrap();
        let mut checksum = NamedTempFile::new().unwrap();
        write!(checksum, "{}  {}", hex, filename).unwrap();
        checksum.flush().unwrap();

        // sha256cmp reads the checksum file and compares the computed digest of the data file
        assert!(sha256cmp(data.path(), checksum.path(), filename).unwrap());
    }
}
