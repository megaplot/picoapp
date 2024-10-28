use cushy::widget::MakeWidget;
use cushy::widget::WidgetList;

use crate::outputs::Output;

use super::ui_audio::audio_player_widget;
use super::ui_plots::plot_widget;

pub fn outputs_widget(outputs: &[Output]) -> impl MakeWidget {
    outputs
        .iter()
        .map(|output| match output {
            // TODO: Decide if things like contain()/expand() should be set by the
            // widgets themselves? I don't think we should set it on any output element
            // unconditionally, because some elements may have a fixed height.
            Output::Plot(plot) => plot_widget(&plot).contain().expand().make_widget(),
            Output::Audio(audio) => audio_player_widget(audio)
                .contain()
                // Why does horizontal alignment mess up vertical alignment?
                // .expand_horizontally()
                .make_widget(),
        })
        .collect::<WidgetList>()
        .into_rows()
        .expand()
}
