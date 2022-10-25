use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResourceAddr {
    Text(PathBuf),
    Image(PathBuf),
    #[cfg(feature = "audio")]
    Audio(PathBuf),
    #[cfg(feature = "stream")]
    Video(PathBuf),
    #[cfg(feature = "stream")]
    Stream(PathBuf),
}

impl ResourceAddr {
    #[inline]
    pub fn path(&self) -> &Path {
        match self {
            ResourceAddr::Text(p)
            | ResourceAddr::Image(p)
            | ResourceAddr::Audio(p)
            | ResourceAddr::Video(p)
            | ResourceAddr::Stream(p) => p,
        }
    }

    #[inline]
    pub fn prefix(&self, parent: &Path) -> Self {
        match self {
            ResourceAddr::Text(p) => ResourceAddr::Text(parent.join(p)),
            ResourceAddr::Image(p) => ResourceAddr::Image(parent.join(p)),
            ResourceAddr::Audio(p) => ResourceAddr::Audio(parent.join(p)),
            ResourceAddr::Video(p) => ResourceAddr::Video(parent.join(p)),
            ResourceAddr::Stream(p) => ResourceAddr::Stream(parent.join(p)),
        }
    }

    pub fn extension(&self) -> Option<String> {
        self.path()
            .extension()
            .map(|ext| ext.to_str().unwrap().to_lowercase())
    }
}
