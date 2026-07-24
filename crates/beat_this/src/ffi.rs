// Conic Launcher
// Copyright 2022-2026 OakChaser and contributors. All rights reserved.
// SPDX-License-Identifier: GPL-3.0-only

use libloader::libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};

#[repr(C)]
pub struct Utf8String {
    pub ptr: *const u8,
    pub len: usize,
}

#[repr(C)]
pub struct UsizeArray {
    ptr: *const usize,
    len: usize,
}

#[repr(C)]
pub struct FloatArray {
    ptr: *const f32,
    len: usize,
}

#[repr(C)]
pub struct TensorView {
    pub shape: UsizeArray,
    pub data: FloatArray,
}

#[repr(C)]
pub struct BeatAnalysisView {
    /// Beat times in seconds (sorted, deduplicated).
    pub beats: FloatArray,
    /// Downbeat times in seconds (sorted, deduplicated, snapped to nearest beat).
    pub downbeats: FloatArray,
    /// Mel spectrogram tensor with shape `[1, T, 128]` at 50 fps.
    pub mel: TensorView,
    /// Raw beat logits, one per spectrogram frame.
    pub beat_logits: FloatArray,
    /// Raw downbeat logits, one per spectrogram frame.
    pub downbeat_logits: FloatArray,
}

struct BeatAnalysisHandle {
    _private: [u8; 0],
}

type ParseAudioFile = unsafe extern "C" fn(
    path: Utf8String,
    result: *mut BeatAnalysisView,
    handle: *mut *mut BeatAnalysisHandle,
) -> i32;

type DestroyHandle = unsafe extern "C" fn(handle: *mut BeatAnalysisHandle);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tensor {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatAnalysis {
    /// Beat times in seconds (sorted, deduplicated).
    pub beats: Vec<f32>,
    /// Downbeat times in seconds (sorted, deduplicated, snapped to nearest beat).
    pub downbeats: Vec<f32>,
    /// Mel spectrogram tensor with shape `[1, T, 128]` at 50 fps.
    pub mel: Tensor,
    /// Raw beat logits, one per spectrogram frame.
    pub beat_logits: Vec<f32>,
    /// Raw downbeat logits, one per spectrogram frame.
    pub downbeat_logits: Vec<f32>,
}

pub fn parse_audio_file(library: Library, path: String) -> crate::error::Result<BeatAnalysis> {
    unsafe {
        let parse: Symbol<ParseAudioFile> = library.get(b"parse_audio_file").unwrap();
        let destroy: Symbol<DestroyHandle> = library.get(b"beat_analysis_destroy").unwrap();

        let mut view = std::mem::zeroed();
        let mut handle = std::ptr::null_mut();

        let path: &str = path.as_ref();
        let code = parse(
            Utf8String {
                ptr: path.as_ptr(),
                len: path.len(),
            },
            &mut view,
            &mut handle,
        );

        println!("code={}", code);
        let beats = std::slice::from_raw_parts(view.beats.ptr, view.beats.len);
        println!("{:#?}", beats);
        let beat_analysis = BeatAnalysis {
            beats: std::slice::from_raw_parts(view.beats.ptr, view.beats.len).to_vec(),
            downbeats: std::slice::from_raw_parts(view.downbeats.ptr, view.downbeats.len).to_vec(),
            mel: Tensor {
                data: std::slice::from_raw_parts(view.mel.data.ptr, view.mel.data.len).to_vec(),
                shape: std::slice::from_raw_parts(view.mel.shape.ptr, view.mel.shape.len).to_vec(),
            },
            beat_logits: std::slice::from_raw_parts(view.beat_logits.ptr, view.beat_logits.len)
                .to_vec(),
            downbeat_logits: std::slice::from_raw_parts(
                view.downbeat_logits.ptr,
                view.downbeat_logits.len,
            )
            .to_vec(),
        };
        destroy(handle);
        Ok(beat_analysis)
    }
}
