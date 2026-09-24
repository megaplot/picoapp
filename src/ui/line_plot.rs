use std::sync::Arc;

use gpui_kit::component::plot::{
    scale::{Scale, ScaleLinear},
    shape::Line,
    AxisLabelSide, AxisText, Grid, IntoPlot, Plot, PlotAxis, StrokeStyle,
};
use gpui_kit::{px, App, Bounds, ContentMask, ElementId, Pixels, TextAlign, Window};

use crate::outputs::Plot as PlotData;
use crate::ui::plot_common::{format_tick, paint_panel, plot_colors};
use crate::ui::ticks::nice_ticks;

#[derive(IntoPlot)]
pub struct LinePlot {
    pub data: Arc<PlotData>,
}

/// Indices of the points to draw for a series drawn `buckets` pixels wide.
///
/// A line with far more samples than horizontal pixels (e.g. 22 050 audio
/// samples on a ~1200px plot) gains nothing from the extra points and is
/// expensive to tessellate — gpui's path renderer draws nothing at all for
/// very large polylines. So each bucket of consecutive samples keeps only
/// its minimum and maximum (in index order), which preserves the visual
/// envelope exactly. Series that are short enough, or whose `xs` are not
/// monotonic (a parametric curve, where index buckets would distort the
/// shape), are kept as they are.
pub fn decimate_min_max(xs: &[f64], ys: &[f64], buckets: usize) -> Vec<usize> {
    let n = xs.len().min(ys.len());
    let buckets = buckets.max(1);
    if n <= buckets * 4 || xs[..n].windows(2).any(|w| w[1] < w[0]) {
        return (0..n).collect();
    }

    let mut keep = Vec::with_capacity(buckets * 2 + 2);
    keep.push(0);
    for b in 0..buckets {
        let (start, end) = (b * n / buckets, (b + 1) * n / buckets);
        let (mut lo, mut hi) = (None::<usize>, None::<usize>);
        for i in start..end {
            if ys[i].is_nan() {
                continue;
            }
            if lo.map_or(true, |l| ys[i] < ys[l]) {
                lo = Some(i);
            }
            if hi.map_or(true, |h| ys[i] > ys[h]) {
                hi = Some(i);
            }
        }
        match (lo, hi) {
            (Some(l), Some(h)) if l == h => keep.push(l),
            (Some(l), Some(h)) => {
                keep.push(l.min(h));
                keep.push(l.max(h));
            }
            _ => {}
        }
    }
    keep.push(n - 1);
    keep.dedup();
    keep
}

impl Plot for LinePlot {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let colors = plot_colors();
        let area = paint_panel(bounds, window, cx);
        let width = area.size.width.as_f32();
        let height = area.size.height.as_f32();
        if width <= 0.0 || height <= 0.0 {
            return;
        }

        let (x0, x1) = (self.data.x_limits.start as f64, self.data.x_limits.end as f64);
        let (y0, y1) = (self.data.y_limits.start as f64, self.data.y_limits.end as f64);
        let x_scale = ScaleLinear::new(vec![x0, x1], vec![0.0, width]);
        // Pixel y grows downward; plot y grows upward.
        let y_scale = ScaleLinear::new(vec![y0, y1], vec![height, 0.0]);

        let x_ticks = nice_ticks(x0, x1, 8);
        let y_ticks = nice_ticks(y0, y1, 8);

        Grid::new()
            .x(x_ticks.iter().filter_map(|t| x_scale.tick(t)).map(px).collect::<Vec<_>>())
            .y(y_ticks.iter().filter_map(|t| y_scale.tick(t)).map(px).collect::<Vec<_>>())
            .stroke(colors.grid)
            .paint(&area, window);

        PlotAxis::new()
            .stroke(colors.axis)
            .x(px(height))
            .y(px(0.))
            .y_label_side(AxisLabelSide::Start)
            .x_label(x_ticks.iter().filter_map(|t| {
                x_scale.tick(t).map(|tick| {
                    AxisText::new(format_tick(*t), px(tick), colors.text)
                        .font_size(px(11.))
                        .align(TextAlign::Center)
                })
            }))
            .y_label(y_ticks.iter().filter_map(|t| {
                y_scale.tick(t).map(|tick| {
                    AxisText::new(format_tick(*t), px(tick), colors.text)
                        .font_size(px(11.))
                        .align(TextAlign::Right)
                })
            }))
            .paint(&area, window, cx);

        let data = self.data.clone();
        let indices = decimate_min_max(&data.xs, &data.ys, width.ceil() as usize);
        let line = Line::new()
            .data(indices)
            .x(move |&i| x_scale.tick(&data.xs[i]))
            .y({
                let data = self.data.clone();
                move |&i| y_scale.tick(&data.ys[i])
            })
            .stroke(colors.line)
            .stroke_style(StrokeStyle::Linear)
            .stroke_width(px(1.5));
        // Clip to the plot area: data outside x_limits/y_limits is cut off at
        // the frame instead of drawing over the tick labels.
        window.with_content_mask(Some(ContentMask { bounds: area }), |window| {
            line.paint(&area, window);
        });
    }

    fn id(&self) -> Option<ElementId> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(n: usize, f: impl Fn(f64) -> f64) -> (Vec<f64>, Vec<f64>) {
        let xs: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let ys = xs.iter().map(|&x| f(x)).collect();
        (xs, ys)
    }

    #[test]
    fn short_series_is_kept_whole() {
        let (xs, ys) = series(100, |x| x);
        assert_eq!(decimate_min_max(&xs, &ys, 50).len(), 100);
    }

    #[test]
    fn long_series_is_reduced_to_about_two_points_per_bucket() {
        let (xs, ys) = series(22_050, |x| (x * 0.9).sin());
        let kept = decimate_min_max(&xs, &ys, 1000);
        assert!(kept.len() <= 2 * 1000 + 2, "kept {}", kept.len());
        assert!(kept.len() > 1000);
    }

    #[test]
    fn decimation_keeps_endpoints_and_index_order() {
        let (xs, ys) = series(10_000, |x| (x * 0.01).sin());
        let kept = decimate_min_max(&xs, &ys, 100);
        assert_eq!(kept[0], 0);
        assert_eq!(*kept.last().unwrap(), 9_999);
        assert!(kept.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn decimation_preserves_a_single_narrow_spike() {
        let (xs, mut ys) = series(10_000, |_| 0.0);
        ys[4_321] = 5.0;
        let kept = decimate_min_max(&xs, &ys, 100);
        assert!(kept.contains(&4_321), "the spike is the bucket maximum");
    }

    #[test]
    fn non_monotonic_x_is_not_decimated() {
        let xs: Vec<f64> = (0..10_000).map(|i| ((i as f64) * 0.1).sin()).collect();
        let ys = xs.clone();
        assert_eq!(decimate_min_max(&xs, &ys, 100).len(), 10_000);
    }

    #[test]
    fn nan_values_do_not_panic() {
        let (xs, mut ys) = series(10_000, |x| x);
        ys[10] = f64::NAN;
        let _ = decimate_min_max(&xs, &ys, 100);
    }
}
