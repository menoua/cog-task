mod activity;
mod cleanup;
mod loading;
mod selection;
mod startup;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Startup,
    Selection,
    Loading,
    Activity,
    CleanUp,
}
