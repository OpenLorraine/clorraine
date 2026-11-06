//! Embedded standard-library surface and sources.
//!
//! The standard library is ordinary Clorraine that the compiler compiles with
//! every program. Loading and name resolution need one fact before those
//! sources can be compiled: which modules exist and which names each one
//! exports. The module bodies themselves are embedded in the binary with
//! `include_str!`, so no install path or filesystem lookup is needed and the
//! emitted translation unit is deterministic.
//!
//! A module that has not been ported yet still contributes its export surface
//! here; the driver only injects the modules that have a source.

/// One standard-library module and the names it exports.
pub struct StdlibModule {
    /// Dotted module name, for example `std.mem`.
    pub name: &'static str,
    /// Names the module exports, in the order the library declares them.
    pub exports: &'static [&'static str],
}

/// One embedded standard-library module and its Clorraine source.
pub struct StdlibSource {
    /// Dotted module name, matching the module declaration in the source.
    pub name: &'static str,
    /// The module body, embedded at build time.
    pub source: &'static str,
}

/// The standard-library modules whose Clorraine sources are embedded.
///
/// `include_str!` reads the file at build time relative to this module, so the
/// path is `<repo>/stdlib/<name>.clo`. Adding a module here makes it available
/// to the driver; a module without an entry stays a declarations-only surface
/// described by [`MODULES`].
pub const SOURCES: &[StdlibSource] = &[
    StdlibSource {
        name: "std.mem",
        source: include_str!("../../stdlib/std.mem.clo"),
    },
    StdlibSource {
        name: "std.text",
        source: include_str!("../../stdlib/std.text.clo"),
    },
    StdlibSource {
        name: "std.fmt",
        source: include_str!("../../stdlib/std.fmt.clo"),
    },
    StdlibSource {
        name: "std.io",
        source: include_str!("../../stdlib/std.io.clo"),
    },
    StdlibSource {
        name: "std.math",
        source: include_str!("../../stdlib/std.math.clo"),
    },
    StdlibSource {
        name: "std.iff",
        source: include_str!("../../stdlib/std.iff.clo"),
    },
    StdlibSource {
        name: "std.amiga.video",
        source: include_str!("../../stdlib/std.amiga.video.clo"),
    },
    StdlibSource {
        name: "std.amiga.ilbm",
        source: include_str!("../../stdlib/std.amiga.ilbm.clo"),
    },
    StdlibSource {
        name: "std.amiga.sprite",
        source: include_str!("../../stdlib/std.amiga.sprite.clo"),
    },
    StdlibSource {
        name: "std.amiga.audio",
        source: include_str!("../../stdlib/std.amiga.audio.clo"),
    },
    StdlibSource {
        name: "std.amiga.input",
        source: include_str!("../../stdlib/std.amiga.input.clo"),
    },
    StdlibSource {
        name: "std.amiga.timer",
        source: include_str!("../../stdlib/std.amiga.timer.clo"),
    },
    StdlibSource {
        name: "std.amiga.text",
        source: include_str!("../../stdlib/std.amiga.text.clo"),
    },
    StdlibSource {
        name: "std.amiga.layers",
        source: include_str!("../../stdlib/std.amiga.layers.clo"),
    },
    StdlibSource {
        name: "std.amiga.dos",
        source: include_str!("../../stdlib/std.amiga.dos.clo"),
    },
    StdlibSource {
        name: "std.amiga.interrupt",
        source: include_str!("../../stdlib/std.amiga.interrupt.clo"),
    },
    StdlibSource {
        name: "std.amiga.intuition",
        source: include_str!("../../stdlib/std.amiga.intuition.clo"),
    },
    StdlibSource {
        name: "std.amiga.graphics",
        source: include_str!("../../stdlib/std.amiga.graphics.clo"),
    },
    StdlibSource {
        name: "std.amiga.custom",
        source: include_str!("../../stdlib/std.amiga.custom.clo"),
    },
    StdlibSource {
        name: "std.amiga.exec",
        source: include_str!("../../stdlib/std.amiga.exec.clo"),
    },
    StdlibSource {
        name: "std.amiga.icon",
        source: include_str!("../../stdlib/std.amiga.icon.clo"),
    },
    StdlibSource {
        name: "std.amiga.workbench",
        source: include_str!("../../stdlib/std.amiga.workbench.clo"),
    },
    StdlibSource {
        name: "std.amiga.expansion",
        source: include_str!("../../stdlib/std.amiga.expansion.clo"),
    },
    StdlibSource {
        name: "std.amiga.semaphore",
        source: include_str!("../../stdlib/std.amiga.semaphore.clo"),
    },
    StdlibSource {
        name: "std.amiga.utility",
        source: include_str!("../../stdlib/std.amiga.utility.clo"),
    },
    StdlibSource {
        name: "std.amiga.pool",
        source: include_str!("../../stdlib/std.amiga.pool.clo"),
    },
    StdlibSource {
        name: "std.amiga.args",
        source: include_str!("../../stdlib/std.amiga.args.clo"),
    },
    StdlibSource {
        name: "std.amiga.datatypes",
        source: include_str!("../../stdlib/std.amiga.datatypes.clo"),
    },
    StdlibSource {
        name: "std.amiga.regions",
        source: include_str!("../../stdlib/std.amiga.regions.clo"),
    },
    StdlibSource {
        name: "std.amiga.eclock",
        source: include_str!("../../stdlib/std.amiga.eclock.clo"),
    },
    StdlibSource {
        name: "std.amiga.lowlevel",
        source: include_str!("../../stdlib/std.amiga.lowlevel.clo"),
    },
    StdlibSource {
        name: "std.amiga.gadtools",
        source: include_str!("../../stdlib/std.amiga.gadtools.clo"),
    },
    StdlibSource {
        name: "std.amiga.asl",
        source: include_str!("../../stdlib/std.amiga.asl.clo"),
    },
    StdlibSource {
        name: "std.amiga.boopsi",
        source: include_str!("../../stdlib/std.amiga.boopsi.clo"),
    },
    StdlibSource {
        name: "std.amiga.commodities",
        source: include_str!("../../stdlib/std.amiga.commodities.clo"),
    },
    StdlibSource {
        name: "std.amiga.locale",
        source: include_str!("../../stdlib/std.amiga.locale.clo"),
    },
    StdlibSource {
        name: "std.amiga.arexx",
        source: include_str!("../../stdlib/std.amiga.arexx.clo"),
    },
    StdlibSource {
        name: "std.amiga.clipboard",
        source: include_str!("../../stdlib/std.amiga.clipboard.clo"),
    },
    StdlibSource {
        name: "std.amiga.devices",
        source: include_str!("../../stdlib/std.amiga.devices.clo"),
    },
    StdlibSource {
        name: "std.amiga.nonvolatile",
        source: include_str!("../../stdlib/std.amiga.nonvolatile.clo"),
    },
    StdlibSource {
        name: "std.amiga.bsdsocket",
        source: include_str!("../../stdlib/std.amiga.bsdsocket.clo"),
    },
    StdlibSource {
        name: "std.amiga.amigaguide",
        source: include_str!("../../stdlib/std.amiga.amigaguide.clo"),
    },
    StdlibSource {
        name: "std.amiga.picasso96",
        source: include_str!("../../stdlib/std.amiga.picasso96.clo"),
    },
];

