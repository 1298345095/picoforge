//! View model for the Audit screen — export and verify the device's
//! tamper-evident security journal.

use crate::ui::DialogSubmit;
use crate::ui::app::AppModels;
use crate::ui::components::applet_gate::AppletGate;
use crate::ui::components::dialog;
use crate::ui::components::dialog::StatusContent;
use crate::ui::models::device::{DeviceEvent, DeviceRepo, FirmwareType, audit};
use gpui::*;
use gpui_component::WindowExt;
use gpui_component::button::ButtonVariants;
use gpui_component::input::InputState;

pub struct AuditViewModel {
    pub(super) device: Entity<DeviceRepo>,
    pub(super) journal: Option<audit::AuditJournal>,
    pub(super) verification: Option<audit::AuditVerification>,
    /// Whether journalling is currently on. `None` until the status is read (the
    /// query is ungated, so it loads automatically). Journalling is opt-in.
    pub(super) enabled: Option<bool>,
    pub(super) loading: bool,
    _task: Option<Task<()>>,
}

impl AuditViewModel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>, models: &AppModels) -> Self {
        let device = models.device.clone();
        cx.subscribe(&device, |this: &mut Self, _, _: &DeviceEvent, cx| {
            if this.device.read(cx).device_changed {
                this.journal = None;
                this.verification = None;
                this.enabled = None;
            }
            this.refresh_status(cx);
            cx.notify();
        })
        .detach();
        let mut this = Self {
            device,
            journal: None,
            verification: None,
            enabled: None,
            loading: false,
            _task: None,
        };
        this.refresh_status(cx);
        this
    }

    /// Load whether journalling is on (ungated — no PIN, no touch).
    pub(super) fn refresh_status(&mut self, cx: &mut Context<Self>) {
        if self.loading || self.gate(cx) != AppletGate::Ready {
            return;
        }
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async { DeviceRepo::audit_status_blocking() })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.enabled = res.ok();
                cx.notify();
            });
        }));
    }

    // ── Enable / disable journalling (PIN + touch) ──────────────────────────

    pub(super) fn open_toggle(
        &mut self,
        enable: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pin = Self::pin_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let pin = pin.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let p = pin.read(cx).text().to_string();
                let p = (!p.is_empty()).then_some(p);
                window.close_dialog(cx);
                let status = dialog::open_status_dialog(
                    if enable {
                        "正在启用审计日志"
                    } else {
                        "正在关闭审计日志"
                    },
                    window,
                    cx,
                );
                let _ = view.update(cx, |this, cx| this.run_toggle(enable, p, status, cx));
            })
        };
        let (title, body) = if enable {
            (
                "启用审计日志",
                "启用防篡改审计日志，将安全事件写入密钥闪存。需要 FIDO PIN（若未设置则触摸验证），并通过触摸确认操作。",
            )
        } else {
            (
                "关闭审计日志",
                "关闭审计日志，之后不再记录安全事件。需要 FIDO PIN（若未设置则触摸验证），并通过触摸确认操作。",
            )
        };
        Self::open_gate_dialog(title, body, pin, None, submit, window, cx);
    }

    fn run_toggle(
        &mut self,
        enable: bool,
        pin: Option<String>,
        status: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        let _ = status.update(cx, |d, cx| {
            d.set_loading("请触摸设备（BOOTSEL）以确认。", cx)
        });
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { DeviceRepo::audit_set_enabled_blocking(enable, pin) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(on) => {
                        this.enabled = Some(on);
                        let _ = status.update(cx, |d, cx| {
                            d.set_success(
                                format!("审计日志已{}。", if on { "启用" } else { "关闭" }),
                                cx,
                            )
                        });
                    }
                    Err(e) => {
                        let _ = status.update(cx, |d, cx| d.set_error(e, cx));
                    }
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn gate(&self, cx: &App) -> AppletGate {
        let repo = self.device.read(cx);
        match &repo.status {
            None => AppletGate::Unsupported,
            Some(s) if s.firmware_type != FirmwareType::RSKey => AppletGate::Unsupported,
            Some(_) => AppletGate::Ready,
        }
    }

    /// Masked, optional PIN input (blank = authorise by touch).
    fn pin_input(window: &mut Window, cx: &mut Context<Self>) -> Entity<InputState> {
        cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder("FIDO PIN（留空则改用触摸确认）")
        })
    }

    // ── Read journal ────────────────────────────────────────────────────────

    pub(super) fn open_read(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pin = Self::pin_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let pin = pin.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let p = pin.read(cx).text().to_string();
                let p = (!p.is_empty()).then_some(p);
                window.close_dialog(cx);
                let status = dialog::open_status_dialog("正在读取日志", window, cx);
                let _ = view.update(cx, |this, cx| this.run_read(p, status, cx));
            })
        };
        Self::open_gate_dialog(
            "读取审计日志",
            "导出安全审计日志。需要 FIDO PIN；若未设置 PIN，则通过触摸确认。",
            pin,
            None,
            submit,
            window,
            cx,
        );
    }

    fn run_read(
        &mut self,
        pin: Option<String>,
        status: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        let _ = status.update(cx, |d, cx| {
            d.set_loading("正在读取…如果设备指示灯闪烁，请触摸设备（BOOTSEL）。", cx)
        });
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { DeviceRepo::audit_log_blocking(pin) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(journal) => {
                        let n = journal.entries.len();
                        this.journal = Some(journal);
                        this.verification = None;
                        let _ = status.update(cx, |d, cx| {
                            d.set_success(format!("日志读取完成，共 {n} 条记录。"), cx)
                        });
                    }
                    Err(e) => {
                        let _ = status.update(cx, |d, cx| d.set_error(e, cx));
                    }
                }
                cx.notify();
            });
        }));
    }

    // ── Verify checkpoint ───────────────────────────────────────────────────

    pub(super) fn open_verify(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pin = Self::pin_input(window, cx);
        let expect = cx.new(|cx| {
            InputState::new(window, cx).placeholder("预期密钥：16 位十六进制指纹或完整公钥（可选）")
        });
        let view = cx.entity().downgrade();
        let submit = {
            let pin = pin.clone();
            let expect = expect.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let p = pin.read(cx).text().to_string();
                let p = (!p.is_empty()).then_some(p);
                let e = expect.read(cx).text().to_string();
                let e = (!e.trim().is_empty()).then_some(e);
                window.close_dialog(cx);
                let status = dialog::open_status_dialog("正在验证检查点", window, cx);
                let _ = view.update(cx, |this, cx| this.run_verify(p, e, status, cx));
            })
        };
        Self::open_gate_dialog(
            "验证审计检查点",
            "导出日志并验证由 DEVK 新签名的检查点，以确认日志完整可信且设备身份真实。",
            pin,
            Some(("预期密钥（可选）", expect)),
            submit,
            window,
            cx,
        );
    }

    fn run_verify(
        &mut self,
        pin: Option<String>,
        expect: Option<String>,
        status: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        let _ = status.update(cx, |d, cx| {
            d.set_loading("正在签署检查点…请触摸设备（BOOTSEL）。", cx)
        });
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { DeviceRepo::audit_verify_blocking(pin, expect) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(v) => {
                        let msg = if v.authentic() {
                            "日志可信——签名与哈希链均已验证。".to_string()
                        } else if !v.signature_ok {
                            "签名无效——请勿信任此日志。".to_string()
                        } else if !v.head_matches {
                            "链头不匹配——日志在读取期间发生变化（可能遭到篡改）。".to_string()
                        } else {
                            "证明密钥不匹配——这不是已登记的设备。".to_string()
                        };
                        this.journal = Some(v.journal.clone());
                        this.verification = Some(v);
                        let _ = status.update(cx, |d, cx| d.set_success(msg, cx));
                    }
                    Err(e) => {
                        let _ = status.update(cx, |d, cx| d.set_error(e, cx));
                    }
                }
                cx.notify();
            });
        }));
    }

    /// A dialog with an optional-PIN field and an optional second field.
    fn open_gate_dialog(
        title: &'static str,
        body: &'static str,
        pin: Entity<InputState>,
        extra: Option<(&'static str, Entity<InputState>)>,
        submit: DialogSubmit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.open_dialog(cx, move |dialog, _w, _| {
            let pin = pin.clone();
            let extra = extra.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            let mut fields = gpui_component::v_flex()
                .gap_3()
                .pb_2()
                .child("FIDO PIN")
                .child(gpui_component::input::Input::new(&pin));
            if let Some((label, input)) = &extra {
                fields = fields
                    .child(label.to_string())
                    .child(gpui_component::input::Input::new(input));
            }
            dialog
                .title(title)
                .child(body)
                .child(fields)
                .on_ok(move |_, window, cx| {
                    ok(window, cx);
                    false
                })
                .footer(move |_, _w, _c, _| {
                    let s = btn.clone();
                    vec![
                        gpui_component::button::Button::new("cancel")
                            .label("取消")
                            .on_click(|_, window, cx| window.close_dialog(cx)),
                        gpui_component::button::Button::new("run")
                            .primary()
                            .label("执行")
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }
}
