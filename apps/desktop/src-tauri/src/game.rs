//! Launching TF2 for HUD testing, the remote console (Source RCON through `-usercon`) and the game's console log.
//!
//! * 64-bit (`tf_win64.exe`) is the default; 32-bit (`tf.exe`) is the legacy build. On Linux TF2 is 64-bit only
//!   and starts through Steam; macOS is no longer supported by TF2.
//! * "Test in itemtest" adds a borderless window at the chosen resolution, `+sv_lan 1 +map itemtest` and the
//!   usual quick-start switches.
//! * The remote console binds RCON to 127.0.0.1 on a free port with a password generated for that session, and
//!   `-condebug` makes the game write `tf/console.log`, which is streamed into the studio as `game-log` events.
use crate::settings::LaunchSettings;
use crate::StoreState;
use serde::Serialize;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, Manager, State};

#[derive(Default)]
pub struct Game {
    child: Option<Child>,
    rcon: Option<RconTarget>,
    conn: Option<Rcon>,
    log: Option<PathBuf>,
    tail_stop: Option<Arc<AtomicBool>>,
}
pub type GameState = Mutex<Game>;

#[derive(Clone)]
struct RconTarget {
    port: u16,
    password: String,
}

fn lock<T>(m: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, String> {
    m.lock().map_err(|_| "Internal state poisoned; restart HUD Studio".to_string())
}

// ───────────────────────────── arguments ─────────────────────────────

/// Switches added when testing in itemtest (a quick, quiet, windowed local game).
pub const ITEMTEST_SWITCHES: [&str; 8] =
    ["-window", "-noborder", "-noipx", "-nosteamcontroller", "-softparticlesdefaultoff", "-nohltv", "-nojoy", "-enablefakeip"];

/// Splits user-entered extra arguments, keeping "quoted values" together.
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut any = false;
    for ch in s.chars() {
        match ch {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(cur);
    }
    out
}

/// The game's command line (after the executable and `-game tf`).
pub fn build_args(s: &LaunchSettings, itemtest: bool, size: (u32, u32), rcon: Option<(&str, u16)>) -> Vec<String> {
    let mut a: Vec<String> = Vec::new();
    if itemtest {
        a.extend(["-window", "-noborder"].map(String::from));
        a.extend(["-w".into(), size.0.to_string(), "-h".into(), size.1.to_string()]);
        a.extend(ITEMTEST_SWITCHES[2..].iter().map(|s| s.to_string()));
    }
    if s.console_log {
        a.extend(["-condebug", "-conclearlog"].map(String::from));
    }
    if let Some((_, port)) = rcon {
        a.extend(["-usercon".into(), "-port".into(), port.to_string()]);
    }
    // group the user's options with their values (`-w 1280`, `+exec my.cfg`); switches go before the + commands
    let mut groups: Vec<Vec<String>> = Vec::new();
    for t in split_args(&s.extra_args) {
        match groups.last_mut() {
            Some(g) if !t.starts_with(['-', '+']) => g.push(t),
            _ => groups.push(vec![t]),
        }
    }
    let (plus, minus): (Vec<_>, Vec<_>) = groups.into_iter().partition(|g| g[0].starts_with('+'));
    a.extend(minus.into_iter().flatten());
    if let Some((pass, _)) = rcon {
        a.extend(["+ip", "127.0.0.1", "+rcon_password", pass].map(String::from));
        a.push("+net_start".into());
    }
    a.extend(plus.into_iter().flatten());
    if itemtest {
        a.extend(["+sv_lan", "1", "+map", "itemtest"].map(String::from));
    }
    a
}

/// Random password for one session's RCON (alphanumeric, so it survives any command line).
fn gen_password() -> String {
    use std::hash::{BuildHasher, Hasher};
    const ALPH: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let mut out = String::new();
    let mut i = 0u64;
    while out.len() < 24 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(nanos);
        h.write_u64(i);
        h.write_u32(std::process::id());
        for b in h.finish().to_le_bytes() {
            out.push(ALPH[b as usize % ALPH.len()] as char);
        }
        i += 1;
    }
    out.truncate(24);
    out
}

