//! Implementation tests for the hosted-amiga shim paths that used to crash or
//! hang a real machine.
//!
//! The specification cases run the probes under `vamos` and on emulated
//! Amigas. These cases pin the shim internals that make them pass: where each
//! entry opens the library it needs, and the order of the calls that keep a
//! safe program away from a missing library base.

use std::path::Path;

/// One shim source file, read directly so each contract is pinned where it is
/// implemented.
fn shim(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime").join(file);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// The text of the function whose definition starts with `signature`, up to
/// its closing brace in column 0.
fn body<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source
        .find(&format!("\n{signature}\n{{"))
        .unwrap_or_else(|| panic!("the shim has no definition {signature:?}"))
        + 1;
    let rest = &source[start..];
    let end = rest
        .find("\n}\n")
        .unwrap_or_else(|| panic!("{signature:?} has no closing brace"));
    &rest[..end + 2]
}

/// Asserts that each needle appears, in the given order.
fn assert_order(tag: &str, haystack: &str, needles: &[&str]) {
    let mut from = 0;
    for needle in needles {
        let at = haystack[from..]
            .find(needle)
            .unwrap_or_else(|| panic!("{tag}: missing {needle:?} after offset {from} in:\n{haystack}"));
        from += at + needle.len();
    }
}

/// The Intuition open both entries use: the shared recorded open, so the exit
/// hook closes a base opened here like one opened by a screen or window.
const OPEN_INTUITION: &str = "cl_lib_open(&IntuitionBase, \"intuition.library\", 33)";

/// `showAlert` checks its body first, so a null or over-long body still opens
/// nothing, then opens Intuition before it allocates the alert message and
/// returns with no alert when the library cannot be opened.
#[test]
fn the_alert_opens_intuition_after_the_body_checks_and_before_the_alert() {
    let source = shim("shim_hosted_amiga.c");
    let alert = body(&source, "void cl_show_alert(char *body)");
    assert_order(
        "alert",
        alert,
        &[
            "if (body == NULL)",
            "length > CL_WINDOW_BODY_MAX",
            &format!("if ({OPEN_INTUITION} == NULL) {{\n        return;\n    }}"),
            "AllocMem(CL_ALERT_MESSAGE_MAX, MEMF_CLEAR)",
            "DisplayAlert(1, (STRPTR)message, 20)",
            "FreeMem(message, CL_ALERT_MESSAGE_MAX)",
        ],
    );
}

/// `savePublicScreen` checks its arguments first, then opens Intuition and
/// returns false unless the library is V36 or newer, because public screens
/// are a 2.0 feature; only then does it lock the screen.
#[test]
fn the_public_screen_capture_opens_intuition_v36_before_locking_the_screen() {
    let source = shim("shim_hosted_amiga_20.c");
    let capture = body(&source, "BOOL cl_save_public_screen(char *name, char *path)");
    assert_order(
        "pubscreen",
        capture,
        &[
            "if (name == NULL || path == NULL)",
            &format!("if ({OPEN_INTUITION} == NULL ||"),
            "IntuitionBase->LibNode.lib_Version < 36) {\n        return FALSE;\n    }",
            "LockPubScreen((STRPTR)name)",
            "cl_screen_save_native((void *)screen, path)",
            "UnlockPubScreen(NULL, screen)",
        ],
    );
}

