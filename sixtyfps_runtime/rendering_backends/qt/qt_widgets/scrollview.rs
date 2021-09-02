/* LICENSE BEGIN
    This file is part of the SixtyFPS Project -- https://sixtyfps.io
    Copyright (c) 2021 Olivier Goffart <olivier.goffart@sixtyfps.io>
    Copyright (c) 2021 Simon Hausmann <simon.hausmann@sixtyfps.io>

    SPDX-License-Identifier: GPL-3.0-only
    This file is also available under commercial licensing terms.
    Please contact info@sixtyfps.io for more information.
LICENSE END */

use super::*;

#[repr(C)]
#[derive(FieldOffsets, Default, SixtyFPSElement)]
#[pin]
pub struct NativeScrollView {
    pub x: Property<f32>,
    pub y: Property<f32>,
    pub width: Property<f32>,
    pub height: Property<f32>,
    pub horizontal_max: Property<f32>,
    pub horizontal_page_size: Property<f32>,
    pub horizontal_value: Property<f32>,
    pub vertical_max: Property<f32>,
    pub vertical_page_size: Property<f32>,
    pub vertical_value: Property<f32>,
    pub cached_rendering_data: CachedRenderingData,
    pub native_padding_left: Property<f32>,
    pub native_padding_right: Property<f32>,
    pub native_padding_top: Property<f32>,
    pub native_padding_bottom: Property<f32>,
    data: Property<NativeSliderData>,
}

impl Item for NativeScrollView {
    fn init(self: Pin<&Self>, _window: &WindowRc) {
        let paddings = Rc::pin(Property::default());

        paddings.as_ref().set_binding(move || {
        cpp!(unsafe [] -> qttypes::QMargins as "QMargins" {
            ensure_initialized();
            QStyleOptionSlider option;
            initQSliderOptions(option, false, true, 0, 0, 1000, 1000);

            int extent = qApp->style()->pixelMetric(QStyle::PM_ScrollBarExtent, &option, nullptr);
            int sliderMin = qApp->style()->pixelMetric(QStyle::PM_ScrollBarSliderMin, &option, nullptr);
            auto horizontal_size = qApp->style()->sizeFromContents(QStyle::CT_ScrollBar, &option, QSize(extent * 2 + sliderMin, extent), nullptr);
            option.state ^= QStyle::State_Horizontal;
            option.orientation = Qt::Vertical;
            extent = qApp->style()->pixelMetric(QStyle::PM_ScrollBarExtent, &option, nullptr);
            sliderMin = qApp->style()->pixelMetric(QStyle::PM_ScrollBarSliderMin, &option, nullptr);
            auto vertical_size = qApp->style()->sizeFromContents(QStyle::CT_ScrollBar, &option, QSize(extent, extent * 2 + sliderMin), nullptr);

            /*int hscrollOverlap = hbar->style()->pixelMetric(QStyle::PM_ScrollView_ScrollBarOverlap, &opt, hbar);
                int vscrollOverlap = vbar->style()->pixelMetric(QStyle::PM_ScrollView_ScrollBarOverlap, &opt, vbar);*/

            QStyleOptionFrame frameOption;
            frameOption.rect = QRect(QPoint(), QSize(1000, 1000));
            frameOption.frameShape = QFrame::StyledPanel;
            frameOption.lineWidth = 1;
            frameOption.midLineWidth = 0;
            QRect cr = qApp->style()->subElementRect(QStyle::SE_ShapedFrameContents, &frameOption, nullptr);
            return {
                cr.left(),
                cr.top(),
                (vertical_size.width() + frameOption.rect.right() - cr.right()),
                (horizontal_size.height() + frameOption.rect.bottom() - cr.bottom())
                };
        })
    });

        self.native_padding_left.set_binding({
            let paddings = paddings.clone();
            move || paddings.as_ref().get().left as _
        });
        self.native_padding_right.set_binding({
            let paddings = paddings.clone();
            move || paddings.as_ref().get().right as _
        });
        self.native_padding_top.set_binding({
            let paddings = paddings.clone();
            move || paddings.as_ref().get().top as _
        });
        self.native_padding_bottom.set_binding({
            let paddings = paddings;
            move || paddings.as_ref().get().bottom as _
        });
    }

    fn geometry(self: Pin<&Self>) -> Rect {
        euclid::rect(self.x(), self.y(), self.width(), self.height())
    }

    fn layouting_info(
        self: Pin<&Self>,
        orientation: Orientation,
        _window: &WindowRc,
    ) -> LayoutInfo {
        LayoutInfo {
            min: match orientation {
                Orientation::Horizontal => self.native_padding_left() + self.native_padding_right(),
                Orientation::Vertical => self.native_padding_top() + self.native_padding_bottom(),
            },
            stretch: 1.,
            ..LayoutInfo::default()
        }
    }