/// A port free for both TCP (RCON) and UDP (the game) on localhost.
fn free_port() -> Result<u16, String> {
    for _ in 0..20 {
        let l = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        let port = l.local_addr().map_err(|e| e.to_string())?.port();
        if port >= 1024 && UdpSocket::bind(("127.0.0.1", port)).is_ok() {
            return Ok(port);
        }
    }
    Err("No free local port for the remote console".into())
}

// ───────────────────────────── finding the game ─────────────────────────────

fn game_root(tf_dir: &Path) -> Result<PathBuf, String> {
    tf_dir.parent().map(Path::to_path_buf).ok_or_else(|| "Set your TF2 folder first".into())
}

#[cfg(windows)]
fn direct_exe(root: &Path, bits: &str) -> Result<PathBuf, String> {
    let names: &[&str] = if bits == "32" { &["tf.exe", "hl2.exe"] } else { &["tf_win64.exe"] };
    names.iter().map(|n| root.join(n)).find(|p| p.is_file()).ok_or_else(|| {
        if bits == "32" {
            format!("Couldn't find the 32-bit TF2 (tf.exe) in {}", root.display())
        } else {
            format!("Couldn't find the 64-bit TF2 (tf_win64.exe) in {}. Try the 32-bit (legacy) build, or verify the game files in Steam.", root.display())
        }
    })
}

/// How to start Steam with `-applaunch`.
fn steam_command() -> Option<Command> {
    #[cfg(windows)]
    {
        hud_core::project::steam_roots().into_iter().map(|r| r.join("steam.exe")).find(|p| p.is_file()).map(Command::new)
    }
    #[cfg(not(windows))]
    {
        let in_path = |n: &str| std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(n).is_file()));
        if in_path("steam") {
            return Some(Command::new("steam"));
        }
        let flatpak = Command::new("flatpak").args(["info", "com.valvesoftware.Steam"]).output().is_ok_and(|o| o.status.success());
        if flatpak {
            let mut c = Command::new("flatpak");
            c.args(["run", "com.valvesoftware.Steam"]);
            return Some(c);
        }
        None
    }
}

// ───────────────────────────── RCON ─────────────────────────────

const SERVERDATA_AUTH: i32 = 3;
const SERVERDATA_AUTH_RESPONSE: i32 = 2;
const SERVERDATA_EXECCOMMAND: i32 = 2;
const SERVERDATA_RESPONSE_VALUE: i32 = 0;

/// A Source RCON connection (the protocol srcds and `-usercon` clients speak).
pub struct Rcon {
    s: TcpStream,
    next: i32,
}

pub fn rcon_packet(id: i32, ty: i32, body: &str) -> Vec<u8> {
    let mut p = Vec::with_capacity(body.len() + 14);
    p.extend_from_slice(&((body.len() + 10) as i32).to_le_bytes());
    p.extend_from_slice(&id.to_le_bytes());
    p.extend_from_slice(&ty.to_le_bytes());
    p.extend_from_slice(body.as_bytes());
    p.extend_from_slice(&[0, 0]);
    p
}

