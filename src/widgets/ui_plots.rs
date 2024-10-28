use cushy::figures::units::Px;
use cushy::widget::{MakeWidget, Widget};
use cushy::widgets::Canvas;
use plotters::prelude::*;

use crate::outputs::Plot;

pub fn plot_widget(plot: &Plot) -> impl Widget {
    // TODO: Perhaps avoid cloning the data.
    let plot = plot.clone();
    Canvas::new({
        move |context| {
            render_plot(&plot, &context.gfx.as_plot_area()).unwrap();
        }
    })
    .width(Px::new(400)..)
    .height(Px::new(400)..)
}

fn render_plot<A>(
    plot: &Plot,
    root: &DrawingArea<A, plotters::coord::Shift>,
) -> Result<(), Box<dyn std::error::Error>>
where
    A: DrawingBackend,
    A::ErrorType: 'static,
{
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(plot.x_limits.clone(), plot.y_limits.clone())?;

    chart.configure_mesh().draw()?;

    chart.draw_series(LineSeries::new(
        plot.xs
            .iter()
            .zip(plot.ys.iter())
            .map(|(&x, &y)| (x as f32, y as f32)),
        &RED,
    ))?;

    Ok(())
}
