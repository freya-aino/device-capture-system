//! A minimal PCM WAV writer (`S16LE`), header patched when the file is finished.

use std::fs::File;
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::Path;

use anyhow::{Context, Result, ensure};

pub struct WavWriter {
    file: BufWriter<File>,
    sample_rate: u32,
    channels: u16,
    data_bytes: u64,
}

impl WavWriter {
    pub fn create(path: &Path, sample_rate: u32, channels: u16) -> Result<Self> {
        ensure!(sample_rate > 0, "WAV sample rate must be positive");
        ensure!(channels > 0, "WAV channel count must be positive");
        let file = File::create(path).with_context(|| format!("creating {}", path.display()))?;
        let mut writer = WavWriter {
            file: BufWriter::new(file),
            sample_rate,
            channels,
            data_bytes: 0,
        };
        writer
            .file
            .write_all(&wav_header(sample_rate, channels, 0))?;
        Ok(writer)
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn write(&mut self, pcm_s16le: &[u8]) -> Result<()> {
        self.file.write_all(pcm_s16le)?;
        self.data_bytes += pcm_s16le.len() as u64;
        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        self.file.flush()?;
        self.file.seek(SeekFrom::Start(0))?;
        let data_bytes = u32::try_from(self.data_bytes).unwrap_or(u32::MAX);
        self.file
            .write_all(&wav_header(self.sample_rate, self.channels, data_bytes))?;
        self.file.flush()?;
        Ok(())
    }
}

/// The 44-byte canonical PCM header.
pub fn wav_header(sample_rate: u32, channels: u16, data_bytes: u32) -> [u8; 44] {
    let block_align = channels * 2;
    let byte_rate = sample_rate * u32::from(block_align);
    let mut h = [0u8; 44];
    h[0..4].copy_from_slice(b"RIFF");
    h[4..8].copy_from_slice(&(36 + data_bytes).to_le_bytes());
    h[8..12].copy_from_slice(b"WAVE");
    h[12..16].copy_from_slice(b"fmt ");
    h[16..20].copy_from_slice(&16u32.to_le_bytes());
    h[20..22].copy_from_slice(&1u16.to_le_bytes());
    h[22..24].copy_from_slice(&channels.to_le_bytes());
    h[24..28].copy_from_slice(&sample_rate.to_le_bytes());
    h[28..32].copy_from_slice(&byte_rate.to_le_bytes());
    h[32..34].copy_from_slice(&block_align.to_le_bytes());
    h[34..36].copy_from_slice(&16u16.to_le_bytes());
    h[36..40].copy_from_slice(b"data");
    h[40..44].copy_from_slice(&data_bytes.to_le_bytes());
    h
}

/// Sample rate, channels and data length of a canonical PCM WAV file, for tests and tools.
pub fn read_wav_header(bytes: &[u8]) -> Result<(u32, u16, u32)> {
    ensure!(bytes.len() >= 44, "WAV header is shorter than 44 bytes");
    ensure!(
        &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE",
        "not a RIFF/WAVE file"
    );
    ensure!(
        &bytes[36..40] == b"data",
        "expected the data chunk at offset 36"
    );
    let le16 = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    let le32 = |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    Ok((le32(24), le16(22), le32(40)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_is_patched_with_the_data_length() {
        let dir = std::env::temp_dir().join(format!("recorder-wav-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mic.wav");
        let mut w = WavWriter::create(&path, 16_000, 1).unwrap();
        w.write(&[0, 0, 255, 127, 0, 128]).unwrap();
        w.write(&[1, 0]).unwrap();
        w.finish().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 52);
        assert_eq!(read_wav_header(&bytes).unwrap(), (16_000, 1, 8));
        assert_eq!(&bytes[44..], &[0, 0, 255, 127, 0, 128, 1, 0]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn header_fields_follow_the_canonical_layout() {
        let h = wav_header(48_000, 2, 1000);
        assert_eq!(&h[20..22], &1u16.to_le_bytes(), "PCM");
        assert_eq!(&h[28..32], &(48_000u32 * 4).to_le_bytes(), "byte rate");
        assert_eq!(&h[32..34], &4u16.to_le_bytes(), "block align");
        assert_eq!(&h[4..8], &1036u32.to_le_bytes(), "RIFF size");
        assert!(WavWriter::create(Path::new("/nonexistent/x.wav"), 0, 1).is_err());
    }
}
