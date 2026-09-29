use std::sync::Arc;

use gpui_kit::component::plot::{
    AxisLabelSide, IntoPlot, Plot, PlotAxis,
    scale::{Scale, ScaleLinear},
};
use gpui_kit::{App, Bounds, ElementId, Pixels, Point, TextAlign, Window, fill, px, size};

use crate::outputs::MatrixPlot as MatrixPlotData;
use crate::ui::color_utils::get_viridis_color;
use crate::ui::plot_common::{axis_label, paint_panel};
use crate::ui::style::plot_colors;
use crate::ui::ticks::nice_ticks;

#[derive(IntoPlot)]
pub struct MatrixPlotView {
    pub data: Arc<MatrixPlotData>,
}

impl Plot for MatrixPlotView {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let colors = plot_colors();
        let area = paint_panel(bounds, window);
        let rows = self.data.num_rows.max(1) as f32;
        let cols = self.data.num_cols.max(1) as f32;
        let width = area.size.width.as_f32();
        let height = area.size.height.as_f32();
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let cell_w = width / cols;
        let cell_h = height / rows;

        // Matches the old plotters cartesian chart: a constant matrix (no
        // range to normalize against) renders as the colormap's midpoint,
        // not its lowest value.
        let span = self.data.max_value - self.data.min_value;
        let normalize = |value: f64| {
            if span > 0.0 {
                ((value - self.data.min_value) / span).clamp(0.0, 1.0)
            } else {
                0.5
            }
        };

        // Pixel y grows downward; a plotters cartesian chart's y grows
        // upward, so row 0 is its *bottom* row (see line_plot.rs).
        let row_y = ScaleLinear::new(vec![0.0, rows as f64], [height, 0.0]);

        for (row_idx, row) in self.data.matrix.iter().enumerate() {
            let Some(y) = row_y.tick(&((row_idx + 1) as f64)) else {
                continue;
            };
            for (col_idx, &value) in row.iter().enumerate() {
                let origin = area.origin + Point::new(px(col_idx as f32 * cell_w), px(y));
                let cell_bounds = Bounds {
                    origin,
                    size: size(px(cell_w), px(cell_h)),
                };
                window.paint_quad(fill(cell_bounds, get_viridis_color(normalize(value))));
            }
        }

        let x_ticks = nice_ticks(0.0, cols as f64, 8);
        let y_ticks = nice_ticks(0.0, rows as f64, 8);
        PlotAxis::new()
            .stroke(colors.axis)
            .x(px(height))
            .y(px(0.))
            .y_label_side(AxisLabelSide::Start)
            .x_label(
                x_ticks.iter().map(|t| {
                    axis_label(*t, *t as f32 / cols * width, colors.text, TextAlign::Center)
                }),
            )
            .y_label(y_ticks.iter().filter_map(|t| {
                row_y
                    .tick(t)
                    .map(|y| axis_label(*t, y, colors.text, TextAlign::Right))
            }))
            .paint(&area, window, cx);
    }

    fn id(&self) -> Option<ElementId> {
        None
    }
}
