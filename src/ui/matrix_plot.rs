use gpui_kit::component::plot::{AxisText, IntoPlot, Plot, PlotAxis};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{fill, px, App, Bounds, ElementId, Pixels, Point, Window};

use crate::outputs::MatrixPlot as MatrixPlotData;
use crate::ui::color_utils::viridis;
use crate::ui::ticks::nice_ticks;

const AXIS_GAP: f32 = 24.0;

#[derive(IntoPlot)]
pub struct MatrixPlotView {
    pub data: MatrixPlotData,
}

impl Plot for MatrixPlotView {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let rows = self.data.num_rows.max(1) as f32;
        let cols = self.data.num_cols.max(1) as f32;
        let width = bounds.size.width.as_f32();
        let height = bounds.size.height.as_f32() - AXIS_GAP;
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let cell_w = width / cols;
        let cell_h = height / rows;

        let range = (self.data.max_value - self.data.min_value).max(f64::EPSILON);

        for (row_idx, row) in self.data.matrix.iter().enumerate() {
            for (col_idx, &value) in row.iter().enumerate() {
                let normalized = ((value - self.data.min_value) / range).clamp(0.0, 1.0);
                let color = viridis(normalized);
                let origin = bounds.origin
                    + Point::new(px(col_idx as f32 * cell_w), px(row_idx as f32 * cell_h));
                let cell_bounds = Bounds {
                    origin,
                    size: gpui_kit::size(px(cell_w), px(cell_h)),
                };
                window.paint_quad(fill(cell_bounds, color));
            }
        }

        let x_ticks = nice_ticks(0.0, cols as f64, 6);
        let y_ticks = nice_ticks(0.0, rows as f64, 6);
        let border = cx.theme().border;
        let muted = cx.theme().muted_foreground;
        let axis = PlotAxis::new()
            .stroke(border)
            .x(px(height))
            .y(px(0.))
            .x_label(
                x_ticks
                    .iter()
                    .map(|t| AxisText::new(format!("{t:.0}"), px(*t as f32 / cols * width), muted)),
            )
            .y_label(
                y_ticks
                    .iter()
                    .map(|t| AxisText::new(format!("{t:.0}"), px(*t as f32 / rows * height), muted)),
            );
        axis.paint(&bounds, window, cx);
    }

    fn id(&self) -> Option<ElementId> {
        None
    }
}
