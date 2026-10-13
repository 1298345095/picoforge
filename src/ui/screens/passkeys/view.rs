use crate::ui::components::{
    button::{PFButton, PFIconButton},
    card::Card,
    dialog,
    page_view::PageView,
};
use crate::ui::models::device::{DeviceMethod, StoredCredential};
use crate::ui::screens::passkeys::view_model::{PasskeysEvent, PasskeysViewModel};
use directories::UserDirs;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::Disableable;
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_component::{
    ActiveTheme, Icon, Sizable, StyledExt, Theme, badge::Badge, h_flex, switch::Switch, v_flex,
};

impl PasskeysViewModel {
    fn render_enterprise_attestation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let csr_ready = self.csr_pem.is_some();
        let show_csr = self.show_csr && csr_ready;
        let is_loading = self.csr_loading;
        let pem = self.csr_pem.clone().unwrap_or_default();
        let pem_for_copy = pem.clone();

        let request_listener = cx.listener(|this, _, window, cx| {
            let status_handle = dialog::open_status_dialog("证书请求", window, cx);
            this.request_csr(status_handle, cx);
        });

        let view_listener = cx.listener(|this, _, _, cx| {
            this.show_csr = !this.show_csr;
            cx.notify();
        });

        let save_listener = cx.listener(|this, _, _, cx| {
            let Some(pem) = this.csr_pem.clone() else {
                return;
            };
            let default_dir = UserDirs::new()
                .and_then(|d| {
                    d.document_dir()
                        .or_else(|| d.download_dir())
                        .map(|p| p.to_path_buf())
                })
                .unwrap_or_else(|| {
                    std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
                });
            let receiver = cx.prompt_for_new_path(&default_dir, Some("device_attestation.csr"));
            let entity = cx.entity().downgrade();
            this._task = Some(cx.spawn(async move |_, cx| match receiver.await {
                Ok(Ok(Some(path))) => match std::fs::write(&path, pem.as_bytes()) {
                    Ok(_) => {
                        let _ = entity.update(cx, |_, cx| {
                            cx.emit(PasskeysEvent::Notification(format!(
                                "CSR 已保存到 {}",
                                path.display()
                            )));
                        });
                    }
                    Err(e) => {
                        let _ = entity.update(cx, |_, cx| {
                            cx.emit(PasskeysEvent::Notification(format!(
                                "保存 CSR 失败：{}",
                                e
                            )));
                        });
                    }
                },
                Ok(Err(e)) => {
                    let _ = entity.update(cx, |_, cx| {
                        cx.emit(PasskeysEvent::Notification(format!(
                            "保存对话框出错：{}",
                            e
                        )));
                    });
                }
                _ => {}
            }));
        });

        let upload_listener = cx.listener(|this, _, window, cx| {
            this.open_upload_cert_dialog(window, cx);
        });

        let theme = cx.theme();

        let fido_info = self.device.read(cx).fido_info.clone();
        let enterprise_attestation_set = fido_info
            .as_ref()
            .and_then(|f| f.options.get("ep").copied())
            .unwrap_or(false);

        let enable_ea_listener = cx.listener(|this, _checked: &bool, window, cx| {
            this.open_enable_ea_dialog(window, cx);
        });