/// The clipboard handle opens its unit for one operation at a time: the open
/// proves the unit exists and closes it again, each write and read opens the
/// unit before its first request and closes it on every path after its last,
/// and closing the handle frees it without a device close.
#[test]
fn the_clipboard_opens_its_unit_only_for_each_operation() {
    let source = shim("shim_hosted_amiga_optional_services.c");
    let open = body(&source, "void *cl_clipboard_open(LONG unit)");
    assert_order(
        "open",
        open,
        &[
            "clipboard->unit = unit;",
            "mn_ReplyPort = clipboard->port;",
            "if (!cl_clipboard_attach(clipboard)) {",
            "cl_clipboard_detach(clipboard);",
            "cl_res_add(&clipboard->res, cl_clipboard_close, clipboard)",
        ],
    );
    let close = body(&source, "void cl_clipboard_close(void *handle)");
    assert!(
        !close.contains("CloseDevice") && !close.contains("cl_clipboard_detach"),
        "close: the unit is not open between operations:\n{close}"
    );
    for (signature, first, last) in [
        (
            "BOOL cl_clipboard_write(void *handle, APTR data, ULONG length)",
            "io_Command = CMD_WRITE;",
            "io_Command = CMD_UPDATE;",
        ),
        (
            "LONG cl_clipboard_read(void *handle, APTR data, ULONG length)",
            "io_Command = CBD_CURRENTREADID;",
            "io_Command = CMD_READ;",
        ),
    ] {
        let operation = body(&source, signature);
        assert_order(
            signature,
            operation,
            &["if (!cl_clipboard_attach(clipboard))", first, last, "cl_clipboard_detach(clipboard);"],
        );
        // Every exit after a successful attach leaves through the single
        // detach: the retry loop breaks instead of returning.
        let from_loop = &operation[operation.find("for (attempts = 0;").unwrap()..];
        let loop_part = &from_loop[..from_loop.find("cl_clipboard_detach(clipboard);").unwrap()];
        assert!(
            !loop_part.contains("return"),
            "{signature}: a return between attach and detach leaves the unit open:\n{operation}"
        );
        assert_eq!(operation.matches("cl_clipboard_detach").count(), 1, "{signature}: one detach");
    }
}

/// A read is one bounded request from the start of the current clip. It never
/// reads on toward the end of the clip, never reads without a buffer, and
/// never asks for zero bytes: on the 2.0 to 3.1 devices a read that returns
/// no data holds off every later write on the unit.
#[test]
fn the_clipboard_read_is_one_bounded_request_from_the_start() {
    let source = shim("shim_hosted_amiga_optional_services.c");
    let read = body(&source, "LONG cl_clipboard_read(void *handle, APTR data, ULONG length)");
    assert_eq!(read.matches("io_Command = CMD_READ;").count(), 1, "one read request");
    assert_order(
        "read",
        read,
        &[
            "io_Data = (STRPTR)data;",
            "io_Length = length;",
            "io_Offset = 0;",
            "got = (LONG)clipboard->request->io_Actual;",
            "io_Error != CBERR_OBSOLETEID",
        ],
    );
    assert!(!read.contains("io_Data = NULL"), "no read without a buffer:\n{read}");
    // An empty buffer returns 0 before the unit is opened: a request for no
    // bytes is itself a read that returns no data.
    assert_order(
        "read",
        read,
        &["if (length == 0) {\n        return 0;\n    }", "if (!cl_clipboard_attach(clipboard))"],
    );
    assert!(!source.contains("0x7FFFFFFF"), "no skip read to the end of the clip");
}

/// The registry walk reads each record's successor before it visits the
/// record, so a visitor may remove it, and it stays a real function so the
/// release-image symbol check can see which programs link it.
#[test]
fn the_registry_walk_reads_the_successor_before_each_visit() {
    let source = shim("shim_hosted_amiga.c");
    let walk = body(
        &source,
        "CL_NOINLINE void cl_res_each(void (*release)(void *handle),\n                 void (*visit)(void *handle, void *context), void *context)",
    );
    assert_order(
        "walk",
        walk,
        &[
            "for (record = cl_res_head; record != NULL; record = next) {",
            "next = record->next;",
            "if (record->release == release) {",
            "visit(record->handle, context);",
        ],
    );
    assert_order(
        "noinline",
        &source,
        &["#if defined(__GNUC__)", "#define CL_NOINLINE __attribute__((noinline))", "#else", "#define CL_NOINLINE\n#endif"],
    );
}