impl Rcon {
    pub fn connect(port: u16, password: &str) -> Result<Rcon, String> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let s = TcpStream::connect_timeout(&addr, Duration::from_millis(1500))
            .map_err(|e| format!("Remote console not reachable yet ({e}). Is TF2 still loading?"))?;
        s.set_read_timeout(Some(Duration::from_secs(4))).ok();
        s.set_nodelay(true).ok();
        let mut r = Rcon { s, next: 10 };
        r.send(1, SERVERDATA_AUTH, password).map_err(|e| e.to_string())?;
        loop {
            let (id, ty, _) = r.read().map_err(|e| format!("No answer from TF2's remote console: {e}"))?;
            if ty == SERVERDATA_AUTH_RESPONSE {
                return if id == -1 { Err("TF2 rejected the remote console password. Relaunch TF2 from HUD Studio.".into()) } else { Ok(r) };
            }
        }
    }
    fn send(&mut self, id: i32, ty: i32, body: &str) -> io::Result<()> {
        self.s.write_all(&rcon_packet(id, ty, body))
    }
    fn read(&mut self) -> io::Result<(i32, i32, String)> {
        let mut len = [0u8; 4];
        self.s.read_exact(&mut len)?;
        let n = i32::from_le_bytes(len);
        if !(10..=1_000_000).contains(&n) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "bad RCON packet"));
        }
        let mut buf = vec![0u8; n as usize];
        self.s.read_exact(&mut buf)?;
        let id = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let ty = i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let body = String::from_utf8_lossy(&buf[8..buf.len() - 2]).to_string();
        Ok((id, ty, body))
    }
    /// Runs a console command and returns its output. A second, empty packet marks the end of the reply.
    pub fn exec(&mut self, cmd: &str) -> io::Result<String> {
        let id = self.next;
        self.next = self.next.wrapping_add(2).max(10);
        self.send(id, SERVERDATA_EXECCOMMAND, cmd)?;
        self.send(id + 1, SERVERDATA_RESPONSE_VALUE, "")?;
        let mut out = String::new();
        loop {
            match self.read() {
                Ok((rid, _, body)) if rid == id => out.push_str(&body),
                Ok((rid, _, _)) if rid == id + 1 => break,
                Ok(_) => {}
                Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) && !out.is_empty() => break,
                Err(e) => return Err(e),
            }
        }
        Ok(out)
    }
}

// ───────────────────────────── console.log ─────────────────────────────

fn start_tail(app: tauri::AppHandle, path: PathBuf, stop: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut pos = 0u64;
        let mut partial = String::new();
        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(300));
            let Ok(mut f) = std::fs::File::open(&path) else { continue };
            let len = f.metadata().map(|m| m.len()).unwrap_or(0);
            if len < pos {
                pos = 0; // -conclearlog emptied it
                partial.clear();
            }
            if len == pos || f.seek(SeekFrom::Start(pos)).is_err() {
                continue;
            }
            let mut buf = Vec::new();
            if f.take(2 * 1024 * 1024).read_to_end(&mut buf).is_err() {
                continue;
            }
            pos += buf.len() as u64;
            partial.push_str(&String::from_utf8_lossy(&buf));
            let mut lines: Vec<String> = partial.split('\n').map(|l| l.trim_end_matches('\r').to_string()).collect();
            partial = lines.pop().unwrap_or_default();
            lines.retain(|l| !l.is_empty());
            if !lines.is_empty() {
                let _ = app.emit("game-log", lines);
            }
        }
    });
}

// ───────────────────────────── commands ─────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchInfo {
    method: String,
    program: String,
    /// The command line, with the RCON password hidden.
    args: Vec<String>,
    rcon_port: Option<u16>,
    log_path: Option<String>,
    size: (u32, u32),
}

/// Primary display size in pixels.
#[tauri::command]
pub fn display_size(app: tauri::AppHandle) -> Result<(u32, u32), String> {
    let window = app.get_webview_window("main").ok_or("No window")?;
    let m = window.primary_monitor().map_err(|e| e.to_string())?.or(window.current_monitor().map_err(|e| e.to_string())?).ok_or("No display found")?;
    Ok((m.size().width, m.size().height))
}

