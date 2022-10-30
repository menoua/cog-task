pub mod action;
pub mod assets;
pub mod comm;
pub mod gui;
pub mod launcher;
pub mod resource;
pub mod server;
pub mod util;

#[cfg(all(feature = "audio", not(any(feature = "rodio"))))]
compile_error!("Cannot enable feature \"audio\" without a backend (\"rodio\").");

#[cfg(all(
    feature = "stream",
    not(any(feature = "gstreamer", feature = "ffmpeg"))
))]
compile_error!("Cannot enable feature \"stream\" without a backend (\"gstreamer\" or \"ffmpeg\").");

macro_rules! impl_verify_features {
    ($($feature:literal),* $(,)?) => {
        pub fn verify_features(content: &str) -> eyre::Result<()> {
            use eyre::eyre;
            use regex::Regex;

            let re = Regex::new(r"^//@[ \t]*([[:alpha:]][[:word:]]*)[ \t]*$").unwrap();
            let features: Vec<_> = content
                .lines()
                .map_while(|p| re.captures(p).map(|c| c[1].to_string()))
                .collect();

            for f in features {
                match f.as_str() {
                    $(
                    $feature => {
                        #[cfg(not(feature = $feature))]
                        Err(eyre!("Task requires missing feature ({}).", $feature))?;
                    }
                    )*
                    f => {
                        Err(eyre!("Task requires unknown feature: {f}"))?;
                    }
                }
            }

            Ok(())
        }
    };
}

impl_verify_features!(
    "rodio",
    "gstreamer",
    "ffmpeg",
    "savage",
    "python",
    "audio",
    "stream"
);