/// Freeing a layer, by the program or at exit, first ends an update the
/// program left open (without completing the redraw), then removes the clip
/// region the program installed, and only then deletes the layer.
#[test]
fn a_freed_layer_ends_its_open_update_and_unclips_before_deletion() {
    let source = shim("shim_hosted_amiga.c");
    let free = body(&source, "void cl_layer_free(void *handle)");
    assert_order(
        "free",
        free,
        &[
            "cl_res_remove(&layer->res);",
            "if (layer->updating) {",
            "EndUpdate(layer->layer, FALSE);",
            "layer->updating = 0;",
            "cl_layer_unclip(layer);",
            "deleted = DeleteLayer(0, layer->layer);",
        ],
    );
    let unclip = body(&source, "static void cl_layer_unclip(cl_layer *layer)");
    assert_order(
        "unclip",
        unclip,
        &["if (layer->layer->ClipRegion != NULL) {", "InstallClipRegion(layer->layer, NULL)"],
    );
    assert!(!unclip.contains("EndUpdate"), "unclipping never ends an update:\n{unclip}");
}

/// The update flag is set on every layerBeginUpdate, before BeginUpdate runs
/// and whatever it returns; layerEndUpdate still completes the redraw with
/// EndUpdate(TRUE) and then clears the flag.
#[test]
fn the_update_flag_follows_the_programs_begin_and_end_calls() {
    let source = shim("shim_hosted_amiga.c");
    let begin = body(&source, "BOOL cl_layer_begin_update(void *handle)");
    assert_order(
        "begin",
        begin,
        &["layer->updating = 1;", "return BeginUpdate(layer->layer) ? TRUE : FALSE;"],
    );
    let end = body(&source, "void cl_layer_end_update(void *handle)");
    assert_order("end", end, &["EndUpdate(layer->layer, TRUE);", "layer->updating = 0;"]);
    assert!(!end.contains("FALSE"), "the program's end completes the redraw:\n{end}");
}

/// Freeing a region unclips every live layer that has it installed before it
/// disposes the region; it leaves an open update alone, and the program's own
/// set and clear calls are unchanged.
#[test]
fn a_freed_region_unclips_every_layer_that_holds_it_first() {
    let source = shim("shim_hosted_amiga_optional.c");
    let free = body(&source, "void cl_free_region(void *handle)");
    assert_order(
        "free",
        free,
        &[
            "cl_res_remove(&region->res);",
            "cl_res_each(cl_layer_free, cl_region_unclip_layer, region->region);",
            "DisposeRegion(region->region);",
        ],
    );
    let visit = body(&source, "static void cl_region_unclip_layer(void *layer_handle, void *region)");
    assert_order(
        "visit",
        visit,
        &["layer->layer->ClipRegion == (struct Region *)region", "cl_layer_unclip(layer);"],
    );
    assert!(!visit.contains("EndUpdate"), "the region walk leaves an update open:\n{visit}");
    let set = body(&source, "void cl_layer_set_clip(void *layer_handle, void *region_handle)");
    assert!(
        set.contains("(void)InstallClipRegion(layer->layer, region->region);") && !set.contains("EndUpdate"),
        "set clip is unchanged:\n{set}"
    );
    let clear = body(&source, "void cl_layer_clear_clip(void *layer_handle)");
    assert!(
        clear.contains("(void)InstallClipRegion(layer->layer, NULL);") && !clear.contains("EndUpdate"),
        "clear clip is unchanged:\n{clear}"
    );
}

