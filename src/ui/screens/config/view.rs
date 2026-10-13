use crate::ui::components::{card::Card, page_view::PageView};
use crate::ui::models::device::{
    DeviceMethod, FirmwareType, LedColor, LedStatus, USB_CAP_FIDO2, USB_CAP_OATH, USB_CAP_OPENPGP,
    USB_CAP_OTP, USB_CAP_PIV, USB_CAP_U2F,
};
use crate::ui::screens::config::view_model::ConfigViewModel;
use gpui::*;
use gpui_component::{button::*, input::*, select::*, slider::*, switch::*, *};

/// Per-status LED brightness is a full u8 on the device (0-255, 0 = off). The
/// +/- steppers move in coarse steps (15 divides 255 evenly) so the whole range
/// stays reachable without hundreds of clicks.
const LED_BRIGHTNESS_MAX: u8 = 255;
const LED_BRIGHTNESS_STEP: u8 = 15;

impl ConfigViewModel {
    fn render_identity_card(
        &self,
        theme: &Theme,
        is_fido: bool,
        hardware_config_disabled: bool,
    ) -> impl IntoElement {
        let content = v_flex()
            .gap_4()
            .child(
                v_flex().gap_2().child("厂商预设").child(
                    Select::new(&self.vendor_select)
                        .bg(rgb(0x222225))
                        .w_full()
                        .disabled(hardware_config_disabled),
                ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_4()
                    .child(
                        v_flex().gap_2().child("厂商 ID（十六进制）").child(
                            Input::new(&self.vid_input)
                                .font_family("Mono")
                                .bg(rgb(0x222225))
                                .disabled(hardware_config_disabled || !self.is_custom_vendor),
                        ),
                    )
                    .child(
                        v_flex().gap_2().child("产品 ID（十六进制）").child(
                            Input::new(&self.pid_input)
                                .font_family("Mono")
                                .bg(rgb(0x222225))
                                .disabled(hardware_config_disabled || !self.is_custom_vendor),
                        ),
                    ),
            )
            .child(div().h_px().bg(theme.border))
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_4()
                    .child(
                        v_flex().gap_2().child("产品名称").child(
                            Input::new(&self.product_name_input)
                                .bg(rgb(0x222225))
                                .disabled(is_fido),
                        ),
                    )
                    .child(
                        v_flex().gap_2().child("制造商").child(
                            Input::new(&self.manufacturer_input)
                                .bg(rgb(0x222225))
                                .disabled(is_fido),
                        ),
                    ),
            );

        Card::new()
            .title("设备标识")
            .description("USB 标识设置")
            .icon(Icon::default().path("icons/tag.svg"))
            .child(content)
    }

