use std::sync::Arc;

use gpui_kit::component::plot::{AxisLabelSide, AxisText, IntoPlot, Plot, PlotAxis};
use gpui_kit::{fill, px, App, Bounds, ElementId, Pixels, Point, TextAlign, Window, size};

use crate::outputs::MatrixPlot as MatrixPlotData;
use crate::ui::color_utils::viridis;
use crate::ui::plot_common::{format_tick, paint_panel};
use crate::ui::style::plot_colors;
use crate::ui::ticks::nice_ticks;

#[derive(IntoPlot)]
pub struct MatrixPlotView {
    pub data: Arc<MatrixPlotData>,
}

impl Plot for MatrixPlotView {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let colors = plot_colors();
        let area = paint_panel(bounds, window, cx);
        let rows = self.data.num_rows.max(1) as f32;
        let cols = self.data.num_cols.max(1) as f32;
        let width = area.size.width.as_f32();
        let height = area.size.height.as_f32();
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let cell_w = width / cols;
        let cell_h = height / rows;

        let range = (self.data.max_value - self.data.min_value).max(f64::EPSILON);

        for (row_idx, row) in self.data.matrix.iter().enumerate() {
            for (col_idx, &value) in row.iter().enumerate() {
                let normalized = ((value - self.data.min_value) / range).clamp(0.0, 1.0);
                let origin = area.origin
                    + Point::new(px(col_idx as f32 * cell_w), px(row_idx as f32 * cell_h));
                let cell_bounds = Bounds {
                    origin,
                    size: size(px(cell_w), px(cell_h)),
                };
                window.paint_quad(fill(cell_bounds, viridis(normalized)));
            }
        }

        let x_ticks = nice_ticks(0.0, cols as f64, 8);
        let y_ticks = nice_ticks(0.0, rows as f64, 8);
        PlotAxis::new()
            .stroke(colors.axis)
            .x(px(height))
            .y(px(0.))
            .y_label_side(AxisLabelSide::Start)
            .x_label(x_ticks.iter().map(|t| {
                AxisText::new(format_tick(*t), px(*t as f32 / cols * width), colors.text)
                    .font_size(px(11.))
                    .align(TextAlign::Center)
            }))
            .y_label(y_ticks.iter().map(|t| {
                AxisText::new(format_tick(*t), px(*t as f32 / rows * height), colors.text)
                    .font_size(px(11.))
                    .align(TextAlign::Right)
            }))
            .paint(&area, window, cx);
    }

    fn id(&self) -> Option<ElementId> {
        None
    }
}
