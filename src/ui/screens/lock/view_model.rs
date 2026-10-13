//! View model for the Lock screen — at-rest soft-lock of the FIDO seed.

use crate::ui::DialogSubmit;
use crate::ui::app::AppModels;
use crate::ui::components::applet_gate::AppletGate;
use crate::ui::components::dialog;
use crate::ui::components::dialog::StatusContent;
use crate::ui::models::device::{DeviceEvent, DeviceRepo, FirmwareType, backup};
use gpui::*;
use gpui_component::WindowExt;
use gpui_component::button::{ButtonVariant, ButtonVariants};
use gpui_component::input::InputState;

pub struct LockViewModel {
    pub(super) device: Entity<DeviceRepo>,
    pub(super) status: Option<backup::BackupStatus>,
    /// The lock-key phrase produced by `enable`, held until cleared.
    pub(super) lock_key: Option<String>,
    pub(super) loading: bool,
    _task: Option<Task<()>>,
}

impl LockViewModel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>, models: &AppModels) -> Self {
        let device = models.device.clone();
        cx.subscribe(&device, |this: &mut Self, _, _: &DeviceEvent, cx| {
            if this.device.read(cx).device_changed {
                this.status = None;
                this.lock_key = None;
            }
            this.load(cx);
            cx.notify();
        })
        .detach();
        let mut this = Self {
            device,
            status: None,
            lock_key: None,
            loading: false,
            _task: None,
        };
        this.load(cx);
        this
    }

    pub(super) fn gate(&self, cx: &App) -> AppletGate {
        let repo = self.device.read(cx);
        match &repo.status {
            None => AppletGate::Unsupported,
            Some(s) if s.firmware_type != FirmwareType::RSKey => AppletGate::Unsupported,
            Some(_) => AppletGate::Ready,
        }
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        if self.loading || self.gate(cx) != AppletGate::Ready {
            return;
        }
        self.loading = true;
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async { DeviceRepo::backup_status_blocking() })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                if let Ok(s) = res {
                    this.status = Some(s);
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn refresh(&mut self, cx: &mut Context<Self>) {
        self.load(cx);
    }

    pub(super) fn clear_lock_key(&mut self, cx: &mut Context<Self>) {
        self.lock_key = None;
        cx.notify();
    }

    fn pin_input(window: &mut Window, cx: &mut Context<Self>) -> Entity<InputState> {
        cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder("FIDO PIN（必填）")
        })
    }

    fn phrase_input(window: &mut Window, cx: &mut Context<Self>) -> Entity<InputState> {
        cx.new(|cx| InputState::new(window, cx).placeholder("24 个助记词组成的锁定密钥"))
    }

    // ── Enable ──────────────────────────────────────────────────────────────

    pub(super) fn open_enable(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pin = Self::pin_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let pin = pin.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let p = pin.read(cx).text().to_string();
                if p.is_empty() {
                    return;
                }
                window.close_dialog(cx);
                let status = dialog::open_status_dialog("正在启用锁定", window, cx);
                let _ = view.update(cx, |this, cx| this.run_enable(p, status, cx));
            })
        };
        Self::dialog(
            "启用存储锁定",
            "使用新的锁定密钥加密 FIDO 种子，并清除明文。此后每次重新上电都必须先解锁才能使用 FIDO 登录。丢失锁定密钥后只能恢复出厂设置，这会销毁当前身份。需要 FIDO PIN，并触摸设备确认。",
            None,
            pin,
            ("启用锁定", ButtonVariant::Danger),
            submit,
            window,
            cx,
        );
    }

    fn run_enable(
        &mut self,
        pin: String,
        status: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        let _ = status.update(cx, |d, cx| {
            d.set_loading("正在锁定…请触摸设备（BOOTSEL）。", cx)
        });
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx
                .background_executor()
                .spawn(async move { DeviceRepo::lock_enable_blocking(pin) })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(phrase) => {
                        this.lock_key = Some(phrase);
                        this.load(cx);
                        let _ = status.update(cx, |d, cx| {
                            d.set_success(
                                "锁定已启用——请记录下面的锁定密钥，每次重新上电都需要它。".into(),
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

    // ── Unlock ──────────────────────────────────────────────────────────────

    pub(super) fn open_unlock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let phrase = Self::phrase_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let phrase = phrase.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let m = phrase.read(cx).text().to_string();
                if m.trim().is_empty() {
                    return;
                }
                window.close_dialog(cx);
                let status = dialog::open_status_dialog("正在解锁", window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run_unit(
                        move || DeviceRepo::lock_unlock_blocking(m),
                        "已解锁——断电前可正常使用 FIDO。",
                        status,
                        cx,
                    );
                });
            })
        };
        Self::dialog_phrase_only(
            "解锁种子",
            "使用锁定密钥将种子加载到内存中，供本次上电周期使用。",
            phrase,
            ("解锁", ButtonVariant::Primary),
            submit,
            window,
            cx,
        );
    }

    // ── Disable ─────────────────────────────────────────────────────────────

    pub(super) fn open_disable(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let pin = Self::pin_input(window, cx);
        let phrase = Self::phrase_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let pin = pin.clone();
            let phrase = phrase.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let p = pin.read(cx).text().to_string();
                let m = phrase.read(cx).text().to_string();
                if p.is_empty() || m.trim().is_empty() {
                    return;
                }
                window.close_dialog(cx);
                let status = dialog::open_status_dialog("正在禁用锁定", window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run_unit(
                        move || DeviceRepo::lock_disable_blocking(p, m),
                        "锁定已禁用——明文种子已恢复。",
                        status,
                        cx,
                    );
                });
            })
        };
        Self::dialog(
            "禁用存储锁定",
            "恢复明文种子，使 FIDO 无需解锁即可使用。需要锁定密钥、FIDO PIN，并触摸设备确认。",
            Some(("锁定密钥（24 个助记词）", phrase)),
            pin,
            ("禁用锁定", ButtonVariant::Primary),
            submit,
            window,
            cx,
        );
    }

    fn run_unit(
        &mut self,
        op: impl FnOnce() -> Result<(), String> + Send + 'static,
        ok_msg: &'static str,
        status: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
        let _ = status.update(cx, |d, cx| {
            d.set_loading("正在处理…如果设备指示灯闪烁，请触摸设备（BOOTSEL）。", cx)
        });
        cx.notify();
        let weak = cx.entity().downgrade();
        self._task = Some(cx.spawn(async move |_, cx| {
            let res = cx.background_executor().spawn(async move { op() }).await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(_) => {
                        let _ = status.update(cx, |d, cx| d.set_success(ok_msg.into(), cx));
                        this.load(cx);
                    }
                    Err(e) => {
                        let _ = status.update(cx, |d, cx| d.set_error(e, cx));
                    }
                }
                cx.notify();
            });
        }));
    }

    /// Dialog with an optional text field above a required PIN field.
    #[allow(clippy::too_many_arguments)]
    fn dialog(
        title: &'static str,
        body: &'static str,
        extra: Option<(&'static str, Entity<InputState>)>,
        pin: Entity<InputState>,
        action: (&'static str, ButtonVariant),
        submit: DialogSubmit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.open_dialog(cx, move |dialog, _w, _| {
            let pin = pin.clone();
            let extra = extra.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            let (action_label, action_variant) = action;
            let mut fields = gpui_component::v_flex().gap_3().pb_2();
            if let Some((label, input)) = &extra {
                fields = fields
                    .child(label.to_string())
                    .child(gpui_component::input::Input::new(input));
            }
            fields = fields
                .child("FIDO PIN")
                .child(gpui_component::input::Input::new(&pin));
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
                        gpui_component::button::Button::new("go")
                            .with_variant(action_variant)
                            .label(action_label)
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    /// Dialog with a single text field (unlock — no PIN).
    fn dialog_phrase_only(
        title: &'static str,
        body: &'static str,
        phrase: Entity<InputState>,
        action: (&'static str, ButtonVariant),
        submit: DialogSubmit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.open_dialog(cx, move |dialog, _w, _| {
            let phrase = phrase.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            let (action_label, action_variant) = action;
            dialog
                .title(title)
                .child(body)
                .child(
                    gpui_component::v_flex()
                        .gap_2()
                        .pb_2()
                        .child("锁定密钥（24 个助记词）")
                        .child(gpui_component::input::Input::new(&phrase)),
                )
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
                        gpui_component::button::Button::new("go")
                            .with_variant(action_variant)
                            .label(action_label)
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }
}
