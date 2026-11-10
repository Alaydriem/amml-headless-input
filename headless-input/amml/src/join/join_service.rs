use std::sync::Arc;
use std::time::{Duration, Instant};

use headless_input_core::structs::JoinTarget;

use crate::game::ScreenReader;
use crate::join::JoinClient;

// Asks the game to join a server by address or to leave the world, once for each request. Each frame passes the join
// and the leave its input frame asks for, then calls `pump` once; a request no frame renews ends at the next pump.
// Game thread.
//
// A join starts only from the title screen, with the primary client ready and not in a world: early in a leave the
// game is no longer in a world while the old world still saves, and a join started there hangs. A request is asked of
// the game once: when the game took it, it is not asked again while frames ask for it, even if the menus come back (the
// join failed; the step then fails). A new request asks again. The leave is asked once, off the title screen, and asked again only after the title showed or no
// frame asked for it. A toast on top counts as the screen the last read saw while a request is asked.
pub struct JoinService {
    // the join the frame asked for since the last pump
    asked_join: Option<Arc<JoinTarget>>,
    asked_leave: bool,
    // the join the game took
    joining: Option<Arc<JoinTarget>>,
    // the target the servers tab state belongs to; another target asks the tab again
    tab_target: Option<Arc<JoinTarget>>,
    // the game was asked to leave and the title has not shown since
    leaving: bool,
    // the game did not take the asked join and that was logged
    refusal_logged: bool,
    // the last read of this request showed the title
    title_seen: bool,
    // when the game took the join, and the last screen name logged since
    taken_at: Option<Instant>,
    logged_screen: String,
    // when the join may go on after the servers tab was asked for this request, and the frames since the call
    tab_ready_at: Option<Instant>,
    tab_failed_logged: bool,
    tab_frames: u32,
}

impl JoinService {
    // the wall-clock time the join waits after the servers tab was asked, so the tab has mounted
    pub const TAB_WAIT: Duration = Duration::from_millis(2500);
    // the frame count after the tab call at which the screen name is logged once
    const TAB_LOG_FRAME: u32 = 3;

    pub fn new() -> Self {
        Self {
            asked_join: None,
            asked_leave: false,
            joining: None,
            tab_target: None,
            leaving: false,
            refusal_logged: false,
            title_seen: false,
            taken_at: None,
            logged_screen: String::new(),
            tab_ready_at: None,
            tab_failed_logged: false,
            tab_frames: 0,
        }
    }

    pub fn request(&mut self, target: Arc<JoinTarget>) {
        self.asked_join = Some(target);
    }

    pub fn request_leave(&mut self) {
        self.asked_leave = true;
    }

    // once per frame, after the frame's requests. It calls the game only while a request is asked. Answers the result
    // code the game gave a taken join, once
    pub fn pump(&mut self, client: &impl JoinClient, now: Instant) -> Option<i32> {
        let join = self.asked_join.take();
        let leave = std::mem::take(&mut self.asked_leave);
        if join.is_none() {
            self.joining = None;
            self.tab_target = None;
            self.refusal_logged = false;
            self.tab_ready_at = None;
            self.tab_frames = 0;
        }
        if !leave {
            self.leaving = false;
        }
        if join.is_none() && !leave {
            self.title_seen = false;
            return None;
        }
        let name = client.top_screen_name();
        let title = if name == ScreenReader::TOAST {
            self.title_seen
        } else {
            ScreenReader::is_title(&name, client.is_in_game())
        };
        self.title_seen = title;
        if title {
            self.leaving = false;
        }
        if leave && !self.leaving && !title {
            curia::info!("asking the game to leave the world");
            client.request_leave_game();
            self.leaving = true;
        }
        join.and_then(|target| self.pump_join(client, target, title, &name, now))
    }

    fn pump_join(
        &mut self,
        client: &impl JoinClient,
        target: Arc<JoinTarget>,
        title: bool,
        name: &str,
        now: Instant,
    ) -> Option<i32> {
        if self.joining.as_ref().is_some_and(|joining| Arc::ptr_eq(joining, &target)) {
            self.log_screen(name);
            return client.take_join_result();
        }
        if !title || !client.is_primary_ready() || client.is_in_game() {
            return None;
        }
        if !self.tab_target.as_ref().is_some_and(|tab_target| Arc::ptr_eq(tab_target, &target)) {
            self.tab_ready_at = None;
            self.tab_frames = 0;
            self.refusal_logged = false;
            self.tab_target = Some(Arc::clone(&target));
        }
        if !self.tab_ready(client, name, now) {
            return None;
        }
        client.clear_join_result();
        if !client.join_server(&target) {
            if !self.refusal_logged {
                curia::warn!("the game did not take the join; it is asked again");
                self.refusal_logged = true;
            }
            return None;
        }
        curia::info!("the game took the join", { address: target.address.to_string(), port: target.port });
        self.refusal_logged = false;
        self.joining = Some(target);
        self.taken_at = Some(now);
        self.logged_screen.clear();
        None
    }

    // opens the servers tab once for the request, then waits `TAB_WAIT` of wall-clock time; true when the join may be
    // asked. A tab that did not open is logged once and the join goes on without a wait. The frames since the call
    // only time the screen log
    fn tab_ready(&mut self, client: &impl JoinClient, name: &str, now: Instant) -> bool {
        let Some(ready_at) = self.tab_ready_at else {
            self.tab_frames = 0;
            if client.open_servers_tab() {
                self.tab_ready_at = Some(now + Self::TAB_WAIT);
                return false;
            }
            self.tab_ready_at = Some(now);
            if !self.tab_failed_logged {
                curia::warn!("the servers tab did not open");
                self.tab_failed_logged = true;
            }
            return true;
        };
        self.tab_frames = self.tab_frames.saturating_add(1);
        if self.tab_frames == Self::TAB_LOG_FRAME {
            curia::info!("screen after the servers tab was asked", { screen: name.to_string() });
        }
        now >= ready_at
    }

    // logs each new screen name that shows while the taken join waits, with the milliseconds since the game took it
    fn log_screen(&mut self, name: &str) {
        if name == ScreenReader::TOAST || name == self.logged_screen {
            return;
        }
        self.logged_screen = name.to_string();
        let millis = self.taken_at.map_or(0, |at| u64::try_from(at.elapsed().as_millis()).unwrap_or(u64::MAX));
        curia::info!("screen after the join was taken", { screen: name.to_string(), millis: millis });
    }
}

impl Default for JoinService {
    fn default() -> Self {
        Self::new()
    }
}
