// Copyright 2022 The Druid Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use vello::kurbo::{Point, Size};
use vello::SceneBuilder;
use druid_shell::kurbo::{Point, Size};
use crate::geometry::Axis;
use crate::widget::{AccessCx, BoxConstraints};

use super::{
    align::{Center, SingleAlignment},
    contexts::LifeCycleCx,
    EventCx, LayoutCx, LifeCycle, PaintCx, Pod, RawEvent, UpdateCx, Widget,
};

pub struct LinearLayout {
    pub children: Vec<Pod>,
    pub spacing: f64,
    pub axis: Axis,
}

impl LinearLayout {
    pub fn new(children: Vec<Pod>, spacing: f64, axis: Axis) -> Self {
        let spacing = 0.0;
        LinearLayout {
            children,
            spacing,
            axis,
        }
    }
}

impl Widget for LinearLayout {
    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        for child in &mut self.children {
            child.event(cx, event);
        }
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        for child in &mut self.children {
            child.lifecycle(cx, event);
        }
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        for child in &mut self.children {
            child.update(cx);
        }
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: &BoxConstraints) -> Size {
        let child_bc = self.axis.with_major(*bc, 0.0..f64::infinity());
        let child_count = self.children.len();

        let mut major_used = 0.0;
        let mut max_minor = 0.0;

        for (index, child) in self.children.iter_mut().enumerate() {
            let size = child.layout(cx, &child_bc);
            child.set_origin(cx, self.axis.pack(major_used, 0.0));
            major_used += self.axis.major(size);
            if index < child_count - 1 {
                major_used += self.spacing;
            }
            max_minor = max_minor.max(self.axis.minor(size));
        }

        self.axis.pack(major_used, max_minor)
    }

    fn accessibility(&mut self, cx: &mut AccessCx) {
        for child in &mut self.children {
            child.accessibility(cx);
        }
    }

    fn paint(&mut self, cx: &mut PaintCx, builder: &mut SceneBuilder) {
        for child in &mut self.children {
            child.paint_into(cx, builder);
        }
    }
}
