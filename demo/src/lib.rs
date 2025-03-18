use serde::{Deserialize, Serialize};
use serde_config_docs::ConfigDocs;

#[derive(Serialize, Deserialize, ConfigDocs)]
#[config_docs(export)]
struct Config {
    global: Global,
}

#[derive(Serialize, Deserialize, ConfigDocs)]
struct Global {
    /// By default, Streamlit displays a warning when a user sets both a widget
    /// default value in the function defining the widget and a widget value via
    /// the widget's key in `st.session_state`.
    /// If you'd like to turn off this warning, set this to True.
    #[serde(default = "_false", rename = "disableWidgetStateDuplicationWarning")]
    disable_widget_state_duplication_warning: bool,

    /// If True, will show a warning when you run a Streamlit-enabled script
    /// via "python my_script.py".
    #[serde(default = "_true", rename = "showWarningOnDirectExecution")]
    show_warning_on_direct_execution: bool,
}

fn _false() -> bool {
    false
}

fn _true() -> bool {
    true
}
