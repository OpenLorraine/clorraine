//! Target profiles: the compiled-in facts codegen needs.
//!
//! A profile bundles the machine facts the language must not know: which
//! runtime shim to embed, where a runtime fault writes, and the stack-frame
//! threshold a warning compares against. The command-line driver already
//! validates the profile name; this module is the single place that maps a
//! validated name to its facts.

/// The C runtime shim source of the host profile.
const HOST_SHIM: &str = include_str!("../runtime/shim_host.c");

/// The C runtime shim source of the hosted-amiga profile.
const HOSTED_AMIGA_SHIM: &str = include_str!("../runtime/shim_hosted_amiga.c");

/// The C runtime shim source of the freestanding profile.
const FREESTANDING_SHIM: &str = include_str!("../runtime/shim_freestanding.c");

/// A target profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// The test profile: host libc, faults on stderr, `abort`.
    Host,
    /// The v1 target profile. It shares the host emitter surface for now; the
    /// exec/dos shim is added with the hosted-amiga runtime.
    HostedAmiga,
    /// The A500/Kickstart 1.3 target: 68000, OCS, 512 KB chip, `nix13`.
    HostedAmiga13,
    /// A platform with no operating system: no libc, its own start-up.
    Freestanding,
}

impl Profile {
    /// Resolves a validated profile name.
    ///
    /// Returns `None` for an unknown name; the driver reports that with
    /// `E11201` before code generation runs.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "host" => Some(Profile::Host),
            "hosted-amiga" => Some(Profile::HostedAmiga),
            "hosted-amiga-13" => Some(Profile::HostedAmiga13),
            "freestanding" => Some(Profile::Freestanding),
            _ => None,
        }
    }

    /// The profile's canonical name.
    pub fn name(self) -> &'static str {
        match self {
            Profile::Host => "host",
            Profile::HostedAmiga => "hosted-amiga",
            Profile::HostedAmiga13 => "hosted-amiga-13",
            Profile::Freestanding => "freestanding",
        }
    }

    /// True when the profile is implemented and may be selected.
    ///
    /// A profile can be named by the specification but not yet implemented;
    /// selecting it is a codegen error (`E11200`), not an unknown name. This is
    /// the single place that answers the question, so the driver never repeats
    /// the name list. Every profile is implemented today, so the driver's
    /// not-implemented branch is dormant but preserved for a future profile.
    pub fn implemented(self) -> bool {
        match self {
            Profile::Host
            | Profile::HostedAmiga
            | Profile::HostedAmiga13
            | Profile::Freestanding => true,
        }
    }

    /// True when the profile provides a standard-library module.
    ///
    /// A profile names its own API surface, so a module outside it cannot be
    /// lowered on that profile. The AmigaOS scheduler and interrupt dispatcher
    /// belong to the hosted-amiga runtime, so `std.amiga.*` is available there
    /// and rejected on the host test profile. The freestanding profile has no
    /// filesystem, heap, or formatter, so it provides only the console module.
    pub fn supports_module(self, module: &str) -> bool {
        match self {
            Profile::Host => !module.starts_with("std.amiga."),
            Profile::HostedAmiga => true,
            // Kickstart 1.3 predates the message-port allocation calls and the
            // later Intuition/utility APIs. The 1.3 surface is the smallest
            // set whose wrappers work on the stock A500; `std.amiga.exec`
            // arrives with the Exec bundle.
            Profile::HostedAmiga13 => matches!(
                module,
                "std.io"
                    | "std.mem"
                    | "std.text"
                    | "std.fmt"
                    | "std.amiga.custom"
                    | "std.amiga.dos"
                    | "std.amiga.graphics"
                    | "std.amiga.intuition"
                    | "std.amiga.interrupt"
            ),
            Profile::Freestanding => module == "std.io",
        }
    }

    /// The runtime shim embedded at the top of the emitted translation unit.
    pub fn shim(self) -> &'static str {
        match self {
            Profile::Host => HOST_SHIM,
            Profile::HostedAmiga | Profile::HostedAmiga13 => HOSTED_AMIGA_SHIM,
            Profile::Freestanding => FREESTANDING_SHIM,
        }
    }

    /// The stack-frame byte threshold for the large-frame warning.
    pub fn stack_frame_limit(self) -> usize {
        match self {
            Profile::Host => 16_384,
            Profile::HostedAmiga | Profile::HostedAmiga13 => 1_024,
            Profile::Freestanding => 1_024,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_names_round_trip_and_reject_unknown() {
        assert_eq!(Profile::parse("host"), Some(Profile::Host));
        assert_eq!(Profile::parse("hosted-amiga"), Some(Profile::HostedAmiga));
        assert_eq!(
            Profile::parse("hosted-amiga-13"),
            Some(Profile::HostedAmiga13)
        );
        assert_eq!(
            Profile::parse("freestanding"),
            Some(Profile::Freestanding)
        );
        assert_eq!(Profile::parse("nope"), None);
        assert_eq!(Profile::Host.name(), "host");
        assert_eq!(Profile::HostedAmiga.name(), "hosted-amiga");
        assert_eq!(Profile::HostedAmiga13.name(), "hosted-amiga-13");
        assert_eq!(Profile::Freestanding.name(), "freestanding");
    }

    #[test]
    fn every_named_profile_is_implemented() {
        for name in ["host", "hosted-amiga", "hosted-amiga-13", "freestanding"] {
            let profile = Profile::parse(name).expect("a known name parses");
            assert!(profile.implemented(), "{name} must be implemented");
        }
    }

    #[test]
    fn the_host_shim_names_the_three_runtime_helpers() {
        let shim = Profile::Host.shim();
        assert!(shim.contains("cl_zero"));
        assert!(shim.contains("cl_isnan"));
        assert!(shim.contains("cl_trap"));
        assert!(shim.contains("cl_panic"));
    }

    #[test]
    fn the_hosted_amiga_shim_names_the_runtime_and_vblank_surface() {
        let shim = Profile::HostedAmiga.shim();
        assert!(shim.contains("cl_zero"));
        assert!(shim.contains("cl_isnan"));
        assert!(shim.contains("cl_trap"));
        assert!(shim.contains("cl_panic"));
        assert!(shim.contains("cl_add_handler"));
        assert!(shim.contains("cl_remove_handler"));
        assert!(shim.contains("INTB_VERTB"));
        assert!(shim.contains("cl_create_msg_port"));
        assert!(shim.contains("cl_put_msg"));
        assert!(shim.contains("cl_signals"));
        assert!(shim.contains("cl_wait_signals"));
        assert!(shim.contains("cl_get_msg"));
        assert!(shim.contains("cl_reply_msg"));
        assert!(shim.contains("cl_free_message"));
        assert!(shim.contains("cl_release_message"));
    }

    #[test]
    fn the_freestanding_shim_names_the_entry_point_and_console_surface() {
        let shim = Profile::Freestanding.shim();
        assert!(shim.contains("void _start(void)"));
        assert!(shim.contains("memcpy"));
        assert!(shim.contains("cl_trap"));
        assert!(shim.contains("cl_write"));
        assert!(shim.contains("cl_write_line"));
        assert!(shim.contains("Open"));
        assert!(shim.contains("Close"));
        assert!(shim.contains("Read"));
        assert!(shim.contains("Write"));
    }

    #[test]
    fn the_freestanding_shim_avoids_libc_symbols_that_break_the_m68k_link() {
        // The m68k link reads libg.a for the soft-float helpers; that archive's
        // exit and allocator objects reach the AmigaOS global `SysBase`. The
        // runtime must reference none of them, so only the float objects are
        // pulled in.
        let shim = Profile::Freestanding.shim();
        assert!(!shim.contains("malloc("));
        assert!(!shim.contains("abort("));
        assert!(shim.contains("LorraineOS will provide"));
    }

    #[test]
    fn stack_frame_thresholds_match_the_profiles() {
        assert_eq!(Profile::Host.stack_frame_limit(), 16_384);
        assert_eq!(Profile::HostedAmiga.stack_frame_limit(), 1_024);
        assert_eq!(Profile::HostedAmiga13.stack_frame_limit(), 1_024);
        assert_eq!(Profile::Freestanding.stack_frame_limit(), 1_024);
    }

    #[test]
    fn only_the_hosted_amiga_profile_provides_the_amiga_modules() {
        assert!(Profile::Host.supports_module("std.mem"));
        assert!(Profile::Host.supports_module("std.amiga.dos") == false);
        assert!(Profile::HostedAmiga.supports_module("std.amiga.dos"));
        assert!(Profile::HostedAmiga.supports_module("std.amiga.interrupt"));
        assert!(Profile::HostedAmiga.supports_module("std.mem"));
        assert!(Profile::HostedAmiga13.supports_module("std.io"));
        assert!(Profile::HostedAmiga13.supports_module("std.mem"));
        assert!(Profile::HostedAmiga13.supports_module("std.amiga.graphics"));
        assert!(Profile::HostedAmiga13.supports_module("std.amiga.intuition"));
        assert!(!Profile::HostedAmiga13.supports_module("std.amiga.exec"));
        assert!(Profile::HostedAmiga.supports_module("std.amiga.exec"));
    }

    #[test]
    fn the_a500_profile_uses_the_shared_hosted_amiga_shim() {
        assert_eq!(Profile::HostedAmiga13.shim(), Profile::HostedAmiga.shim());
    }

    #[test]
    fn the_freestanding_profile_provides_only_the_console_module() {
        assert!(Profile::Freestanding.supports_module("std.io"));
        assert!(!Profile::Freestanding.supports_module("std.mem"));
        assert!(!Profile::Freestanding.supports_module("std.text"));
        assert!(!Profile::Freestanding.supports_module("std.fmt"));
        assert!(!Profile::Freestanding.supports_module("std.amiga.dos"));
    }
}