        let enable_row = div()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_4()
                    .child(
                        v_flex().child(div().font_medium().child("启用企业证明")),
                    )
                    .child(
                        h_flex().gap_2().child(
                            Switch::new("enable-ea-switch")
                                .checked(enterprise_attestation_set)
                                .disabled(enterprise_attestation_set)
                                .on_click(enable_ea_listener),
                        ),
                    ),
            );

        let csr_row = div()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_4()
                    .child(
                        v_flex()
                            .child(div().font_medium().child("证书签名请求（CSR）"))
                            .child(div().text_sm().text_color(theme.muted_foreground).child(
                                if csr_ready {
                                    "已获取 CSR"
                                } else {
                                    "获取用于企业证明注册的 CSR"
                                },
                            )),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .when(csr_ready, |el| {
                                el.child(
                                    PFButton::new(if show_csr { "隐藏 CSR" } else { "查看 CSR" })
                                        .id("view-csr-btn")
                                        .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
                                        .on_click(view_listener),
                                )
                            })
                            .child(
                                PFButton::new(if csr_ready { "刷新" } else { "请求 CSR" })
                                    .id("request-csr-btn")
                                    .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
                                    .loading(is_loading)
                                    .on_click(request_listener),
                            ),
                    ),
            )
            .when(show_csr, |el| {
                el.child(
                    div().border_t_1().border_color(theme.border).p_4().child(
                        v_flex()
                            .gap_3()
                            .child(div().text_sm().text_color(theme.muted_foreground).child(
                                "由设备证明密钥生成的证书签名请求。",
                            ))
                            .child(
                                div()
                                    .font_family("monospace")
                                    .text_xs()
                                    .bg(theme.muted)
                                    .p_3()
                                    .rounded_lg()
                                    .overflow_hidden()
                                    .child(pem.clone()),
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        Button::new("copy-csr")
                                            .label("复制到剪贴板")
                                            .on_click(move |_, _, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    pem_for_copy.clone(),
                                                ));
                                            }),
                                    )
                                    .child(
                                        Button::new("save-csr")
                                            .primary()
                                            .label("保存到文件")
                                            .on_click(save_listener),
                                    ),
                            ),
                    ),
                )
            });

        let upload_row = div()
            .flex()
            .items_center()
            .justify_between()
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                v_flex()
                    .child(div().font_medium().child("上传证书"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("将签名后的证书上传到设备"),
                    ),
            )
            .child(
                PFButton::new("上传证书")
                    .id("upload-cert-btn")
                    .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
                    .on_click(upload_listener),
            );

        Card::new()
            .title("企业证明")
            .description("配置企业专属功能")
            .icon(Icon::default().path("icons/shield-check.svg"))
            .child(
                v_flex()
                    .gap_3()
                    .child(enable_row)
                    .child(csr_row)
                    .child(upload_row),
            )
    }

    fn render_reset_device_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        let header = gpui_component::h_flex()
            .items_center()
            .justify_between()
            .w_full()
            .gap_4()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_base()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.foreground)
                            .child("恢复出厂设置"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("将清除所有通行密钥、凭据及 PIN，此操作不可撤销。"),
                    ),
            )
            .child(
                Button::new("reset-device")
                    .icon(Icon::default().path("icons/circle-alert.svg"))
                    .child("重置设备")
                    .custom(
                        ButtonCustomVariant::new(cx)
                            .color(theme.danger)
                            .hover(theme.danger_hover)
                            .active(theme.danger_active)
                            .foreground(theme.danger_foreground),
                    )
                    .disabled(self.loading)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_reset_dialog(window, cx);
                    })),
            );

        Card::new()
            .title("重置")
            .description("执行不可逆的恢复出厂设置")
            .icon(Icon::default().path("icons/trash.svg"))
            .child(header)
    }

    fn render_no_device(&self, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .h_64()
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .child(
                div()
                    .text_color(theme.muted_foreground)
                    .child("请连接 pico-key 以管理通行密钥。"),
            )
            .into_any_element()
    }

    fn render_not_supported(&self, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .h_64()
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .child(
                div()
                    .text_color(theme.muted_foreground)
                    .child("此设备不支持 FIDO 通行密钥。"),
            )
            .into_any_element()
    }

    fn render_pin_management(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let status_row = self.render_pin_status_row(cx).into_any_element();
        let min_len_row = self.render_min_pin_length_row(cx).into_any_element();

        Card::new()
            .title("PIN 管理")
            .icon(Icon::default().path("icons/key.svg"))
            .description("配置 FIDO2 PIN 安全设置")
            .child(v_flex().gap_4().child(status_row).child(min_len_row))
    }

    fn render_pin_status_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let fido_info = self.device.read(cx).fido_info.clone();
        let pin_set = fido_info
            .as_ref()
            .and_then(|f| f.options.get("clientPin").copied())
            .unwrap_or(false);

        let listener = cx.listener(move |this, _, window, cx| {
            if pin_set {
                this.open_change_pin_dialog(window, cx);
            } else {
                this.open_setup_pin_dialog(window, cx);
            }
        });

        let theme = cx.theme();

        div()
            .flex()
            .items_center()
            .justify_between()
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                v_flex()
                    .child(div().font_medium().child("当前 PIN 状态"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(if pin_set {
                                "已设置 PIN"
                            } else {
                                "尚未设置 PIN"
                            }),
                    ),
            )
            .child(
                PFButton::new(if pin_set { "修改 PIN" } else { "设置 PIN" })
                    .id("change-pin-btn")
                    .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
                    .on_click(listener),
            )
    }

    fn render_min_pin_length_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let fido_info = self.device.read(cx).fido_info.clone();
        let min_len = fido_info.as_ref().map(|f| f.min_pin_length).unwrap_or(4);
        let pin_set = fido_info
            .as_ref()
            .and_then(|f| f.options.get("clientPin").copied())
            .unwrap_or(false);

        let theme = cx.theme();

        div()
            .flex()
            .items_center()
            .justify_between()
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                v_flex()
                    .child(div().font_medium().child("PIN 最短长度"))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!("当前：{} 个字符", min_len)),
                    ),
            )
            .child(
                PFButton::new("修改最短长度")
                    .id("update-min-len-btn")
                    .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
                    .disabled(!pin_set)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_min_pin_length_dialog(window, cx);
                    })),
            )
    }

    fn render_stored_passkeys(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.unlocked {
            self.render_locked_state(cx).into_any_element()
        } else {
            self.render_unlocked_state(cx).into_any_element()
        }
    }

    fn render_locked_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let listener = cx.listener(|this, _, window, cx| {
            this.open_unlock_dialog(window, cx);
        });
        let theme = cx.theme();

        Card::new()
            .title("已保存的通行密钥")
            .icon(Icon::default().path("icons/key-round.svg"))
            .description("查看并管理驻留凭据")
            .child(
                v_flex()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .py_3()
                    .child(
                        div().rounded_full().bg(theme.muted).p_4().child(
                            Icon::default()
                                .path("icons/shield.svg")
                                .size_12()
                                .text_color(theme.muted_foreground),
                        ),
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .child("需要身份验证"),
                    )
                    .child(
                        div()
                            .text_color(theme.muted_foreground)
                            .text_sm()
                            .child("请先解锁设备，以查看和管理通行密钥。"),
                    )
                    .child(
                        PFIconButton::new(
                            Icon::default().path("icons/lock-open.svg"),
                            "解锁存储区",
                        )
                        .on_click(listener)
                        .with_colors(rgb(0xe4e4e7), rgb(0xd0d0d3), rgb(0xe4e4e7))
                        .with_text_color(rgb(0x18181b)),
                    ),
            )
    }

    fn render_unlocked_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let creds_len = self.credentials.len();
        let lock_listener = cx.listener(|this, _, _, cx| {
            this.lock_storage(cx);
        });

        let mut cards = Vec::new();
        for cred in &self.credentials {
            cards.push(self.render_credential_card(cred, cx).into_any_element());
        }

        let theme = cx.theme();

        Card::new()
            .title("已保存的通行密钥")
            .icon(Icon::default().path("icons/key-round.svg"))
            .description("查看并管理驻留凭据")
            .child(
                v_flex()
                    .gap_6()
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                h_flex()
                                    .gap_4()
                                    .items_center()
                                    .child(
                                        Badge::new()
                                            .child(
                                                h_flex()
                                                    .gap_1()
                                                    .items_center()
                                                    .child(
                                                        Icon::default()
                                                            .path("icons/lock-open.svg")
                                                            .size_3p5(),
                                                    )
                                                    .child("已解锁"),
                                            )
                                            .color(gpui::green()),
                                    )
                                    .child(div().w_px().h_4().bg(theme.border))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child(format!("已保存 {} 个凭据", creds_len)),
                                    ),
                            )
                            .child(
                                PFIconButton::new(
                                    Icon::default().path("icons/lock.svg").size_3p5(),
                                    "锁定存储区",
                                )
                                .small()
                                .on_click(lock_listener),
                            ),
                    )
                    .child(if self.credentials.is_empty() {
                        self.render_empty_credentials_with_theme(theme)
                            .into_any_element()
                    } else {
                        div()
                            .grid()
                            .grid_cols(3)
                            .gap_4()
                            .children(cards)
                            .into_any_element()
                    }),
            )
    }

    fn render_empty_credentials_with_theme(&self, theme: &Theme) -> impl IntoElement {
        v_flex()
            .items_center()
            .justify_center()
            .py_12()
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .gap_4()
            .child(
                div()
                    .rounded_full()
                    .bg(theme.muted)
                    .p_4()
                    .child(
                        Icon::default()
                            .path("icons/key-round.svg")
                            .size_8()
                            .text_color(theme.muted_foreground),
                    ),
            )
            .child(div().text_lg().font_semibold().child("未找到通行密钥"))
            .child(
                div()
                    .text_color(theme.muted_foreground)
                    .text_sm()
                    .text_center()
                    .max_w(px(384.0))
                    .child("此设备尚未保存驻留凭据。请先在网站上创建通行密钥，之后即可在此查看。"),
            )
    }

    fn render_credential_card(
        &self,
        cred: &StoredCredential,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let cred_clone = cred.clone();
        let cred_for_click = cred.clone();

        let delete_listener = cx.listener(move |this, _, window, cx| {
            this.open_ask_delete_pin(cred_clone.clone(), window, cx);
        });

        let click_listener = cx.listener(move |this, _, window, cx| {
            this.open_credential_details(&cred_for_click, window, cx);
        });

        let theme = cx.theme();

        div()
            .id(SharedString::from(format!(
                "cred-card-{}",
                cred.credential_id
            )))
            .cursor_pointer()
            .on_click(click_listener)
            .border_1()
            .border_color(theme.border)
            .rounded_xl()
            .p_4()
            .hover(|s| s.bg(theme.accent).border_color(theme.primary))
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .size_10()
                                    .rounded_md()
                                    .bg(rgb(0x3b3b3e))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        Icon::default()
                                            .path("icons/key-round.svg")
                                            .text_color(theme.primary)
                                            .size_5(),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .font_semibold()
                                            .whitespace_nowrap()
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(if !cred.rp_name.is_empty() {
                                                cred.rp_name.clone()
                                            } else if !cred.rp_id.is_empty() {
                                                cred.rp_id.clone()
                                            } else {
                                                "未知服务".to_string()
                                            }),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .whitespace_nowrap()
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(cred.user_name.clone()),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                cx.stop_propagation();
                            })
                            .child(
                                Button::new("delete-cred-btn")
                                    .ghost()
                                    .small()
                                    .child(
                                        Icon::default()
                                            .path("icons/trash-2.svg")
                                            .size_4()
                                            .text_color(theme.muted_foreground),
                                    )
                                    .on_click(delete_listener),
                            ),
                    ),
            )
    }
}

