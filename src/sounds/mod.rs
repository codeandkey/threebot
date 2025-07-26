use std::path::PathBuf;

pub mod manager;

pub use manager::*;

/// Represents a sound file in the system
#[derive(Debug, Clone)]
pub struct SoundFile {
    pub code: String,
    pub file_path: PathBuf,
    pub metadata: Option<crate::database::entities::sounds::Model>,
}

impl SoundFile {
    /// Creates a new SoundFile with the given code
    pub fn new(code: String, sounds_dir: &PathBuf) -> Self {
        let file_path = sounds_dir.join(format!("{}.mp3", code));
        Self {
            code,
            file_path,
            metadata: None,
        }
    }

    /// Checks if the sound file exists on disk
    pub fn exists(&self) -> bool {
        self.file_path.exists()
    }

    /// Gets the file path as a string
    pub fn path_str(&self) -> Option<&str> {
        self.file_path.to_str()
    }
}

/// Validates that a sound code is 4 alphabetic characters
pub fn validate_sound_code(code: &str) -> bool {
    code.len() == 4 && code.chars().all(|c| c.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_sound_code() {
        assert!(validate_sound_code("ABCD"));
        assert!(validate_sound_code("abcd"));
        assert!(validate_sound_code("AbCd"));

        assert!(!validate_sound_code("ABC")); // too short
        assert!(!validate_sound_code("ABCDE")); // too long
        assert!(!validate_sound_code("AB12")); // contains numbers
        assert!(!validate_sound_code("AB-D")); // contains special chars
        assert!(!validate_sound_code("")); // empty
    }
}