/// The compiler-known intrinsic helper names.
///
/// These helpers are implemented by the compiler and declared by the embedded
/// standard library. A user module may not declare an `intrinsic func` with one
/// of these names.
pub const COMPILER_INTRINSICS: &[&str] = &["rawSlice", "rawStr", "slice", "len"];

/// True when `name` is a compiler-known intrinsic helper.
pub fn is_compiler_intrinsic(name: &str) -> bool {
    COMPILER_INTRINSICS.contains(&name)
}

/// The embedded source of a standard-library module, when it has one.
pub fn source(name: &str) -> Option<&'static str> {
    SOURCES
        .iter()
        .find(|module| module.name == name)
        .map(|module| module.source)
}

/// True when the module is a compiler-known standard-library module.
///
/// The visibility rules use this to grant the module's export surface even
/// where a generic declaration omits `export`.
pub fn is_standard_module(name: &str) -> bool {
    lookup(name).is_some()
}

/// Every standard-library module the compiler provides.
///
/// The export lists mirror the standard-library signatures: a handle or struct
/// name, a constant, or a function name. Overloads share one name, so `text`
/// appears once even though it has one overload per argument type.
pub const MODULES: &[StdlibModule] = &[
    StdlibModule {
        name: "std.mem",
        exports: &[
            "Buffer",
            "Status",
            "Span",
            "bufferNew",
            "bufferFrom",
            "bufferBytes",
            "bufferLen",
            "slice",
            "spanOf",
            "spanBytes",
            "Array",
            "arrayNew",
            "push",
            "get",
            "set",
            "reserve",
        ],
    },
    StdlibModule {
        name: "std.text",
        exports: &["String", "stringOf", "clone", "bytes", "toCString"],
    },
    StdlibModule {
        name: "std.fmt",
        exports: &["text", "toHex", "toDecimal", "padLeft", "padRight"],
    },
    StdlibModule {
        name: "std.io",
        exports: &[
            "File",
            "MODE_OLDFILE",
            "MODE_NEWFILE",
            "openFile",
            "createFile",
            "readFile",
            "writeBytes",
            "print",
            "println",
        ],
    },
    StdlibModule {
        name: "std.math",
        exports: &["sin", "cos", "sqrt"],
    },
    StdlibModule {
        name: "std.iff",
        exports: &[
            "IlbmInfo",
            "ilbmInfo",
            "ilbmDecode",
            "ilbmPalette",
            "ilbmPixelHash",
        ],
    },
    StdlibModule {
        name: "std.amiga.ilbm",
        exports: &["IlbmImage", "loadIlbm"],
    },
    StdlibModule {
        name: "std.amiga.video",
        exports: &[
            "ChipBuffer",
            "Bitmap",
            "Copper",
            "Display",
            "allocChip",
            "freeChip",
            "chipSize",
            "chipRead8",
            "chipWrite8",
            "chipRead16",
            "chipWrite16",
            "chipRead32",
            "chipWrite32",
            "newBitmap",
            "freeBitmap",
            "bitmapWidth",
            "bitmapHeight",
            "bitmapDepth",
            "bitmapBytesPerRow",
            "bitmapClear",
            "bitmapPlot",
            "bitmapLine",
            "bitmapFill",
            "bitmapPlaneBit",
            "bitmapSetPlaneBit",
            "bitmapBlit",
            "bitmapBlitMasked",
            "bitmapChecksum",
            "bitmapFromIndices",
            "newCopper",
            "freeCopper",
            "copperWait",
            "copperMove",
            "copperColor",
            "copperEnd",
            "openDisplay",
            "freeDisplay",
            "displaySetColor",
            "displaySetCopper",
            "displayFlip",
            "displayFailed",
            "displayIndex",
            "waitVBlank",
        ],
    },
    StdlibModule {
        name: "std.amiga.sprite",
        exports: &[
            "Sprite",
            "SpriteAnimation",
            "newSprite",
            "freeSprite",
            "spriteSetPixels",
            "spriteMove",
            "spriteShow",
            "spriteHide",
            "spriteAttach",
            "spriteAttachScreen",
            "spriteDetach",
            "spriteChannel",
            "spriteHeight",
            "spriteX",
            "spriteY",
            "spriteVisible",
            "enableCollisions",
            "collisions",
            "newAnimation",
            "freeAnimation",
            "animationSetFrame",
            "animationShowFrame",
            "animationNext",
            "animationMove",
            "animationShow",
            "animationHide",
            "animationFrame",
            "animationAttachScreen",
            "animationDetach",
        ],
    },
    StdlibModule {
        name: "std.amiga.audio",
        exports: &[
            "Audio",
            "Sample",
            "openAudio",
            "freeAudio",
            "audioChannels",
            "audioPlay",
            "audioStop",
            "audioControl",
            "newSample",
            "freeSample",
            "sampleLength",
            "samplePeriod",
            "sampleVolume",
            "sampleCycles",
        ],
    },
    StdlibModule {
        name: "std.amiga.input",
        exports: &[
            "JOY_UP",
            "JOY_DOWN",
            "JOY_LEFT",
            "JOY_RIGHT",
            "JOY_FIRE",
            "readJoystick",
            "decodeJoystick",
        ],
    },
    StdlibModule {
        name: "std.amiga.timer",
        exports: &["Timer", "openTimer", "freeTimer", "timerWait"],
    },
    StdlibModule {
        name: "std.amiga.text",
        exports: &[
            "Font",
            "openFont",
            "openDiskFont",
            "defaultFont",
            "freeFont",
            "textWidth",
            "textHeight",
            "drawText",
            "windowText",
        ],
    },
    StdlibModule {
        name: "std.amiga.layers",
        exports: &[
            "Layer",
            "newLayer",
            "freeLayer",
            "layerMove",
            "layerSize",
            "layerFill",
            "layerClear",
            "layerText",
            "layerBeginUpdate",
            "layerEndUpdate",
        ],
    },
    StdlibModule {
        name: "std.amiga.dos",
        exports: &[
            "SEEK_SET",
            "SEEK_CUR",
            "SEEK_END",
            "Lock",
            "PathInfo",
            "VolumeInfo",
            "delayTicks",
            "ticks",
            "seekFile",
            "filePosition",
            "fileLength",
            "lockPath",
            "unlockPath",
            "pathInfo",
            "pathInfoIsDir",
            "pathInfoSize",
            "pathInfoName",
            "volumeInfo",
            "volumeName",
            "volumeFreeBytes",
            "volumeUsedBytes",
            "makeDir",
            "deleteFile",
            "renameFile",
            "setCurrentDir",
            "currentDirName",
            "parentDirName",
        ],
    },
    StdlibModule {
        name: "std.amiga.interrupt",
        exports: &["HandlerToken", "addHandler", "addPortHandler", "removeHandler"],
    },
    StdlibModule {
        name: "std.amiga.intuition",
        exports: &[
            "Screen",
            "openScreen",
            "saveScreen",
            "Window",
            "WindowEvent",
            "EVENT_NONE",
            "EVENT_CLOSE",
            "EVENT_GADGET",
            "EVENT_MENU",
            "EVENT_MOUSE",
            "EVENT_KEY",
            "EVENT_ACTIVE",
            "EVENT_INACTIVE",
            "openWindow",
            "closeWindow",
            "windowWidth",
            "windowHeight",
            "addButton",
            "addString",
            "windowString",
            "addMenu",
            "addMenuItem",
            "addMenuItemKey",
            "activateGadget",
            "askYesNo",
            "showAlert",
            "setPointer",
            "waitEvent",
            "pollEvent",
        ],
    },
    StdlibModule {
        name: "std.amiga.graphics",
        exports: &[
            "setColor",
            "fill",
            "draw",
            "clear",
            "windowFill",
            "windowDraw",
            "windowClear",
        ],
    },
    StdlibModule {
        name: "std.amiga.custom",
        exports: &["setDma", "setColor"],
    },
    StdlibModule {
        name: "std.amiga.exec",
        exports: &[
            "MsgPort",
            "NewMessage",
            "Message",
            "Task",
            "Library",
            "Device",
            "CMD_INVALID",
            "CMD_RESET",
            "CMD_READ",
            "CMD_WRITE",
            "CMD_UPDATE",
            "CMD_CLEAR",
            "CMD_STOP",
            "CMD_START",
            "CMD_FLUSH",
            "createMsgPort",
            "createPublicPort",
            "deleteMsgPort",
            "createMessage",
            "messagePayload",
            "putMsg",
            "signals",
            "waitSignals",
            "getMsg",
            "replyMsg",
            "freeMessage",
            "releaseMsg",
            "createTask",
            "reportTaskResult",
            "joinTask",
            "openLibrary",
            "closeLibrary",
            "libraryVersion",
            "libraryRevision",
            "openDevice",
            "openConsole",
            "closeDevice",
            "deviceCommand",
            "deviceSetBuffer",
            "deviceSetOffset",
            "deviceDoIO",
            "deviceSendIO",
            "deviceWaitIO",
            "deviceCheckIO",
            "deviceAbortIO",
            "deviceActual",
            "deviceError",
        ],
    },
    StdlibModule {
        name: "std.amiga.icon",
        exports: &[
            "DiskObject",
            "ICON_DISK",
            "ICON_DRAWER",
            "ICON_TOOL",
            "ICON_PROJECT",
            "ICON_GARBAGE",
            "ICON_DEVICE",
            "ICON_KICK",
            "ICON_APPICON",
            "getDiskObject",
            "putDiskObject",
            "freeDiskObject",
            "getDefDiskObject",
            "diskObjectType",
            "setDiskObjectType",
        ],
    },
    StdlibModule {
        name: "std.amiga.workbench",
        exports: &[
            "AppIcon",
            "AppWindow",
            "AppMenuItem",
            "AppMessage",
            "APPMSG_APPWINDOW",
            "APPMSG_APPICON",
            "APPMSG_APPMENUITEM",
            "APPMSG_OPEN",
            "addAppIcon",
            "removeAppIcon",
            "addAppWindow",
            "removeAppWindow",
            "addAppMenuItem",
            "removeAppMenuItem",
            "waitAppMessage",
            "appMessageType",
            "appMessageId",
            "appMessageUserData",
            "appMessageClass",
            "appMessageArgCount",
            "replyAppMessage",
            "savePublicScreen",
        ],
    },
    StdlibModule {
        name: "std.amiga.expansion",
        exports: &[
            "expansionBoardCount",
            "expansionBoardManufacturer",
            "expansionBoardProduct",
            "expansionBoardSerial",
            "expansionBoardKind",
            "cardPresent",
            "cardStatus",
            "mapRawKey",
        ],
    },
    StdlibModule {
        name: "std.amiga.semaphore",
        exports: &[
            "Semaphore",
            "createSemaphore",
            "freeSemaphore",
            "obtainSemaphore",
            "releaseSemaphore",
            "attemptSemaphore",
        ],
    },
    StdlibModule {
        name: "std.amiga.utility",
        exports: &[
            "TagList",
            "DateStamp",
            "Hook",
            "newTagList",
            "freeTagList",
            "tagSet",
            "tagGet",
            "tagPresent",
            "tagCount",
            "currentDateStamp",
            "freeDateStamp",
            "dateStampDays",
            "dateStampMinute",
            "dateStampTick",
            "newHook",
            "callHook",
            "hookData",
            "hookCurrentData",
            "hookSetResult",
            "freeHook",
        ],
    },
    StdlibModule {
        name: "std.amiga.pool",
        exports: &[
            "UtilityPool",
            "UtilityBlock",
            "createUtilityPool",
            "allocUtilityBlock",
            "utilityBlockSize",
            "writeUtilityBlock",
            "readUtilityBlock",
            "freeUtilityBlock",
            "freeUtilityPool",
        ],
    },
    StdlibModule {
        name: "std.amiga.args",
        exports: &[
            "Args",
            "readArgs",
            "readArgsFrom",
            "freeArgs",
            "argCount",
            "argPresent",
            "argNumber",
            "argText",
            "argSwitch",
        ],
    },
    StdlibModule {
        name: "std.amiga.datatypes",
        exports: &[
            "DTObject",
            "openPicture",
            "disposeDTObject",
            "dtWidth",
            "dtHeight",
            "dtDepth",
            "dtColors",
            "drawDTObject",
        ],
    },
    StdlibModule {
        name: "std.amiga.regions",
        exports: &[
            "Region",
            "RegionBounds",
            "newRegion",
            "freeRegion",
            "regionOrRect",
            "regionAndRect",
            "regionClearRect",
            "regionBounds",
            "regionIsEmpty",
            "layerSetClipRegion",
            "layerClearClipRegion",
        ],
    },
    StdlibModule {
        name: "std.amiga.eclock",
        exports: &[
            "EClock",
            "EClockTicks",
            "openEClock",
            "readEClock",
            "eclockFrequency",
            "freeEClock",
        ],
    },
    StdlibModule {
        name: "std.amiga.lowlevel",
        exports: &[
            "readJoyPort",
            "joyPortType",
            "decodeJoyPort",
            "readJoyPortDecoded",
            "lowlevelVersion",
        ],
    },
    StdlibModule {
        name: "std.amiga.gadtools",
        exports: &[
            "GadgetList",
            "newGadgetList",
            "addButton",
            "addText",
            "addString",
            "addInteger",
            "buildGadgets",
            "freeGadgetList",
            "gadgetWaitEvent",
            "gadgetPollEvent",
        ],
    },
    StdlibModule {
        name: "std.amiga.asl",
        exports: &[
            "FileRequester",
            "newFileRequester",
            "requestFile",
            "fileRequesterPath",
            "freeFileRequester",
        ],
    },
    StdlibModule {
        name: "std.amiga.boopsi",
        exports: &[
            "BoopsiObject",
            "newStringGadget",
            "attachObject",
            "detachObject",
            "setStringValue",
            "stringValue",
            "disposeObject",
        ],
    },
    StdlibModule {
        name: "std.amiga.commodities",
        exports: &["Commodity", "openCommodity", "closeCommodity"],
    },
    StdlibModule {
        name: "std.amiga.locale",
        exports: &[
            "Locale",
            "openLocale",
            "closeLocale",
            "localeLanguageName",
            "localeCountryCode",
            "localeGmtOffset",
            "localeDateTimeFormat",
        ],
    },
    StdlibModule {
        name: "std.amiga.arexx",
        exports: &[
            "RexxMessage",
            "createRexxMessage",
            "deleteRexxMessage",
            "setRexxAction",
            "setRexxArg",
            "sendRexxMessage",
            "receiveRexxMessage",
            "replyRexxMessage",
            "reclaimRexxMessage",
            "rexxMessageAction",
            "rexxMessageResult1",
            "rexxMessageResult2",
            "rexxMessageArg",
        ],
    },
    StdlibModule {
        name: "std.amiga.clipboard",
        exports: &[
            "Clipboard",
            "openClipboard",
            "closeClipboard",
            "clipboardWrite",
            "clipboardRead",
            "clipboardClear",
        ],
    },
    StdlibModule {
        name: "std.amiga.devices",
        exports: &[
            "Serial",
            "Parallel",
            "Printer",
            "openSerial",
            "closeSerial",
            "serialWrite",
            "serialRead",
            "openParallel",
            "closeParallel",
            "parallelWrite",
            "openPrinter",
            "closePrinter",
            "printerWrite",
        ],
    },
    StdlibModule {
        name: "std.amiga.nonvolatile",
        exports: &[
            "NonvolatileData",
            "getNonvolatile",
            "freeNonvolatileData",
            "nonvolatileSize",
            "nonvolatileRead",
            "storeNonvolatile",
            "deleteNonvolatile",
        ],
    },
    StdlibModule {
        name: "std.amiga.bsdsocket",
        exports: &[
            "Socket",
            "AF_INET",
            "SOCK_STREAM",
            "openSocket",
            "connectSocket",
            "sendSocket",
            "recvSocket",
            "closeSocket",
            "tcpEcho",
        ],
    },
    StdlibModule {
        name: "std.amiga.amigaguide",
        exports: &[
            "AmigaGuide",
            "GuideMessage",
            "openAmigaGuide",
            "closeAmigaGuide",
            "amigaGuideSignal",
            "waitAmigaGuideMessage",
            "guideMessageType",
            "guideMessagePrimary",
            "guideMessageSecondary",
            "replyGuideMessage",
            "sendAmigaGuideCommand",
        ],
    },
    StdlibModule {
        name: "std.amiga.picasso96",
        exports: &[
            "RtgScreen",
            "RtgBitmap",
            "RTG_FORMAT_A8R8G8B8",
            "RTG_FORMAT_A8B8G8R8",
            "RTG_FORMAT_R8G8B8A8",
            "RTG_FORMAT_B8G8R8A8",
            "rtgBoards",
            "rtgMode",
            "rtgModeWidth",
            "rtgModeHeight",
            "rtgModeDepth",
            "rtgModeFormat",
            "openRtgScreen",
            "closeRtgScreen",
            "rtgScreenWidth",
            "rtgScreenHeight",
            "rtgScreenDepth",
            "rtgScreenFormat",
            "rtgScreenMode",
            "allocRtgBitmap",
            "freeRtgBitmap",
            "rtgBitmapWidth",
            "rtgBitmapHeight",
            "rtgBitmapDepth",
            "rtgBitmapFormat",
            "rtgBitmapBytesPerRow",
            "rtgScreenWritePixel",
            "rtgScreenReadPixel",
            "rtgScreenFillRect",
            "rtgBitmapWritePixel",
            "rtgBitmapReadPixel",
            "rtgBitmapFillRect",
            "rtgScreenBlitBitmap",
            "rtgScreenReadPixels",
            "rtgBitmapReadPixels",
        ],
    },
];