    fn render_led_card(
        &mut self,
        cx: &mut Context<Self>,
        is_fido: bool,
        is_rskey: bool,
        hardware_config_disabled: bool,
    ) -> impl IntoElement {
        // GPIO pin + driver are the LED hardware topology — always shown.
        let mut content = v_flex().gap_4().child(
            div()
                .grid()
                .grid_cols(2)
                .gap_4()
                .child(
                    v_flex().gap_2().child("LED GPIO 引脚").child(
                        Input::new(&self.led_gpio_input)
                            .bg(rgb(0x222225))
                            .disabled(hardware_config_disabled),
                    ),
                )
                .child(
                    v_flex().gap_2().child("LED 驱动").child(
                        Select::new(&self.led_driver_select)
                            .w_full()
                            .bg(rgb(0x222225))
                            .disabled(is_fido),
                    ),
                ),
        );

        // Colour order is an RS-Key extension (phy tag 0x0D); pico-fido ignores
        // it, so only surface it for RS-Key. Fixes red/green swap on GRB panels.
        if is_rskey {
            content = content.child(
                v_flex().gap_2().child("LED 颜色顺序").child(
                    Select::new(&self.led_order_select)
                        .w_full()
                        .bg(rgb(0x222225))
                        .disabled(hardware_config_disabled),
                ),
            );
        }

        // Global brightness / dimmable / steady live in the phy record. On RS-Key
        // the per-status EF_LED_CONF (Status LED Colors card) overrides them at
        // boot, so showing them here too would be duplicate, dead controls.
        if !is_rskey {
            let dim_listener = cx.listener(|this, checked, _, cx| {
                this.led_dimmable = *checked;
                cx.notify();
            });
            let steady_listener = cx.listener(|this, checked, _, cx| {
                this.led_steady = *checked;
                cx.notify();
            });
            let theme = cx.theme();
            let brightness = self.led_brightness_slider.read(cx).value().start() as i32;

            content = content
                .child(div().h_px().bg(theme.border))
                .child(
                    v_flex().gap_2().child("亮度（0–15）").child(
                        h_flex()
                            .items_center()
                            .gap_4()
                            .child(
                                Slider::new(&self.led_brightness_slider)
                                    .flex_1()
                                    .disabled(hardware_config_disabled),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child(format!("等级 {}", brightness)),
                            ),
                    ),
                )
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            v_flex().gap_0p5().child("LED 可调光").child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("允许调节亮度"),
                            ),
                        )
                        .child(
                            Switch::new("led-dimmable")
                                .checked(self.led_dimmable)
                                .disabled(hardware_config_disabled)
                                .on_click(dim_listener),
                        ),
                )
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(
                            v_flex().gap_0p5().child("LED 常亮模式").child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child("使 LED 持续点亮"),
                            ),
                        )
                        .child(
                            Switch::new("led-steady")
                                .checked(self.led_steady)
                                .disabled(hardware_config_disabled)
                                .on_click(steady_listener),
                        ),
                );
        }

        Card::new()
            .title("LED 设置")
            .description("调整灯光反馈方式")
            .icon(Icon::default().path("icons/microchip.svg"))
            .child(content)
    }

    fn render_touch_card(&self, _theme: &Theme, is_fido: bool) -> impl IntoElement {
        let content = v_flex().gap_4().child(
            v_flex().gap_2().child("触摸超时（秒）").child(
                Input::new(&self.touch_timeout_input)
                    .bg(rgb(0x222225))
                    .disabled(is_fido),
            ),
        );

        Card::new()
            .title("触摸与计时")
            .description("配置交互超时时间")
            .icon(Icon::default().path("icons/settings.svg"))
            .child(content)
    }

    fn render_options_card(
        &mut self,
        cx: &mut Context<Self>,
        hardware_config_disabled: bool,
    ) -> impl IntoElement {
        let power_cycle_listener = cx.listener(|this, checked, _, cx| {
            this.power_cycle = *checked;
            cx.notify();
        });

        let theme = cx.theme();

        let content = v_flex().gap_4().child(
            h_flex()
                .items_center()
                .justify_between()
                .child(
                    v_flex().gap_0p5().child("重置时重新上电").child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("重置时重新启动设备"),
                    ),
                )
                .child(
                    Switch::new("power-cycle")
                        .checked(self.power_cycle)
                        .disabled(hardware_config_disabled)
                        .on_click(power_cycle_listener),
                ),
        );

        Card::new()
            .title("设备选项")
            .description("启用或禁用高级功能")
            .icon(Icon::default().path("icons/settings.svg"))
            .child(content)
    }

    fn render_rskey_led_card(&mut self, cx: &mut Context<Self>, is_fido: bool) -> impl IntoElement {
        let theme = cx.theme();
        let mut rows = v_flex().gap_4();

        let steady_listener = cx.listener(|this, checked, _, cx| {
            this.led_status_steady = *checked;
            cx.notify();
        });

        rows = rows.child(
            h_flex()
                .items_center()
                .justify_between()
                .child(
                    v_flex().gap_0p5().child("全局常亮模式").child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("使状态 LED 持续点亮"),
                    ),
                )
                .child(
                    Switch::new("rskey-led-steady")
                        .checked(self.led_status_steady)
                        .disabled(is_fido)
                        .on_click(steady_listener),
                ),
        );

        rows = rows.child(div().h_px().bg(theme.border));

        for (i, status) in LedStatus::all().iter().enumerate() {
            let color_val = self.led_status_colors[i];
            let brightness_val = self.led_status_brightness[i];

            let cycle_color_listener = cx.listener(move |this, _, _, cx| {
                let mut c = this.led_status_colors[i];
                c = (c + 1) % LedColor::all().len() as u8;
                this.led_status_colors[i] = c;
                cx.notify();
            });

            let dec_bright_listener = cx.listener(move |this, _, _, cx| {
                let b = this.led_status_brightness[i];
                this.led_status_brightness[i] = b.saturating_sub(LED_BRIGHTNESS_STEP);
                cx.notify();
            });

            let inc_bright_listener = cx.listener(move |this, _, _, cx| {
                let b = this.led_status_brightness[i];
                this.led_status_brightness[i] = b.saturating_add(LED_BRIGHTNESS_STEP);
                cx.notify();
            });

            let color_name = LedColor::from_u8(color_val)
                .map(|c| c.label())
                .unwrap_or("未知");

            rows = rows.child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(div().w_24().child(status.label()))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                Button::new(gpui::SharedString::from(format!("color-btn-{}", i)))
                                    .child(color_name)
                                    .custom(
                                        ButtonCustomVariant::new(cx)
                                            .color(rgb(0x27272a).into())
                                            .hover(rgb(0x3f3f46).into())
                                            .active(rgb(0x52525b).into())
                                            .border(theme.border),
                                    )
                                    .disabled(is_fido)
                                    .on_click(cycle_color_listener),
                            )
                            .child(div().w_4())
                            .child(
                                Button::new(gpui::SharedString::from(format!("bdec-btn-{}", i)))
                                    .child("-")
                                    .custom(
                                        ButtonCustomVariant::new(cx)
                                            .color(rgb(0x1b1b1d).into())
                                            .hover(rgb(0x232325).into())
                                            .active(rgb(0x3f3f46).into())
                                            .border(theme.border),
                                    )
                                    .disabled(is_fido || brightness_val == 0)
                                    .on_click(dec_bright_listener),
                            )
                            .child(
                                div()
                                    .w_8()
                                    .flex()
                                    .justify_center()
                                    .child(brightness_val.to_string()),
                            )
                            .child(
                                Button::new(gpui::SharedString::from(format!("binc-btn-{}", i)))
                                    .child("+")
                                    .custom(
                                        ButtonCustomVariant::new(cx)
                                            .color(rgb(0x1b1b1d).into())
                                            .hover(rgb(0x232325).into())
                                            .active(rgb(0x3f3f46).into())
                                            .border(theme.border),
                                    )
                                    .disabled(is_fido || brightness_val == LED_BRIGHTNESS_MAX)
                                    .on_click(inc_bright_listener),
                            ),
                    ),
            );
        }

        Card::new()
            .title("状态 LED 颜色")
            .description("按设备状态配置 LED 颜色和亮度")
            .icon(Icon::default().path("icons/palette.svg"))
            .child(rows)
    }

    fn render_rskey_apps_card(
        &mut self,
        cx: &mut Context<Self>,
        is_fido: bool,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let mut rows = v_flex().gap_4();

        let apps = [
            ("FIDO2", USB_CAP_FIDO2),
            ("OATH", USB_CAP_OATH),
            ("PIV", USB_CAP_PIV),
            ("OpenPGP", USB_CAP_OPENPGP),
            ("U2F", USB_CAP_U2F),
            ("OTP", USB_CAP_OTP),
        ];

        for (name, cap) in apps {
            let is_supported = (self.usb_apps_supported & cap) != 0;
            let is_enabled = (self.usb_apps_enabled & cap) != 0;

            let toggle_listener = cx.listener(move |this, checked, _, cx| {
                if *checked {
                    this.usb_apps_enabled |= cap;
                } else {
                    this.usb_apps_enabled &= !cap;
                }
                cx.notify();
            });

            rows =
                rows.child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .child(v_flex().gap_0p5().child(name).child(
                            div().text_sm().text_color(theme.muted_foreground).child(
                                if is_supported {
                                    "支持"
                                } else {
                                    "固件不支持"
                                },
                            ),
                        ))
                        .child(
                            Switch::new(gpui::SharedString::from(format!("app-toggle-{}", cap)))
                                .checked(is_enabled)
                                .disabled(is_fido || !is_supported)
                                .on_click(toggle_listener),
                        ),
                );
        }

        Card::new()
            .title("USB 应用")
            .description("启用或禁用特定 USB 功能")
            .icon(Icon::default().path("icons/microchip.svg"))
            .child(rows)
    }

    fn render_rskey_usb_itf_card(
        &mut self,
        cx: &mut Context<Self>,
        is_fido: bool,
    ) -> impl IntoElement {
        let theme = cx.theme();
        // Only the interfaces the firmware actually instantiates (USB_ITF_SUPPORTED
        // = CCID | HID | KB). WCID (WebUSB) and LWIP are pico-fido concepts RS-Key
        // never builds, so toggling them would be a no-op — don't offer them.
        let mut rows = v_flex().gap_4().child(
            div()
                .text_sm()
                .text_color(rgb(0xf59e0b))
                .w_full()
                .max_w(px(800.0))
                .child("高级设置：关闭 HID 将禁用全部 FIDO2/U2F 功能；关闭 CCID 将禁用全部智能卡应用（包括救援模块）。固件会始终保留其中一种接口，避免设备失去访问能力。"),
        );

        let interfaces = [
            (
                "CCID（智能卡）",
                0x01u8,
                "救援模块和所有智能卡应用均需要此接口",
            ),
            (
                "HID（FIDO）",
                0x04u8,
                "FIDO/CTAP 通信接口——关闭后将禁用所有 FIDO2 和 U2F 功能",
            ),
            (
                "KB（键盘）",
                0x08u8,
                "OTP 键盘——用于输入 Yubico OTP 和静态密码",
            ),
        ];

        let current_mask = self.enabled_usb_itf.unwrap_or(0x1F);

        for (name, bit, desc) in interfaces {
            let is_enabled = (current_mask & bit) != 0;
            let is_ccid = bit == 0x01;

            let toggle_listener = cx.listener(move |this, checked, _, cx| {
                let mut mask = this.enabled_usb_itf.unwrap_or(0x1F);
                if *checked {
                    mask |= bit;
                } else {
                    mask &= !bit;
                }

                if bit == 0x01 {
                    mask |= 0x01;
                }

                this.enabled_usb_itf = Some(mask);
                cx.notify();
            });

            rows = rows.child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        v_flex().gap_0p5().child(name).child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .w_full()
                                .max_w(px(600.0))
                                .child(desc),
                        ),
                    )
                    .child(
                        Switch::new(gpui::SharedString::from(format!("usb-itf-toggle-{}", bit)))
                            .checked(is_enabled || is_ccid)
                            .disabled(is_fido || is_ccid)
                            .on_click(toggle_listener),
                    ),
            );
        }

        Card::new()
            .title("硬件接口")
            .description("启用或禁用底层 USB 接口")
            .icon(Icon::default().path("icons/cpu.svg"))
            .child(rows)
    }
}