#[tauri::command]
pub fn game_launch(
    app: tauri::AppHandle,
    bits: Option<String>,
    itemtest: Option<bool>,
    store: State<'_, StoreState>,
    game: State<'_, GameState>,
) -> Result<LaunchInfo, String> {
    if cfg!(target_os = "macos") {
        return Err("TF2 no longer runs on macOS: Valve ended Mac support in 2024.".into());
    }
    let (settings, tf_dir) = {
        let mut st = lock(&store)?;
        // first launch: take the display's size, once; the user can change it in Settings
        if st.settings.launch.width.is_none() || st.settings.launch.height.is_none() {
            if let Ok((w, h)) = display_size(app.clone()) {
                st.settings.launch.width = Some(w);
                st.settings.launch.height = Some(h);
                let _ = st.save();
            }
        }
        (st.settings.launch.clone(), st.settings.tf_dir.clone().ok_or("Set your TF2 folder in Settings first")?)
    };
    let tf_dir = PathBuf::from(tf_dir);
    #[cfg_attr(not(windows), allow(unused_variables))]
    let root = game_root(&tf_dir)?;
    let bits = bits.unwrap_or_else(|| settings.bits.clone());
    let itemtest = itemtest.unwrap_or(settings.itemtest);
    let size = (settings.width.unwrap_or(1920), settings.height.unwrap_or(1080));
    let rcon = if settings.rcon { Some(RconTarget { port: free_port()?, password: gen_password() }) } else { None };
    let args = build_args(&settings, itemtest, size, rcon.as_ref().map(|r| (r.password.as_str(), r.port)));

    let mut g = lock(&game)?;
    if let Some(child) = g.child.as_mut() {
        if child.try_wait().ok().flatten().is_none() {
            return Err("TF2 is already running from HUD Studio. Close it first, or use Reload HUD.".into());
        }
    }
    let use_steam = cfg!(not(windows)) || settings.method == "steam";
    if cfg!(not(windows)) && bits == "32" {
        return Err("TF2 on Linux is 64-bit only; the 32-bit (legacy) build is Windows-only.".into());
    }
    let (method, program, child) = if use_steam {
        let mut c = steam_command().ok_or("Couldn't find Steam to start TF2")?;
        let program = format!("{:?}", c.get_program());
        c.args(["-applaunch", "440"]).args(&args);
        c.spawn().map_err(|e| format!("Couldn't start Steam: {e}"))?;
        ("steam".to_string(), program, None)
    } else {
        #[cfg(windows)]
        {
            let exe = direct_exe(&root, &bits)?;
            let mut c = Command::new(&exe);
            c.current_dir(&root).env("SteamAppId", "440").env("SteamGameId", "440").args(["-game", "tf", "-steam"]).args(&args);
            let child = c.spawn().map_err(|e| format!("Couldn't start {}: {e}", exe.display()))?;
            ("direct".to_string(), exe.to_string_lossy().to_string(), Some(child))
        }
        #[cfg(not(windows))]
        {
            unreachable!()
        }
    };

    if let Some(stop) = g.tail_stop.take() {
        stop.store(true, Ordering::Relaxed);
    }
    let log = settings.console_log.then(|| tf_dir.join("console.log"));
    if let Some(p) = &log {
        let stop = Arc::new(AtomicBool::new(false));
        start_tail(app.clone(), p.clone(), stop.clone());
        g.tail_stop = Some(stop);
    }
    g.child = child;
    g.conn = None;
    g.log = log.clone();
    let rcon_port = rcon.as_ref().map(|r| r.port);
    let shown = args.iter().map(|a| if rcon.as_ref().is_some_and(|r| &r.password == a) { "••••••".to_string() } else { a.clone() }).collect();
    g.rcon = rcon;
    let _ = app.emit("game-state", "launched");
    Ok(LaunchInfo { method, program, args: shown, rcon_port, log_path: log.map(|p| p.to_string_lossy().to_string()), size })
}

