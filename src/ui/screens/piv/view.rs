//! PIV screen rendering.

use crate::ui::components::button::PFButton;
use crate::ui::components::card::Card;
use crate::ui::components::page_view::PageView;
use crate::ui::models::device::piv;
use crate::ui::screens::piv::view_model::PivViewModel;
use gpui::*;
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_component::{ActiveTheme, Disableable, Icon, StyledExt, Theme, h_flex, v_flex};

fn empty_state(heading: &str, body: String, theme: &Theme) -> AnyElement {
    v_flex()
        .items_center()
        .justify_center()
        .h_64()
        .gap_2()
        .border_1()
        .border_color(theme.border)
        .rounded_xl()
        .child(div().font_semibold().child(heading.to_string()))
        .child(
            div()
                .text_sm()
                .max_w(px(380.))
                .text_color(theme.muted_foreground)
                .child(body),
        )
        .into_any_element()
}

fn kv(label: &str, value: String, theme: &Theme) -> impl IntoElement {
    v_flex()
        .gap_1()
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(label.to_string()),
        )
        .child(div().text_sm().font_medium().child(value))
}

fn origin_label(o: u8) -> &'static str {
    match o {
        piv::ORIGIN_GENERATED => "generated",
        piv::ORIGIN_IMPORTED => "imported",
        _ => "?",
    }
}

impl PivViewModel {
    fn render_slot_row(&self, s: piv::SlotStatus, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let slot = s.slot;
        let has_key = s.meta.is_some();
        let is_generated = s
            .meta
            .map(|m| m.origin == piv::ORIGIN_GENERATED)
            .unwrap_or(false);
        let status_text = match s.meta {
            Some(m) => format!("{} · {}", piv::algo_label(m.algo), origin_label(m.origin)),
            None => "空".to_string(),
        };
        let cert = if s.has_cert { " · 证书" } else { "" };
        let d = self.loading;

        macro_rules! btn {
            ($id:expr, $label:expr, $method:ident) => {
                PFButton::new($label)
                    .id(format!("{}-{slot:02x}", $id))
                    .disabled(d)
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.$method(slot, window, cx)),
                    )
                    .into_any_element()
            };
        }

        let mut btns: Vec<AnyElement> = vec![
            PFButton::new(if has_key { "重新生成" } else { "生成" })
                .id(format!("gen-{slot:02x}"))
                .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
                .disabled(d)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_generate_dialog(slot, window, cx);
                }))
                .into_any_element(),
            btn!("impk", "导入密钥", open_import_key),
            btn!("impc", "导入证书", open_import_cert),
        ];
        if s.has_cert {
            btns.push(btn!("exp", "导出证书", open_export_cert));
        }
        if is_generated {
            btns.push(btn!("att", "生成证明", open_attest));
        }
        if has_key {
            btns.push(btn!("mv", "移动", open_move_key));
        }
        if s.has_cert {
            btns.push(btn!("delc", "删除证书", open_delete_cert));
        }
        if has_key {
            btns.push(
                Button::new(SharedString::from(format!("delk-{slot:02x}")))
                    .icon(Icon::default().path("icons/trash-2.svg"))
                    .label("删除密钥")
                    .ghost()
                    .disabled(d)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_delete_key(slot, window, cx);
                    }))
                    .into_any_element(),
            );
        }

        v_flex()
            .gap_3()
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                v_flex()
                    .gap_0p5()
                    .child(div().font_medium().child(piv::slot_label(slot)))
                    .child(
                        div()
                            .text_sm()
                            .text_color(if has_key {
                                theme.foreground
                            } else {
                                theme.muted_foreground
                            })
                            .child(format!("{status_text}{cert}")),
                    ),
            )
            .child(h_flex().gap_2().flex_wrap().children(btns))
            .into_any_element()
    }

    fn action_row(
        &self,
        title: &'static str,
        subtitle: &'static str,
        btn: impl IntoElement,
        theme: &Theme,
    ) -> impl IntoElement {
        h_flex()
            .items_center()
            .justify_between()
            .p_4()
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                v_flex()
                    .gap_0p5()
                    .child(div().font_medium().child(title))
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(subtitle),
                    ),
            )
            .child(btn)
    }
}

