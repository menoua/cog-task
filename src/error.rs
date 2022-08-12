use std::fmt::{Debug, Formatter};

#[derive(Clone)]
pub enum Error {
    ResourceLoadError(String),
    AudioDecodingError(String),
    VideoDecodingError(String),
    TriggerConfigError(String),
    InvalidResourceError(String),
    InvalidNameError(String),
    ActionEvolveError(String),
    ActionUpdateError(String),
    ActionViewError(String),
    IoAccessError(String),
    InternalError(String),
    TaskDefinitionError(String),
    InvalidConfigError(String),
    EnvironmentError(String),
    LoggerError(String),
    FlowError(String),
    GraphError(String),
    ChecksumError(String),
}

impl Debug for Error {
    // fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    //     match self {
    //         Error::ResourceLoadError(e) => write!(f, "ResourceLoadError ->\n{e}"),
    //         Error::AudioDecodingError(e) => write!(f, "AudioDecodingError ->\n{e}"),
    //         Error::VideoDecodingError(e) => {
    //             write!(
    //                 f,
    //                 "VideoDecodingError ->\n{e}\n\
    //                 --------------------------------------------------------------------\n\
    //                 If the video file isn't corrupted, this is most likely a problem with \
    //                 finding GStreamer and the necessary plugins on the system. Make sure \
    //                 that you have GStreamer and its plugins installed on your system. If \
    //                 you are still having problems, try setting the environment variable \
    //                 `GST_PLUGIN_PATH` to the location of your GStreamer plugins before \
    //                 launching this application, e.g., by running the following command:\n\
    //                 ```GST_PLUGIN_PATH=\"/usr/local/lib/gstreamer-1.0\" ./launcher```"
    //             )
    //         }
    //         Error::TriggerConfigError(e) => write!(f, "TriggerConfigError ->\n{e}"),
    //         Error::InvalidResourceError(e) => write!(f, "InvalidResourceError ->\n{e}"),
    //         Error::InvalidNameError(e) => write!(f, "InvalidNameError ->\n{e}"),
    //         Error::ActionEvolveError(e) => write!(f, "ActionEvolveError ->\n{e}"),
    //         Error::ActionUpdateError(e) => write!(f, "ActionUpdateError ->\n{e}"),
    //         Error::ActionViewError(e) => write!(f, "ActionViewError ->\n{e}"),
    //         Error::IoAccessError(e) => write!(f, "IoAccessError ->\n{e}"),
    //         Error::InternalError(e) => write!(f, "InternalError ->\n{e}"),
    //         Error::TaskDefinitionError(e) => write!(f, "TaskDefinitionError ->\n{e}"),
    //         Error::InvalidConfigError(e) => write!(f, "InvalidConfigError ->\n{e}"),
    //         Error::EnvironmentError(e) => write!(f, "EnvironmentError ->\n{e}"),
    //         Error::LoggerError(e) => write!(f, "LoggerError ->\n{e}"),
    //         Error::FlowError(e) => write!(f, "FlowError ->\n{e}"),
    //         Error::GraphError(e) => write!(f, "GraphError ->\n{e}"),
    //         Error::ChecksumError(e) => write!(f, "ChecksumError ->\n{e}"),
    //     }
    // }

    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ResourceLoadError(e)
            | Error::AudioDecodingError(e)
            | Error::TriggerConfigError(e)
            | Error::InvalidResourceError(e)
            | Error::InvalidNameError(e)
            | Error::ActionEvolveError(e)
            | Error::ActionUpdateError(e)
            | Error::ActionViewError(e)
            | Error::IoAccessError(e)
            | Error::InternalError(e)
            | Error::TaskDefinitionError(e)
            | Error::InvalidConfigError(e)
            | Error::EnvironmentError(e)
            | Error::LoggerError(e)
            | Error::FlowError(e)
            | Error::GraphError(e)
            | Error::ChecksumError(e) => write!(f, "{e}"),
            Error::VideoDecodingError(e) => {
                write!(
                    f,
                    "{e}\n\
                    --------------------------------------------------------------------\n\
                    If the video file isn't corrupted, this is most likely a problem with \
                    finding GStreamer and the necessary plugins on the system. Make sure \
                    that you have GStreamer and its plugins installed on your system. If \
                    you are still having problems, try setting the environment variable \
                    `GST_PLUGIN_PATH` to the location of your GStreamer plugins before \
                    launching this application, e.g., by running the following command:\n\
                    ```GST_PLUGIN_PATH=\"/usr/local/lib/gstreamer-1.0\" ./launcher```"
                )
            }
        }
    }
}

impl Error {
    pub fn type_(&self) -> &str {
        match self {
            Error::ResourceLoadError(_) => "ResourceLoadError",
            Error::AudioDecodingError(_) => "AudioDecodingError",
            Error::VideoDecodingError(_) => "VideoDecodingError",
            Error::TriggerConfigError(_) => "TriggerConfigError",
            Error::InvalidResourceError(_) => "InvalidResourceError",
            Error::InvalidNameError(_) => "InvalidNameError",
            Error::ActionEvolveError(_) => "ActionEvolveError",
            Error::ActionUpdateError(_) => "ActionUpdateError",
            Error::ActionViewError(_) => "ActionViewError",
            Error::IoAccessError(_) => "IoAccessError",
            Error::InternalError(_) => "InternalError",
            Error::TaskDefinitionError(_) => "TaskDefinitionError",
            Error::InvalidConfigError(_) => "InvalidConfigError",
            Error::EnvironmentError(_) => "EnvironmentError",
            Error::LoggerError(_) => "LoggerError",
            Error::FlowError(_) => "FlowError",
            Error::GraphError(_) => "GraphError",
            Error::ChecksumError(_) => "ChecksumError",
        }
    }
}