impl Render for ConfigViewModel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_device = self.device.read(cx).status.is_some();

        if !has_device {
            let theme = cx.theme();
            return PageView::build(
                "配置",
                "自定义设备设置和工作方式。",
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .h_64()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_xl()
                    .child(div().text_color(theme.muted_foreground).child("未连接设备")),
                theme,
            );
        }

        let device = self.device.read(cx);
        let status = device.status.clone();
        let is_fido = status.as_ref().map(|s| s.method.clone()) == Some(DeviceMethod::Fido);
        let is_rskey = status.as_ref().map(|s| &s.firmware_type) == Some(&FirmwareType::RSKey);

        let supports_legacy_fido_config = status
            .as_ref()
            .map(ConfigViewModel::status_supports_legacy_fido_config)
            .unwrap_or(false);

        let hardware_config_disabled = is_fido && !supports_legacy_fido_config && !is_rskey;

        // RS-Key supports full config read/write over FIDO via CONFIG_READ/CONFIG_WRITE.
        // Other firmwares (pico-fido) don't: product name, LED driver, curves, etc.
        let is_fido_no_rskey = is_fido && !is_rskey;

        let led_card = self
            .render_led_card(cx, is_fido_no_rskey, is_rskey, hardware_config_disabled)
            .into_any_element();
        let options_card = self
            .render_options_card(cx, hardware_config_disabled)
            .into_any_element();

        let identity_card = self
            .render_identity_card(cx.theme(), is_fido_no_rskey, hardware_config_disabled)
            .into_any_element();
        let touch_card = self
            .render_touch_card(cx.theme(), is_fido_no_rskey)
            .into_any_element();

        let mut inner = v_flex().gap_6().w_full().child(identity_card);

        // RS-Key: put the functional config (which apps + transports are on)
        // right after Identity, before appearance/misc, so the panel reads
        // top-down by importance rather than burying it under the LED cards.
        // No curves card: the firmware ignores the phy ENABLED_CURVES tag
        // (curve support is compile-time), so exposing it would only mislead.
        if is_rskey {
            inner = inner
                .child(self.render_rskey_apps_card(cx, false))
                .child(self.render_rskey_usb_itf_card(cx, false));
        }

        inner = inner.child(led_card);

        if is_rskey {
            inner = inner.child(self.render_rskey_led_card(cx, false));
        }

        inner = inner.child(touch_card).child(options_card);

        inner = inner.child(
            h_flex().justify_end().pt_4().child(
                Button::new("apply-changes")
                    .icon(Icon::default().path("icons/save.svg"))
                    .child("应用更改")
                    .disabled(self.loading || hardware_config_disabled)
                    .custom(
                        ButtonCustomVariant::new(cx)
                            .color(rgb(0xe3e3e6).into())
                            .hover(rgb(0xcfcfd1).into())
                            .active(rgb(0xe3e3e6).into())
                            .foreground(rgb(0x4b4b4e).into()),
                    )
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.apply_changes(window, cx);
                    })),
            ),
        );

        let theme = cx.theme();
        PageView::build("配置", "自定义设备设置和工作方式。", inner, theme)
    }
}
