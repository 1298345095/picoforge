//! Shared empty-state gating for the CCID applet screens (Accounts, Slots,
//! PIV, OpenPGP).
//!
//! Three orthogonal questions decide whether a screen can show its content, in
//! priority order: is the CCID interface on, is the applet enabled on the
//! device, does this firmware expose it. Each screen computes an [`AppletGate`]
//! and, unless [`AppletGate::Ready`], renders the message below.

/// Why an applet screen cannot show its content — or `Ready` to proceed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppletGate {
    /// The applet is reachable; render the real UI.
    Ready,
    /// The CCID / smart-card USB interface is turned off.
    CcidOff,
    /// The applet is disabled in USB Applications (carries its display name).
    Disabled(&'static str),
    /// This firmware does not expose the applet.
    Unsupported,
}

impl AppletGate {
    /// Heading + body copy for the empty state, or `None` when [`Self::Ready`].
    ///
    /// Copy is firmware-neutral by design — it never names "pico-fido".
    pub fn message(&self) -> Option<(&'static str, String)> {
        match self {
            Self::Ready => None,
            Self::CcidOff => Some((
                "智能卡接口已关闭",
                "请在“配置 → 硬件接口”中启用 CCID 接口，然后重新连接设备。"
                    .into(),
            )),
            Self::Disabled(name) => Some((
                "应用模块已禁用",
                format!("{name} 已关闭。请在“配置 → USB 应用”中启用。"),
            )),
            Self::Unsupported => Some((
                "不可用",
                "此固件未提供该应用模块。".into(),
            )),
        }
    }
}