impl Render for PivViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        const TITLE: &str = "PIV";
        const SUBTITLE: &str = "智能卡证书和密钥（PIV）。";

        if let Some((heading, body)) = self.gate(cx).message() {
            let theme = cx.theme();
            return PageView::build(TITLE, SUBTITLE, empty_state(heading, body, theme), theme)
                .into_any_element();
        }

        let info = self.info.clone();
        let slots = info.as_ref().map(|i| i.slots.clone()).unwrap_or_default();

        // Slot rows (mutable cx).
        let mut slot_rows = Vec::new();
        for s in slots {
            slot_rows.push(self.render_slot_row(s, cx));
        }

        // Buttons.
        let theme = cx.theme();
        let refresh_btn = Button::new("piv-refresh")
            .icon(Icon::default().path("icons/refresh-cw.svg"))
            .custom(
                ButtonCustomVariant::new(cx)
                    .color(rgb(0x1b1b1d).into())
                    .hover(rgb(0x232325).into())
                    .active(rgb(0x3f3f46).into())
                    .border(theme.border),
            )
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)));
        let change_pin_btn = PFButton::new("修改 PIN")
            .id("piv-change-pin")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .on_click(cx.listener(|this, _, window, cx| this.open_change_pin(false, window, cx)));
        let change_puk_btn = PFButton::new("修改 PUK")
            .id("piv-change-puk")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .on_click(cx.listener(|this, _, window, cx| this.open_change_pin(true, window, cx)));
        let unblock_btn = PFButton::new("解锁 PIN")
            .id("piv-unblock")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .on_click(cx.listener(|this, _, window, cx| this.open_unblock_pin(window, cx)));
        let retries_btn = PFButton::new("设置重试次数")
            .id("piv-retries")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .on_click(cx.listener(|this, _, window, cx| this.open_set_retries(window, cx)));
        let mgm_btn = PFButton::new("修改密钥")
            .id("piv-mgm")
            .with_colors(rgb(0x222225), rgb(0x2a2a2d), rgb(0x333336))
            .on_click(cx.listener(|this, _, window, cx| this.open_change_mgm(window, cx)));
        let reset_btn = Button::new("piv-reset")
            .label("重置 PIV 模块")
            .danger()
            .disabled(self.loading)
            .on_click(cx.listener(|this, _, window, cx| this.open_reset_dialog(window, cx)));

        // Card information.
        let info_card = {
            let body = match &info {
                Some(i) => {
                    let pin = i
                        .pin
                        .map(|p| {
                            format!(
                                "{}/{}{}",
                                p.left,
                                p.total,
                                if p.is_default { "（默认）" } else { "" }
                            )
                        })
                        .unwrap_or_else(|| "—".into());
                    let puk = i
                        .puk
                        .map(|p| {
                            format!(
                                "{}/{}{}",
                                p.left,
                                p.total,
                                if p.is_default { "（默认）" } else { "" }
                            )
                        })
                        .unwrap_or_else(|| "—".into());
                    let mgm = format!(
                        "{}{}",
                        piv::algo_label(i.mgm_algo),
                        if i.mgm_default { "（默认）" } else { "" }
                    );
                    div()
                        .grid()
                        .grid_cols(2)
                        .gap_4()
                        .child(kv(
                            "固件",
                            format!("{}.{}.{}", i.version[0], i.version[1], i.version[2]),
                            theme,
                        ))
                        .child(kv("序列号", i.serial.to_string(), theme))
                        .child(kv("PIN 剩余尝试次数", pin, theme))
                        .child(kv("PUK 剩余尝试次数", puk, theme))
                        .child(kv("管理密钥", mgm, theme))
                        .into_any_element()
                }
                None => div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("正在读取智能卡…")
                    .into_any_element(),
            };
            Card::new()
                .title("智能卡信息")
                .description("PIV 智能卡状态")
                .icon(Icon::default().path("icons/cpu.svg"))
                .header_right(refresh_btn)
                .child(body)
        };

        let slots_card = Card::new()
            .title("密钥槽")
            .description("证书插槽 9A / 9C / 9D / 9E")
            .icon(Icon::default().path("icons/key.svg"))
            .child(v_flex().gap_2().children(slot_rows));

        let pin_card = Card::new()
            .title("PIN 与 PUK")
            .description("管理 PIV 的 PIN 与 PUK")
            .icon(Icon::default().path("icons/lock.svg"))
            .child(
                v_flex()
                    .gap_2()
                    .child(self.action_row(
                        "PIN",
                        "修改 6～8 位 PIV PIN",
                        change_pin_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        "PUK",
                        "修改 PIN 解锁密钥（PUK）",
                        change_puk_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        "解除锁定",
                        "使用 PUK 解锁被锁定的 PIN",
                        unblock_btn,
                        theme,
                    ))
                    .child(self.action_row(
                        "重试次数限制",
                        "设置 PIN/PUK 重试次数（同时重置两者的值）",
                        retries_btn,
                        theme,
                    )),
            );

        let mgm_card = Card::new()
            .title("管理密钥")
            .description("用于授权修改密钥和证书的密钥")
            .icon(Icon::default().path("icons/key-round.svg"))
            .child(self.action_row(
                "管理密钥",
                "修改 PIV 管理密钥",
                mgm_btn,
                theme,
            ));

        let reset_card = Card::new()
            .title("重置")
            .description("清除全部 PIV 密钥和证书")
            .icon(Icon::default().path("icons/trash.svg"))
            .child(self.action_row(
                "恢复 PIV 出厂设置",
                "将锁定 PIN 和 PUK，然后清除全部数据，此操作不可撤销。",
                reset_btn,
                theme,
            ));

        let content = v_flex()
            .gap_6()
            .child(info_card)
            .child(slots_card)
            .child(pin_card)
            .child(mgm_card)
            .child(reset_card);

        PageView::build(TITLE, SUBTITLE, content, theme).into_any_element()
    }
}