/// Looks up a standard-library module by its dotted name.
pub fn lookup(name: &str) -> Option<&'static StdlibModule> {
    MODULES.iter().find(|module| module.name == name)
}

/// True when the standard-library module exports the name.
pub fn exports(module: &str, name: &str) -> bool {
    lookup(module).is_some_and(|found| found.exports.contains(&name))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::ast::{Item, Program};
    use crate::diag::Diagnostics;
    use crate::parse::Parser;
    use crate::profile::Profile;
    use crate::span::SourceMap;

    /// The export names a module may leave to the compiler.
    ///
    /// The built-in `String` handle and the compiler-known `text` formatter are
    /// exported by the table but declared by the compiler rather than by
    /// Clorraine, so they do not appear as declarations in the source.
    const COMPILER_KNOWN_EXPORTS: &[&str] = &["String", "text"];

    /// Parses one embedded module and returns it with the codes of any
    /// diagnostics the parser reported.
    ///
    /// This is the same parse the driver runs on an embedded source, so a
    /// module that does not parse here would fail every program that imports
    /// it.
    fn parse_module(name: &str, source: &str) -> (Program, Vec<&'static str>) {
        let mut sources = SourceMap::new();
        let file = sources.add_file(format!("{name}.clo"), source.to_owned());
        let mut diagnostics = Diagnostics::new(false, None);
        let program = Parser::new(&sources, file, &mut diagnostics).parse_program();
        let codes = diagnostics
            .items()
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect();
        (program, codes)
    }

    /// The names a parsed program declares at top level.
    fn declared_names(program: &Program) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        for item in &program.items {
            let name = match item {
                Item::Func(decl) | Item::Comptime(decl) => &decl.name,
                Item::Struct(decl) => &decl.name,
                Item::Enum(decl) => &decl.name,
                Item::Handle(decl) => &decl.name,
                Item::Intrinsic(signature) => &signature.name,
                Item::GlobalLet(decl) => &decl.name,
                Item::GlobalConst(decl) => &decl.name,
                Item::Ffi(_) => continue,
            };
            names.insert(name.clone());
        }
        names
    }

    /// The `ffi` symbols a parsed module binds.
    fn ffi_symbols(program: &Program) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        for item in &program.items {
            if let Item::Ffi(block) = item {
                for signature in &block.decls {
                    names.insert(signature.name.clone());
                }
            }
        }
        names
    }

    /// The names a parsed module declares with `intrinsic func`.
    fn intrinsic_names(program: &Program) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        for item in &program.items {
            if let Item::Intrinsic(signature) = item {
                names.insert(signature.name.clone());
            }
        }
        names
    }

    /// Loads the sources and returns the diagnostic codes of the load stage,
    /// which owns the reserved-intrinsic check.
    fn load_codes(files: &[String]) -> Vec<&'static str> {
        let mut sources = SourceMap::new();
        let mut diagnostics = Diagnostics::new(false, None);
        let mut programs = Vec::new();
        for (index, text) in files.iter().enumerate() {
            let id = sources.add_file(format!("file{index}.clo"), text.clone());
            let program = Parser::new(&sources, id, &mut diagnostics).parse_program();
            programs.push((id, program));
        }
        let _loaded = crate::load::load(programs, &mut diagnostics);
        diagnostics
            .items()
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    #[test]
    fn modules_are_found_by_their_dotted_name() {
        assert!(lookup("std.mem").is_some());
        assert!(lookup("std.amiga.interrupt").is_some());
        assert!(lookup("std.nope").is_none());
        assert!(lookup("std").is_none());
    }

    #[test]
    fn exports_are_scoped_to_their_module() {
        assert!(exports("std.mem", "Buffer"));
        assert!(exports("std.io", "println"));
        assert!(!exports("std.mem", "println"));
        assert!(!exports("std.mem", "Missing"));
        assert!(!exports("std.nope", "Buffer"));
    }

    #[test]
    fn the_custom_module_is_registered_with_its_wrappers() {
        assert!(lookup("std.iff").is_some());
        assert!(lookup("std.amiga.ilbm").is_some());
        assert!(exports("std.amiga.ilbm", "loadIlbm"));
        assert!(exports("std.amiga.video", "bitmapFromIndices"));
        for name in [
            "seekFile",
            "filePosition",
            "fileLength",
            "lockPath",
            "pathInfo",
            "volumeInfo",
            "makeDir",
            "deleteFile",
            "renameFile",
            "setCurrentDir",
            "currentDirName",
            "parentDirName",
        ] {
            assert!(exports("std.amiga.dos", name), "missing DOS export {name}");
        }
        assert!(source("std.iff").is_some());
        assert!(exports("std.iff", "ilbmInfo"));
        assert!(lookup("std.amiga.custom").is_some());
        assert!(source("std.amiga.custom").is_some());
        assert!(exports("std.amiga.custom", "setDma"));
        assert!(exports("std.amiga.custom", "setColor"));
    }

    #[test]
    fn the_icon_module_is_registered_with_its_wrappers() {
        assert!(lookup("std.amiga.icon").is_some());
        assert!(source("std.amiga.icon").is_some());
        for name in [
            "DiskObject",
            "ICON_DISK",
            "ICON_DRAWER",
            "ICON_TOOL",
            "ICON_PROJECT",
            "ICON_GARBAGE",
            "ICON_DEVICE",
            "ICON_KICK",
            "ICON_APPICON",
            "getDiskObject",
            "putDiskObject",
            "freeDiskObject",
            "getDefDiskObject",
            "diskObjectType",
            "setDiskObjectType",
        ] {
            assert!(exports("std.amiga.icon", name), "missing export {name}");
        }
    }

    #[test]
    fn the_workbench_module_is_registered_with_its_wrappers() {
        assert!(lookup("std.amiga.workbench").is_some());
        assert!(source("std.amiga.workbench").is_some());
        for name in [
            "AppIcon",
            "AppWindow",
            "AppMenuItem",
            "AppMessage",
            "APPMSG_APPWINDOW",
            "APPMSG_APPICON",
            "APPMSG_APPMENUITEM",
            "APPMSG_OPEN",
            "addAppIcon",
            "removeAppIcon",
            "addAppWindow",
            "removeAppWindow",
            "addAppMenuItem",
            "removeAppMenuItem",
            "waitAppMessage",
            "appMessageType",
            "appMessageId",
            "appMessageUserData",
            "appMessageClass",
            "appMessageArgCount",
            "replyAppMessage",
            "savePublicScreen",
        ] {
            assert!(exports("std.amiga.workbench", name), "missing export {name}");
        }
    }

    #[test]
    fn the_expansion_module_is_registered_with_its_wrappers() {
        assert!(lookup("std.amiga.expansion").is_some());
        assert!(source("std.amiga.expansion").is_some());
        for name in [
            "expansionBoardCount",
            "expansionBoardManufacturer",
            "expansionBoardProduct",
            "expansionBoardSerial",
            "expansionBoardKind",
            "cardPresent",
            "cardStatus",
            "mapRawKey",
        ] {
            assert!(exports("std.amiga.expansion", name), "missing export {name}");
        }
    }

    #[test]
    fn the_semaphore_utility_pool_modules_are_registered() {
        for (module, names) in [
            (
                "std.amiga.semaphore",
                &["Semaphore", "createSemaphore", "freeSemaphore",
                  "obtainSemaphore", "releaseSemaphore", "attemptSemaphore"][..],
            ),
            (
                "std.amiga.utility",
                &["TagList", "DateStamp", "Hook", "newTagList", "freeTagList",
                  "tagSet", "tagGet", "tagPresent", "tagCount",
                  "currentDateStamp", "freeDateStamp", "dateStampDays",
                  "dateStampMinute", "dateStampTick", "newHook", "callHook",
                  "hookData", "hookCurrentData", "hookSetResult", "freeHook"][..],
            ),
            (
                "std.amiga.pool",
                &["UtilityPool", "UtilityBlock", "createUtilityPool",
                  "allocUtilityBlock", "utilityBlockSize", "writeUtilityBlock",
                  "readUtilityBlock", "freeUtilityBlock", "freeUtilityPool"][..],
            ),
            (
                "std.amiga.args",
                &["Args", "readArgs", "readArgsFrom", "freeArgs", "argCount",
                  "argPresent", "argNumber", "argText", "argSwitch"][..],
            ),
            (
                "std.amiga.datatypes",
                &["DTObject", "openPicture", "disposeDTObject", "dtWidth",
                  "dtHeight", "dtDepth", "dtColors", "drawDTObject"][..],
            ),
            (
                "std.amiga.regions",
                &["Region", "RegionBounds", "newRegion", "freeRegion",
                  "regionOrRect", "regionAndRect", "regionClearRect",
                  "regionBounds", "regionIsEmpty", "layerSetClipRegion",
                  "layerClearClipRegion"][..],
            ),
            (
                "std.amiga.eclock",
                &["EClock", "EClockTicks", "openEClock", "readEClock",
                  "eclockFrequency", "freeEClock"][..],
            ),
            (
                "std.amiga.lowlevel",
                &["readJoyPort", "joyPortType", "decodeJoyPort",
                  "readJoyPortDecoded", "lowlevelVersion"][..],
            ),
            (
                "std.amiga.gadtools",
                &["GadgetList", "newGadgetList", "addButton", "addText",
                  "addString", "addInteger", "buildGadgets", "freeGadgetList",
                  "gadgetWaitEvent", "gadgetPollEvent"][..],
            ),
            (
                "std.amiga.asl",
                &["FileRequester", "newFileRequester", "requestFile",
                  "fileRequesterPath", "freeFileRequester"][..],
            ),
            (
                "std.amiga.boopsi",
                &["BoopsiObject", "newStringGadget", "attachObject",
                  "detachObject", "setStringValue", "stringValue",
                  "disposeObject"][..],
            ),
            (
                "std.amiga.commodities",
                &["Commodity", "openCommodity", "closeCommodity"][..],
            ),
            (
                "std.amiga.locale",
                &["Locale", "openLocale", "closeLocale", "localeLanguageName",
                  "localeCountryCode", "localeGmtOffset",
                  "localeDateTimeFormat"][..],
            ),
            (
                "std.amiga.arexx",
                &["RexxMessage", "createRexxMessage", "deleteRexxMessage",
                  "setRexxAction", "setRexxArg", "sendRexxMessage",
                  "receiveRexxMessage", "replyRexxMessage",
                  "reclaimRexxMessage", "rexxMessageAction",
                  "rexxMessageResult1", "rexxMessageResult2",
                  "rexxMessageArg"][..],
            ),
            (
                "std.amiga.clipboard",
                &["Clipboard", "openClipboard", "closeClipboard",
                  "clipboardWrite", "clipboardRead", "clipboardClear"][..],
            ),
            (
                "std.amiga.devices",
                &["Serial", "Parallel", "Printer", "openSerial", "closeSerial",
                  "serialWrite", "serialRead", "openParallel", "closeParallel",
                  "parallelWrite", "openPrinter", "closePrinter",
                  "printerWrite"][..],
            ),
            (
                "std.amiga.nonvolatile",
                &["NonvolatileData", "getNonvolatile", "freeNonvolatileData",
                  "nonvolatileSize", "nonvolatileRead", "storeNonvolatile",
                  "deleteNonvolatile"][..],
            ),
            (
                "std.amiga.bsdsocket",
                &["Socket", "AF_INET", "SOCK_STREAM", "openSocket",
                  "connectSocket", "sendSocket", "recvSocket",
                  "closeSocket", "tcpEcho"][..],
            ),
            (
                "std.amiga.amigaguide",
                &["AmigaGuide", "GuideMessage", "openAmigaGuide",
                  "closeAmigaGuide", "amigaGuideSignal",
                  "waitAmigaGuideMessage", "guideMessageType",
                  "guideMessagePrimary", "guideMessageSecondary",
                  "replyGuideMessage", "sendAmigaGuideCommand"][..],
            ),
        ] {
            assert!(lookup(module).is_some(), "missing module {module}");
            assert!(source(module).is_some(), "missing source {module}");
            for name in names {
                assert!(exports(module, name), "missing export {module}.{name}");
            }
        }
    }

    #[test]
    fn the_exec_module_is_registered_with_its_wrappers() {
        assert!(lookup("std.amiga.exec").is_some());
        assert!(source("std.amiga.exec").is_some());
        for name in [
            "MsgPort",
            "NewMessage",
            "Message",
            "Task",
            "Library",
            "Device",
            "CMD_INVALID",
            "CMD_RESET",
            "CMD_READ",
            "CMD_WRITE",
            "CMD_UPDATE",
            "CMD_CLEAR",
            "CMD_STOP",
            "CMD_START",
            "CMD_FLUSH",
            "createMsgPort",
            "createPublicPort",
            "deleteMsgPort",
            "createMessage",
            "messagePayload",
            "putMsg",
            "signals",
            "waitSignals",
            "getMsg",
            "replyMsg",
            "freeMessage",
            "releaseMsg",
            "createTask",
            "reportTaskResult",
            "joinTask",
            "openLibrary",
            "closeLibrary",
            "libraryVersion",
            "libraryRevision",
            "openDevice",
            "openConsole",
            "closeDevice",
            "deviceCommand",
            "deviceSetBuffer",
            "deviceSetOffset",
            "deviceDoIO",
            "deviceSendIO",
            "deviceWaitIO",
            "deviceCheckIO",
            "deviceAbortIO",
            "deviceActual",
            "deviceError",
        ] {
            assert!(exports("std.amiga.exec", name), "missing export {name}");
        }
    }

    #[test]
    fn embedded_sources_match_their_export_table_entry() {
        for module in SOURCES {
            assert!(
                lookup(module.name).is_some(),
                "embedded module '{}' has no export table entry",
                module.name
            );
            assert!(
                module.source.contains("module "),
                "embedded module '{}' has no module declaration",
                module.name
            );
        }
        assert!(source("std.mem").is_some());
        assert!(source("std.nope").is_none());
    }

    #[test]
    fn compiler_intrinsics_are_recognized() {
        assert!(is_compiler_intrinsic("rawSlice"));
        assert!(is_compiler_intrinsic("rawStr"));
        assert!(is_compiler_intrinsic("slice"));
        assert!(is_compiler_intrinsic("len"));
        assert!(!is_compiler_intrinsic("sizeof"));
        assert!(is_standard_module("std.mem"));
        assert!(!is_standard_module("app.core"));
    }

    #[test]
    fn every_module_name_is_unique() {
        let mut names: Vec<&str> = MODULES.iter().map(|module| module.name).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count);
    }

    #[test]
    fn every_corpus_import_is_exported() {
        let corpus = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../spec/tests/corpus");
        let mut checked = 0;
        for entry in std::fs::read_dir(&corpus).expect("corpus directory") {
            let program = entry.expect("corpus entry").path().join("program.clo");
            let text = std::fs::read_to_string(&program)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", program.display()));
            for (module, name) in corpus_imports(&text) {
                assert!(
                    exports(&module, &name),
                    "the corpus imports '{name}' from '{module}', but the table does not export it"
                );
                checked += 1;
            }
        }
        assert!(checked > 0, "no corpus imports were found to check");
    }

    /// Reads the `import { ... } from A.B;` lines of one program.
    fn corpus_imports(text: &str) -> Vec<(String, String)> {
        let mut imports = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            let Some(rest) = line.strip_prefix("import") else {
                continue;
            };
            let Some(rest) = rest.trim().strip_prefix('{') else {
                continue;
            };
            let Some((names, tail)) = rest.split_once('}') else {
                continue;
            };
            let Some(module) = tail.trim().strip_prefix("from") else {
                continue;
            };
            let module = module.trim().trim_end_matches(';').trim();
            for name in names.split(',') {
                let name = name.split_whitespace().next().unwrap_or("");
                if !name.is_empty() {
                    imports.push((module.to_owned(), name.to_owned()));
                }
            }
        }
        imports
    }

    #[test]
    fn every_embedded_source_parses_and_names_its_table_module() {
        for module in MODULES {
            let text = source(module.name)
                .unwrap_or_else(|| panic!("module '{}' has no embedded source", module.name));
            let (program, codes) = parse_module(module.name, text);
            assert!(
                codes.is_empty(),
                "embedded module '{}' does not parse: {codes:?}",
                module.name
            );
            assert_eq!(
                program.module.name.to_dotted(),
                module.name,
                "embedded module '{}' declares a different module name",
                module.name
            );
        }
    }

    #[test]
    fn every_export_is_declared_or_compiler_known() {
        for module in MODULES {
            let text = source(module.name).expect("module has an embedded source");
            let (program, _) = parse_module(module.name, text);
            let declared = declared_names(&program);
            for export in module.exports {
                assert!(
                    declared.contains(*export) || COMPILER_KNOWN_EXPORTS.contains(export),
                    "module '{}' exports '{export}', but the source declares neither it nor a compiler-known name",
                    module.name
                );
            }
        }
    }

    #[test]
    fn each_profile_shim_defines_every_ffi_symbol_its_modules_bind() {
        for profile in [
            Profile::Host,
            Profile::HostedAmiga,
            Profile::HostedAmiga13,
            Profile::Freestanding,
        ] {
            let shim = format!("{}{}", profile.shim(), profile.shim_suffix());
            let mut checked = 0usize;
            for module in SOURCES {
                if !profile.supports_module(module.name) {
                    continue;
                }
                let (program, _) = parse_module(module.name, module.source);
                for symbol in ffi_symbols(&program) {
                    assert!(
                        shim.contains(&format!("{symbol}(")),
                        "the {} shim does not define '{symbol}', which '{}' binds",
                        profile.name(),
                        module.name
                    );
                    checked += 1;
                }
            }
            assert!(
                checked > 0,
                "the {} profile checked no ffi symbols",
                profile.name()
            );
        }
    }

    #[test]
    fn the_compiler_intrinsic_set_matches_the_library_declarations() {
        let mut declared = BTreeSet::new();
        for module in SOURCES {
            let (program, _) = parse_module(module.name, module.source);
            declared.extend(intrinsic_names(&program));
        }
        let expected: BTreeSet<String> = COMPILER_INTRINSICS
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        assert_eq!(
            declared, expected,
            "the intrinsic names declared by the library and the recognized set differ"
        );
        for name in COMPILER_INTRINSICS {
            assert!(is_compiler_intrinsic(name), "'{name}' must be recognized");
        }
    }

    #[test]
    fn a_user_module_cannot_redeclare_a_compiler_intrinsic() {
        for name in COMPILER_INTRINSICS {
            let text = format!(
                "module app;\nintrinsic func {name}<T>(p: raw T, n: usize): T[];\nfunc main(): i32 {{ return 0; }}\n"
            );
            let codes = load_codes(&[text]);
            assert!(
                codes.contains(&"E20006"),
                "declaring intrinsic '{name}' must be rejected, got {codes:?}"
            );
        }
    }
}
