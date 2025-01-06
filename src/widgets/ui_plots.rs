use cushy::figures::units::Px;
use cushy::widget::{MakeWidget, Widget};
use cushy::widgets::Canvas;
use plotters::prelude::*;

use crate::outputs::{MatrixPlot, Plot};

use super::color_utils::get_viridis_color;

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

pub fn matrix_plot_widget(matrix_plot: &MatrixPlot) -> impl Widget {
    // TODO: Perhaps avoid cloning the data.
    let matrix_plot = matrix_plot.clone();
    Canvas::new({
        move |context| {
            render_matrix_plot(&matrix_plot, &context.gfx.as_plot_area()).unwrap();
        }
    })
    .width(Px::new(400)..)
    .height(Px::new(400)..)
}

fn render_matrix_plot<A>(
    matrix_plot: &MatrixPlot,
    root: &DrawingArea<A, plotters::coord::Shift>,
) -> Result<(), Box<dyn std::error::Error>>
where
    A: DrawingBackend,
    A::ErrorType: 'static,
{
    root.fill(&WHITE)?;

    // For an example, see:
    // https://github.com/plotters-rs/plotters/blob/master/plotters/examples/matshow.rs

    let mut chart = ChartBuilder::on(&root)
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0..matrix_plot.num_cols, 0..matrix_plot.num_rows)?;

    chart.configure_mesh().draw()?;

    let normalized = |v: f64| {
        if matrix_plot.max_value > matrix_plot.min_value {
            (v - matrix_plot.min_value) / (matrix_plot.max_value - matrix_plot.min_value)
        } else {
            0.5
        }
    };

    chart.draw_series(
        matrix_plot
            .matrix
            .iter()
            .zip(0..)
            .flat_map(|(l, y)| l.iter().zip(0..).map(move |(v, x)| (x, y, v)))
            .map(|(x, y, v)| {
                Rectangle::new(
                    [(x, y), (x + 1, y + 1)],
                    get_viridis_color(normalized(*v)).filled(),
                )
            }),
    )?;

    Ok(())
}
