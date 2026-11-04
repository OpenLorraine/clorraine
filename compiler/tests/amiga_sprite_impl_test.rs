//! Sprite standard-library implementation tests.
//!
//! These cases inspect the C the compiler emits for a hosted-Amiga program that
//! uses `std.amiga.sprite`: the application surface lowers to the standard-
//! library wrappers, each wrapper body carries its raw shim call, and every
//! owned sprite/animation handle drops behind a null guard in reverse
//! declaration order.

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
            "clc-sprite-impl-{}-{tag}-{sequence}",
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

/// A program that uses one call from every part of the sprite surface.
const ROUTING_PROGRAM: &str = r#"module impl.sprite.routing;

import {
    Sprite, SpriteAnimation,
    newSprite, newAnimation, spriteShow, spriteHide, spriteAttach,
    enableCollisions, collisions, animationShow
} from std.amiga.sprite;

func main(): i32 {
    let even: Sprite? = newSprite(2, 32);
    if (even == null) {
        return 1;
    }
    let odd: Sprite? = newSprite(3, 32);
    if (odd == null) {
        return 2;
    }
    spriteShow(even);
    spriteShow(odd);
    if (!spriteAttach(even, odd)) {
        return 3;
    }
    enableCollisions();
    let bits: u32 = collisions();
    spriteHide(even);
    spriteHide(odd);

    let anim: SpriteAnimation? = newAnimation(4, 16, 4);
    if (anim == null) {
        return 4;
    }
    animationShow(anim);
    return 0;
}
"#;

/// Every sprite call sits above `ffi` and reaches the shim.
#[test]
fn every_sprite_call_sits_above_ffi() {
    let c = emit_hosted_amiga("routing", ROUTING_PROGRAM);
    for wrapper_call in [
        "cl_std_amiga_sprite_newSprite(2u, 32u)",
        "cl_std_amiga_sprite_spriteShow(even)",
        "cl_std_amiga_sprite_spriteAttach(even, odd)",
        "cl_std_amiga_sprite_enableCollisions(",
        "cl_std_amiga_sprite_collisions(",
        "cl_std_amiga_sprite_newAnimation(4u, 16u, 4u)",
        "cl_std_amiga_sprite_animationShow(anim)",
    ] {
        assert!(
            c.contains(wrapper_call),
            "the application must call the wrapper `{wrapper_call}`"
        );
    }
    for shim_call in [
        "cl_sprite_new(channel",
        "cl_sprite_show(sprite)",
        "cl_sprite_attach(even",
        "cl_sprite_enable_collisions(",
        "cl_sprite_collisions(",
        "cl_sprite_animation_new(channel",
        "cl_sprite_animation_show(animation)",
    ] {
        assert!(
            c.contains(shim_call),
            "a wrapper body must call the shim `{shim_call}`"
        );
    }
}

/// A program that relies on generated drops for every owned handle.
const DROP_PROGRAM: &str = r#"module impl.sprite.drop;

import { Sprite, SpriteAnimation, newSprite, newAnimation } from std.amiga.sprite;

func main(): i32 {
    let even: Sprite? = newSprite(2, 32);
    if (even == null) {
        return 1;
    }
    let odd: Sprite? = newSprite(3, 32);
    if (odd == null) {
        return 2;
    }
    let anim: SpriteAnimation? = newAnimation(4, 16, 4);
    if (anim == null) {
        return 3;
    }
    return 0;
}
"#;

/// Every owned handle drops behind a null guard in reverse declaration order.
#[test]
fn every_sprite_drop_is_null_guarded() {
    let c = emit_hosted_amiga("drop", DROP_PROGRAM);
    let start = c
        .find("cl_impl_sprite_drop_main(void)\n{")
        .expect("the emitted C must contain the program entry definition");
    let body = &c[start..];
    let end = body
        .find("\n}\n")
        .map(|offset| offset + 3)
        .unwrap_or(body.len());
    let body = &body[..end];
    for (call, guarded) in [
        (
            "cl_sprite_animation_free(anim)",
            "if (anim) { cl_sprite_animation_free(anim); }",
        ),
        (
            "cl_sprite_free(odd)",
            "if (odd) { cl_sprite_free(odd); }",
        ),
        (
            "cl_sprite_free(even)",
            "if (even) { cl_sprite_free(even); }",
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
    let anim = position("if (anim) { cl_sprite_animation_free(anim); }");
    let odd = position("if (odd) { cl_sprite_free(odd); }");
    let even = position("if (even) { cl_sprite_free(even); }");
    assert!(
        anim < odd && odd < even,
        "the generated cleanup must run animation, odd, even"
    );
}