    fn input_event_filter_before_children(
        self: Pin<&Self>,
        _: MouseEvent,
        _window: &WindowRc,
        _self_rc: &ItemRc,
    ) -> InputEventFilterResult {
        InputEventFilterResult::ForwardEvent
    }

    fn input_event(
        self: Pin<&Self>,
        event: MouseEvent,
        _window: &WindowRc,
        _self_rc: &sixtyfps_corelib::items::ItemRc,
    ) -> InputEventResult {
        let size: qttypes::QSize = get_size!(self);
        let mut data = self.data();
        let active_controls = data.active_controls;
        let pressed = data.pressed;
        let left = self.native_padding_left();
        let right = self.native_padding_right();
        let top = self.native_padding_top();
        let bottom = self.native_padding_bottom();

        let mut handle_scrollbar = |horizontal: bool,
                                    pos: qttypes::QPoint,
                                    size: qttypes::QSize,
                                    value_prop: Pin<&Property<f32>>,
                                    page_size: i32,
                                    max: i32| {
            let pressed: bool = data.pressed != 0;
            let value: i32 = value_prop.get() as i32;
            let new_control = cpp!(unsafe [
                pos as "QPoint",
                value as "int",
                page_size as "int",
                max as "int",
                size as "QSize",
                active_controls as "int",
                pressed as "bool",
                horizontal as "bool"
            ] -> u32 as "int" {
                ensure_initialized();
                QStyleOptionSlider option;
                initQSliderOptions(option, pressed, true, active_controls, 0, max, -value);
                option.pageStep = page_size;
                if (!horizontal) {
                    option.state ^= QStyle::State_Horizontal;
                    option.orientation = Qt::Vertical;
                }
                auto style = qApp->style();
                option.rect = { QPoint{}, size };
                return style->hitTestComplexControl(QStyle::CC_ScrollBar, &option, pos, nullptr);
            });

            #[allow(non_snake_case)]
            let SC_ScrollBarSlider =
                cpp!(unsafe []->u32 as "int" { return QStyle::SC_ScrollBarSlider;});

            let (pos, size) = if horizontal { (pos.x, size.width) } else { (pos.y, size.height) };

            let result = match event {
                MouseEvent::MousePressed { .. } => {
                    data.pressed = if horizontal { 1 } else { 2 };
                    if new_control == SC_ScrollBarSlider {
                        data.pressed_x = pos as f32;
                        data.pressed_val = -value as f32;
                    }
                    data.active_controls = new_control;
                    InputEventResult::GrabMouse
                }
                MouseEvent::MouseExit => {
                    data.pressed = 0;
                    InputEventResult::EventIgnored
                }
                MouseEvent::MouseReleased { .. } => {
                    data.pressed = 0;
                    let new_val = cpp!(unsafe [active_controls as "int", value as "int", max as "int", page_size as "int"] -> i32 as "int" {
                        switch (active_controls) {
                            case QStyle::SC_ScrollBarAddPage:
                                return -value + page_size;
                            case QStyle::SC_ScrollBarSubPage:
                                return -value - page_size;
                            case QStyle::SC_ScrollBarAddLine:
                                return -value + 3.;
                            case QStyle::SC_ScrollBarSubLine:
                                return -value - 3.;
                            case QStyle::SC_ScrollBarFirst:
                                return 0;
                            case QStyle::SC_ScrollBarLast:
                                return max;
                            default:
                                return -value;
                        }
                    });
                    value_prop.set(-(new_val.min(max).max(0) as f32));
                    InputEventResult::EventIgnored
                }
                MouseEvent::MouseMoved { .. } => {
                    if data.pressed != 0 && data.active_controls == SC_ScrollBarSlider {
                        let max = max as f32;
                        let new_val = data.pressed_val
                            + ((pos as f32) - data.pressed_x) * (max + (page_size as f32))
                                / size as f32;
                        value_prop.set(-new_val.min(max).max(0.));
                        InputEventResult::GrabMouse
                    } else {
                        InputEventResult::EventAccepted
                    }
                }
                MouseEvent::MouseWheel { .. } => {
                    // TODO
                    InputEventResult::EventAccepted
                }
            };
            self.data.set(data);
            result
        };

        let pos = event.pos().unwrap_or_default();

        if pressed == 2 || (pressed == 0 && pos.x > (size.width as f32 - right)) {
            handle_scrollbar(
                false,
                qttypes::QPoint {
                    x: (pos.x - (size.width as f32 - right)) as _,
                    y: (pos.y - top) as _,
                },
                qttypes::QSize {
                    width: (right - left) as _,
                    height: (size.height as f32 - (bottom + top)) as _,
                },
                Self::FIELD_OFFSETS.vertical_value.apply_pin(self),
                self.vertical_page_size() as i32,
                self.vertical_max() as i32,
            )
        } else if pressed == 1 || pos.y > (size.height as f32 - bottom) {
            handle_scrollbar(
                true,
                qttypes::QPoint {
                    x: (pos.x - left) as _,
                    y: (pos.y - (size.height as f32 - bottom)) as _,
                },
                qttypes::QSize {
                    width: (size.width as f32 - (right + left)) as _,
                    height: (bottom - top) as _,
                },
                Self::FIELD_OFFSETS.horizontal_value.apply_pin(self),
                self.horizontal_page_size() as i32,
                self.horizontal_max() as i32,
            )
        } else {
            Default::default()
        }
    }