impl Render for PasskeysViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let device = self.device.read(cx);
        let device_connected = device.status.is_some();

        if !device_connected {
            let theme = cx.theme();
            return PageView::build(
                "通行密钥",
                "管理设备上的安全 PIN 及 FIDO 凭据（通行密钥）。",
                self.render_no_device(theme).into_any_element(),
                theme,
            )
            .into_any_element();
        }

        let has_fido = device
            .status
            .as_ref()
            .map(|s| s.method == DeviceMethod::Fido)
            .unwrap_or(false)
            || device.fido_info.is_some();

        if !has_fido {
            let theme = cx.theme();
            return PageView::build(
                "通行密钥",
                "管理设备上的安全 PIN 及 FIDO 凭据（通行密钥）。",
                self.render_not_supported(theme).into_any_element(),
                theme,
            )
            .into_any_element();
        }

        let content = v_flex()
            .gap_6()
            .child(self.render_pin_management(cx))
            .child(self.render_stored_passkeys(cx))
            .child(self.render_enterprise_attestation(cx))
            .child(self.render_reset_device_row(cx));

        let theme = cx.theme();

        div()
            .size_full()
            .relative()
            .child(PageView::build(
                "通行密钥",
                "管理设备上的安全 PIN 及 FIDO 凭据（通行密钥）。",
                content.into_any_element(),
                theme,
            ))
            .into_any_element()
    }
}
