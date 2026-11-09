//! Records selected hub streams to disk: one video per camera, one WAV per microphone, and a
//! per-frame sidecar with the wire timestamps (`docs/step-24-recorder.md`).

mod config;
mod ffmpeg;
mod input;
mod naming;
mod recorder;
mod session;
mod utc;
mod wav;
mod writer;

pub use config::{Codec, RecorderConfig, Source};
pub use ffmpeg::{codec_name, ffmpeg_args, ffmpeg_available};
pub use naming::{StemAllocator, file_stem, session_dir, session_id};
pub use recorder::{REVISION, Recording, record};
pub use session::{DeviceRecord, SessionInfo, SidecarLine};
pub use utc::{micros_since_epoch, rfc3339};
pub use wav::{WavWriter, read_wav_header, wav_header};
pub use writer::{FALLBACK_FPS, QUEUE_LEN, RATE_SAMPLE_FRAMES, measured_fps};