    fn key_event(self: Pin<&Self>, _: &KeyEvent, _window: &WindowRc) -> KeyEventResult {
        KeyEventResult::EventIgnored
    }

    fn focus_event(self: Pin<&Self>, _: &FocusEvent, _window: &WindowRc) {}

    fn_render! { this dpr size painter =>

        let data = this.data();
        let left = this.native_padding_left();
        let right = this.native_padding_right();
        let top = this.native_padding_top();
        let bottom = this.native_padding_bottom();
        let corner_rect = qttypes::QRectF {
            x: (size.width as f32 / dpr - (right - left)) as _,
            y: (size.height as f32 / dpr - (bottom - top)) as _,
            width: (right - left) as _,
            height: (bottom - top) as _,
        };
        cpp!(unsafe [painter as "QPainter*", corner_rect as "QRectF"] {
            ensure_initialized();
            QStyleOptionFrame frameOption;
            frameOption.frameShape = QFrame::StyledPanel;
            frameOption.lineWidth = 1;
            frameOption.midLineWidth = 0;
            frameOption.rect = corner_rect.toAlignedRect();
            qApp->style()->drawPrimitive(QStyle::PE_PanelScrollAreaCorner, &frameOption, painter, nullptr);
            frameOption.rect = QRect(QPoint(), corner_rect.toAlignedRect().topLeft());
            qApp->style()->drawControl(QStyle::CE_ShapedFrame, &frameOption, painter, nullptr);
        });

        let draw_scrollbar = |horizontal: bool,
                              rect: qttypes::QRectF,
                              value: i32,
                              page_size: i32,
                              max: i32,
                              active_controls: u32,
                              pressed: bool| {
            cpp!(unsafe [
                painter as "QPainter*",
                value as "int",
                page_size as "int",
                max as "int",
                rect as "QRectF",
                active_controls as "int",
                pressed as "bool",
                dpr as "float",
                horizontal as "bool"
            ] {
                auto r = rect.toAlignedRect();
                // The mac style ignores painter translations (due to CGContextRef redirection) as well as
                // option.rect's top-left - hence this hack with an intermediate buffer.
            #if defined(Q_OS_MAC)
                QImage scrollbar_image(r.size(), QImage::Format_ARGB32_Premultiplied);
                scrollbar_image.fill(Qt::transparent);
                {QPainter p(&scrollbar_image); QPainter *painter = &p;
            #else
                painter->save();
                auto cleanup = qScopeGuard([&] { painter->restore(); });
                painter->translate(r.topLeft()); // There is bugs in the styles if the scrollbar is not in (0,0)
            #endif
                QStyleOptionSlider option;
                option.rect = QRect(QPoint(), r.size());
                initQSliderOptions(option, pressed, true, active_controls, 0, max / dpr, -value / dpr);
                option.subControls = QStyle::SC_All;
                option.pageStep = page_size / dpr;

                if (!horizontal) {
                    option.state ^= QStyle::State_Horizontal;
                    option.orientation = Qt::Vertical;
                }

                auto style = qApp->style();
                style->drawComplexControl(QStyle::CC_ScrollBar, &option, painter, nullptr);
            #if defined(Q_OS_MAC)
                }
                painter->drawImage(r.topLeft(), scrollbar_image);
            #endif
            });
        };

        draw_scrollbar(
            false,
            qttypes::QRectF {
                x: ((size.width as f32 - right + left) / dpr) as _,
                y: 0.,
                width: ((right - left) / dpr) as _,
                height: ((size.height as f32 - bottom + top) / dpr) as _,
            },
            this.vertical_value() as i32,
            this.vertical_page_size() as i32,
            this.vertical_max() as i32,
            data.active_controls,
            data.pressed == 2,
        );
        draw_scrollbar(
            true,
            qttypes::QRectF {
                x: 0.,
                y: ((size.height as f32 - bottom + top) / dpr) as _,
                width: ((size.width as f32 - right + left) / dpr) as _,
                height: ((bottom - top) / dpr) as _,
            },
            this.horizontal_value() as i32,
            this.horizontal_page_size() as i32,
            this.horizontal_max() as i32,
            data.active_controls,
            data.pressed == 1,
        );
    }
}

impl ItemConsts for NativeScrollView {
    const cached_rendering_data_offset: const_field_offset::FieldOffset<Self, CachedRenderingData> =
        Self::FIELD_OFFSETS.cached_rendering_data.as_unpinned_projection();
}

declare_item_vtable! {
fn sixtyfps_get_NativeScrollViewVTable() -> NativeScrollViewVTable for NativeScrollView
}
