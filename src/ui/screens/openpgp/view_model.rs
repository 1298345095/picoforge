//! View model for the OpenPGP screen — card status, PIN management, per-key
//! touch policy, on-device key generation, cardholder editing, and reset.

use crate::error::PFError;
use crate::ui::app::AppModels;
use crate::ui::components::applet_gate::AppletGate;
use crate::ui::components::dialog;
use crate::ui::components::dialog::StatusContent;
use crate::ui::components::form::{select_state, selected_key};
use crate::ui::models::device::{DeviceEvent, DeviceRepo, USB_CAP_OPENPGP, openpgp};
use gpui::*;
use gpui_component::WindowExt;
use gpui_component::button::ButtonVariants;
use openpgp::PgpSlot;

/// Translated names for the three OpenPGP key slots (display only).
pub(super) fn slot_display_name(slot: PgpSlot) -> &'static str {
    match slot {
        PgpSlot::Sig => "签名",
        PgpSlot::Dec => "加密",
        PgpSlot::Aut => "身份认证",
    }
}

const OPT_TOUCH: &[(&str, u8)] = &[("关闭", 0), ("开启", 1)];
/// OpenPGP sex (DO 5F35): the byte is an ISO-5218-style ASCII digit.
const OPT_SEX: &[(&str, u8)] = &[("未指定", 0x39), ("男", 0x31), ("女", 0x32)];

pub struct OpenPgpViewModel {
    pub(super) device: Entity<DeviceRepo>,
    pub(super) info: Option<openpgp::PgpInfo>,
    pub(super) loaded: bool,
    pub(super) loading: bool,
    _task: Option<Task<()>>,
}

pub enum OpenPgpEvent {
    Notification(String),
}

impl EventEmitter<OpenPgpEvent> for OpenPgpViewModel {}

