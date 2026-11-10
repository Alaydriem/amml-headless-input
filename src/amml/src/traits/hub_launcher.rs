use std::time::Duration;

use headless_input_mcp::HubRole;

// Starts and stops this instance's hub node for `HubControl`. The real one makes a `HubNode` (`NodeLauncher`); a test
// gives a fake. Both calls block and run on a blocking thread
pub trait HubLauncher: Send + Sync + 'static {
    type Node: Send + 'static;

    fn start(&self, port: u16) -> Self::Node;

    // returns within about `wait`, whether or not the node has ended by then
    fn stop(&self, node: Self::Node, wait: Duration);

    fn role(&self, node: &Self::Node) -> HubRole;
}
