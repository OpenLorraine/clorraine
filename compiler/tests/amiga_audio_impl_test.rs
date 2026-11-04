//! Audio/input/timer standard-library implementation tests.
//!
//! These cases inspect the C the compiler emits for a hosted-Amiga program
//! that uses `std.amiga.audio`, `std.amiga.input`, and `std.amiga.timer`: the
//! application surface lowers to the standard-library wrappers, each wrapper
//! body carries its raw shim call, and every owned handle drops behind a null
//! guard in reverse declaration order.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Temporary directory that removes itself when the test finishes.
struct ScratchDir {
    path: PathBuf,
}

impl ScratchDir {
    /// Creates a unique directory for one test process and tag.
    fn new(tag: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "clc-audio-impl-{}-{tag}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("cannot create {}: {err}", path.display()));
        Self { path }
    }

    /// Path of the scratch directory.
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Runs the built compiler with the given arguments.
fn run_clc(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clc"))
        .args(args)
        .output()
        .expect("failed to spawn the compiler binary")
}

/// Compiles `source` for hosted-amiga and returns the emitted C.
fn emit_hosted_amiga(tag: &str, source: &str) -> String {
    let scratch = ScratchDir::new(tag);
    let source_path = scratch.path().join("main.clo");
    std::fs::write(&source_path, source).expect("write the snippet");
    let output_path = scratch.path().join("out.c");
    let run = run_clc(&[
        "--profile".to_owned(),
        "hosted-amiga".to_owned(),
        "-o".to_owned(),
        output_path.display().to_string(),
        source_path.display().to_string(),
    ]);
    assert_eq!(
        run.status.code(),
        Some(0),
        "{tag}: the front end rejected the program; stderr:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    std::fs::read_to_string(&output_path).expect("read the emitted C")
}

/// A program that uses one call from every part of the surface.
const ROUTING_PROGRAM: &str = r#"module impl.audio.routing;

import {
    Audio, Sample, openAudio, newSample, audioChannels, audioPlay,
    audioStop, audioControl
} from std.amiga.audio;
import { readJoystick, decodeJoystick } from std.amiga.input;
import { Timer, openTimer, timerWait } from std.amiga.timer;
import { slice } from std.mem;

func main(): i32 {
    let data: u8[8] = [; 0];
    let sample: Sample? = newSample(slice(data, 0, 8), 428, 64, 1);
    if (sample == null) {
        return 1;
    }
    let audio: Audio? = openAudio(4);
    if (audio == null) {
        return 2;
    }
    let mask: u8 = audioChannels(audio);
    let played: bool = audioPlay(audio, 0, sample);
    if (played) {
        audioControl(audio, 0, 400, 32);
        audioStop(audio, 0);
    }
    let joy: u8 = readJoystick(0);
    let decoded: u8 = decodeJoystick(0x0002, false);
    let timer: Timer? = openTimer();
    if (timer == null) {
        return 3;
    }
    let waited: bool = timerWait(timer, 1);
    if (mask == 0 || joy == 0xFF || decoded == 0xFF || waited) {
        return 4;
    }
    return 0;
}
"#;

/// Every audio/input/timer call sits above `ffi` and reaches the shim.
#[test]
fn every_audio_call_sits_above_ffi() {
    let c = emit_hosted_amiga("routing", ROUTING_PROGRAM);
    for wrapper_call in [
        "cl_std_amiga_audio_newSample(",
        "cl_std_amiga_audio_openAudio(4u)",
        "cl_std_amiga_audio_audioChannels(audio)",
        "cl_std_amiga_audio_audioPlay(audio, 0u, sample)",
        "cl_std_amiga_audio_audioControl(audio, 0u, 400u, 32u)",
        "cl_std_amiga_audio_audioStop(audio, 0u)",
        "cl_std_amiga_input_readJoystick(0u)",
        "cl_std_amiga_timer_openTimer(",
        "cl_std_amiga_timer_timerWait(timer, 1u)",
    ] {
        assert!(
            c.contains(wrapper_call),
            "the application must call the wrapper `{wrapper_call}`"
        );
    }
    for shim_call in [
        "cl_sample_new(",
        "cl_audio_open(channels)",
        "cl_audio_channels(audio)",
        "cl_audio_play(audio, channel",
        "cl_audio_control(audio, channel",
        "cl_audio_stop(audio, channel",
        "cl_input_state(port)",
        "cl_timer_open(",
        "cl_timer_wait(timer",
    ] {
        assert!(
            c.contains(shim_call),
            "a wrapper body must call the shim `{shim_call}`"
        );
    }
}

/// A program that relies on generated drops for every owned handle.
const DROP_PROGRAM: &str = r#"module impl.audio.drop;

import { Audio, Sample, newSample, openAudio } from std.amiga.audio;
import { Timer, openTimer } from std.amiga.timer;
import { slice } from std.mem;

func main(): i32 {
    let data: u8[4] = [1, 2, 3, 4];
    let sample: Sample? = newSample(slice(data, 0, 4), 428, 64, 1);
    if (sample == null) {
        return 1;
    }
    let audio: Audio? = openAudio(1);
    if (audio == null) {
        return 2;
    }
    let timer: Timer? = openTimer();
    if (timer == null) {
        return 3;
    }
    return 0;
}
"#;

/// Every owned handle drops behind a null guard in reverse declaration order.
#[test]
fn every_audio_drop_is_null_guarded() {
    let c = emit_hosted_amiga("drop", DROP_PROGRAM);
    let start = c
        .find("cl_impl_audio_drop_main(void)\n{")
        .expect("the emitted C must contain the program entry definition");
    let body = &c[start..];
    let end = body
        .find("\n}\n")
        .map(|offset| offset + 3)
        .unwrap_or(body.len());
    let body = &body[..end];
    for (call, guarded) in [
        (
            "cl_timer_free(timer)",
            "if (timer) { cl_timer_free(timer); }",
        ),
        (
            "cl_audio_free(audio)",
            "if (audio) { cl_audio_free(audio); }",
        ),
        (
            "cl_sample_free(sample)",
            "if (sample) { cl_sample_free(sample); }",
        ),
    ] {
        assert!(
            body.contains(guarded),
            "the emitted main must drop the handle behind `{guarded}`"
        );
        assert_eq!(
            body.matches(call).count(),
            body.matches(guarded).count(),
            "every `{call}` call in main must sit inside `{guarded}`"
        );
    }
    let position = |needle: &str| {
        body.rfind(needle)
            .unwrap_or_else(|| panic!("missing cleanup `{needle}` in main"))
    };
    let timer = position("if (timer) { cl_timer_free(timer); }");
    let audio = position("if (audio) { cl_audio_free(audio); }");
    let sample = position("if (sample) { cl_sample_free(sample); }");
    assert!(
        timer < audio && audio < sample,
        "the generated cleanup must run timer, audio, sample"
    );
}
