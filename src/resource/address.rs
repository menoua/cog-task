use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ResourceAddr {
    Ref(PathBuf),
    Text(PathBuf),
    Image(PathBuf),
    Audio(PathBuf),
    Video(PathBuf),
    Stream(PathBuf),
}

impl ResourceAddr {
    #[inline]
    pub fn path(&self) -> &Path {
        match self {
            ResourceAddr::Ref(p) => p,
            ResourceAddr::Text(p) => p,
            ResourceAddr::Image(p) => p,
            ResourceAddr::Audio(p) => p,
            ResourceAddr::Video(p) => p,
            ResourceAddr::Stream(p) => p,
        }
    }

    #[inline]
    pub fn prefix(&self, parent: &Path) -> Self {
        match self {
            ResourceAddr::Ref(p) => ResourceAddr::Ref(parent.join(p)),
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