impl OpenPgpViewModel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>, models: &AppModels) -> Self {
        let device = models.device.clone();
        cx.subscribe(&device, |this: &mut Self, _, _: &DeviceEvent, cx| {
            this.on_device_event(cx);
        })
        .detach();
        let mut this = Self {
            device,
            info: None,
            loaded: false,
            loading: false,
            _task: None,
        };
        this.load(cx);
        this
    }

    fn on_device_event(&mut self, cx: &mut Context<Self>) {
        if self.device.read(cx).device_changed {
            self.info = None;
            self.loaded = false;
        }
        self.load(cx);
        cx.notify();
    }

    pub(super) fn gate(&self, cx: &App) -> AppletGate {
        let repo = self.device.read(cx);
        if repo.status.is_none() {
            return AppletGate::Unsupported;
        }
        match repo.openpgp_features() {
            None => AppletGate::Unsupported,
            Some(_) if !repo.ccid_on() => AppletGate::CcidOff,
            Some(_) if !repo.applet_enabled(USB_CAP_OPENPGP) => AppletGate::Disabled("OpenPGP"),
            Some(_) => AppletGate::Ready,
        }
    }

    /// Whether the firmware advertises elliptic-curve keys (else RSA-only).
    fn ecc(&self, cx: &App) -> bool {
        self.device
            .read(cx)
            .openpgp_features()
            .map(|f| f.ecc)
            .unwrap_or(false)
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
                .spawn(async { DeviceRepo::openpgp_read_info_blocking() })
                .await;
            let _ = weak.update(cx, |this, cx| {
                this.loading = false;
                match res {
                    Ok(info) => {
                        this.info = Some(info);
                        this.loaded = true;
                    }
                    Err(e) => {
                        log::warn!("OpenPGP read failed: {e}");
                        cx.emit(OpenPgpEvent::Notification(format!("OpenPGP: {e}")));
                    }
                }
                cx.notify();
            });
        }));
    }

    pub(super) fn refresh(&mut self, cx: &mut Context<Self>) {
        self.load(cx);
    }

    /// Run a blocking op, report on `status`, and reload on success.
    fn run(
        &mut self,
        op: impl FnOnce() -> Result<(), PFError> + Send + 'static,
        ok_msg: &'static str,
        status: WeakEntity<StatusContent>,
        cx: &mut Context<Self>,
    ) {
        if self.loading {
            return;
        }
        self.loading = true;
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
                        let _ = status.update(cx, |d, cx| d.set_error(format!("{e}"), cx));
                    }
                }
                cx.notify();
            });
        }));
    }

    // ── PIN management ──────────────────────────────────────────────────────

    pub(super) fn open_change_user_pin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.two_secret_dialog(
            "修改用户 PIN",
            "当前 PIN（PW1）",
            "新 PIN",
            None,
            window,
            cx,
            DeviceRepo::openpgp_change_user_pin_blocking,
            "用户 PIN 已修改。",
        );
    }

    pub(super) fn open_change_admin_pin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.two_secret_dialog(
            "修改管理员 PIN",
            "当前管理员 PIN（PW3）",
            "New admin PIN",
            None,
            window,
            cx,
            DeviceRepo::openpgp_change_admin_pin_blocking,
            "管理员 PIN 已修改。",
        );
    }

    pub(super) fn open_unblock_with_code(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.two_secret_dialog(
            "使用重置码解锁",
            "重置码",
            "新用户 PIN",
            None,
            window,
            cx,
            DeviceRepo::openpgp_unblock_with_code_blocking,
            "用户 PIN 已解除锁定。",
        );
    }

    pub(super) fn open_unblock_with_admin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.two_secret_dialog(
            "使用管理员 PIN 解锁",
            "管理员 PIN（PW3）",
            "新用户 PIN",
            Some(openpgp::DEFAULT_PW3),
            window,
            cx,
            DeviceRepo::openpgp_unblock_with_admin_blocking,
            "用户 PIN 已解除锁定。",
        );
    }

    pub(super) fn open_set_reset_code(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.two_secret_dialog(
            "设置重置码",
            "管理员 PIN（PW3）",
            "新重置码",
            Some(openpgp::DEFAULT_PW3),
            window,
            cx,
            DeviceRepo::openpgp_set_reset_code_blocking,
            "重置码已更新。",
        );
    }

    /// A two-masked-field dialog running a blocking `op(a, b)`. `prefill_a`
    /// seeds the first field (e.g. the default admin PIN) for convenience.
    #[allow(clippy::too_many_arguments)]
    fn two_secret_dialog(
        &mut self,
        title: &'static str,
        label_a: &'static str,
        label_b: &'static str,
        prefill_a: Option<&'static str>,
        window: &mut Window,
        cx: &mut Context<Self>,
        op: impl Fn(String, String) -> Result<(), PFError> + Send + Clone + 'static,
        ok_msg: &'static str,
    ) {
        let a = cx.new(|cx| {
            let st = gpui_component::input::InputState::new(window, cx).masked(true);
            match prefill_a {
                Some(v) => st.default_value(v),
                None => st,
            }
        });
        let b = cx.new(|cx| gpui_component::input::InputState::new(window, cx).masked(true));
        let view = cx.entity().downgrade();
        let submit = {
            let a = a.clone();
            let b = b.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let av = a.read(cx).text().to_string();
                let bv = b.read(cx).text().to_string();
                if av.is_empty() || bv.is_empty() {
                    return;
                }
                window.close_dialog(cx);
                let status = dialog::open_status_dialog(title, window, cx);
                let op = op.clone();
                let _ = view.update(cx, |this, cx| {
                    this.run(move || op(av, bv), ok_msg, status, cx);
                });
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let a = a.clone();
            let b = b.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            dialog
                .title(title)
                .child(
                    gpui_component::v_flex()
                        .gap_3()
                        .pb_2()
                        .child(label_a)
                        .child(gpui_component::input::Input::new(&a))
                        .child(label_b)
                        .child(gpui_component::input::Input::new(&b)),
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
                        gpui_component::button::Button::new("ok")
                            .primary()
                            .label("保存")
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    // ── Generate ────────────────────────────────────────────────────────────

    pub(super) fn open_generate(
        &mut self,
        slot: PgpSlot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let algos: &'static [(&str, u8)] = if self.ecc(cx) {
            openpgp::GENERATE_ALGOS
        } else {
            &openpgp::GENERATE_ALGOS[..3]
        };
        let algo_sel = select_state(window, cx, algos, 0);
        let admin = admin_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let algo_sel = algo_sel.clone();
            let admin = admin.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let admin_pin = admin.read(cx).text().to_string();
                if admin_pin.is_empty() {
                    return;
                }
                let choice = selected_key(&algo_sel, algos, cx);
                window.close_dialog(cx);
                let status = dialog::open_status_dialog("正在生成密钥", window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run(
                        move || DeviceRepo::openpgp_generate_blocking(admin_pin, slot, choice),
                        "密钥已生成。请使用 GnuPG 设置指纹并发布公钥。",
                        status,
                        cx,
                    );
                });
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let algo_sel = algo_sel.clone();
            let admin = admin.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            dialog
                .title(format!("生成密钥 — {}", slot_display_name(slot)))
                .child("将在此插槽中生成新的密钥对（会覆盖原有密钥）。RSA 密钥生成可能需要几秒钟。")
                .child(
                    gpui_component::v_flex()
                        .gap_3()
                        .pb_2()
                        .child("算法")
                        .child(
                            gpui_component::select::Select::new(&algo_sel)
                                .w_full()
                                .bg(rgb(0x222225)),
                        )
                        .child("管理员 PIN（PW3）")
                        .child(gpui_component::input::Input::new(&admin)),
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
                        gpui_component::button::Button::new("gen")
                            .primary()
                            .label("生成")
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    // ── Touch policy ──────────────────────────────────────────────────────────

    pub(super) fn open_touch(
        &mut self,
        slot: PgpSlot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current = self
            .info
            .as_ref()
            .and_then(|i| i.keys.iter().find(|k| k.slot == slot))
            .map(|k| k.touch)
            .unwrap_or(false);
        let touch_sel = select_state(window, cx, OPT_TOUCH, if current { 1 } else { 0 });
        let admin = admin_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let touch_sel = touch_sel.clone();
            let admin = admin.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let admin_pin = admin.read(cx).text().to_string();
                if admin_pin.is_empty() {
                    return;
                }
                let on = selected_key(&touch_sel, OPT_TOUCH, cx) == 1;
                window.close_dialog(cx);
                let status = dialog::open_status_dialog("正在更新触摸策略", window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run(
                        move || DeviceRepo::openpgp_set_touch_blocking(admin_pin, slot, on),
                        "触摸策略已更新。",
                        status,
                        cx,
                    );
                });
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let touch_sel = touch_sel.clone();
            let admin = admin.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            dialog
                .title(format!("触摸策略 — {}", slot_display_name(slot)))
                .child("启用后，每次使用此密钥进行操作都需要触摸设备确认。")
                .child(
                    gpui_component::v_flex()
                        .gap_3()
                        .pb_2()
                        .child("触摸要求")
                        .child(
                            gpui_component::select::Select::new(&touch_sel)
                                .w_full()
                                .bg(rgb(0x222225)),
                        )
                        .child("管理员 PIN（PW3）")
                        .child(gpui_component::input::Input::new(&admin)),
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
                        gpui_component::button::Button::new("ok")
                            .primary()
                            .label("保存")
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    // ── Cardholder ────────────────────────────────────────────────────────────

    pub(super) fn open_cardholder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (cur_name, cur_login, cur_url, cur_lang, cur_sex) = self
            .info
            .as_ref()
            .map(|i| {
                (
                    i.name.clone(),
                    i.login.clone(),
                    i.url.clone(),
                    i.lang.clone(),
                    i.sex,
                )
            })
            .unwrap_or((
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                0x39,
            ));
        let name =
            cx.new(|cx| gpui_component::input::InputState::new(window, cx).default_value(cur_name));
        let login = cx
            .new(|cx| gpui_component::input::InputState::new(window, cx).default_value(cur_login));
        let url =
            cx.new(|cx| gpui_component::input::InputState::new(window, cx).default_value(cur_url));
        let lang =
            cx.new(|cx| gpui_component::input::InputState::new(window, cx).default_value(cur_lang));
        let sex_row = OPT_SEX.iter().position(|(_, k)| *k == cur_sex).unwrap_or(0);
        let sex = select_state(window, cx, OPT_SEX, sex_row);
        let admin = admin_input(window, cx);
        let view = cx.entity().downgrade();
        let submit = {
            let name = name.clone();
            let login = login.clone();
            let url = url.clone();
            let lang = lang.clone();
            let sex = sex.clone();
            let admin = admin.clone();
            let view = view.clone();
            std::rc::Rc::new(move |window: &mut Window, cx: &mut App| {
                let admin_pin = admin.read(cx).text().to_string();
                if admin_pin.is_empty() {
                    return;
                }
                let name_v = name.read(cx).text().to_string();
                let login_v = login.read(cx).text().to_string();
                let url_v = url.read(cx).text().to_string();
                let lang_v = lang.read(cx).text().to_string();
                let sex_v = selected_key(&sex, OPT_SEX, cx);
                window.close_dialog(cx);
                let status = dialog::open_status_dialog("正在保存持卡人信息", window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run(
                        move || {
                            DeviceRepo::openpgp_set_cardholder_blocking(
                                admin_pin, name_v, login_v, url_v, lang_v, sex_v,
                            )
                        },
                        "持卡人信息已保存。",
                        status,
                        cx,
                    );
                });
            })
        };
        window.open_dialog(cx, move |dialog, _w, _| {
            let name = name.clone();
            let login = login.clone();
            let url = url.clone();
            let lang = lang.clone();
            let sex = sex.clone();
            let admin = admin.clone();
            let ok = submit.clone();
            let btn = submit.clone();
            dialog
                .title("编辑持卡人信息")
                .child("编辑存储在卡中的持卡人信息，需要管理员 PIN。")
                .child(
                    gpui_component::v_flex()
                        .gap_3()
                        .pb_2()
                        .child("姓名")
                        .child(gpui_component::input::Input::new(&name))
                        .child("登录名")
                        .child(gpui_component::input::Input::new(&login))
                        .child("URL")
                        .child(gpui_component::input::Input::new(&url))
                        .child("语言（ISO-639，例如 zh）")
                        .child(gpui_component::input::Input::new(&lang))
                        .child("性别")
                        .child(gpui_component::select::Select::new(&sex))
                        .child("管理员 PIN（PW3）")
                        .child(gpui_component::input::Input::new(&admin)),
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
                        gpui_component::button::Button::new("ok")
                            .primary()
                            .label("保存")
                            .on_click(move |_, window, cx| s(window, cx)),
                    ]
                })
        });
    }

    // ── Reset ─────────────────────────────────────────────────────────────────

    pub(super) fn open_reset_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = cx.entity().downgrade();
        dialog::open_confirm(
            "重置 OpenPGP 模块",
            "此操作将锁定两个 PIN，随后恢复 OpenPGP 模块出厂设置，删除所有密钥并将 PIN 恢复为默认值（123456 / 12345678）。操作不可撤销。".to_string(),
            "重置",
            gpui_component::button::ButtonVariant::Danger,
            window,
            cx,
            move |_dh, window, cx| {
                window.close_dialog(cx);
                let status = dialog::open_status_dialog("正在重置 OpenPGP", window, cx);
                let _ = view.update(cx, |this, cx| {
                    this.run(DeviceRepo::openpgp_reset_blocking, "OpenPGP 模块已重置。", status, cx);
                });
            },
        );
    }
}

/// A masked input seeded with the default admin PIN for management dialogs.
fn admin_input(
    window: &mut Window,
    cx: &mut Context<OpenPgpViewModel>,
) -> Entity<gpui_component::input::InputState> {
    cx.new(|cx| {
        gpui_component::input::InputState::new(window, cx)
            .masked(true)
            .default_value(openpgp::DEFAULT_PW3)
    })
}
