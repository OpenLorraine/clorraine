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

/// A target profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// The test profile: host libc, faults on stderr, `abort`.
    Host,
    /// The v1 target profile. It shares the host emitter surface for now; the
    /// exec/dos shim is added with the hosted-amiga runtime.
    HostedAmiga,
}

impl Profile {
    /// Resolves a validated profile name.
    ///
    /// Returns `None` for `freestanding` and for unknown names; the driver
    /// reports those before code generation runs.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "host" => Some(Profile::Host),
            "hosted-amiga" => Some(Profile::HostedAmiga),
            _ => None,
        }
    }

    /// The profile's canonical name.
    pub fn name(self) -> &'static str {
        match self {
            Profile::Host => "host",
            Profile::HostedAmiga => "hosted-amiga",
        }
    }

    /// The runtime shim embedded at the top of the emitted translation unit.
    pub fn shim(self) -> &'static str {
        match self {
            Profile::Host => HOST_SHIM,
            Profile::HostedAmiga => HOSTED_AMIGA_SHIM,
        }
    }

    /// The stack-frame byte threshold for the large-frame warning.
    pub fn stack_frame_limit(self) -> usize {
        match self {
            Profile::Host => 16_384,
            Profile::HostedAmiga => 1_024,
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
        assert_eq!(Profile::parse("freestanding"), None);
        assert_eq!(Profile::parse("nope"), None);
        assert_eq!(Profile::Host.name(), "host");
        assert_eq!(Profile::HostedAmiga.name(), "hosted-amiga");
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
    }

    #[test]
    fn stack_frame_thresholds_match_the_profiles() {
        assert_eq!(Profile::Host.stack_frame_limit(), 16_384);
        assert_eq!(Profile::HostedAmiga.stack_frame_limit(), 1_024);
    }
}
