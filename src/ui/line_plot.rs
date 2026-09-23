use gpui_kit::component::plot::{
    scale::{Scale, ScaleLinear},
    shape::Line,
    AxisText, Grid, IntoPlot, Plot, PlotAxis,
};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{px, App, Bounds, ElementId, Pixels, Window};

use crate::outputs::Plot as PlotData;
use crate::ui::ticks::nice_ticks;

const AXIS_GAP: f32 = 24.0;

#[derive(IntoPlot)]
pub struct LinePlot {
    pub data: PlotData,
}

impl Plot for LinePlot {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let width = bounds.size.width.as_f32();
        let height = bounds.size.height.as_f32() - AXIS_GAP;
        if width <= 0.0 || height <= 0.0 {
            return;
        }

        let x_scale = ScaleLinear::new(
            vec![self.data.x_limits.start as f64, self.data.x_limits.end as f64],
            vec![0.0, width],
        );
        let y_scale = ScaleLinear::new(
            vec![self.data.y_limits.start as f64, self.data.y_limits.end as f64],
            // Pixel y grows downward; plot y grows upward.
            vec![height, 0.0],
        );

        let x_ticks = nice_ticks(self.data.x_limits.start as f64, self.data.x_limits.end as f64, 6);
        let y_ticks = nice_ticks(self.data.y_limits.start as f64, self.data.y_limits.end as f64, 5);

        let border = cx.theme().border;
        let muted = cx.theme().muted_foreground;

        Grid::new()
            .y(y_ticks.iter().filter_map(|t| y_scale.tick(t)).map(px).collect::<Vec<_>>())
            .stroke(border)
            .dash_array(&[px(4.), px(2.)])
            .paint(&bounds, window);

        let mut axis = PlotAxis::new().stroke(border).x(px(height));
        axis = axis.x_label(x_ticks.iter().filter_map(|t| {
            x_scale.tick(t).map(|tick| AxisText::new(format!("{t:.3}").trim_end_matches('0').trim_end_matches('.').to_string(), px(tick), muted))
        }));
        axis.paint(&bounds, window, cx);

        let xs = self.data.xs.clone();
        let ys = self.data.ys.clone();
        let n = xs.len().min(ys.len());
        let line = Line::new()
            .data(0..n)
            .x(move |&i| x_scale.tick(&xs[i]))
            .y(move |&i| y_scale.tick(&ys[i]))
            .stroke(cx.theme().chart_2)
            .stroke_width(px(2.));
        line.paint(&bounds, window);
    }

    fn id(&self) -> Option<ElementId> {
        None
    }
}
