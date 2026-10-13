use crate::ui::components::page_view::PageView;
use crate::ui::screens::security::view_model::SecurityViewModel;
use gpui::*;
use gpui_component::{
    ActiveTheme, Disableable, Icon, StyledExt,
    button::{Button, ButtonCustomVariant, ButtonVariants},
    h_flex,
    switch::Switch,
    v_flex,
};

impl Render for SecurityViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let fg = theme.foreground;
        let muted_fg = theme.muted_foreground;
        let border = theme.border;
        let card_bg = theme.secondary;

        let destructive_red = rgb(0xef4444);
        let destructive_red_hover = rgb(0xdc2626);
        let destructive_red_active = rgb(0xb91c1c);
        let destructive_border = rgba(0xef44444d);
        let destructive_bg_muted = rgba(0xef44441a);

        let content = v_flex()
            .gap_6()
            .w_full()
            .child(
                v_flex()
                    .w_full()
                    .p_4()
                    .gap_2()
                    .border_1()
                    .border_color(destructive_border)
                    .bg(card_bg)
                    .rounded_md()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                Icon::default()
                                    .path("icons/triangle-alert.svg")
                                    .text_color(destructive_red),
                            )
                            .child(
                                div()
                                    .font_bold()
                                    .text_color(destructive_red)
                                    .child("功能尚不稳定"),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(destructive_red)
                            .child("此功能仍在开发中，为安全起见暂时禁用。"),
                    ),
            )
            .child(
                v_flex()
                    .w_full()
                    .border_1()
                    .border_color(destructive_border)
                    .bg(card_bg)
                    .rounded_xl()
                    .overflow_hidden()
                    .child(
                        div().p_6().child(
                            div()
                                .text_lg()
                                .font_bold()
                                .text_color(fg)
                                .child("锁定设置"),
                        ),
                    )
                    .child(
                        v_flex()
                            .px_6()
                            .pb_6()
                            .gap_6()
                            .child(
                                h_flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_medium()
                                                    .child("启用安全启动"),
                                            )
                                            .child(
                                                div().text_xs().text_color(muted_fg).child(
                                                    "启动时验证固件签名",
                                                ),
                                            ),
                                    )
                                    .child(
                                        Switch::new("secure-boot-switch")
                                            .checked(false)
                                            .disabled(true),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .justify_between()
                                    .items_center()
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(
                                                div().text_sm().font_medium().child("安全锁定"),
                                            )
                                            .child(div().text_xs().text_color(muted_fg).child(
                                                "防止通过调试端口读取密钥材料",
                                            )),
                                    )
                                    .child(
                                        Switch::new("secure-lock-switch")
                                            .checked(false)
                                            .disabled(true),
                                    ),
                            )
                            .child(div().h_px().bg(border))
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_4()
                                    .p_4()
                                    .rounded_md()
                                    .bg(destructive_bg_muted)
                                    .border_1()
                                    .border_color(destructive_border)
                                    .child(
                                        Switch::new("confirm-switch").checked(false).disabled(true),
                                    )
                                    .child(
                                        div()
                                            .font_medium()
                                            .text_color(destructive_red)
                                            .child("我了解此操作可能导致设备变砖。"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .border_t_1()
                            .border_color(border)
                            .bg(gpui::rgba(0x00000033))
                            .px_6()
                            .py_4()
                            .flex()
                            .justify_end()
                            .child(
                                Button::new("lock-device-btn")
                                    .custom(
                                        ButtonCustomVariant::new(cx)
                                            .color(destructive_red.into())
                                            .hover(destructive_red_hover.into())
                                            .active(destructive_red_active.into()),
                                    )
                                    .disabled(true)
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .items_center()
                                            .child(Icon::default().path("icons/lock.svg").size_4())
                                            .child("永久锁定设备"),
                                    ),
                            ),
                    ),
            );

        PageView::build(
            "安全启动",
            "将设备永久锁定到当前固件厂商。",
            content,
            theme,
        )
    }
}