/// Each guide message records its guide when it is received. Closing the
/// guide replies the messages of that guide the program still holds, clears
/// their message and guide, and only then closes the context; the orphaned
/// handles stay registered, so their getters read 0 and their reply only
/// frees them.
#[test]
fn closing_a_guide_replies_and_orphans_its_held_messages_first() {
    let source = shim("shim_hosted_amiga_optional_net.c");
    let wait = body(&source, "void *cl_guide_wait(void *handle_void)");
    assert_order(
        "wait",
        wait,
        &["message->message = raw;", "message->guide = guide;", "cl_res_add(&message->res, cl_guide_reply, message)"],
    );
    let close = body(&source, "void cl_guide_close(void *handle_void)");
    assert_order(
        "close",
        close,
        &[
            "cl_res_remove(&guide->res);",
            "cl_res_each(cl_guide_reply, cl_guide_reply_held, guide);",
            "CloseAmigaGuide(guide->context);",
        ],
    );
    let held = body(&source, "static void cl_guide_reply_held(void *message_void, void *guide)");
    assert_order(
        "held",
        held,
        &[
            "message->guide == (cl_guide *)guide && message->message != NULL",
            "ReplyAmigaGuideMsg(message->message);",
            "message->message = NULL;",
            "message->guide = NULL;",
        ],
    );
    assert!(!held.contains("cl_res_remove"), "an orphan stays registered:\n{held}");
    let reply = body(&source, "void cl_guide_reply(void *handle_void)");
    assert_order(
        "reply",
        reply,
        &["if (message->message != NULL) {", "ReplyAmigaGuideMsg(message->message);", "FreeMem(message, (ULONG)sizeof *message);"],
    );
}

/// Screen text reads the font's metrics after SetFont, rejects out-of-range
/// and empty calls before it measures, and draws text that fits with exactly
/// the old Move/Text sequence; only a partly visible plain font takes the clip
/// path, and the screen font is restored on every path.
#[test]
fn screen_text_measures_after_set_font_and_keeps_the_old_path_for_text_that_fits() {
    let source = shim("shim_hosted_amiga.c");
    let draw = body(
        &source,
        "void cl_text_draw(void *screen_handle, void *font_handle, LONG x, LONG y,\n                  UBYTE pen, APTR text, ULONG count)",
    );
    assert_order(
        "draw",
        draw,
        &[
            "(void)SetFont(rast_port, font->font);",
            "SetAPen(rast_port, (ULONG)pen);",
            "face = rast_port->Font;",
            "x < -32768 || x > 32767 || y < -32768 || y > 32767 ||",
            "count == 0 || count > 32767",
            "(void)SetFont(rast_port, old_font);\n        return;",
            "width = (LONG)screen->Width;",
            "(LONG)rast_port->BitMap->BytesPerRow * 8 < width",
            "height = (LONG)screen->Height;",
            "(LONG)rast_port->BitMap->Rows < height",
            "top = y - (LONG)face->tf_Baseline;",
            "bottom = top + (LONG)face->tf_YSize - 1;",
            "cl_text_box(face, rast_port, (const UBYTE *)text, count, x, width,",
            "span.left >= 0 && span.right <= width - 1 && top >= 0 &&",
            "bottom <= height - 1) {\n            Move(rast_port, x, y);\n            (void)Text(rast_port, (STRPTR)text, count);",
            "(face->tf_Style & FSF_COLORFONT) == 0",
            "cl_text_draw_clipped(rast_port, top, width, height,",
            "(void)SetFont(rast_port, old_font);\n}",
        ],
    );
}

/// The box walk: the default glyph for codes outside the font, kern and
/// space defaults, the advance exactly as the signed tables give it (a
/// right-to-left font's tables already point left), every point the pen
/// passes (after the kern, after the space, after the extra spacing), the
/// kerned glyph and the XSize cell, all in LONG, with the pen bounded so it
/// cannot overflow.
#[test]
fn the_text_box_contains_every_column_text_can_touch() {
    let source = shim("shim_hosted_amiga.c");
    assert!(!source.contains("advance = -advance"), "the walk must not reverse the signed advance");
    let walk = body(
        &source,
        "static BOOL cl_text_box(const struct TextFont *font, const struct RastPort *rp,\n                        const UBYTE *text, ULONG count, LONG x, LONG width,\n                        cl_text_span *span)",
    );
    assert_order(
        "walk",
        walk,
        &[
            "if (code >= font->tf_LoChar && code <= font->tf_HiChar) {",
            "index = code - font->tf_LoChar;",
            "index = (ULONG)(font->tf_HiChar - font->tf_LoChar) + 1UL;",
            "kern = kerns != NULL ? (LONG)kerns[index] : 0;",
            "space = spaces != NULL ? (LONG)spaces[index] : (LONG)font->tf_XSize;",
            "glyph = locs != NULL ? (LONG)locs[index * 2 + 1] : (LONG)font->tf_XSize;",
            "advance = kern + space + (LONG)rp->TxSpacing;",
            "left = pen;",
            "if (pen + kern < left) {",
            "if (pen + kern + space < left) {",
            "if (pen + advance < left) {",
            "right = pen + kern + glyph;",
            "if (pen + kern + space > right) {",
            "if (pen + advance > right) {",
            "if (pen + (LONG)font->tf_XSize > right) {",
            "right -= 1;",
            "if (right >= 0 && left <= width - 1) {",
            "pen += advance;",
            "if (pen > CL_TEXT_PEN_LIMIT || pen < -CL_TEXT_PEN_LIMIT) {\n            return FALSE;",
        ],
    );
}

