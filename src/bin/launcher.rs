use cog_task::launcher::Launcher;
use eyre::Result;

fn main() -> Result<()> {
    Launcher::default().run()
}