/// Runs a console command in TF2 through the remote console.
#[tauri::command]
pub async fn game_rcon(command: String, game: State<'_, GameState>) -> Result<String, String> {
    let cmd = command.trim().to_string();
    if cmd.is_empty() {
        return Ok(String::new());
    }
    let mut g = lock(&game)?;
    let target = g.rcon.clone().ok_or("The remote console is off. Turn it on in Settings → Launching TF2, then launch TF2 from HUD Studio.")?;
    for attempt in 0..2 {
        if g.conn.is_none() {
            g.conn = Some(Rcon::connect(target.port, &target.password)?);
        }
        match g.conn.as_mut().unwrap().exec(&cmd) {
            Ok(out) => return Ok(out),
            Err(e) => {
                g.conn = None;
                if attempt == 1 {
                    return Err(format!("Lost the remote console: {e}"));
                }
            }
        }
    }
    unreachable!()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameStatus {
    /// The game process started from HUD Studio is still running (unknown when Steam started it).
    running: Option<bool>,
    rcon: bool,
    log_path: Option<String>,
}

#[tauri::command]
pub fn game_status(game: State<'_, GameState>) -> Result<GameStatus, String> {
    let mut g = lock(&game)?;
    let running = g.child.as_mut().map(|c| c.try_wait().ok().flatten().is_none());
    if running == Some(false) {
        g.child = None;
        g.conn = None;
        g.rcon = None;
        if let Some(stop) = g.tail_stop.take() {
            stop.store(true, Ordering::Relaxed);
        }
    }
    Ok(GameStatus { running, rcon: g.rcon.is_some(), log_path: g.log.as_ref().map(|p| p.to_string_lossy().to_string()) })
}

/// Forgets the current session (stops following console.log and drops the remote console).
#[tauri::command]
pub fn game_detach(game: State<'_, GameState>) -> Result<(), String> {
    let mut g = lock(&game)?;
    if let Some(stop) = g.tail_stop.take() {
        stop.store(true, Ordering::Relaxed);
    }
    g.conn = None;
    g.rcon = None;
    g.child = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> LaunchSettings {
        LaunchSettings { console_log: true, rcon: true, extra_args: "-novid +exec \"my hud.cfg\"".into(), ..Default::default() }
    }

    #[test]
    fn itemtest_command_line() {
        let a = build_args(&settings(), true, (2560, 1440), Some(("pw", 27123)));
        let s = a.join(" ");
        assert!(s.starts_with("-window -noborder -w 2560 -h 1440 -noipx -nosteamcontroller -softparticlesdefaultoff -nohltv -nojoy -enablefakeip"), "{s}");
        assert!(
            s.contains(
                "-condebug -conclearlog -usercon -port 27123 -novid +ip 127.0.0.1 +rcon_password pw +net_start +exec my hud.cfg +sv_lan 1 +map itemtest"
            ),
            "{s}"
        );
        assert!(a.contains(&"my hud.cfg".to_string()), "quoted extra args stay together");
        assert!(s.ends_with("+sv_lan 1 +map itemtest"));
    }

    #[test]
    fn plain_launch_has_no_window_switches() {
        let s = LaunchSettings { console_log: false, rcon: false, ..Default::default() };
        assert!(build_args(&s, false, (1920, 1080), None).is_empty());
    }

    #[test]
    fn passwords_and_packets() {
        let a = gen_password();
        assert_eq!(a.len(), 24);
        assert!(a.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_ne!(a, gen_password());
        let p = rcon_packet(7, SERVERDATA_EXECCOMMAND, "status");
        assert_eq!(i32::from_le_bytes([p[0], p[1], p[2], p[3]]) as usize, p.len() - 4);
        assert_eq!(&p[12..18], b"status");
    }

    #[test]
    fn rcon_against_a_fake_server() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let read = |s: &mut TcpStream| {
                let mut len = [0u8; 4];
                s.read_exact(&mut len).unwrap();
                let mut b = vec![0u8; i32::from_le_bytes(len) as usize];
                s.read_exact(&mut b).unwrap();
                (i32::from_le_bytes([b[0], b[1], b[2], b[3]]), String::from_utf8_lossy(&b[8..b.len() - 2]).to_string())
            };
            let (id, pass) = read(&mut s);
            s.write_all(&rcon_packet(id, SERVERDATA_RESPONSE_VALUE, "")).unwrap();
            s.write_all(&rcon_packet(if pass == "pw" { id } else { -1 }, SERVERDATA_AUTH_RESPONSE, "")).unwrap();
            let (cid, cmd) = read(&mut s);
            let (mid, _) = read(&mut s);
            s.write_all(&rcon_packet(cid, SERVERDATA_RESPONSE_VALUE, &format!("ran {cmd}\n"))).unwrap();
            s.write_all(&rcon_packet(mid, SERVERDATA_RESPONSE_VALUE, "")).unwrap();
        });
        let mut r = Rcon::connect(port, "pw").unwrap();
        assert_eq!(r.exec("hud_reloadscheme").unwrap(), "ran hud_reloadscheme\n");
    }

    #[test]
    fn splits_quoted_args() {
        assert_eq!(split_args(r#"-novid  +exec "a b.cfg" -x"#), vec!["-novid", "+exec", "a b.cfg", "-x"]);
        assert!(split_args("   ").is_empty());
    }
}