/// The clip path: the raster rules, both chip planes cleared and checked,
/// Text() of the run with pens 3 and 2 in the screen's mode and spacing, the
/// two template blits (or one complement blit) over the visible part only,
/// the screen mode restored, and WaitBlit before the planes are freed.
#[test]
fn the_text_clip_renders_the_run_off_screen_and_blits_only_the_visible_part() {
    let source = shim("shim_hosted_amiga.c");
    let clip = body(
        &source,
        "CL_NOINLINE void cl_text_draw_clipped(struct RastPort *screen_rp, LONG top,\n                                      LONG width, LONG height,\n                                      const UBYTE *text,\n                                      const cl_text_span *span)",
    );
    assert_order(
        "clip",
        clip,
        &[
            "raster_width > width + 512 ||",
            "GfxBase->LibNode.lib_Version < 36 && raster_width > 1008",
            "memset(&raster, 0, sizeof raster);",
            "InitBitMap(&raster, 2, raster_width, raster_height);",
            "AllocMem(plane_size, MEMF_CHIP | MEMF_CLEAR);",
            "AllocMem(plane_size, MEMF_CHIP | MEMF_CLEAR);",
            "if (raster.Planes[0] == NULL || raster.Planes[1] == NULL) {",
            "InitRastPort(&temp);",
            "(void)SetFont(&temp, font);",
            "SetDrMd(&temp, (ULONG)mode);",
            "temp.TxSpacing = screen_rp->TxSpacing;",
            "SetAPen(&temp, 3);",
            "SetBPen(&temp, 2);",
            "Move(&temp, span->run_pen - span->run_left, (LONG)font->tf_Baseline);",
            "(void)Text(&temp, (STRPTR)(text + span->first), span->count);",
            "if (mode & COMPLEMENT) {",
            "SetDrMd(screen_rp, COMPLEMENT);",
            "BltTemplate(raster.Planes[0] + offset",
            "SetDrMd(screen_rp, JAM1);",
            "SetAPen(screen_rp, (ULONG)bpen);",
            "BltTemplate(raster.Planes[1] + offset",
            "SetAPen(screen_rp, (ULONG)apen);",
            "BltTemplate(raster.Planes[0] + offset",
            "SetDrMd(screen_rp, (ULONG)mode);",
            "WaitBlit();",
            "FreeMem(raster.Planes[0], plane_size);",
            "FreeMem(raster.Planes[1], plane_size);",
        ],
    );
}

/// Only the two screen and window constructors and the two entries that can
/// run first open Intuition; every other Intuition user receives a screen or
/// window handle, so the library is already open when it runs.
#[test]
fn intuition_is_opened_only_where_it_may_not_be_open_yet() {
    let base = shim("shim_hosted_amiga.c");
    let suffix = shim("shim_hosted_amiga_20.c");
    assert_eq!(base.matches(OPEN_INTUITION).count(), 3, "base shim opens");
    assert_eq!(suffix.matches(OPEN_INTUITION).count(), 1, "2.0 suffix opens");
}
